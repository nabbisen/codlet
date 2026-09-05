//! SQLite implementation of [`codlet::store::code::CodeStore`].

use codlet::hashing::{KeyVersion, LookupKey};
use codlet::secret::CodeId;
use codlet::state::{ClaimOutcome, classify_claim};
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use codlet::store::error::StoreError;

use crate::SqliteStore;

/// Columns returned by the `find_one` SELECT:
/// (id, lookup_key, key_version, purpose, grant_payload, scope, expires_at,
///  used_at, revoked_at)
type CodeRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    Option<i64>,
    Option<i64>,
);

impl CodeStore for SqliteStore {
    async fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        _now: u64,
        scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        // Build a parameterised `IN (?, ?, ...)` clause for the candidate keys.
        // SQLx doesn't support dynamic IN lists directly, so we iterate.
        for candidate in candidates {
            let row = find_one(&self.pool, candidate.as_str(), scope).await?;
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
        // A fixed set of complete, constant SQL strings — `purpose`/`scope`
        // are always bound as parameters, never interpolated (RFC-048). Each
        // literal is `&'static str`, so `AssertSqlSafe` is not needed here:
        // there is no dynamically-assembled fragment for it to assert about.
        let result = match (req.purpose, req.scope) {
            (Some(p), Some(s)) => {
                sqlx::query(
                    "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                     WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ? AND purpose = ? AND scope = ?",
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
                    "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                     WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ? AND purpose = ?",
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
                    "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                     WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ? AND scope = ?",
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
                    "UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
                     WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
                       AND expires_at > ?",
                )
                .bind(now)
                .bind(subject)
                .bind(id)
                .bind(now)
                .execute(&self.pool)
                .await
            }
        }
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
    scope: Option<&str>,
) -> Result<Option<RedeemableCode>, StoreError> {
    // RFC-047: matches on lookup key and scope only. No expiry/revocation/use
    // predicate here -- `classify_code_lookup` decides that from the returned
    // state fields; `claim_code`'s conditional UPDATE remains the actual
    // enforcement point (INV-5).
    let row: Option<CodeRow> = if let Some(s) = scope {
        sqlx::query_as(
            "SELECT id, lookup_key, key_version, purpose, grant_payload, scope, expires_at,
                    used_at, revoked_at
             FROM codlet_codes
             WHERE lookup_key = ?
               AND scope      = ?
             LIMIT 1",
        )
        .bind(lookup_key)
        .bind(s)
        .fetch_optional(pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?
    } else {
        sqlx::query_as(
            "SELECT id, lookup_key, key_version, purpose, grant_payload, scope, expires_at,
                    used_at, revoked_at
             FROM codlet_codes
             WHERE lookup_key = ?
             LIMIT 1",
        )
        .bind(lookup_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?
    };

    Ok(row.map(
        |(id, _lk, kv, purpose_val, grant, scope_val, exp, used_at, revoked_at)| RedeemableCode {
            id: CodeId::new(id),
            key_version: KeyVersion::new(kv),
            grant,
            purpose: purpose_val,
            scope: scope_val,
            expires_at: exp as u64,
            used_at: used_at.map(|t| t as u64),
            revoked_at: revoked_at.map(|t| t as u64),
        },
    ))
}
