//! Code-store conformance tests (RFC-023).

use std::future::Future;
use std::sync::Arc;

use crate::fixtures::*;
use codlet::state::{ClaimOutcome, CodeLookupOutcome, classify_code_lookup};

// ── CodeStore conformance ────────────────────────────────────────────────────

/// Run the full code-store conformance suite against a store produced by
/// `factory`. The factory is called once per sub-test so each test starts with
/// a clean store.
pub async fn run_code_store_conformance<F, Fut, S>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    test_insert_and_find_redeemable(&factory).await;
    test_nonexistent_returns_none(&factory).await;
    test_expired_returned_and_classifier_rejects(&factory).await;
    test_used_returned_and_classifier_rejects(&factory).await;
    test_revoked_returned_and_classifier_rejects(&factory).await;
    test_revoked_and_expired_classifies_revoked(&factory).await;
    test_exactly_one_claim_winner(&factory).await;
    test_scope_revoke_returned_and_classifier_rejects(&factory).await;
    test_wrong_scope_does_not_revoke(&factory).await;
}

async fn test_insert_and_find_redeemable<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("c1", "secret1", LATER, None))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("secret1")], NOW, None)
        .await
        .unwrap();
    assert!(
        found.is_some(),
        "insert_and_find: inserted code must be found"
    );
    let r = found.unwrap();
    assert_eq!(r.id, CodeId::new("c1".into()));
    assert_eq!(r.grant.as_deref(), Some("grant-c1"));
}

async fn test_nonexistent_returns_none<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    let found = store
        .find_redeemable(&[code_lk("ghost")], NOW, None)
        .await
        .unwrap();
    assert!(found.is_none(), "nonexistent: must return None");
}

/// RFC-047 §4.3: this must fail against an adapter that kept its old
/// exclusion filter, not merely pass against a migrated one. An adapter that
/// still filters expired rows out of `find_redeemable` returns `None` here,
/// which trips the `.expect(...)` below -- the inverted assertion carries the
/// security property this suite exists to prove (verified by temporarily
/// reintroducing the filter in `MemCodeStore` and confirming this test fails;
/// see the RFC-047 review request).
async fn test_expired_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("cx", "expiredsec", EXPIRED, None))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("expiredsec")], NOW, None)
        .await
        .unwrap()
        .expect("RFC-047: the store must return an expired record, not filter it out");
    assert_eq!(found.expires_at, EXPIRED);
    assert_eq!(
        classify_code_lookup(found.revoked_at, found.used_at, found.expires_at, NOW),
        CodeLookupOutcome::Expired,
        "the classifier must reject the returned record"
    );
}

/// See `test_expired_returned_and_classifier_rejects` for why this asserts
/// return-and-reject rather than exclusion.
async fn test_used_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("cu", "usedsec", LATER, None))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("usedsec")], NOW, None)
        .await
        .unwrap()
        .unwrap();
    let won = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &SubjectId::new("u1".into()),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(won, ClaimOutcome::Won);
    let again = store
        .find_redeemable(&[code_lk("usedsec")], NOW, None)
        .await
        .unwrap()
        .expect("RFC-047: the store must return a used record, not filter it out");
    assert!(again.used_at.is_some());
    assert_eq!(
        classify_code_lookup(again.revoked_at, again.used_at, again.expires_at, NOW),
        CodeLookupOutcome::Used
    );
}

/// See `test_expired_returned_and_classifier_rejects` for why this asserts
/// return-and-reject rather than exclusion.
async fn test_revoked_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("cr", "revokedsec", LATER, None))
        .await
        .unwrap();
    store
        .revoke_code(&CodeId::new("cr".into()), None, NOW)
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("revokedsec")], NOW, None)
        .await
        .unwrap()
        .expect("RFC-047: the store must return a revoked record, not filter it out");
    assert!(found.revoked_at.is_some());
    assert_eq!(
        classify_code_lookup(found.revoked_at, found.used_at, found.expires_at, NOW),
        CodeLookupOutcome::Revoked
    );
}

