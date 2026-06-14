//! PostgreSQL implementation of [`SessionStore`] (RFC-034).

use codlet_core::hashing::LookupKey;
use codlet_core::secret::{SessionId, SubjectId};
use codlet_core::store::error::StoreError;
use codlet_core::store::session::{ActiveSessionRecord, SessionRecord, SessionStore};

use super::PostgresStore;

fn to_err(e: sqlx::Error) -> StoreError {
    StoreError::Backend(e.to_string())
}

// (id, subject, expires_at)
type ActiveRow = (String, String, i64);

impl SessionStore for PostgresStore {
    async fn find_active_session(
        &self,
        candidates: &[LookupKey],
        now: u64,
    ) -> Result<Option<ActiveSessionRecord>, StoreError> {
        let now_i = now as i64;
        for candidate in candidates {
            let row: Option<ActiveRow> = sqlx::query_as(
                "SELECT id, subject, expires_at
                 FROM codlet_sessions
                 WHERE lookup_key = $1
                   AND revoked_at IS NULL
                   AND expires_at  > $2
                 LIMIT 1",
            )
            .bind(candidate.as_str())
            .bind(now_i)
            .fetch_optional(&self.pool)
            .await
            .map_err(to_err)?;

            if let Some((id, subject, expires_at)) = row {
                return Ok(Some(ActiveSessionRecord {
                    id: SessionId::new(id),
                    subject: SubjectId::new(subject),
                    expires_at: expires_at as u64,
                }));
            }
        }
        Ok(None)
    }

    async fn insert_session(&self, record: SessionRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO codlet_sessions
             (id, lookup_key, key_version, subject, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(record.id.as_str())
        .bind(record.lookup_key.as_str())
        .bind(record.key_version.as_str())
        .bind(record.subject.as_str())
        .bind(record.created_at as i64)
        .bind(record.expires_at as i64)
        .execute(&self.pool)
        .await
        .map_err(to_err)?;
        Ok(())
    }

    async fn revoke_session(&self, session_id: &SessionId, now: u64) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE codlet_sessions
             SET revoked_at = $1
             WHERE id = $2 AND revoked_at IS NULL",
        )
        .bind(now as i64)
        .bind(session_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(to_err)?;
        Ok(())
    }
}

/// Convenience alias.
pub type PostgresSessionStore = PostgresStore;
