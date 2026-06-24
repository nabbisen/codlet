//! D1 implementation of [`SessionStore`] (RFC-033).

use serde::Deserialize;
use std::rc::Rc;

use codlet::hashing::LookupKey;
use codlet::secret::{SessionId, SubjectId};
use codlet::store::error::StoreError;
use codlet::store::session::{ActiveSessionRecord, SessionRecord, SessionStore};

use crate::d1::{bind, to_store_err, ts};
use crate::table_config::D1TableConfig;

/// D1-backed session store (RFC-033).
pub struct D1SessionStore {
    db: Rc<worker::d1::D1Database>,
    table: &'static str,
}

impl D1SessionStore {
    /// Construct from a D1 database handle and table config.
    /// Construct from a shared D1 database handle.
    ///
    /// Pass `Rc::clone(&db)` to share one handle across multiple stores:
    /// ```rust,ignore
    /// let db = std::rc::Rc::new(env.d1("DB")?);
    /// let store = D1SessionStore::new(std::rc::Rc::clone(&db), config);
    /// ```
    pub fn new(db: std::rc::Rc<worker::d1::D1Database>, config: D1TableConfig) -> Self {
        Self {
            db,
            table: config.sessions,
        }
    }
}

#[derive(Deserialize)]
struct ActiveRow {
    id: String,
    subject: String,
    expires_at: f64,
}

impl SessionStore for D1SessionStore {
    async fn find_active_session(
        &self,
        candidates: &[LookupKey],
        now: u64,
    ) -> Result<Option<ActiveSessionRecord>, StoreError> {
        use worker::d1::D1Type;
        for candidate in candidates {
            let sql = format!(
                "SELECT id, subject, expires_at FROM {t}
                 WHERE lookup_key = ? AND revoked_at IS NULL AND expires_at > ?
                 LIMIT 1",
                t = self.table
            );
            let stmt = bind(
                self.db.prepare(&sql),
                &[D1Type::Text(candidate.as_str()), ts(now)],
            )?;
            let row: Option<ActiveRow> = stmt.first(None).await.map_err(to_store_err)?;
            if let Some(r) = row {
                return Ok(Some(ActiveSessionRecord {
                    id: SessionId::new(r.id),
                    subject: SubjectId::new(r.subject),
                    expires_at: r.expires_at as u64,
                }));
            }
        }
        Ok(None)
    }

    async fn insert_session(&self, record: SessionRecord) -> Result<(), StoreError> {
        use worker::d1::D1Type;
        let sql = format!(
            "INSERT INTO {t} (id, lookup_key, key_version, subject, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            t = self.table
        );
        let stmt = bind(
            self.db.prepare(&sql),
            &[
                D1Type::Text(record.id.as_str()),
                D1Type::Text(record.lookup_key.as_str()),
                D1Type::Text(record.key_version.as_str()),
                D1Type::Text(record.subject.as_str()),
                ts(record.created_at),
                ts(record.expires_at),
            ],
        )?;
        stmt.run().await.map_err(to_store_err)?;
        Ok(())
    }

    async fn revoke_session(&self, session_id: &SessionId, now: u64) -> Result<(), StoreError> {
        use worker::d1::D1Type;
        let sql = format!(
            "UPDATE {t} SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
            t = self.table
        );
        let stmt = bind(
            self.db.prepare(&sql),
            &[ts(now), D1Type::Text(session_id.as_str())],
        )?;
        stmt.run().await.map_err(to_store_err)?;
        Ok(())
    }
}
