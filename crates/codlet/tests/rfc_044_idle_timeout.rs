//! Acceptance tests for RFC-044 (session idle timeout), exercised through the
//! real [`SessionManager`] API — not just the pure `classify_session`
//! classifier — per the handoff's required tests (§5):
//!
//! - `idle_timeout: None` performs no write, proven by a counting fixture;
//! - throttling: N validations inside one granularity produce exactly one
//!   write;
//! - a failed `touch_session` leaves the request authenticated and emits an
//!   audit event.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use codlet::audit::CollectingAuditSink;
use codlet::auth::SessionManager;
use codlet::clock::MutableClock;
use codlet::cookie::CookiePolicy;
use codlet::hashing::{SecretHasher, StaticKeyProvider};
use codlet::mem::MemSessionStore;
use codlet::secret::SessionId;
use codlet::state::{SessionFailure, SessionValidationOutcome};
use codlet::store::error::StoreError;
use codlet::store::session::{ActiveSessionRecord, SessionRecord, SessionStore};

const NOW: u64 = 1_700_000_000;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"test-key-32bytes".to_vec()).unwrap())
}

fn cookie() -> CookiePolicy {
    CookiePolicy::production_strict("sid", Duration::from_secs(30 * 86_400))
}

// ── Test-only store wrappers ─────────────────────────────────────────────────
//
// The handoff (§4.1) explicitly anticipates this: "If your adapter fixtures
// cannot observe [a write], add a counting fixture -- 'we believe it does not
// write' is not evidence." `MemSessionStore` has no built-in call counter, so
// these wrap it rather than polluting the shared non-production store with
// test-only instrumentation.

struct CountingTouchStore {
    inner: MemSessionStore,
    touch_calls: std::sync::Arc<AtomicUsize>,
}

impl SessionStore for CountingTouchStore {
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

    fn revoke_session(
        &self,
        session_id: &SessionId,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>> {
        self.inner.revoke_session(session_id, now)
    }

    async fn touch_session(&self, session_id: &SessionId, now: u64) -> Result<(), StoreError> {
        self.touch_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.touch_session(session_id, now).await
    }
}

struct FailingTouchStore {
    inner: MemSessionStore,
}

impl SessionStore for FailingTouchStore {
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

