//! Code-store conformance tests (RFC-023).

use std::future::Future;
use std::sync::Arc;

use crate::fixtures::*;
use codlet_core::state::ClaimOutcome;

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
    test_expired_not_redeemable(&factory).await;
    test_used_not_redeemable(&factory).await;
    test_revoked_not_redeemable(&factory).await;
    test_exactly_one_claim_winner(&factory).await;
    test_scope_revoke_works(&factory).await;
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

async fn test_expired_not_redeemable<F, Fut, S>(factory: &F)
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
        .unwrap();
    assert!(found.is_none(), "expired: must not be redeemable");
}

async fn test_used_not_redeemable<F, Fut, S>(factory: &F)
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
        .unwrap();
    assert!(again.is_none(), "used: must not be redeemable after claim");
}

async fn test_revoked_not_redeemable<F, Fut, S>(factory: &F)
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
        .unwrap();
    assert!(found.is_none(), "revoked: must not be redeemable");
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

async fn test_scope_revoke_works<F, Fut, S>(factory: &F)
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
    let found = store
        .find_redeemable(&[code_lk("scopedsec")], NOW, Some("scope-A"))
        .await
        .unwrap();
    assert!(
        found.is_none(),
        "scope revoke: record within scope must be revoked"
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
