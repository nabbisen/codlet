//! Acceptance tests for RFC-055: a configured rate-limit policy with no
//! `rate_key` is a configuration error, not a silent opt-out.
//!
//! Before RFC-055, `CodeAuth::find`/`claim` applied the limiter only when both
//! a policy *and* a key were present, so `CodeAuth::new(.., policy)` followed by
//! `find(code, None)` ran with no rate limiting, no error and no audit event.
//! These tests count calls on both stores so they prove the request was
//! rejected *before any work*, not merely that it failed.
#![cfg(feature = "test-utils")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use codlet::audit::NoopAuditSink;
use codlet::auth::{CodeAuth, NoRateLimit, RedeemError};
use codlet::clock::FixedClock;
use codlet::error::PublicRedemptionError;
use codlet::hashing::{SecretHasher, StaticKeyProvider};
use codlet::mem::{MemCodeStore, MemRateLimitStore};
use codlet::rng::SystemRandom;
use codlet::secret::{CodeId, SubjectId};
use codlet::state::ClaimOutcome;
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore, StoredCode};
use codlet::store::error::StoreError;
use codlet::store::ratelimit::{RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore};
use codlet::{CodePolicy, LookupKey};

const NOW: u64 = 1_700_000_000;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", vec![3u8; 32]).unwrap())
}

fn policy() -> CodePolicy {
    CodePolicy::default_human(Duration::from_secs(3600)).unwrap()
}

/// A `CodeStore` that delegates to the in-memory store and counts every call.
#[derive(Clone, Default)]
struct CountingCodeStore {
    inner: Arc<MemCodeStore>,
    calls: Arc<AtomicUsize>,
}

