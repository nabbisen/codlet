//! PostgreSQL implementation of [`CodeStore`] and [`CodeAdminStore`] (RFC-034).

use codlet::admin::{CodeAdminStore, CodeListFilter, CodeMeta};
use codlet::hashing::{KeyVersion, LookupKey};
use codlet::secret::{CodeId, SubjectId};
use codlet::state::{ClaimOutcome, classify_claim};
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use codlet::store::error::StoreError;

use super::PostgresStore;

fn to_err(e: sqlx::Error) -> StoreError {
    StoreError::Backend(e.to_string())
}

// Tuple type for the redeemable SELECT.
// (id, key_version, purpose, grant_payload, scope, expires_at, used_at, revoked_at)
type RedeemableRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    Option<i64>,
    Option<i64>,
);

// Tuple type for admin SELECT.
// (id, key_version, purpose, scope, grant_payload,
//  created_at, expires_at, used_at, used_by_subject, revoked_at)
type AdminRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    i64,
    Option<i64>,
    Option<String>,
    Option<i64>,
);

fn admin_row_to_meta(r: AdminRow) -> CodeMeta {
    let (id, kv, purpose, scope, grant, created_at, expires_at, used_at, used_by, revoked_at) = r;
    CodeMeta {
        id: CodeId::new(id),
        key_version: KeyVersion::new(kv),
        purpose,
        scope,
        grant,
        created_at: Some(created_at as u64),
        expires_at: expires_at as u64,
        used_at: used_at.map(|t| t as u64),
        used_by: used_by.map(SubjectId::new),
        revoked_at: revoked_at.map(|t| t as u64),
    }
}

impl CodeStore for PostgresStore {
    async fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        _now: u64,
        scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        // RFC-047: matches on lookup key and scope only. No expiry/revocation/
        // use predicate here -- `classify_code_lookup` decides that from the
        // returned state fields; `claim_code`'s conditional UPDATE remains
        // the actual enforcement point (INV-5).
        for candidate in candidates {
            let row: Option<RedeemableRow> = if let Some(s) = scope {
                sqlx::query_as(
                    "SELECT id, key_version, purpose, grant_payload, scope, expires_at,
                            used_at, revoked_at
                     FROM codlet_codes
                     WHERE lookup_key = $1 AND scope = $2
                     LIMIT 1",
                )
                .bind(candidate.as_str())
                .bind(s)
                .fetch_optional(&self.pool)
                .await
                .map_err(to_err)?
            } else {
                sqlx::query_as(
                    "SELECT id, key_version, purpose, grant_payload, scope, expires_at,
                            used_at, revoked_at
                     FROM codlet_codes
                     WHERE lookup_key = $1
                     LIMIT 1",
                )
                .bind(candidate.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(to_err)?
            };

            if let Some((id, kv, purpose_val, grant, scope_val, exp, used_at, revoked_at)) = row {
                return Ok(Some(RedeemableCode {
                    id: CodeId::new(id),
                    key_version: KeyVersion::new(kv),
                    grant,
                    purpose: purpose_val,
                    scope: scope_val,
                    expires_at: exp as u64,
                    used_at: used_at.map(|t| t as u64),
                    revoked_at: revoked_at.map(|t| t as u64),
                }));
            }
        }
        Ok(None)
    }

    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        // Conditional UPDATE — READ COMMITTED row-level lock (RFC-034 §7, INV-5).
        // RETURNING is not used; RFC-034 §7 documents the decision.
        // purpose/scope are enforced in the WHERE clause to prevent cross-flow
        // redemption (RFC-C). A fixed set of complete, constant SQL strings —
        // `purpose`/`scope` are always bound as `$N` parameters, never
        // interpolated (RFC-048). `AssertSqlSafe` is not needed: each literal
        // is `&'static str`, with no dynamically-assembled fragment to assert
        // about.
        let now = req.now as i64;
        let id = req.code_id.as_str();
        let subject = req.subject.as_str();
        let result = match (req.purpose, req.scope) {
            (Some(p), Some(s)) => {
                sqlx::query(
                    "UPDATE codlet_codes SET used_at = $1, used_by_subject = $2
                     WHERE id = $3 AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > $4 AND purpose = $5 AND scope = $6",
                )
                .bind(now)
                .bind(subject)
                .bind(id)
                .bind(now)
                .bind(p)
                .bind(s)
                .execute(&self.pool)
                .await
            }
            (Some(p), None) => {
                sqlx::query(
                    "UPDATE codlet_codes SET used_at = $1, used_by_subject = $2
                     WHERE id = $3 AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > $4 AND purpose = $5",
                )
                .bind(now)
                .bind(subject)
                .bind(id)
                .bind(now)
                .bind(p)
                .execute(&self.pool)
                .await
            }
            (None, Some(s)) => {
                sqlx::query(
                    "UPDATE codlet_codes SET used_at = $1, used_by_subject = $2
                     WHERE id = $3 AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > $4 AND scope = $5",
                )
                .bind(now)
                .bind(subject)
                .bind(id)
                .bind(now)
                .bind(s)
                .execute(&self.pool)
                .await
            }
            (None, None) => {
                sqlx::query(
                    "UPDATE codlet_codes SET used_at = $1, used_by_subject = $2
                     WHERE id = $3 AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > $4",
                )
                .bind(now)
                .bind(subject)
                .bind(id)
                .bind(now)
                .execute(&self.pool)
                .await
            }
        }
        .map_err(to_err)?;

