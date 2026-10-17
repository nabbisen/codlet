//! PostgreSQL implementation of [`FormTokenStore`] (RFC-034, INV-6).

use codlet::hashing::LookupKey;
use codlet::state::{TokenConsumeOutcome, classify_token_consume};
use codlet::store::error::StoreError;
use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

use super::PostgresStore;

fn to_err(e: sqlx::Error) -> StoreError {
    StoreError::Backend(e.to_string())
}

// (consumed_at, result_ref, bound_resource)
type ConsumeCheckRow = (Option<i64>, Option<String>, Option<String>);

impl FormTokenStore for PostgresStore {
    async fn insert_form_token(&self, record: FormTokenRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO codlet_form_tokens
             (lookup_key, key_version, subject_kind, purpose, bound_resource, issued_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(record.lookup_key.as_str())
        .bind(record.key_version.as_str())
        .bind(record.subject.as_binding_str())
        .bind(record.purpose.as_str())
        .bind(record.bound_resource.as_deref())
        .bind(record.issued_at as i64)
        .bind(record.expires_at as i64)
        .execute(&self.pool)
        .await
        .map_err(to_err)?;
        Ok(())
    }

    async fn consume_form_token(
        &self,
        candidates: &[LookupKey],
        subject: &TokenSubject,
        purpose: &str,
        bound_resource: Option<&str>,
        now: u64,
    ) -> Result<(TokenConsumeOutcome, Option<String>), StoreError> {
        let subj = subject.as_binding_str();
        let br = bound_resource.unwrap_or("");

        // RFC-051: try every candidate in order (active key first, then
        // previous keys), as `FormTokenStore` documents, so a token issued
        // under a previous key stays consumable during a rotation grace
        // period. A candidate that matches no row falls through to the next.
        for lookup_key in candidates {
            let lk = lookup_key.as_str();

            // Atomic conditional UPDATE (INV-6, RFC-022, RFC-034 §7).
            let result = sqlx::query(
                "UPDATE codlet_form_tokens
                 SET consumed_at = $1
                 WHERE lookup_key = $2
                   AND subject_kind = $3
                   AND purpose = $4
                   AND COALESCE(bound_resource, '') = $5
                   AND expires_at > $6
                   AND consumed_at IS NULL",
            )
            .bind(now as i64)
            .bind(lk)
            .bind(subj.as_str())
            .bind(purpose)
            .bind(br)
            .bind(now as i64)
            .execute(&self.pool)
            .await
            .map_err(to_err)?;

            let changed = result.rows_affected() as usize;
            if changed > 1 {
                return Err(StoreError::InvariantViolation(format!(
                    "consume_form_token changed {changed} rows for lookup_key={lk}"
                )));
            }
            if changed == 1 {
                return Ok((TokenConsumeOutcome::Proceed, None));
            }

            // changed == 0: classify reason via follow-up SELECT.
            let row: Option<ConsumeCheckRow> = sqlx::query_as(
                "SELECT consumed_at, result_ref, bound_resource
                 FROM codlet_form_tokens
                 WHERE lookup_key = $1 AND subject_kind = $2 AND purpose = $3
                 LIMIT 1",
            )
            .bind(lk)
            .bind(subj.as_str())
            .bind(purpose)
            .fetch_optional(&self.pool)
            .await
            .map_err(to_err)?;

            let Some((consumed_at, stored_rr, stored_br)) = row else {
                continue;
            };

            let binding_ok = match bound_resource {
                Some(expected) => stored_br.as_deref().unwrap_or("") == expected,
                None => true,
            };

            let outcome = classify_token_consume(0, true, consumed_at.is_some(), binding_ok);
            let result_ref = if outcome == TokenConsumeOutcome::Replay {
                stored_rr
            } else {
                None
            };
            return Ok((outcome, result_ref));
        }

        // No candidate matched any row (including an empty slice): unknown token.
        Ok((classify_token_consume(0, false, false, false), None))
    }

    async fn set_token_result(
        &self,
        candidates: &[LookupKey],
        result_ref: &str,
    ) -> Result<(), StoreError> {
        // RFC-051: act on the first candidate that matches a consumed row.
        for lookup_key in candidates {
            let updated = sqlx::query(
                "UPDATE codlet_form_tokens
                 SET result_ref = $1
                 WHERE lookup_key = $2 AND consumed_at IS NOT NULL",
            )
            .bind(result_ref)
            .bind(lookup_key.as_str())
            .execute(&self.pool)
            .await
            .map_err(to_err)?;
            if updated.rows_affected() > 0 {
                break;
            }
        }
        Ok(())
    }
}

/// Convenience alias.
pub type PostgresFormTokenStore = PostgresStore;
