//! Session-store conformance tests (RFC-023).

use std::future::Future;

use crate::fixtures::*;
use codlet::secret::SessionId;

// ── SessionStore conformance ──────────────────────────────────────────────────

/// Run the full session-store conformance suite.
pub async fn run_session_store_conformance<F, Fut, S>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    test_active_session_found(&factory).await;
    test_expired_session_not_active(&factory).await;
    test_revoked_session_not_active(&factory).await;
    test_wrong_hmac_not_active(&factory).await;
}

async fn test_active_session_found<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s1", "sessec1", LATER))
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec1")], NOW)
        .await
        .unwrap();
    assert!(found.is_some(), "active session must be found");
    assert_eq!(found.unwrap().subject.as_str(), "user-s1");
}

async fn test_expired_session_not_active<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s2", "sessec2", EXPIRED))
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec2")], NOW)
        .await
        .unwrap();
    assert!(found.is_none(), "expired session must not be active");
}

async fn test_revoked_session_not_active<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s3", "sessec3", LATER))
        .await
        .unwrap();
    store
        .revoke_session(&SessionId::new("s3".into()), NOW)
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec3")], NOW)
        .await
        .unwrap();
    assert!(found.is_none(), "revoked session must not be active");
}

async fn test_wrong_hmac_not_active<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s4", "sessec4", LATER))
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("wrong-secret")], NOW)
        .await
        .unwrap();
    assert!(found.is_none(), "wrong HMAC must not match");
}