/// RFC-047 §8.1: fixed decision order. A record that is both revoked and
/// expired must classify as `Revoked`, not `Expired`.
async fn test_revoked_and_expired_classifies_revoked<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("cre", "bothsec", EXPIRED, None))
        .await
        .unwrap();
    store
        .revoke_code(&CodeId::new("cre".into()), None, NOW)
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("bothsec")], NOW, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        classify_code_lookup(found.revoked_at, found.used_at, found.expires_at, NOW),
        CodeLookupOutcome::Revoked,
        "revoked must win over expired"
    );
}

/// RFC-022: exactly one concurrent claim winner (RFC-023 requirement).
///
/// Runs `CONCURRENCY` tasks all attempting to claim the same code concurrently.
/// Exactly one must return `Won`; the rest must return `Lost`.
///
/// Uses `spawn_local` on a `LocalSet` so the test works with store
/// implementations whose futures are `!Send` (e.g. Cloudflare Workers D1).
async fn test_exactly_one_claim_winner<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    const CONCURRENCY: usize = 8;
    let store = Arc::new(factory().await);
    store
        .insert_code(code_record("race", "racesec", LATER, None))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("racesec")], NOW, None)
        .await
        .unwrap()
        .unwrap();
    let code_id = found.id.clone();

    // Use a tokio barrier to maximise overlap between the concurrent claims.
    let barrier = Arc::new(tokio::sync::Barrier::new(CONCURRENCY));
    let local = tokio::task::LocalSet::new();

    let mut handles = Vec::with_capacity(CONCURRENCY);
    for i in 0..CONCURRENCY {
        let store = Arc::clone(&store);
        let code_id = code_id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(local.spawn_local(async move {
            barrier.wait().await;
            store
                .claim_code(&ClaimRequest {
                    code_id: &code_id,
                    subject: &SubjectId::new(format!("u{i}")),
                    now: NOW,
                    purpose: None,
                    scope: None,
                })
                .await
                .unwrap()
        }));
    }

    local.await;

    let outcomes: Vec<ClaimOutcome> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("task panicked"))
        .collect();

    let wins = outcomes.iter().filter(|o| **o == ClaimOutcome::Won).count();
    assert_eq!(wins, 1, "exactly one winner expected, got {wins}");
}

async fn test_scope_revoke_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    let rec = code_record("cs1", "scopedsec", LATER, Some("scope-A"));
    store.insert_code(rec).await.unwrap();
    store
        .revoke_code(&CodeId::new("cs1".into()), Some("scope-A"), NOW)
        .await
        .unwrap();
    // RFC-047: scope is still a real lookup criterion (unlike state) -- the
    // record within the matching scope is still found by lookup key + scope;
    // the classifier is what rejects it now that it has been revoked.
    let found = store
        .find_redeemable(&[code_lk("scopedsec")], NOW, Some("scope-A"))
        .await
        .unwrap()
        .expect("scope revoke: the record must still be found by lookup key + scope");
    assert!(found.revoked_at.is_some());
    assert_eq!(
        classify_code_lookup(found.revoked_at, found.used_at, found.expires_at, NOW),
        CodeLookupOutcome::Revoked
    );
}

async fn test_wrong_scope_does_not_revoke<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: CodeStore + Send + Sync + 'static,
{
    let store = factory().await;
    store
        .insert_code(code_record("cs2", "scoped2sec", LATER, Some("scope-A")))
        .await
        .unwrap();
    // Attempt to revoke using the wrong scope.
    store
        .revoke_code(&CodeId::new("cs2".into()), Some("scope-B"), NOW)
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[code_lk("scoped2sec")], NOW, Some("scope-A"))
        .await
        .unwrap();
    assert!(
        found.is_some(),
        "wrong scope revoke: record must still be redeemable"
    );
}
