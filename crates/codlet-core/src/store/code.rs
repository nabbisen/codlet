//! Code storage trait (RFC-005).
//!
//! Adapters must implement [`CodeStore`] and prove atomic single-winner claim
//! behaviour by running the conformance suite (RFC-023).

use crate::hashing::{KeyVersion, LookupKey};
use crate::secret::{CodeId, SubjectId};
use crate::state::ClaimOutcome;

use super::error::StoreError;

/// Record returned by a successful `find_redeemable` call.
#[derive(Debug, Clone)]
pub struct RedeemableCode {
    /// Opaque record identifier (not a secret, safe for logs and audit).
    pub id: CodeId,
    /// The lookup key version under which this code was stored. Needed to
    /// re-derive the comparison candidate during claim.
    pub key_version: KeyVersion,
    /// Opaque host-owned grant payload, returned after a won claim.
    pub grant: Option<String>,
    /// Optional scope label set at issuance; restricts claim to matching scope.
    pub scope: Option<String>,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
}

/// Parameters for inserting a new code record.
pub struct CodeRecord {
    /// Storage identifier (caller-assigned; UUID recommended).
    pub id: CodeId,
    /// Domain-separated HMAC of the normalized code (never the plaintext).
    pub lookup_key: LookupKey,
    /// Key version that produced `lookup_key`.
    pub key_version: KeyVersion,
    /// Optional host-owned purpose label (e.g. `"redeem_invite"`).
    pub purpose: Option<String>,
    /// Optional scope key (e.g. a community ID).
    pub scope: Option<String>,
    /// Optional opaque grant returned to the host after a won claim.
    pub grant: Option<String>,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
}

/// Parameters for a claim attempt.
pub struct ClaimRequest<'a> {
    /// The record to attempt to claim (from `find_redeemable`).
    pub code_id: &'a CodeId,
    /// The subject that is claiming this code. Stored on the record for audit.
    pub subject: &'a SubjectId,
    /// Current time as Unix seconds (UTC). Used as `used_at` and in the
    /// expiry guard of the conditional UPDATE.
    pub now: u64,
    /// Optional purpose label checked against the stored purpose.
    pub purpose: Option<&'a str>,
    /// Optional scope checked against the stored scope.
    pub scope: Option<&'a str>,
}

/// Atomic, single-winner code storage (RFC-005).
///
/// Implementors must guarantee:
///
/// - `find_redeemable` never returns expired, used, or revoked records;
/// - `claim_code` uses a conditional UPDATE (not read-then-write); the
///   affected-row count is exactly 1 for a winner and 0 for all others;
/// - `changed > 1` is surfaced as [`StoreError::InvariantViolation`], not
///   silently mapped to `Lost`.
pub trait CodeStore {
    /// Look up a redeemable code by its HMAC lookup key candidates.
    ///
    /// Returns the first record that matches any candidate key and is currently
    /// redeemable (not used, revoked, or expired at `now`). Returns `Ok(None)`
    /// if no such record exists.
    fn find_redeemable(
        &self,
        candidates: &[LookupKey],
        now: u64,
        scope: Option<&str>,
    ) -> impl Future<Output = Result<Option<RedeemableCode>, StoreError>>;

    /// Attempt to atomically claim a code record.
    ///
    /// The adapter must execute a conditional UPDATE and classify via
    /// [`crate::state::classify_claim`]. Returns [`ClaimOutcome::Won`] if and
    /// only if exactly one row was updated.
    fn claim_code(
        &self,
        req: &ClaimRequest<'_>,
    ) -> impl Future<Output = Result<ClaimOutcome, StoreError>>;

    /// Insert a new code record. Returns [`StoreError`] if the lookup key
    /// already exists (unique constraint violation on the HMAC column).
    fn insert_code(&self, record: CodeRecord) -> impl Future<Output = Result<(), StoreError>>;

    /// Revoke a code by its record ID, scoped to `scope` when provided.
    /// Only affects records that are not yet used or revoked.
    fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>>;
}

use std::future::Future;

/// Compute an `expires_at` Unix timestamp from a now value and a TTL.
pub fn expires_at_from_ttl(now: u64, ttl: std::time::Duration) -> u64 {
    now.saturating_add(ttl.as_secs())
}

/// Derive all candidate lookup keys for a normalized code value.
///
/// v0.1 produces only the active-key candidate. Key-rotation multi-candidate
/// support is introduced in RFC-031.
pub fn code_lookup_candidates<K: crate::hashing::KeyProvider>(
    hasher: &crate::hashing::SecretHasher<K>,
    normalized: &str,
) -> Vec<(LookupKey, KeyVersion)> {
    hasher
        .lookup_key(crate::hashing::SecretDomain::Code, normalized)
        .map(|(lk, kv)| vec![(lk, kv)])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn expires_at_from_ttl_adds_correctly() {
        assert_eq!(expires_at_from_ttl(1_000, Duration::from_secs(3600)), 4_600);
        assert_eq!(
            expires_at_from_ttl(u64::MAX, Duration::from_secs(1)),
            u64::MAX
        );
    }
}
