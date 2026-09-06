//! Acceptance tests for RFC-045 (session rotation on privilege change),
//! exercised through the real [`SessionManager`] API per the handoff's
//! required tests (§6):
//!
//! - rotation succeeds and the new secret validates;
//! - the old secret reports `Unauthenticated { reason: Revoked }`, not
//!   merely "not authenticated" (§5.4 — testable precisely only because of
//!   RFC-047 step 2);
//! - `expires_at` is carried forward unchanged, asserted directly;
//! - a failed revoke still returns `Ok`, leaves the old session valid (proof
//!   that insert precedes revoke, §5.1), and fires a distinct audit event;
//! - the host-supplied reason reaches the audit event.
//!
//! The compile-fail proof that `rotate` cannot be handed a fabricated
//! `Authenticated` outcome lives in
//! `rfc_045_rotate_requires_authenticated_compile_fail.rs`, not here.

use std::future::Future;
use std::time::Duration;

use codlet::audit::{CodeAuthEvent, CollectingAuditSink};
use codlet::auth::SessionManager;
use codlet::clock::FixedClock;
use codlet::cookie::CookiePolicy;
use codlet::hashing::{SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemSessionStore;
use codlet::rng::SystemRandom;
use codlet::secret::{SessionId, SubjectId};
use codlet::state::{SessionFailure, SessionValidationOutcome};
use codlet::store::error::StoreError;
use codlet::store::session::{ActiveSessionRecord, SessionRecord, SessionStore};

const NOW: u64 = 1_700_000_000;
const OLD_EXPIRES_AT: u64 = NOW + 3_600;
const OLD_SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"test-key-32bytes".to_vec()).unwrap())
}

fn cookie() -> CookiePolicy {
    CookiePolicy::production_strict("sid", Duration::from_secs(30 * 86_400))
}

async fn insert_old_session(store: &impl SessionStore) {
    let h = hasher();
    let (lk, kv) = h.lookup_key(SecretDomain::Session, OLD_SECRET).unwrap();
    store
        .insert_session(SessionRecord {
            id: SessionId::new("sess-old".into()),
            lookup_key: lk,
            key_version: kv,
            subject: SubjectId::new("user-1".into()),
            created_at: NOW,
            expires_at: OLD_EXPIRES_AT,
        })
        .await
        .unwrap();
}

fn cookie_value(set_cookie: &str) -> &str {
    set_cookie
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("sid=")
}

// ── Test-only store wrapper: revoke always fails (§5.3's required fixture) ──
//
// `MemSessionStore` cannot be made to fail on demand, so this wraps it rather
// than polluting the shared non-production store with test-only behavior —
// same rationale as RFC-044's `FailingTouchStore` (rfc_044_idle_timeout.rs).

struct FailingRevokeStore {
    inner: MemSessionStore,
}

impl SessionStore for FailingRevokeStore {
    fn find_active_session(
        &self,
        candidates: &[codlet::LookupKey],
        now: u64,
    ) -> impl Future<Output = Result<Option<ActiveSessionRecord>, StoreError>> {
        self.inner.find_active_session(candidates, now)
    }

    fn insert_session(
        &self,
        record: SessionRecord,
    ) -> impl Future<Output = Result<(), StoreError>> {
        self.inner.insert_session(record)
    }

    async fn revoke_session(&self, _session_id: &SessionId, _now: u64) -> Result<(), StoreError> {
        Err(StoreError::Backend("simulated revoke failure".into()))
    }

    fn touch_session(
        &self,
        session_id: &SessionId,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>> {
        self.inner.touch_session(session_id, now)
    }
}

// ── Success path ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn rotation_succeeds_new_secret_validates_and_old_reports_revoked() {
    let store = MemSessionStore::new();
    insert_old_session(&store).await;

    let mgr = SessionManager::new(
        store,
        hasher(),
        FixedClock::at(NOW),
        CollectingAuditSink::new(),
        cookie(),
    );

    let current = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    assert!(current.is_authenticated());

    let mut rng = SystemRandom::new();
    let issued = mgr
        .rotate(
            &current,
            SessionId::new("sess-new".into()),
            "privilege_change",
            &mut rng,
        )
        .await
        .unwrap();
    assert_eq!(issued.session_id.as_str(), "sess-new");

    // The new secret validates.
    let new_secret = cookie_value(&issued.set_cookie);
    let new_outcome = mgr.validate(Some(new_secret)).await.unwrap();
    assert!(new_outcome.is_authenticated());
    assert_eq!(new_outcome.subject().unwrap().as_str(), "user-1");

