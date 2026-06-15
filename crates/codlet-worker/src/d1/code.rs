//! D1 implementation of [`CodeStore`] and [`CodeAdminStore`] (RFC-033).

use std::rc::Rc;

use serde::Deserialize;

use codlet_core::admin::{CodeAdminStore, CodeListFilter, CodeMeta};
use codlet_core::hashing::{KeyVersion, LookupKey};
use codlet_core::secret::{CodeId, SubjectId};
use codlet_core::state::{ClaimOutcome, classify_claim};
use codlet_core::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use codlet_core::store::error::StoreError;

use crate::d1::{bind, changes, to_store_err, ts};
use crate::table_config::D1TableConfig;

/// D1-backed one-time code store (RFC-033).
pub struct D1CodeStore {
    db: Rc<worker::d1::D1Database>,
    table: &'static str,
}

impl D1CodeStore {
    /// Construct from a D1 database handle and table config.
    ///
    /// Wraps `db` in `Rc` — D1Database is not `Clone`; Workers are single-threaded.
    /// Construct from a shared D1 database handle.
    ///
    /// Pass `Rc::clone(&db)` to share one handle across multiple stores:
    /// ```rust,ignore
    /// let db = std::rc::Rc::new(env.d1("DB")?);
    /// let store = D1CodeStore::new(std::rc::Rc::clone(&db), config);
    /// ```
    pub fn new(db: std::rc::Rc<worker::d1::D1Database>, config: D1TableConfig) -> Self {
        Self {
            db,
            table: config.codes,
        }
    }
}

#[derive(Deserialize)]
struct RedeemableRow {
    id: String,
    key_version: String,
    grant_payload: Option<String>,
    purpose: Option<String>,
    scope: Option<String>,
    expires_at: f64,
}

#[derive(Deserialize)]
struct AdminRow {
    id: String,
    key_version: String,
    purpose: Option<String>,
    scope: Option<String>,
    grant_payload: Option<String>,
    created_at: f64,
    expires_at: f64,
    used_at: Option<f64>,
    used_by_subject: Option<String>,
    revoked_at: Option<f64>,
}

fn admin_row_to_meta(r: AdminRow) -> CodeMeta {
    CodeMeta {
        id: CodeId::new(r.id),
        key_version: KeyVersion::new(r.key_version),
        purpose: r.purpose,
        scope: r.scope,
        grant: r.grant_payload,
        created_at: Some(r.created_at as u64),
        expires_at: r.expires_at as u64,
        used_at: r.used_at.map(|t| t as u64),
        used_by: r.used_by_subject.map(SubjectId::new),
        revoked_at: r.revoked_at.map(|t| t as u64),
    }
}

impl CodeStore for D1CodeStore {
    async fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        now: u64,
        scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        use worker::d1::D1Type;