impl CountingCodeStore {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    fn hit(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

impl CodeStore for CountingCodeStore {
    async fn lookup_code(
        &self,
        candidates: &[LookupKey],
        scope: Option<&str>,
    ) -> Result<Option<StoredCode>, StoreError> {
        self.hit();
        self.inner.lookup_code(candidates, scope).await
    }
    async fn claim_code(&self, req: &ClaimRequest<'_>) -> Result<ClaimOutcome, StoreError> {
        self.hit();
        self.inner.claim_code(req).await
    }
    async fn insert_code(&self, record: CodeRecord) -> Result<(), StoreError> {
        self.hit();
        self.inner.insert_code(record).await
    }
    async fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
        now: u64,
    ) -> Result<(), StoreError> {
        self.hit();
        self.inner.revoke_code(code_id, scope, now).await
    }
}

/// A `RateLimitStore` that delegates to the in-memory store and counts each
/// operation separately.
#[derive(Clone, Default)]
struct CountingRateLimitStore {
    inner: Arc<MemRateLimitStore>,
    checks: Arc<AtomicUsize>,
    failures: Arc<AtomicUsize>,
    clears: Arc<AtomicUsize>,
}

impl CountingRateLimitStore {
    fn total(&self) -> usize {
        self.checks.load(Ordering::SeqCst)
            + self.failures.load(Ordering::SeqCst)
            + self.clears.load(Ordering::SeqCst)
    }
}

impl RateLimitStore for CountingRateLimitStore {
    async fn check(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitOutcome, StoreError> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        self.inner.check(key, policy).await
    }
    async fn record_failure(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<(), StoreError> {
        self.failures.fetch_add(1, Ordering::SeqCst);
        self.inner.record_failure(key, policy).await
    }
    async fn clear_failures(&self, key: &RateLimitKey) -> Result<(), StoreError> {
        self.clears.fetch_add(1, Ordering::SeqCst);
        self.inner.clear_failures(key).await
    }
}

type Limited = CodeAuth<
    CountingCodeStore,
    CountingRateLimitStore,
    StaticKeyProvider,
    FixedClock,
    NoopAuditSink,
>;

fn limited() -> (Limited, CountingCodeStore, CountingRateLimitStore) {
    let store = CountingCodeStore::default();
    let rl = CountingRateLimitStore::default();
    let auth = CodeAuth::new(
        store.clone(),
        rl.clone(),
        hasher(),
        FixedClock::at(NOW),
        NoopAuditSink,
        policy(),
        RateLimitPolicy::default_invite(),
    );
    (auth, store, rl)
}

async fn issue(auth: &Limited) -> String {
    let mut rng = SystemRandom::new();
    let (_, plain) = auth
        .issue_code(&mut rng, CodeId::new("c1".into()), None, None, None)
        .await
        .unwrap();
    plain.expose().to_string()
}

fn assert_misconfiguration(err: &RedeemError) {
    match err {
        RedeemError::Misconfigured { cause, public } => {
            assert!(cause.contains("rate_key"), "cause should name the problem");
            assert_eq!(*public, PublicRedemptionError::TemporarilyUnavailable);
        }
        other => panic!("expected RedeemError::Misconfigured, got {other:?}"),
    }
}

// 1. Policy configured, rate_key None → error, before any store access.

#[tokio::test]
async fn find_with_policy_and_no_key_is_rejected_before_any_store_access() {
    let (auth, store, rl) = limited();
    let code = issue(&auth).await;
    let (store_before, rl_before) = (store.calls(), rl.total());

    let err = auth.find(&code, None).await.unwrap_err();

    assert_misconfiguration(&err);
    assert_eq!(
        store.calls(),
        store_before,
        "code store must not be touched"
    );
    assert_eq!(
        rl.total(),
        rl_before,
        "rate-limit store must not be touched"
    );
}

#[tokio::test]
async fn claim_with_policy_and_no_key_is_rejected_before_any_store_access() {
    let (auth, store, rl) = limited();
    let code = issue(&auth).await;
    let key = RateLimitKey::new("client");
    let found = auth.find(&code, Some(&key)).await.unwrap();
    let (store_before, rl_before) = (store.calls(), rl.total());

    let err = auth
        .claim(&found, SubjectId::new("user-1".into()), None)
        .await
        .unwrap_err();

    assert_misconfiguration(&err);
    assert_eq!(store.calls(), store_before, "claim_code must not be called");
    assert_eq!(
        rl.total(),
        rl_before,
        "rate-limit store must not be touched"
    );
    // And the code is still redeemable: nothing was consumed.
    let again = auth.find(&code, Some(&key)).await.unwrap();
    auth.claim(&again, SubjectId::new("user-1".into()), Some(&key))
        .await
        .expect("an uncounted rejection must not have consumed the code");
}

// 2. No policy and no key (the deprecated opt-out) → unchanged.

#[tokio::test]
// RFC-055's regression: the deliberate, deprecated opt-out must still work.
#[allow(deprecated)]
async fn without_rate_limit_still_accepts_no_key() {
    let auth: CodeAuth<MemCodeStore, NoRateLimit, StaticKeyProvider, FixedClock, NoopAuditSink> =
        CodeAuth::without_rate_limit(
            MemCodeStore::new(),
            hasher(),
            FixedClock::at(NOW),
            NoopAuditSink,
            policy(),
        );
    let mut rng = SystemRandom::new();
    let (_, plain) = auth
        .issue_code(&mut rng, CodeId::new("c2".into()), None, None, None)
        .await
        .unwrap();
    let found = auth.find(plain.expose(), None).await.unwrap();
    auth.claim(&found, SubjectId::new("user-2".into()), None)
        .await
        .expect("no policy and no key is the legitimate opt-out");
}

// 3. Policy with a real key → unchanged, including clear_failures on success
//    (the nested success-path site) and record_failure on a bad guess.

#[tokio::test]
async fn policy_with_key_checks_records_and_clears() {
    let (auth, _store, rl) = limited();
    let code = issue(&auth).await;
    let key = RateLimitKey::new("client");

    // A wrong guess: checked, then a failure recorded.
    assert!(auth.find("ZZZZZZZZ", Some(&key)).await.is_err());
    assert_eq!(rl.checks.load(Ordering::SeqCst), 1);
    assert_eq!(rl.failures.load(Ordering::SeqCst), 1);
    assert_eq!(rl.clears.load(Ordering::SeqCst), 0);

    // The right code: found, claimed, and the counter cleared exactly once.
    let found = auth.find(&code, Some(&key)).await.unwrap();
    auth.claim(&found, SubjectId::new("user-3".into()), Some(&key))
        .await
        .unwrap();
    assert_eq!(
        rl.clears.load(Ordering::SeqCst),
        1,
        "a successful claim must clear the failure counter"
    );
}

// 4. The public error reveals nothing about configuration.

#[tokio::test]
async fn public_error_reveals_nothing_about_configuration() {
    let (auth, _store, _rl) = limited();
    let err = auth.find("ABCD2345", None).await.unwrap_err();

    let public = err.public();
    assert_eq!(*public, PublicRedemptionError::TemporarilyUnavailable);
    // The same opaque error an ordinary transient failure already surfaces.
    let shown = public.to_string().to_lowercase();
    for word in ["rate", "limit", "policy", "key", "config"] {
        assert!(
            !shown.contains(word),
            "public message leaks `{word}`: {shown}"
        );
    }
}
