//! Session-store conformance tests (RFC-023).

use std::future::Future;

use crate::fixtures::*;
use codlet::secret::SessionId;
use codlet::state::{SessionFailure, SessionValidationOutcome, classify_session};

// ── SessionStore conformance ──────────────────────────────────────────────────

/// Run the full session-store conformance suite.
pub async fn run_session_store_conformance<F, Fut, S>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    test_active_session_found(&factory).await;
    test_expired_session_returned_and_classifier_rejects(&factory).await;
    test_revoked_session_returned_and_classifier_rejects(&factory).await;
    test_revoked_and_expired_classifies_revoked(&factory).await;
    test_wrong_hmac_not_active(&factory).await;
    test_new_session_has_no_last_seen_at(&factory).await;
    test_touch_session_sets_last_seen_at(&factory).await;
    test_touch_session_overwrites_previous_value(&factory).await;
    test_rotation_overlap_then_old_revoked(&factory).await;
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

/// RFC-047 step 2: this must fail against an adapter that kept its old
/// exclusion filter, not merely pass against a migrated one. An adapter that
/// still filters expired rows out of `find_active_session` returns `None`
/// here, which trips the `.expect(...)` below -- the inverted assertion
/// carries the security property this suite exists to prove (verified by
/// temporarily reintroducing the filter in `MemSessionStore` and confirming
/// this test fails; see the RFC-047 step 2 review request).
async fn test_expired_session_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
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
        .unwrap()
        .expect("RFC-047: the store must return the expired record, not filter it out");
    assert_eq!(found.expires_at, EXPIRED);
    assert_eq!(
        classify_session(Some(found), None, NOW),
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Expired
        },
        "the classifier must reject the returned record"
    );
}

/// See `test_expired_session_returned_and_classifier_rejects` for why this
/// asserts return-and-reject rather than exclusion.
async fn test_revoked_session_returned_and_classifier_rejects<F, Fut, S>(factory: &F)
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
        .unwrap()
        .expect("RFC-047: the store must return the revoked record, not filter it out");
    assert!(found.revoked_at.is_some());
    assert_eq!(
        classify_session(Some(found), None, NOW),
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );
}

/// RFC-047 §8.1 (restated for sessions): fixed decision order. A record that
/// is both revoked and expired must classify as `Revoked`, not `Expired`.
async fn test_revoked_and_expired_classifies_revoked<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s3e", "bothsessec", EXPIRED))
        .await
        .unwrap();
    store
        .revoke_session(&SessionId::new("s3e".into()), NOW)
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("bothsessec")], NOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        classify_session(Some(found), None, NOW),
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        },
        "revoked must win over expired"
    );
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

// ── RFC-044: touch_session and last_seen_at ─────────────────────────────────

async fn test_new_session_has_no_last_seen_at<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s5", "sessec5", LATER))
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec5")], NOW)
        .await
        .unwrap()
        .expect("active session must be found");
    assert_eq!(
        found.last_seen_at, None,
        "a never-touched session must read back last_seen_at = NULL, not a \
         backfilled value (RFC-044 §5: additive, no backfill)"
    );
    assert_eq!(
        found.created_at, NOW,
        "created_at must round-trip through the store"
    );
}

async fn test_touch_session_sets_last_seen_at<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s6", "sessec6", LATER))
        .await
        .unwrap();
    store
        .touch_session(&SessionId::new("s6".into()), NOW + 10)
        .await
        .unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec6")], NOW)
        .await
        .unwrap()
        .expect("active session must be found");
    assert_eq!(found.last_seen_at, Some(NOW + 10));
}

async fn test_touch_session_overwrites_previous_value<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    store
        .insert_session(session_record("s7", "sessec7", LATER))
        .await
        .unwrap();
    let id = SessionId::new("s7".into());
    store.touch_session(&id, NOW + 10).await.unwrap();
    store.touch_session(&id, NOW + 20).await.unwrap();
    let found = store
        .find_active_session(&[session_lk("sessec7")], NOW)
        .await
        .unwrap()
        .expect("active session must be found");
    assert_eq!(
        found.last_seen_at,
        Some(NOW + 20),
        "a later touch must overwrite, not merely set-if-absent"
    );
}

// ── RFC-045: what session rotation composes from existing store ops ────────
//
// `SessionManager::rotate` adds no new store trait method (RFC-045 §5) — it
// composes `insert_session` and `revoke_session`, in that order, on purpose
// (RFC-045 §3.2: revoking first would leave a window where neither record is
// valid). This test proves every adapter actually supports the sequence
// rotation depends on: two live records for the same subject can coexist
// briefly, and revoking the old one afterward does not disturb the new one.

async fn test_rotation_overlap_then_old_revoked<F, Fut, S>(factory: &F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
    S: SessionStore,
{
    let store = factory().await;
    let subject = SubjectId::new("user-rot".into());

    store
        .insert_session(SessionRecord {
            id: SessionId::new("rot-old".into()),
            lookup_key: session_lk("rot-old-secret"),
            key_version: kv(),
            subject: subject.clone(),
            created_at: NOW,
            expires_at: LATER,
        })
        .await
        .unwrap();

    // §3.2: insert the new record while the old one is still unrevoked.
    // Both must be simultaneously active -- the overlap is deliberate, not a
    // bug the store is expected to prevent.
    store
        .insert_session(SessionRecord {
            id: SessionId::new("rot-new".into()),
            lookup_key: session_lk("rot-new-secret"),
            key_version: kv(),
            subject: subject.clone(),
            created_at: NOW,
            expires_at: LATER,
        })
        .await
        .unwrap();

    let old_before = store
        .find_active_session(&[session_lk("rot-old-secret")], NOW)
        .await
        .unwrap()
        .expect("the old record must still be active during the overlap window");
    assert!(old_before.revoked_at.is_none());
    let new_during_overlap = store
        .find_active_session(&[session_lk("rot-new-secret")], NOW)
        .await
        .unwrap()
        .expect("the new record must already be active during the overlap window");
    assert_eq!(new_during_overlap.subject.as_str(), "user-rot");

    // Now revoke the old one, as `rotate`'s final step does.
    store
        .revoke_session(&SessionId::new("rot-old".into()), NOW)
        .await
        .unwrap();

    let old_after = store
        .find_active_session(&[session_lk("rot-old-secret")], NOW)
        .await
        .unwrap()
        .expect("RFC-047: the store must return the revoked record, not filter it out");
    assert_eq!(
        classify_session(Some(old_after), None, NOW),
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );

    // Revoking the old record must not disturb the new one.
    let new_after = store
        .find_active_session(&[session_lk("rot-new-secret")], NOW)
        .await
        .unwrap()
        .expect("the new record must remain active after the old one is revoked");
    assert!(
        classify_session(Some(new_after), None, NOW).is_authenticated(),
        "revoking the old session must not affect the new one"
    );
}
