//! Run the full conformance suite against the in-memory adapters (RFC-023).
//!
//! This proves that the in-memory stores satisfy the same contract as
//! production adapters.  A failure here would indicate a bug in the
//! conformance suite itself or in the in-memory reference implementation.

use codlet::mem::{MemCodeStore, MemFormTokenStore, MemSessionStore};

#[tokio::test]
async fn mem_code_store_conformance() {
    codlet_conformance::run_code_store_conformance(|| async { MemCodeStore::new() }).await;
}

#[tokio::test]
async fn mem_session_store_conformance() {
    codlet_conformance::run_session_store_conformance(|| async { MemSessionStore::new() }).await;
}

#[tokio::test]
async fn mem_form_token_store_conformance() {
    codlet_conformance::run_form_token_store_conformance(|| async { MemFormTokenStore::new() })
        .await;
}