    // §5.4: the old secret reports Revoked specifically, not a bare
    // "unauthenticated" -- distinguishing "this session was rotated away"
    // from "this cookie was never valid" is exactly the confusion RFC-046
    // existed to end.
    let old_outcome = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    assert_eq!(
        old_outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );
}

// ── §3.1: absolute expiry carried forward, not extended ─────────────────────

#[tokio::test]
async fn expires_at_is_carried_forward_unchanged() {
    let store = MemSessionStore::new();
    insert_old_session(&store).await;

    let mgr = SessionManager::new(
        store,
        hasher(),
        FixedClock::at(NOW),
        CollectingAuditSink::new(),
        cookie(),
    );

    let current = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    let SessionValidationOutcome::Authenticated { expires_at, .. } = &current else {
        panic!("expected Authenticated");
    };
    assert_eq!(*expires_at, OLD_EXPIRES_AT);

    let mut rng = SystemRandom::new();
    let issued = mgr
        .rotate(
            &current,
            SessionId::new("sess-new".into()),
            "privilege_change",
            &mut rng,
        )
        .await
        .unwrap();

    let new_secret = cookie_value(&issued.set_cookie);
    let new_outcome = mgr.validate(Some(new_secret)).await.unwrap();
    let SessionValidationOutcome::Authenticated { expires_at, .. } = new_outcome else {
        panic!("expected Authenticated");
    };
    assert_eq!(
        expires_at, OLD_EXPIRES_AT,
        "rotation must carry the absolute expiry forward unchanged, never extend it"
    );
}

// ── §3.3: a failed revoke returns Ok, with an audit event ───────────────────

#[tokio::test]
async fn failed_revoke_returns_ok_old_session_still_valid_and_audit_event_fires() {
    let store = FailingRevokeStore {
        inner: MemSessionStore::new(),
    };
    insert_old_session(&store).await;

    let audit = CollectingAuditSink::new();
    let mgr = SessionManager::new(
        store,
        hasher(),
        FixedClock::at(NOW),
        audit.clone(),
        cookie(),
    );

    let current = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    let mut rng = SystemRandom::new();
    let issued = mgr
        .rotate(
            &current,
            SessionId::new("sess-new".into()),
            "privilege_change",
            &mut rng,
        )
        .await
        .expect("a failed revoke must not turn rotation into an error (§3.3)");

    // The new session works.
    let new_secret = cookie_value(&issued.set_cookie);
    let new_outcome = mgr.validate(Some(new_secret)).await.unwrap();
    assert!(new_outcome.is_authenticated());

    // §5.1: proof that insert precedes revoke -- the old session is still
    // findable and authenticated, since the revoke attempt (which failed)
    // never touched it.
    let old_outcome = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    assert!(
        old_outcome.is_authenticated(),
        "insert must precede revoke: a failed revoke leaves the old session valid"
    );

    // The "and_audit_event_fires" half of this test's own name: the failure
    // must be surfaced, not merely swallowed -- assert the specific event,
    // naming both session ids, not just that rotation returned Ok (RFC-044
    // follow-up #2, Finding 2: this used to be asserted only by a
    // differently-named sibling test, never by the test whose name promised
    // it).
    let events = audit.drain();
    let found = events.iter().any(|e| {
        matches!(
            e,
            CodeAuthEvent::SessionRotationRevokeFailed { old_session_id, new_session_id }
                if old_session_id.as_str() == "sess-old" && new_session_id.as_str() == "sess-new"
        )
    });
    assert!(
        found,
        "a failed revoke must fire SessionRotationRevokeFailed naming both session ids; got {events:?}"
    );
}

// ── §8: the host-supplied reason reaches the audit event ────────────────────

#[tokio::test]
async fn reason_is_recorded_in_the_audit_event() {
    let store = MemSessionStore::new();
    insert_old_session(&store).await;

    let audit = CollectingAuditSink::new();
    let mgr = SessionManager::new(
        store,
        hasher(),
        FixedClock::at(NOW),
        audit.clone(),
        cookie(),
    );

    let current = mgr.validate(Some(OLD_SECRET)).await.unwrap();
    let mut rng = SystemRandom::new();
    mgr.rotate(
        &current,
        SessionId::new("sess-new".into()),
        "role_changed_to_admin",
        &mut rng,
    )
    .await
    .unwrap();

    let events = audit.drain();
    let found = events.iter().any(|e| {
        matches!(
            e,
            CodeAuthEvent::SessionRotated { reason, .. } if reason == "role_changed_to_admin"
        )
    });
    assert!(
        found,
        "SessionRotated must carry the host-supplied reason verbatim; got {events:?}"
    );
}
