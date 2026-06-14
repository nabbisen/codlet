//! Form-token-store conformance tests (RFC-023).

use std::future::Future;

use crate::fixtures::*;
use codlet_core::state::TokenConsumeOutcome;

// ── FormTokenStore conformance ────────────────────────────────────────────────

/// Run the full form-token-store conformance suite.
pub async fn run_form_token_store_conformance<F, Fut, S>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    test_valid_consume_proceeds(&factory).await;
    test_replay_returns_replay(&factory).await;
    test_expired_token_invalid(&factory).await;
    test_purpose_mismatch_invalid(&factory).await;
    test_subject_mismatch_invalid(&factory).await;
    test_bound_resource_mismatch_invalid(&factory).await;
    test_changed_zero_never_proceeds(&factory).await;
}

async fn test_valid_consume_proceeds<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t1", auth(1), "logout", None, LATER))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&token_lk("t1"), &auth(1), "logout", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Proceed,
        "valid consume must Proceed"
    );
}

async fn test_replay_returns_replay<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t2", auth(1), "save", None, LATER))
        .await
        .unwrap();
    let (r1, _) = store
        .consume_form_token(&token_lk("t2"), &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(r1, TokenConsumeOutcome::Proceed);
    let (r2, _) = store
        .consume_form_token(&token_lk("t2"), &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        r2,
        TokenConsumeOutcome::Replay,
        "second consume must Replay"
    );
}

async fn test_expired_token_invalid<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t3", auth(1), "act", None, EXPIRED))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&token_lk("t3"), &auth(1), "act", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "expired must be Invalid"
    );
}

async fn test_purpose_mismatch_invalid<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t4", auth(1), "save", None, LATER))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&token_lk("t4"), &auth(1), "delete", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "purpose mismatch must be Invalid"
    );
}

async fn test_subject_mismatch_invalid<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t5", auth(1), "save", None, LATER))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&token_lk("t5"), &auth(2), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "subject mismatch must be Invalid"
    );
}

async fn test_bound_resource_mismatch_invalid<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t6", auth(1), "edit", Some("res-A"), LATER))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&token_lk("t6"), &auth(1), "edit", Some("res-B"), NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "bound resource mismatch must be Invalid"
    );
}

async fn test_changed_zero_never_proceeds<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    // Unknown token → changed==0 → must be Invalid, never Proceed.
    let store = factory().await;
    let (outcome, _) = store
        .consume_form_token(&token_lk("ghost"), &auth(1), "act", None, NOW)
        .await
        .unwrap();
    assert_ne!(
        outcome,
        TokenConsumeOutcome::Proceed,
        "unknown token must not Proceed"
    );
    assert_eq!(outcome, TokenConsumeOutcome::Invalid);
}