        for candidate in candidates {
            let row: Option<RedeemableRow> = if let Some(s) = scope {
                let sql = format!(
                    "SELECT id, key_version, grant_payload, scope, expires_at
                     FROM {t}
                     WHERE lookup_key = ? AND scope = ?
                       AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ? LIMIT 1",
                    t = self.table
                );
                let stmt = bind(
                    self.db.prepare(&sql),
                    &[D1Type::Text(candidate.as_str()), D1Type::Text(s), ts(now)],
                )?;
                stmt.first(None).await.map_err(to_store_err)?
            } else {
                let sql = format!(
                    "SELECT id, key_version, grant_payload, scope, expires_at
                     FROM {t}
                     WHERE lookup_key = ?
                       AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ? LIMIT 1",
                    t = self.table
                );
                let stmt = bind(
                    self.db.prepare(&sql),
                    &[D1Type::Text(candidate.as_str()), ts(now)],
                )?;
                stmt.first(None).await.map_err(to_store_err)?
            };

            if let Some(r) = row {
                return Ok(Some(RedeemableCode {
                    id: CodeId::new(r.id),
                    key_version: KeyVersion::new(r.key_version),
                    grant: r.grant_payload,
                    purpose: r.purpose,
                    scope: r.scope,
                    expires_at: r.expires_at as u64,
                }));
            }
        }
        Ok(None)
    }

    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        use worker::d1::D1Type;

        // Atomic conditional UPDATE (INV-5, RFC-022).
        // purpose/scope enforced in WHERE to prevent cross-flow redemption (RFC-C).
        let mut sql = format!(
            "UPDATE {t} SET used_at = ?, used_by_subject = ?
             WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
               AND expires_at > ?",
            t = self.table
        );
        if let Some(p) = req.purpose {
            sql.push_str(&format!(" AND purpose = {p:?}"));
        }
        if let Some(s) = req.scope {
            sql.push_str(&format!(" AND scope = {s:?}"));
        }
        let stmt = bind(
            self.db.prepare(&sql),
            &[
                ts(req.now),
                D1Type::Text(req.subject.as_str()),
                D1Type::Text(req.code_id.as_str()),
                ts(req.now),
            ],
        )?;
        let result = stmt.run().await.map_err(to_store_err)?;
        let changed = changes(&result)?;
        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "claim_code changed {changed} rows for id={}",
                req.code_id.as_str()
            )));
        }
        Ok(classify_claim(changed))
    }

    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        use worker::d1::D1Type;

        let sql = format!(
            "INSERT INTO {t}
             (id, lookup_key, key_version, purpose, scope, grant_payload, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            t = self.table
        );
        // Build args — bind nullable fields as NULL when absent.
        let purpose_str;
        let scope_str;
        let grant_str;
        let purpose = match &record.purpose {
            Some(p) => {
                purpose_str = p.clone();
                D1Type::Text(&purpose_str)
            }
            None => D1Type::Null,
        };
        let scope = match &record.scope {
            Some(s) => {
                scope_str = s.clone();
                D1Type::Text(&scope_str)
            }
            None => D1Type::Null,
        };
        let grant = match &record.grant {
            Some(g) => {
                grant_str = g.clone();
                D1Type::Text(&grant_str)
            }
            None => D1Type::Null,
        };
        let stmt = bind(
            self.db.prepare(&sql),
            &[
                D1Type::Text(record.id.as_str()),
                D1Type::Text(record.lookup_key.as_str()),
                D1Type::Text(record.key_version.as_str()),
                purpose,
                scope,
                grant,
                ts(record.created_at),
                ts(record.expires_at),
            ],
        )?;
        stmt.run().await.map_err(to_store_err)?;
        Ok(())
    }

    async fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> Result<(), StoreError> {
        use worker::d1::D1Type;

        let stmt = if let Some(s) = scope {
            let sql = format!(
                "UPDATE {t}
                 SET revoked_at = ?
                 WHERE id = ? AND scope = ?
                   AND used_at IS NULL AND revoked_at IS NULL",
                t = self.table
            );
            bind(
                self.db.prepare(&sql),
                &[ts(now), D1Type::Text(code_id.as_str()), D1Type::Text(s)],
            )?
        } else {
            let sql = format!(
                "UPDATE {t}
                 SET revoked_at = ?
                 WHERE id = ?
                   AND used_at IS NULL AND revoked_at IS NULL",
                t = self.table
            );
            bind(
                self.db.prepare(&sql),
                &[ts(now), D1Type::Text(code_id.as_str())],
            )?
        };
        stmt.run().await.map_err(to_store_err)?;
        Ok(())
    }
}

impl CodeAdminStore for D1CodeStore {
    async fn list_codes(
        &self,
        filter: &CodeListFilter,
        now: u64,
    ) -> Result<Vec<CodeMeta>, StoreError> {
        use worker::d1::D1Type;

        let mut where_parts: Vec<&str> = Vec::new();
        let mut args: Vec<D1Type<'_>> = Vec::new();

        // Build filter args — must outlive the args Vec.
        let scope_str: String;
        if let Some(scope) = &filter.scope {
            where_parts.push("scope = ?");
            scope_str = scope.as_str().to_string();
            args.push(D1Type::Text(&scope_str));
        }
        if filter.active_only {
            where_parts.push("used_at IS NULL");
            where_parts.push("revoked_at IS NULL");
            where_parts.push("expires_at > ?");
            args.push(ts(now));
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let limit_clause = filter
            .limit
            .map(|n| format!("LIMIT {n}"))
            .unwrap_or_default();

        let sql = format!(
            "SELECT id, key_version, purpose, scope, grant_payload,
                    created_at, expires_at, used_at, used_by_subject, revoked_at
             FROM {t} {where_clause}
             ORDER BY expires_at DESC {limit_clause}",
            t = self.table,
        );
        let stmt = bind(self.db.prepare(&sql), &args)?;
        let result = stmt.all().await.map_err(to_store_err)?;
        let rows: Vec<AdminRow> = result.results().map_err(to_store_err)?;
        Ok(rows.into_iter().map(admin_row_to_meta).collect())
    }

    async fn get_code_meta(&self, code_id: &CodeId) -> Result<Option<CodeMeta>, StoreError> {
        use worker::d1::D1Type;

        let sql = format!(
            "SELECT id, key_version, purpose, scope, grant_payload,
                    created_at, expires_at, used_at, used_by_subject, revoked_at
             FROM {t}
             WHERE id = ? LIMIT 1",
            t = self.table
        );
        let stmt = bind(self.db.prepare(&sql), &[D1Type::Text(code_id.as_str())])?;
        let row: Option<AdminRow> = stmt.first(None).await.map_err(to_store_err)?;
        Ok(row.map(admin_row_to_meta))
    }
}
