//! RFC-054 stage B: the sequential conformance entry points must accept a store
//! that is `!Send + !Sync`, as the D1 stores are (they hold an `Rc` handle).
//!
//! This runs the real suite against a deliberately `!Send` wrapper around the
//! in-memory store. If a `Send`/`Sync` bound ever creeps back onto
//! `run_code_store_conformance`, this file stops compiling.
use std::rc::Rc;

use codlet::LookupKey;
use codlet::mem::MemCodeStore;
use codlet::state::ClaimOutcome;
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore, StoredCode};
use codlet::store::error::StoreError;

/// A `CodeStore` that is `!Send + !Sync`: `Rc` is neither.
struct NotSendCodeStore {
    inner: Rc<MemCodeStore>,
}

impl NotSendCodeStore {
    fn new() -> Self {
        Self {
            inner: Rc::new(MemCodeStore::new()),
        }
    }
}

impl CodeStore for NotSendCodeStore {
    async fn lookup_code(
        &self,
        candidates: &[LookupKey],
        scope: Option<&str>,
    ) -> Result<Option<StoredCode>, StoreError> {
        self.inner.lookup_code(candidates, scope).await
    }
    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        self.inner.claim_code(req).await
    }
    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        self.inner.insert_code(record).await
    }
    async fn revoke_code(
        &self,
        code_id: &codlet::secret::CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> Result<(), StoreError> {
        self.inner.revoke_code(code_id, scope, now).await
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sequential_code_suite_runs_against_a_not_send_store() {
    codlet_conformance::run_code_store_conformance(|| async { NotSendCodeStore::new() }).await;
}