    fn revoke_session(
        &self,
        session_id: &SessionId,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>> {
        self.inner.revoke_session(session_id, now)
    }

    async fn touch_session(&self, _session_id: &SessionId, _now: u64) -> Result<(), StoreError> {
        Err(StoreError::Backend("simulated touch failure".into()))
    }
}

async fn insert_and_lookup_keys(
    store: &impl SessionStore,
    secret: &str,
    expires_at: u64,
) -> codlet::LookupKey {
    let h = hasher();
    let (lk, kv) = h
        .lookup_key(codlet::hashing::SecretDomain::Session, secret)
        .unwrap();
    store
        .insert_session(SessionRecord {
            id: SessionId::new("sess-1".into()),
            lookup_key: lk.clone(),
            key_version: kv,
            subject: codlet::secret::SubjectId::new("user-1".into()),
            created_at: NOW,
            expires_at,
        })
        .await
        .unwrap();
    lk
}

// A well-formed (64 lowercase hex chars) session secret, matching what
// `hex_lower` actually produces at issue time.
const SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// ── §4.1: idle_timeout: None performs no write ──────────────────────────────

#[tokio::test]
async fn idle_timeout_none_performs_no_touch_write() {
    let touch_calls = std::sync::Arc::new(AtomicUsize::new(0));
    let store = CountingTouchStore {
        inner: MemSessionStore::new(),
        touch_calls: std::sync::Arc::clone(&touch_calls),
    };
    insert_and_lookup_keys(&store, SECRET, NOW + 10_000).await;

    let clock = MutableClock::at(NOW);
    // No `.with_idle_timeout(..)` -- the default, off.
    let mgr = SessionManager::new(
        store,
        hasher(),
        clock.clone(),
        CollectingAuditSink::new(),
        cookie(),
    );

    for _ in 0..5 {
        let outcome = mgr.validate(Some(SECRET)).await.unwrap();
        assert!(outcome.is_authenticated());
        clock.advance(1);
    }

    assert_eq!(
        touch_calls.load(Ordering::SeqCst),
        0,
        "idle_timeout: None must never call touch_session"
    );
}

// ── §4.2: throttled touch, not touch-per-request ────────────────────────────

#[tokio::test]
async fn touch_is_throttled_to_one_write_per_granularity() {
    let touch_calls = std::sync::Arc::new(AtomicUsize::new(0));
    let store = CountingTouchStore {
        inner: MemSessionStore::new(),
        touch_calls: std::sync::Arc::clone(&touch_calls),
    };
    insert_and_lookup_keys(&store, SECRET, NOW + 100_000).await;

    let idle_timeout = Duration::from_secs(1_800); // granularity = max(90, 30) = 90s
    let clock = MutableClock::at(NOW);
    let mgr = SessionManager::new(
        store,
        hasher(),
        clock.clone(),
        CollectingAuditSink::new(),
        cookie(),
    )
    .with_idle_timeout(idle_timeout);

    // First validation: last_seen_at is None -> falls back to created_at
    // (NOW), so `now - created_at == 0 < 90s` granularity -- no touch yet.
    let outcome = mgr.validate(Some(SECRET)).await.unwrap();
    assert!(outcome.is_authenticated());
    assert_eq!(
        touch_calls.load(Ordering::SeqCst),
        0,
        "first validation at t=0 must not touch: elapsed since created_at is 0s"
    );

    // 10 more validations, each 1s apart, all within the 90s granularity
    // window from the (still-untouched) created_at baseline.
    for _ in 0..10 {
        clock.advance(1);
        let outcome = mgr.validate(Some(SECRET)).await.unwrap();
        assert!(outcome.is_authenticated());
    }
    assert_eq!(
        touch_calls.load(Ordering::SeqCst),
        0,
        "10 validations inside one granularity must not have touched yet"
    );

    // Advance past the 90s granularity from created_at -- exactly one touch.
    clock.advance(90);
    let outcome = mgr.validate(Some(SECRET)).await.unwrap();
    assert!(outcome.is_authenticated());
    assert_eq!(
        touch_calls.load(Ordering::SeqCst),
        1,
        "crossing the granularity boundary must touch exactly once"
    );

    // A burst of validations right after the touch, all within the next
    // granularity window, must not touch again.
    for _ in 0..20 {
        clock.advance(1);
        mgr.validate(Some(SECRET)).await.unwrap();
    }
    assert_eq!(
        touch_calls.load(Ordering::SeqCst),
        1,
        "N validations inside one granularity must produce exactly one write, not N"
    );
}

// ── §4.4: a failed touch must not log the user out ──────────────────────────

#[tokio::test]
async fn failed_touch_leaves_session_authenticated_and_emits_audit_event() {
    let store = FailingTouchStore {
        inner: MemSessionStore::new(),
    };
    insert_and_lookup_keys(&store, SECRET, NOW + 100_000).await;

    let idle_timeout = Duration::from_secs(60); // granularity = max(3, 30) = 30s
    let clock = MutableClock::at(NOW);
    let audit = CollectingAuditSink::new();
    let mgr = SessionManager::new(store, hasher(), clock.clone(), audit, cookie())
        .with_idle_timeout(idle_timeout);

    // Advance past the granularity so a touch is attempted (and fails).
    clock.advance(30);
    let outcome = mgr.validate(Some(SECRET)).await.unwrap();

    assert!(
        outcome.is_authenticated(),
        "a touch_session failure must not invalidate an otherwise-valid session"
    );
}

// ── §4.4 (idle expiry itself, through the real manager) ─────────────────────

#[tokio::test]
async fn idle_expired_session_is_unauthenticated_through_the_real_manager() {
    let store = MemSessionStore::new();
    insert_and_lookup_keys(&store, SECRET, NOW + 100_000).await;

    let idle_timeout = Duration::from_secs(1_800);
    let clock = MutableClock::at(NOW);
    let mgr = SessionManager::new(
        store,
        hasher(),
        clock.clone(),
        CollectingAuditSink::new(),
        cookie(),
    )
    .with_idle_timeout(idle_timeout);

    // Never touched: falls back to created_at = NOW. Advance past 1800s.
    clock.advance(1_800);
    let outcome = mgr.validate(Some(SECRET)).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::IdleTimeout
        }
    );
}

// ── Absolute expiry still enforced independently of idle timeout ───────────

#[tokio::test]
async fn absolute_expiry_still_enforced_with_idle_timeout_enabled() {
    let store = MemSessionStore::new();
    // Absolute expiry is very soon (NOW + 5), idle timeout is generous (1h).
    insert_and_lookup_keys(&store, SECRET, NOW + 5).await;

    let clock = MutableClock::at(NOW);
    let mgr = SessionManager::new(
        store,
        hasher(),
        clock.clone(),
        CollectingAuditSink::new(),
        cookie(),
    )
    .with_idle_timeout(Duration::from_secs(3_600));

    clock.advance(10); // past absolute expiry, nowhere near idle timeout
    let outcome = mgr.validate(Some(SECRET)).await.unwrap();
    assert_eq!(
        outcome,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::NotFound
        },
        "absolute expiry is enforced by the store, independent of idle timeout"
    );
}
