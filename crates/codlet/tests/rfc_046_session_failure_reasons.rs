//! Acceptance tests for RFC-046 (host-visible session failure reasons),
//! exercised through the real [`SessionManager::validate`] API rather than by
//! constructing `SessionFailure` values directly (handoff §4.4: "a test that
//! builds the value proves the enum compiles, not that the code path produces
//! it").
//!
//! `IdleTimeout`'s real-condition test lives in `rfc_044_idle_timeout.rs`
//! (`idle_expired_session_is_unauthenticated_through_the_real_manager`) since
//! it requires RFC-044's idle-timeout machinery to be reachable at all
//! (handoff §4.3). That same file's
//! `absolute_expiry_still_enforced_with_idle_timeout_enabled` covers
//! `Expired` in the presence of a configured idle timeout (RFC-047 step 2:
//! absolute expiry wins over idle timeout in the decision order). This file
//! adds the ones specific to RFC-046's own boundary: `NoCookie`, `Malformed`,
//! `NotFound`, and `Revoked` (the last of these was, pre-RFC-047-step-2, a
//! disclosed gap where revocation collapsed to `NotFound` -- see
//! `revoked_session_is_reported_as_revoked` below).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codlet::audit::{AuditSink, CodeAuthEvent, CollectingAuditSink};
use codlet::auth::SessionManager;
use codlet::clock::FixedClock;
use codlet::cookie::CookiePolicy;
use codlet::hashing::{SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemSessionStore;
use codlet::secret::{SessionId, SubjectId};
use codlet::state::{SessionFailure, SessionValidationOutcome};
use codlet::store::session::{SessionRecord, SessionStore};

const NOW: u64 = 1_700_000_000;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"test-key-32bytes".to_vec()).unwrap())
}

fn cookie() -> CookiePolicy {
    CookiePolicy::production_strict("sid", Duration::from_secs(30 * 86_400))
}

fn mgr(
    audit: CollectingAuditSink,
) -> SessionManager<MemSessionStore, StaticKeyProvider, FixedClock, CollectingAuditSink> {
    SessionManager::new(
        MemSessionStore::new(),
        hasher(),
        FixedClock::at(NOW),
        audit,
        cookie(),
    )
}

// A well-formed (64 lowercase hex chars) secret, matching `hex_lower`'s
// output shape, but never inserted -- exercises the NotFound path.
const WELL_FORMED_UNKNOWN: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A shared audit sink whose events remain inspectable after being moved into
/// a `SessionManager` by value -- `CollectingAuditSink` itself has no
/// externally-orphan-rule-friendly way to do this from outside the crate, so
/// this test file defines its own thin `Arc`-backed one.
#[derive(Clone, Default)]
struct SharedAuditSink(Arc<Mutex<Vec<CodeAuthEvent>>>);

impl SharedAuditSink {
    fn events(&self) -> Vec<CodeAuthEvent> {
        self.0.lock().unwrap().clone()
    }
}

impl AuditSink for SharedAuditSink {
    fn record(&self, event: CodeAuthEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn no_cookie_reports_no_cookie() {
    let m = mgr(CollectingAuditSink::new());
    let outcome = m.validate(None).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::NoCookie
        }
    );
}

#[tokio::test]
async fn no_cookie_does_not_emit_a_validate_failed_audit_event() {
    // RFC-046 does not ask for this, but it protects the pre-existing
    // "SessionValidateFailed is opt-in signal, not every anonymous page view"
    // contract (audit.rs) from silently changing meaning now that `validate`
    // is called for every request, cookie or not.
    let audit = SharedAuditSink::default();
    let m = SessionManager::new(
        MemSessionStore::new(),
        hasher(),
        FixedClock::at(NOW),
        audit.clone(),
        cookie(),
    );
    let _ = m.validate(None).await.unwrap();
    assert!(
        audit.events().is_empty(),
        "an anonymous request (no cookie) must not emit session.validate.failed"
    );
}

#[tokio::test]
async fn malformed_cookie_is_rejected_before_touching_the_store() {
    let m = mgr(CollectingAuditSink::new());
    // Too short to be a real session secret (64 lowercase hex chars).
    let outcome = m.validate(Some("not-a-real-secret")).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Malformed
        }
    );
}

#[tokio::test]
async fn uppercase_hex_is_malformed_not_matched_case_insensitively() {
    // hex_lower only ever emits lowercase; codlet does not re-derive a
    // lookup key case-insensitively (unlike code normalization), so
    // uppercase input is a shape violation, not a valid-but-unknown secret.
    let m = mgr(CollectingAuditSink::new());
    let uppercase = WELL_FORMED_UNKNOWN.to_ascii_uppercase();
    let outcome = m.validate(Some(&uppercase)).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Malformed
        }
    );
}

#[tokio::test]
async fn well_formed_but_unknown_secret_reports_not_found() {
    let m = mgr(CollectingAuditSink::new());
    let outcome = m.validate(Some(WELL_FORMED_UNKNOWN)).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::NotFound
        }
    );
}

#[tokio::test]
async fn revoked_session_is_reported_as_revoked() {
    // The gap this test used to document (RFC-046 review request: Revoked
    // collapsed to NotFound because find_active_session's active-row filter
    // couldn't tell "revoked" apart from "never issued") is closed by
    // RFC-047 step 2: find_active_session returns the record regardless of
    // state, and classify_session reports the precise reason.
    let store = MemSessionStore::new();
    let h = hasher();
    let (lk, kv) = h
        .lookup_key(SecretDomain::Session, WELL_FORMED_UNKNOWN)
        .unwrap();
    store
        .insert_session(SessionRecord {
            id: SessionId::new("sess-revoked".into()),
            lookup_key: lk,
            key_version: kv,
            subject: SubjectId::new("user-1".into()),
            created_at: NOW,
            expires_at: NOW + 100_000,
        })
        .await
        .unwrap();
    store
        .revoke_session(&SessionId::new("sess-revoked".into()), NOW)
        .await
        .unwrap();

    let m = SessionManager::new(
        store,
        hasher(),
        FixedClock::at(NOW),
        CollectingAuditSink::new(),
        cookie(),
    );
    let outcome = m.validate(Some(WELL_FORMED_UNKNOWN)).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );
}

#[tokio::test]
async fn a_failed_validation_with_a_real_cookie_does_emit_the_audit_event() {
    let audit = SharedAuditSink::default();
    let m = SessionManager::new(
        MemSessionStore::new(),
        hasher(),
        FixedClock::at(NOW),
        audit.clone(),
        cookie(),
    );
    let outcome = m.validate(Some(WELL_FORMED_UNKNOWN)).await.unwrap();
    assert!(!outcome.is_authenticated());
    assert_eq!(
        audit.events(),
        vec![CodeAuthEvent::SessionValidateFailed],
        "a genuine failed validation attempt (real cookie, wrong/unknown) \
         must still emit session.validate.failed -- only NoCookie is exempt"
    );
}
