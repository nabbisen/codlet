//! SQLite implementation of [`codlet_core::store::code::CodeStore`].

use codlet_core::hashing::{KeyVersion, LookupKey};
use codlet_core::secret::CodeId;
use codlet_core::state::{ClaimOutcome, classify_claim};
use codlet_core::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use codlet_core::store::error::StoreError;

use crate::SqliteStore;

/// Columns returned by the `find_one` SELECT:
/// (id, lookup_key, key_version, grant_payload, scope, expires_at)
type CodeRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

impl CodeStore for SqliteStore {
    async fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        now: u64,
        scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        // Build a parameterised `IN (?, ?, ...)` clause for the candidate keys.
        // SQLx doesn't support dynamic IN lists directly, so we iterate.
        for candidate in candidates {
            let row = find_one(&self.pool, candidate.as_str(), now, scope).await?;
            if row.is_some() {
                return Ok(row);
            }
        }
        Ok(None)
    }

    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        let now = req.now as i64;
        let id = req.code_id.as_str();
        let subject = req.subject.as_str();

        // Enforce purpose and scope to prevent cross-flow redemption (RFC-C).
        let sql = match (req.purpose, req.scope) {
            (Some(p), Some(s)) => format!(
                "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                 WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                   AND expires_at > ? AND purpose = {p:?} AND scope = {s:?}"
            ),
            (Some(p), None) => format!(
                "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                 WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                   AND expires_at > ? AND purpose = {p:?}"
            ),
            (None, Some(s)) => format!(
                "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                 WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                   AND expires_at > ? AND scope = {s:?}"
            ),
            (None, None) => "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                 WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                   AND expires_at > ?"
                .to_string(),
        };
        let result = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(now)
            .bind(subject)
            .bind(id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let changed = result.rows_affected() as usize;
        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "claim_code changed {changed} rows for id={id}"
            )));
        }
        Ok(classify_claim(changed))
    }

    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO codlet_codes
             (id, lookup_key, key_version, purpose, scope, grant_payload, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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
            if e.to_string().contains("UNIQUE") {
                StoreError::Backend("duplicate lookup key (unique constraint)".into())
            } else {
                StoreError::Backend(e.to_string())
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
        let now_i = now as i64;
        let id = code_id.as_str();

        if let Some(scope_val) = scope {
            sqlx::query(
                "UPDATE codlet_codes
                 SET revoked_at = ?
                 WHERE id = ? AND scope = ?
                   AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(now_i)
            .bind(id)
            .bind(scope_val)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        } else {
            sqlx::query(
                "UPDATE codlet_codes
                 SET revoked_at = ?
                 WHERE id = ?
                   AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(now_i)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        }
        Ok(())
    }
}

async fn find_one(
    pool: &sqlx::SqlitePool,
    lookup_key: &str,
    now: u64,
    scope: Option<&str>,
) -> Result<Option<RedeemableCode>, StoreError> {
    let now_i = now as i64;

    // Build scope clause: when scope is provided, filter by it; when None, accept any scope.
    let row: Option<CodeRow> = if let Some(s) = scope {
        sqlx::query_as(
            "SELECT id, lookup_key, key_version, purpose, grant_payload, scope, expires_at
             FROM codlet_codes
             WHERE lookup_key = ?
               AND scope       = ?
               AND used_at     IS NULL
               AND revoked_at  IS NULL
               AND expires_at  > ?
             LIMIT 1",
        )
        .bind(lookup_key)
        .bind(s)
        .bind(now_i)
        .fetch_optional(pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?
    } else {
        sqlx::query_as(
            "SELECT id, lookup_key, key_version, purpose, grant_payload, scope, expires_at
             FROM codlet_codes
             WHERE lookup_key = ?
               AND used_at    IS NULL
               AND revoked_at IS NULL
               AND expires_at > ?
             LIMIT 1",
        )
        .bind(lookup_key)
        .bind(now_i)
        .fetch_optional(pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?
    };

    Ok(row.map(
        |(id, _lk, kv, purpose_val, grant, scope_val, exp)| RedeemableCode {
            id: CodeId::new(id),
            key_version: KeyVersion::new(kv),
            grant,
            purpose: purpose_val,
            scope: scope_val,
            expires_at: exp as u64,
        },
    ))
}