        let changed = result.rows_affected() as usize;
        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "claim_code changed {changed} rows for id={}",
                req.code_id.as_str()
            )));
        }
        Ok(classify_claim(changed))
    }

    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO codlet_codes
             (id, lookup_key, key_version, purpose, scope, grant_payload, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(record.id.as_str())
        .bind(record.lookup_key.as_str())
        .bind(record.key_version.as_str())
        .bind(record.purpose.as_deref())
        .bind(record.scope.as_deref())
        .bind(record.grant.as_deref())
        .bind(record.created_at as i64)
        .bind(record.expires_at as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().to_lowercase().contains("unique")
                || e.to_string().to_lowercase().contains("duplicate")
            {
                StoreError::Backend("duplicate lookup key (unique constraint)".into())
            } else {
                to_err(e)
            }
        })?;
        Ok(())
    }

    async fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> Result<(), StoreError> {
        if let Some(s) = scope {
            sqlx::query(
                "UPDATE codlet_codes
                 SET revoked_at = $1
                 WHERE id = $2 AND scope = $3
                   AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(now as i64)
            .bind(code_id.as_str())
            .bind(s)
            .execute(&self.pool)
            .await
            .map_err(to_err)?;
        } else {
            sqlx::query(
                "UPDATE codlet_codes
                 SET revoked_at = $1
                 WHERE id = $2
                   AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(now as i64)
            .bind(code_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(to_err)?;
        }
        Ok(())
    }
}

impl CodeAdminStore for PostgresStore {
    async fn list_codes(
        &self,
        filter: &CodeListFilter,
        now: u64,
    ) -> Result<Vec<CodeMeta>, StoreError> {
        let mut where_parts: Vec<String> = Vec::new();
        let mut param_idx: u32 = 1;
        let mut scope_val: Option<String> = None;
        let mut now_val: Option<i64> = None;

        if let Some(scope) = &filter.scope {
            where_parts.push(format!("scope = ${param_idx}"));
            scope_val = Some(scope.as_str().to_string());
            param_idx += 1;
        }
        if filter.active_only {
            where_parts.push("used_at IS NULL".into());
            where_parts.push("revoked_at IS NULL".into());
            where_parts.push(format!("expires_at > ${param_idx}"));
            now_val = Some(now as i64);
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
             FROM codlet_codes
             {where_clause}
             ORDER BY expires_at DESC
             {limit_clause}"
        );

        // Safety: `sql` is built from constant strings and $N placeholders only.
        // No user input is interpolated — values go through .bind().
        let mut query = sqlx::query_as::<_, AdminRow>(sqlx::AssertSqlSafe(sql.as_str()));
        if let Some(s) = &scope_val {
            query = query.bind(s.as_str());
        }
        if let Some(n) = now_val {
            query = query.bind(n);
        }

        let rows = query.fetch_all(&self.pool).await.map_err(to_err)?;
        Ok(rows.into_iter().map(admin_row_to_meta).collect())
    }

    async fn get_code_meta(&self, code_id: &CodeId) -> Result<Option<CodeMeta>, StoreError> {
        let row: Option<AdminRow> = sqlx::query_as(
            "SELECT id, key_version, purpose, scope, grant_payload,
                    created_at, expires_at, used_at, used_by_subject, revoked_at
             FROM codlet_codes
             WHERE id = $1
             LIMIT 1",
        )
        .bind(code_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(to_err)?;
        Ok(row.map(admin_row_to_meta))
    }
}

/// Convenience alias.
pub type PostgresCodeStore = PostgresStore;
