//! D1 implementation of [`FormTokenStore`] (RFC-033, INV-6).

use serde::Deserialize;
use std::rc::Rc;

use codlet::hashing::LookupKey;
use codlet::state::{TokenConsumeOutcome, classify_token_consume};
use codlet::store::error::StoreError;
use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

use crate::d1::{bind, changes, to_store_err, ts};
use crate::table_config::D1TableConfig;

/// D1-backed form-token store (RFC-033).
pub struct D1FormTokenStore {
    db: Rc<worker::d1::D1Database>,
    table: &'static str,
}

impl D1FormTokenStore {
    /// Construct from a D1 database handle and table config.
    /// Construct from a shared D1 database handle.
    ///
    /// Pass `Rc::clone(&db)` to share one handle across multiple stores:
    /// ```rust,ignore
    /// let db = std::rc::Rc::new(env.d1("DB")?);
    /// let store = D1FormTokenStore::new(std::rc::Rc::clone(&db), config);
    /// ```
    pub fn new(db: std::rc::Rc<worker::d1::D1Database>, config: D1TableConfig) -> Self {
        Self {
            db,
            table: config.form_tokens,
        }
    }
}

#[derive(Deserialize)]
struct ConsumeCheckRow {
    consumed_at: Option<f64>,
    result_ref: Option<String>,
    bound_resource: Option<String>,
}

impl FormTokenStore for D1FormTokenStore {
    async fn insert_form_token(&self, record: FormTokenRecord) -> Result<(), StoreError> {
        use worker::d1::D1Type;
        let sql = format!(
            "INSERT INTO {t}
             (lookup_key, key_version, subject_kind, purpose, bound_resource, issued_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            t = self.table
        );
        let subj = record.subject.as_binding_str();
        let br_str;
        let br = match &record.bound_resource {
            Some(b) => {
                br_str = b.clone();
                D1Type::Text(&br_str)
            }
            None => D1Type::Null,
        };
        let stmt = bind(
            self.db.prepare(&sql),
            &[
                D1Type::Text(record.lookup_key.as_str()),
                D1Type::Text(record.key_version.as_str()),
                D1Type::Text(&subj),
                D1Type::Text(&record.purpose),
                br,
                ts(record.issued_at),
                ts(record.expires_at),
            ],
        )?;
        stmt.run().await.map_err(to_store_err)?;
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
        use worker::d1::D1Type;

        let lk = lookup_key.as_str();
        let subj = subject.as_binding_str();
        let br = bound_resource.unwrap_or("");

        // Atomic conditional UPDATE (INV-6, RFC-022).
        let update_sql = format!(
            "UPDATE {t}
             SET consumed_at = ?
             WHERE lookup_key = ? AND subject_kind = ? AND purpose = ?
               AND COALESCE(bound_resource, '') = ?
               AND expires_at > ? AND consumed_at IS NULL",
            t = self.table
        );
        let stmt = bind(
            self.db.prepare(&update_sql),
            &[
                ts(now),
                D1Type::Text(lk),
                D1Type::Text(&subj),
                D1Type::Text(purpose),
                D1Type::Text(br),
                ts(now),
            ],
        )?;
        let result = stmt.run().await.map_err(to_store_err)?;
        let changed = changes(&result)?;

        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "consume_form_token changed {changed} rows for lookup_key={lk}"
            )));
        }
        if changed == 1 {
            return Ok((TokenConsumeOutcome::Proceed, None));
        }

        // changed == 0: classify reason via follow-up SELECT.
        let check_sql = format!(
            "SELECT consumed_at, result_ref, bound_resource
             FROM {t}
             WHERE lookup_key = ? AND subject_kind = ? AND purpose = ? LIMIT 1",
            t = self.table
        );
        let stmt = bind(
            self.db.prepare(&check_sql),
            &[D1Type::Text(lk), D1Type::Text(&subj), D1Type::Text(purpose)],
        )?;
        let row: Option<ConsumeCheckRow> = stmt.first(None).await.map_err(to_store_err)?;

        let found = row.is_some();
        let (already_consumed, stored_rr, stored_br) = row
            .map(|r| (r.consumed_at.is_some(), r.result_ref, r.bound_resource))
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
        candidates: &[LookupKey],
        result_ref: &str,
    ) -> Result<(), StoreError> {
        let lookup_key = candidates.first().expect("at least one candidate");
        use worker::d1::D1Type;
        let sql = format!(
            "UPDATE {t} SET result_ref = ?
             WHERE lookup_key = ? AND consumed_at IS NOT NULL",
            t = self.table
        );
        let stmt = bind(
            self.db.prepare(&sql),
            &[D1Type::Text(result_ref), D1Type::Text(lookup_key.as_str())],
        )?;
        stmt.run().await.map_err(to_store_err)?;
        Ok(())
    }
}
