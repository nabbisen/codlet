//! In-memory form-token store (RFC-011 §10.3). Non-production.

use std::sync::Mutex;

use crate::hashing::LookupKey;
use crate::state::{TokenConsumeOutcome, classify_token_consume};
use crate::store::error::StoreError;
use crate::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

#[derive(Debug, Clone)]
struct MemTokenRow {
    lookup_key: LookupKey,
    subject: TokenSubject,
    purpose: String,
    bound_resource: Option<String>,
    expires_at: u64,
    consumed_at: Option<u64>,
    result_ref: Option<String>,
}

/// **Non-production** in-memory form-token store.
#[derive(Debug, Default)]
pub struct MemFormTokenStore {
    rows: Mutex<Vec<MemTokenRow>>,
}

impl MemFormTokenStore {
    /// Construct an empty in-memory form-token store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl FormTokenStore for MemFormTokenStore {
    async fn insert_form_token(&self, record: FormTokenRecord) -> Result<(), StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.push(MemTokenRow {
            lookup_key: record.lookup_key,
            subject: record.subject,
            purpose: record.purpose,
            bound_resource: record.bound_resource,
            expires_at: record.expires_at,
            consumed_at: None,
            result_ref: None,
        });
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
        let lookup_key = candidates.first().expect("at least one candidate");
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        // Atomic conditional update: set consumed_at if all conditions hold.
        let mut changed = 0usize;
        let mut found_row_idx: Option<usize> = None;

        for (i, row) in rows.iter_mut().enumerate() {
            // Binding checks: lookup key + subject + purpose + binding.
            if !row.lookup_key.ct_eq(lookup_key)
                || row.subject != *subject
                || row.purpose != purpose
            {
                continue;
            }
            // Remember the index for the follow-up classification SELECT.
            found_row_idx = Some(i);

            // Conditional consume: unconsumed AND not expired.
            if row.consumed_at.is_none() && row.expires_at > now {
                // Binding check on bound_resource — exact match semantics (RFC-E).
                // Aligns with SQL/D1: caller None only matches stored None.
                let br_ok = match (bound_resource, &row.bound_resource) {
                    (Some(expected), Some(stored)) => expected == stored.as_str(),
                    (None, None) => true,
                    _ => false, // None vs Some or Some vs None → mismatch
                };
                if br_ok {
                    row.consumed_at = Some(now);
                    changed += 1;
                }
            }
        }

        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "consume_form_token changed {changed} rows"
            )));
        }

        if changed == 1 {
            return Ok((TokenConsumeOutcome::Proceed, None));
        }

        // changed == 0: classify via follow-up read.
        match found_row_idx {
            None => Ok((TokenConsumeOutcome::Invalid, None)),
            Some(idx) => {
                let row = &rows[idx];
                let already_consumed = row.consumed_at.is_some();
                let binding_ok = match (bound_resource, &row.bound_resource) {
                    (Some(expected), Some(stored)) => expected == stored.as_str(),
                    (None, _) => true,
                    (Some(_), None) => false,
                };
                let outcome = classify_token_consume(0, true, already_consumed, binding_ok);
                let result_ref = if outcome == TokenConsumeOutcome::Replay {
                    row.result_ref.clone()
                } else {
                    None
                };
                Ok((outcome, result_ref))
            }
        }
    }

    async fn set_token_result(
        &self,
        candidates: &[LookupKey],
        result_ref: &str,
    ) -> Result<(), StoreError> {
        let lookup_key = candidates.first().expect("at least one candidate");
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        for row in rows.iter_mut() {
            if row.lookup_key.ct_eq(lookup_key) && row.consumed_at.is_some() {
                row.result_ref = Some(result_ref.to_string());
            }
        }
        Ok(())
    }
}
