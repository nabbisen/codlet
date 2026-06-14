//! PostgreSQL implementation of [`FormTokenStore`] (RFC-034, INV-6).

use codlet_core::hashing::LookupKey;
use codlet_core::state::{TokenConsumeOutcome, classify_token_consume};
use codlet_core::store::error::StoreError;
use codlet_core::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

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
        lookup_key: &LookupKey,
        subject: &TokenSubject,
        purpose: &str,
        bound_resource: Option<&str>,
        now: u64,
    ) -> Result<(TokenConsumeOutcome, Option<String>), StoreError> {
        let lk = lookup_key.as_str();
        let subj = subject.as_binding_str();
        let br = bound_resource.unwrap_or("");

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

        let found = row.is_some();
        let (already_consumed, stored_rr, stored_br) = row
            .map(|(ca, rr, br)| (ca.is_some(), rr, br))
            .unwrap_or((false, None, None));

        let binding_ok = match bound_resource {
            Some(expected) => stored_br.as_deref().unwrap_or("") == expected,
            None => true,
        };

        let outcome = classify_token_consume(0, found, already_consumed, binding_ok);
        let result_ref = if outcome == TokenConsumeOutcome::Replay {
            stored_rr
        } else {
            None
        };
        Ok((outcome, result_ref))
    }

    async fn set_token_result(
        &self,
        lookup_key: &LookupKey,
        result_ref: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE codlet_form_tokens
             SET result_ref = $1
             WHERE lookup_key = $2 AND consumed_at IS NOT NULL",
        )
        .bind(result_ref)
        .bind(lookup_key.as_str())
        .execute(&self.pool)
        .await
        .map_err(to_err)?;
        Ok(())
    }
}

/// Convenience alias.
pub type PostgresFormTokenStore = PostgresStore;
