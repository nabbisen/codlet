//! In-memory code store (RFC-011 §10.3).
//!
//! For tests and local development **only**. Not for production:
//! - data is lost on drop;
//! - uses a `Mutex` that serialises concurrent claims (which is correct for
//!   single-process tests but wrong for multi-instance deployments);
//! - does not enforce UNIQUE on the lookup key at the DB level.

use std::sync::Mutex;

use crate::hashing::LookupKey;
use crate::secret::{CodeId, SubjectId};
use crate::state::{ClaimOutcome, classify_claim};
use crate::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use crate::store::error::StoreError;

#[derive(Debug, Clone)]
struct MemCodeRow {
    id: CodeId,
    lookup_keys: Vec<LookupKey>,
    key_version: crate::hashing::KeyVersion,
    purpose: Option<String>,
    scope: Option<String>,
    grant: Option<String>,
    expires_at: u64,
    used_at: Option<u64>,
    revoked_at: Option<u64>,
    used_by: Option<SubjectId>,
}

impl MemCodeRow {
    fn is_redeemable_at(&self, now: u64, scope: Option<&str>) -> bool {
        self.used_at.is_none()
            && self.revoked_at.is_none()
            && self.expires_at > now
            && match scope {
                Some(s) => self.scope.as_deref() == Some(s),
                None => true,
            }
    }

    fn matches_any(&self, candidates: &[LookupKey]) -> bool {
        candidates
            .iter()
            .any(|c| self.lookup_keys.iter().any(|k| k.ct_eq(c)))
    }
}

/// **Non-production** in-memory code store (RFC-011 §10.3).
///
/// Safe for deterministic tests and single-process local development. The
/// `Mutex` ensures concurrent claim tests prove single-winner semantics within
/// one process, but cannot substitute for a real DB in multi-instance
/// deployments.
#[derive(Debug, Default)]
pub struct MemCodeStore {
    rows: Mutex<Vec<MemCodeRow>>,
}

impl MemCodeStore {
    /// Construct an empty in-memory code store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CodeStore for MemCodeStore {
    async fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        now: u64,
        scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        let rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let found = rows
            .iter()
            .find(|r| r.is_redeemable_at(now, scope) && r.matches_any(candidates))
            .map(|r| RedeemableCode {
                id: r.id.clone(),
                key_version: r.key_version.clone(),
                grant: r.grant.clone(),
                purpose: r.purpose.clone(),
                scope: r.scope.clone(),
                expires_at: r.expires_at,
            });
        Ok(found)
    }

    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let mut changed = 0usize;
        for row in rows.iter_mut() {
            if &row.id == req.code_id
                && row.used_at.is_none()
                && row.revoked_at.is_none()
                && row.expires_at > req.now
                && req
                    .purpose
                    .is_none_or(|p| row.purpose.as_deref() == Some(p))
                && req.scope.is_none_or(|s| row.scope.as_deref() == Some(s))
            {
                row.used_at = Some(req.now);
                row.used_by = Some(req.subject.clone());
                changed += 1;
            }
        }
        if changed > 1 {
            return Err(StoreError::InvariantViolation(format!(
                "claim_code changed {changed} rows for {:?}",
                req.code_id
            )));
        }
        Ok(classify_claim(changed))
    }

    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        // Reject duplicate active lookup keys (mirrors the UNIQUE constraint).
        let lk = record.lookup_key.clone();
        if rows
            .iter()
            .any(|r| r.lookup_keys.iter().any(|k| k.ct_eq(&lk)))
        {
            return Err(StoreError::Backend(
                "duplicate lookup key (unique constraint)".to_string(),
            ));
        }
        rows.push(MemCodeRow {
            id: record.id,
            lookup_keys: vec![record.lookup_key],
            key_version: record.key_version,
            purpose: record.purpose,
            scope: record.scope,
            grant: record.grant,
            expires_at: record.expires_at,
            used_at: None,
            revoked_at: None,
            used_by: None,
        });
        Ok(())
    }

    async fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> Result<(), StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        for row in rows.iter_mut() {
            if &row.id == code_id
                && row.used_at.is_none()
                && row.revoked_at.is_none()
                && scope.is_none_or(|s| row.scope.as_deref() == Some(s))
            {
                row.revoked_at = Some(now);
            }
        }
        Ok(())
    }
}
