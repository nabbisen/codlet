//! RFC-009 acceptance compile tests.
//!
//! Item 2: A `!Send` type can implement `CodeStore` — the trait has no
//! implicit `Send` bound.  This is required for Cloudflare Workers where
//! D1 handles are `!Send`.
//!
//! Item 3: A `Send + Sync` type also satisfies the same trait without any
//! adapter shim.  The in-memory and SQLite stores are both `Send + Sync`,
//! so native Tokio/Axum integration is ergonomic without wrapper types.

use std::ptr;

use codlet_core::LookupKey;
use codlet_core::secret::CodeId;
use codlet_core::state::ClaimOutcome;
use codlet_core::store::code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
use codlet_core::store::error::StoreError;

// ── Item 2: !Send store satisfies CodeStore ───────────────────────────────────

/// A deliberately `!Send` type that wraps a raw pointer.
/// In production this corresponds to a D1Database handle from the `worker`
/// crate, which is `!Send` on the wasm32-unknown-unknown target.
struct NotSendStore(#[allow(dead_code)] *const u8);
impl CodeStore for NotSendStore {
    async fn find_redeemable(
        &self,
        _candidates: &[LookupKey],
        _now: u64,
        _scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        Ok(None)
    }

    async fn claim_code(&self, _req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        Ok(ClaimOutcome::Lost)
    }

    async fn insert_code(&self, _record: CodeRecord) -> Result<(), StoreError> {
        Ok(())
    }

    async fn revoke_code(
        &self,
        _code_id: &CodeId,
        _scope: Option<&str>,
        _now: u64,
    ) -> Result<(), StoreError> {
        Ok(())
    }
}

// This function compiles only if CodeStore has no Send bound.
fn accepts_not_send_store<S: CodeStore>(_store: &S) {}

#[test]
fn not_send_store_satisfies_code_store_trait() {
    // RFC-009 item 2: no accidental Send requirement on the core trait.
    // The trait compiles with a !Send implementor — proof that there is no
    // implicit Send bound.  (Raw pointers are !Send by construction in Rust.)
    let store = NotSendStore(ptr::null());
    accepts_not_send_store(&store);
}

// ── Item 3: Send + Sync store is ergonomic for native (Axum/Tower/Tokio) ─────

/// A `Send + Sync` store (the common case for native backends like SQLite).
#[derive(Default)]
struct SendStore;

// Send and Sync are auto-derived since SendStore has no non-Send fields.
static_assertions::assert_impl_all!(SendStore: Send, Sync);

impl CodeStore for SendStore {
    async fn find_redeemable(
        &self,
        _candidates: &[LookupKey],
        _now: u64,
        _scope: Option<&str>,
    ) -> Result<Option<RedeemableCode>, StoreError> {
        Ok(None)
    }

    async fn claim_code(&self, _req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        Ok(ClaimOutcome::Lost)
    }

    async fn insert_code(&self, _record: CodeRecord) -> Result<(), StoreError> {
        Ok(())
    }

    async fn revoke_code(
        &self,
        _code_id: &CodeId,
        _scope: Option<&str>,
        _now: u64,
    ) -> Result<(), StoreError> {
        Ok(())
    }
}

// In Axum/Tower handlers the store must be Send + Sync + 'static to share
// across request tasks.  This function proves a concrete store can satisfy
// those bounds without any wrapper type or adapter shim.
fn accepts_axum_style_store<S: CodeStore + Send + Sync + 'static>(_store: S) {}

#[test]
fn send_sync_store_satisfies_axum_style_bounds() {
    // RFC-009 item 3: Send integration remains ergonomic.
    accepts_axum_style_store(SendStore);
}
