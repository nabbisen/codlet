//! Administrative code management API (RFC-030).
//!
//! This module provides metadata-only operations for host admin panels.
//! **Authorization for calling these APIs is entirely host-owned.** codlet
//! does not check who may issue or revoke codes; the host must enforce that.
//!
//! The [`CodeAdminStore`] trait is optional — adapters implement it in addition
//! to [`crate::store::code::CodeStore`] when they want to expose admin listing.
//! Metadata returned by this trait never includes plaintext codes or HMAC
//! lookup keys (RFC-030 §returned metadata).

use std::future::Future;

use crate::hashing::KeyVersion;
use crate::secret::{CodeId, ScopeKey, SubjectId};
use crate::store::error::StoreError;

/// Metadata record for a code — safe for admin display (RFC-030).
///
/// Contains no plaintext code value and no HMAC lookup key.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CodeMeta {
    /// Opaque record identifier.
    pub id: CodeId,
    /// The key version under which this code was stored.
    pub key_version: KeyVersion,
    /// Optional host-owned purpose label.
    pub purpose: Option<String>,
    /// Optional scope label.
    pub scope: Option<String>,
    /// Opaque host-owned grant payload (safe to return to admins).
    pub grant: Option<String>,
    /// Creation time as Unix seconds (UTC), if available.
    pub created_at: Option<u64>,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
    /// When the code was claimed, if it was.
    pub used_at: Option<u64>,
    /// Which subject claimed it, if it was claimed.
    pub used_by: Option<SubjectId>,
    /// When the code was revoked, if it was.
    pub revoked_at: Option<u64>,
}

impl CodeMeta {
    /// Whether the code is currently redeemable at `now`.
    #[must_use]
    pub fn is_redeemable_at(&self, now: u64) -> bool {
        self.used_at.is_none() && self.revoked_at.is_none() && self.expires_at > now
    }
}

/// Filter for code listing queries (RFC-030).
#[derive(Debug, Default, Clone)]
pub struct CodeListFilter {
    /// Restrict to codes with this scope key.
    pub scope: Option<ScopeKey>,
    /// Include only active (unused, unrevoked, unexpired at `now`) codes.
    pub active_only: bool,
    /// Maximum number of records to return.
    pub limit: Option<usize>,
}

impl CodeListFilter {
    /// An empty filter that matches all codes.
    #[must_use]
    pub fn all() -> Self {
        Self::default()
    }

    /// Filter to active codes only within a scope.
    #[must_use]
    pub fn active_in_scope(scope: ScopeKey) -> Self {
        Self {
            scope: Some(scope),
            active_only: true,
            limit: None,
        }
    }
}

/// Optional admin extension trait for code stores (RFC-030).
///
/// Adapters that support admin listing implement this in addition to
/// [`crate::store::code::CodeStore`]. Authorization remains host-owned.
pub trait CodeAdminStore {
    /// List code metadata matching `filter`, ordered by `expires_at` descending.
    ///
    /// Never returns plaintext codes or HMAC lookup keys.
    ///
    /// # Errors
    /// [`StoreError::Backend`] on storage failure.
    fn list_codes(
        &self,
        filter: &CodeListFilter,
        now: u64,
    ) -> impl Future<Output = Result<Vec<CodeMeta>, StoreError>>;

    /// Retrieve a single code's metadata by its record ID.
    ///
    /// Returns `Ok(None)` if no record with that ID exists.
    ///
    /// # Errors
    /// [`StoreError::Backend`] on storage failure.
    fn get_code_meta(
        &self,
        code_id: &CodeId,
    ) -> impl Future<Output = Result<Option<CodeMeta>, StoreError>>;
}

/// Admin statistics snapshot (RFC-030).
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CodeStats {
    /// Codes that are currently redeemable.
    pub active: usize,
    /// Codes that have been successfully claimed.
    pub used: usize,
    /// Codes that were revoked before use.
    pub revoked: usize,
    /// Codes that expired without being claimed or revoked.
    pub expired: usize,
}

impl CodeStats {
    /// Total number of records across all states.
    #[must_use]
    pub fn total(&self) -> usize {
        self.active + self.used + self.revoked + self.expired
    }
}

/// In-memory implementation of `CodeAdminStore` for tests.
/// Available under the `test-utils` feature.
#[cfg(any(test, feature = "test-utils"))]
pub mod mem_admin {
    use super::*;
    use crate::mem::MemCodeStore;

    impl CodeAdminStore for MemCodeStore {
        async fn list_codes(
            &self,
            filter: &CodeListFilter,
            now: u64,
        ) -> Result<Vec<CodeMeta>, StoreError> {
            // Reflect on internal rows via the store's public query surface.
            // The in-memory implementation has no index, so we synthesise from
            // what find_redeemable exposes. For the test implementation we
            // return a best-effort view without touching private fields.
            //
            // Real adapters (SQLx, D1) implement this with a direct SELECT.
            // This stub always returns empty — tests that need listing should
            // use the SQLite adapter.
            let _ = (filter, now);
            Ok(Vec::new())
        }

        async fn get_code_meta(&self, _code_id: &CodeId) -> Result<Option<CodeMeta>, StoreError> {
            // Stub — see above.
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_meta_is_redeemable_logic() {
        let now = 1_000;
        let base = CodeMeta {
            id: CodeId::new("c1".into()),
            key_version: crate::hashing::KeyVersion::new("v1"),
            purpose: None,
            scope: None,
            grant: None,
            created_at: Some(now - 10),
            expires_at: now + 100,
            used_at: None,
            used_by: None,
            revoked_at: None,
        };
        assert!(base.is_redeemable_at(now));
        assert!(
            !CodeMeta {
                used_at: Some(now),
                ..base.clone()
            }
            .is_redeemable_at(now)
        );
        assert!(
            !CodeMeta {
                revoked_at: Some(now),
                ..base.clone()
            }
            .is_redeemable_at(now)
        );
        assert!(
            !CodeMeta {
                expires_at: now - 1,
                ..base
            }
            .is_redeemable_at(now)
        );
    }

    #[test]
    fn code_list_filter_helpers() {
        let all = CodeListFilter::all();
        assert!(all.scope.is_none() && !all.active_only);
        let scoped = CodeListFilter::active_in_scope(ScopeKey::new("community-1"));
        assert!(scoped.active_only);
        assert_eq!(scoped.scope.unwrap().as_str(), "community-1");
    }

    #[test]
    fn code_stats_total() {
        let s = CodeStats {
            active: 3,
            used: 10,
            revoked: 2,
            expired: 5,
        };
        assert_eq!(s.total(), 20);
    }

    #[test]
    fn code_meta_contains_no_secrets() {
        // Verify the type has no field named plaintext / lookup_key / hmac.
        // This is enforced by the type definition but we assert via Debug.
        let m = CodeMeta {
            id: CodeId::new("c1".into()),
            key_version: crate::hashing::KeyVersion::new("v1"),
            purpose: Some("invite".into()),
            scope: Some("community-1".into()),
            grant: Some("role:member".into()),
            created_at: None,
            expires_at: 9_999_999,
            used_at: None,
            used_by: None,
            revoked_at: None,
        };
        let dbg = format!("{m:?}");
        let forbidden = ["lookup_key", "hmac", "plain_code", "secret", "pepper"];
        for word in forbidden {
            assert!(
                !dbg.to_lowercase().contains(word),
                "CodeMeta debug contains {word:?}: {dbg}"
            );
        }
    }
}
