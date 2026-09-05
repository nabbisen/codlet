//! Acceptance tests for RFC-047 (the code path): `Expired`, `Revoked`, and
//! `AlreadyUsed` are now reachable through `CodeAuth::find`'s real lookup
//! path, not only from a lost `claim_code` race. Each test builds the store
//! into the exact state under test (using `CodeStore` directly, the same way
//! the RFC-005 acceptance suite does) before wrapping it in `CodeAuth`,
//! rather than constructing a `RedemptionFailReason` value directly, and
//! confirms the public error surface is unchanged (handoff §5.3 / acceptance
//! criterion 6): every one of these still collapses to `InvalidOrExpired`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codlet::audit::{AuditSink, CodeAuthEvent};
use codlet::auth::{CodeAuth, NoRateLimit};
use codlet::clock::FixedClock;
use codlet::error::{PublicRedemptionError, RedemptionFailReason};
use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemCodeStore;
use codlet::secret::{CodeId, SubjectId};
use codlet::state::ClaimOutcome;
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore};
use codlet::{CodePolicy, LookupKey};

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;
const EXPIRED: u64 = NOW - 1;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"test-key-32bytes".to_vec()).unwrap())
}

fn code_lk(val: &str) -> LookupKey {
    hasher().lookup_key(SecretDomain::Code, val).unwrap().0
}

/// A shared audit sink whose events remain inspectable after being moved into
/// a `CodeAuth` by value.
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

/// Insert a code record directly into `store`, in the given state.
async fn insert(store: &MemCodeStore, code: &str, expires_at: u64) -> CodeId {
    let id = CodeId::new(format!("id-{code}"));
    store
        .insert_code(CodeRecord {
            id: id.clone(),
            lookup_key: code_lk(code),
            key_version: KeyVersion::new("v1"),
            purpose: None,
            scope: None,
            grant: Some("grant".into()),
            created_at: NOW,
            expires_at,
        })
        .await
        .unwrap();
    id
}

fn code_auth(
    store: MemCodeStore,
    audit: SharedAuditSink,
) -> CodeAuth<MemCodeStore, NoRateLimit, StaticKeyProvider, FixedClock, SharedAuditSink> {
    CodeAuth::without_rate_limit(
        store,
        hasher(),
        FixedClock::at(NOW),
        audit,
        CodePolicy::default_human(Duration::from_secs(3600)).unwrap(),
    )
}

#[tokio::test]
async fn expired_code_reports_expired_internally_and_invalid_or_expired_publicly() {
    let store = MemCodeStore::new();
    insert(&store, "EXPCARD2", EXPIRED).await;
    let audit = SharedAuditSink::default();
    let ca = code_auth(store, audit.clone());

    let err = ca.find("EXPCARD2", None).await.unwrap_err();
    assert_eq!(*err.public(), PublicRedemptionError::InvalidOrExpired);
    assert_eq!(
        audit.events(),
        vec![CodeAuthEvent::RedemptionFailed {
            reason: RedemptionFailReason::Expired
        }],
        "the audit event must carry the true reason, not NotFound"
    );
}

#[tokio::test]
async fn revoked_code_reports_revoked_internally_and_invalid_or_expired_publicly() {
    let store = MemCodeStore::new();
    let id = insert(&store, "REVCARD3", LATER).await;
    store.revoke_code(&id, None, NOW).await.unwrap();
    let audit = SharedAuditSink::default();
    let ca = code_auth(store, audit.clone());

    let err = ca.find("REVCARD3", None).await.unwrap_err();
    assert_eq!(*err.public(), PublicRedemptionError::InvalidOrExpired);
    assert_eq!(
        audit.events(),
        vec![CodeAuthEvent::RedemptionFailed {
            reason: RedemptionFailReason::Revoked
        }],
    );
}

#[tokio::test]
async fn used_code_found_before_claim_reports_already_used_internally() {
    let store = MemCodeStore::new();
    let id = insert(&store, "USEDCRD4", LATER).await;
    // Claim it directly (simulating a second lookup attempt on a code
    // someone else already claimed).
    let outcome = store
        .claim_code(&ClaimRequest {
            code_id: &id,
            subject: &SubjectId::new("someone-else".into()),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(outcome, ClaimOutcome::Won);

    let audit = SharedAuditSink::default();
    let ca = code_auth(store, audit.clone());

    let err = ca.find("USEDCRD4", None).await.unwrap_err();
    assert_eq!(*err.public(), PublicRedemptionError::InvalidOrExpired);
    assert_eq!(
        audit.events(),
        vec![CodeAuthEvent::RedemptionFailed {
            reason: RedemptionFailReason::AlreadyUsed
        }],
        "AlreadyUsed must be reachable from find()'s classifier too, not \
         only from a lost claim_code race"
    );
}

#[tokio::test]
async fn revoked_and_expired_code_reports_revoked_per_the_fixed_decision_order() {
    // RFC-047 §8.1: a record satisfying more than one condition classifies
    // by the fixed order -- revoked wins.
    let store = MemCodeStore::new();
    let id = insert(&store, "BTHCARD5", EXPIRED).await;
    store.revoke_code(&id, None, NOW).await.unwrap();
    let audit = SharedAuditSink::default();
    let ca = code_auth(store, audit.clone());

    let err = ca.find("BTHCARD5", None).await.unwrap_err();
    assert_eq!(*err.public(), PublicRedemptionError::InvalidOrExpired);
    assert_eq!(
        audit.events(),
        vec![CodeAuthEvent::RedemptionFailed {
            reason: RedemptionFailReason::Revoked
        }],
        "a record that is both revoked and expired must classify as Revoked"
    );
}
