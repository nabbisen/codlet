//! Form-token-store conformance tests (RFC-023).

use std::future::Future;

use crate::fixtures::*;
use codlet::state::TokenConsumeOutcome;

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
    test_consume_honours_every_candidate(&factory).await;
    test_set_token_result_honours_every_candidate(&factory).await;
    test_empty_candidates_match_nothing(&factory).await;
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
        .consume_form_token(&[token_lk("t1")], &auth(1), "logout", None, NOW)
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
        .consume_form_token(&[token_lk("t2")], &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(r1, TokenConsumeOutcome::Proceed);
    let (r2, _) = store
        .consume_form_token(&[token_lk("t2")], &auth(1), "save", None, NOW)
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
        .consume_form_token(&[token_lk("t3")], &auth(1), "act", None, NOW)
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
        .consume_form_token(&[token_lk("t4")], &auth(1), "delete", None, NOW)
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
        .consume_form_token(&[token_lk("t5")], &auth(2), "save", None, NOW)
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
        .consume_form_token(&[token_lk("t6")], &auth(1), "edit", Some("res-B"), NOW)
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
        .consume_form_token(&[token_lk("ghost")], &auth(1), "act", None, NOW)
        .await
        .unwrap();
    assert_ne!(
        outcome,
        TokenConsumeOutcome::Proceed,
        "unknown token must not Proceed"
    );
    assert_eq!(outcome, TokenConsumeOutcome::Invalid);
}

// ── Multi-candidate contract (RFC-051) ───────────────────────────────────────
//
// `FormTokenStore` takes `candidates: &[LookupKey]` — one per held key, active
// key first — so that a token issued under a previous key stays usable during a
// rotation grace period. After a rotation the matching key is therefore *not*
// first. These tests put a key that matches nothing in front of the one that
// does, which is the only ordering that distinguishes "tries every candidate"
// from "tries the first".

async fn test_consume_honours_every_candidate<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t6", auth(1), "save", None, LATER))
        .await
        .unwrap();
    // `[non-matching, matching]`: the post-rotation order.
    let candidates = [token_lk("issued-under-no-key"), token_lk("t6")];
    let (outcome, _) = store
        .consume_form_token(&candidates, &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Proceed,
        "a token matching a later candidate must be consumable (RFC-051)"
    );
    // The follow-up classification must also reach the later candidate.
    let (replay, _) = store
        .consume_form_token(&candidates, &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        replay,
        TokenConsumeOutcome::Replay,
        "a replay through a later candidate must classify as Replay (RFC-051)"
    );
}

async fn test_set_token_result_honours_every_candidate<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    let store = factory().await;
    store
        .insert_form_token(token_record("t7", auth(1), "save", None, LATER))
        .await
        .unwrap();
    let (first, _) = store
        .consume_form_token(&[token_lk("t7")], &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(first, TokenConsumeOutcome::Proceed);

    // `[non-matching, matching]`: the post-rotation order.
    store
        .set_token_result(
            &[token_lk("issued-under-no-key"), token_lk("t7")],
            "result-7",
        )
        .await
        .unwrap();

    let (replay, result_ref) = store
        .consume_form_token(&[token_lk("t7")], &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(replay, TokenConsumeOutcome::Replay);
    assert_eq!(
        result_ref.as_deref(),
        Some("result-7"),
        "set_token_result must reach a token matching a later candidate (RFC-051)"
    );
}

async fn test_empty_candidates_match_nothing<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: FormTokenStore,
{
    // An empty slice is "no match", not a panic (RFC-051 §5.2): the store layer
    // is a supported surface, and a host can legally pass nothing.
    let store = factory().await;
    store
        .insert_form_token(token_record("t8", auth(1), "save", None, LATER))
        .await
        .unwrap();
    let (outcome, _) = store
        .consume_form_token(&[], &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "no candidates must classify as an unknown token"
    );
    store.set_token_result(&[], "ignored").await.unwrap();

    // …and it must not have touched the stored token.
    let (after, result_ref) = store
        .consume_form_token(&[token_lk("t8")], &auth(1), "save", None, NOW)
        .await
        .unwrap();
    assert_eq!(after, TokenConsumeOutcome::Proceed);
    assert!(result_ref.is_none());
}
