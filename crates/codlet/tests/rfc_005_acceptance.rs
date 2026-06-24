//! Acceptance tests for RFC-005: code lifecycle (store find, claim, revoke).
use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemCodeStore;
use codlet::state::ClaimOutcome;
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore};

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;
const EXPIRED: u64 = NOW - 1;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap())
}

fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

fn code_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Code, val).unwrap().0
}

fn subject(n: u8) -> codlet::secret::SubjectId {
    codlet::secret::SubjectId::new(format!("user-{n}"))
}

fn code_id(n: u8) -> codlet::secret::CodeId {
    codlet::secret::CodeId::new(format!("code-{n}"))
}

fn basic_code_record(
    id: codlet::secret::CodeId,
    lk: codlet::LookupKey,
    expires_at: u64,
) -> CodeRecord {
    CodeRecord {
        id,
        lookup_key: lk,
        key_version: kv(),
        purpose: None,
        scope: None,
        grant: Some("grant-payload".to_string()),
        created_at: NOW,
        expires_at,
    }
}

// ── RFC-005: Code lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn find_redeemable_returns_valid_code() {
    let store = MemCodeStore::new();
    let lk = code_lookup("ABCD2345");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), LATER))
        .await
        .unwrap();
    let found = store.find_redeemable(&[lk], NOW, None).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().grant.as_deref(), Some("grant-payload"));
}

#[tokio::test]
async fn find_redeemable_rejects_expired_record() {
    let store = MemCodeStore::new();
    let lk = code_lookup("EXP00001");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), EXPIRED))
        .await
        .unwrap();
    let found = store.find_redeemable(&[lk], NOW, None).await.unwrap();
    assert!(found.is_none(), "expired code must not be found");
}

#[tokio::test]
async fn find_redeemable_rejects_used_code() {
    let store = MemCodeStore::new();
    let lk = code_lookup("USED0001");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), LATER))
        .await
        .unwrap();
    let found = store
        .find_redeemable(std::slice::from_ref(&lk), NOW, None)
        .await
        .unwrap()
        .unwrap();
    let claim = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &subject(1),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(claim, ClaimOutcome::Won);
    // Now find again — must return None.
    let again = store.find_redeemable(&[lk], NOW, None).await.unwrap();
    assert!(again.is_none(), "used code must not be redeemable");
}

#[tokio::test]
async fn claim_returns_won_exactly_once() {
    // Acceptance checklist RFC-005 §14.5: "Adapter conformance test proves
    // exactly one winner under concurrency."
    // In-memory: the Mutex ensures sequential access, so we verify the state
    // machine rather than OS-level races (the real concurrency test belongs
    // in the integration suite against a real DB).
    let store = MemCodeStore::new();
    let lk = code_lookup("ONCONLY1");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), LATER))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[lk], NOW, None)
        .await
        .unwrap()
        .unwrap();

    let r1 = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &subject(1),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    let r2 = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &subject(2),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();

    let wins = [&r1, &r2]
        .iter()
        .filter(|&&o| *o == ClaimOutcome::Won)
        .count();
    assert_eq!(wins, 1, "exactly one claim must win");
    let losses = [&r1, &r2]
        .iter()
        .filter(|&&o| *o == ClaimOutcome::Lost)
        .count();
    assert_eq!(losses, 1);
}

#[tokio::test]
async fn claim_after_revoke_returns_lost() {
    let store = MemCodeStore::new();
    let lk = code_lookup("REVOKED1");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), LATER))
        .await
        .unwrap();
    let found = store
        .find_redeemable(std::slice::from_ref(&lk), NOW, None)
        .await
        .unwrap()
        .unwrap();
    store.revoke_code(&found.id, None, NOW).await.unwrap();

    // After revocation, find_redeemable must return None.
    assert!(
        store
            .find_redeemable(&[lk], NOW, None)
            .await
            .unwrap()
            .is_none()
    );
    // And a direct claim must return Lost.
    let claim = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &subject(1),
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(
        claim,
        ClaimOutcome::Lost,
        "claim after revoke must return Lost"
    );
}

#[tokio::test]
async fn claim_after_expiry_returns_lost() {
    let store = MemCodeStore::new();
    let lk = code_lookup("EXPCLM01");
    store
        .insert_code(basic_code_record(code_id(1), lk.clone(), NOW + 10))
        .await
        .unwrap();
    let found = store
        .find_redeemable(&[lk], NOW, None)
        .await
        .unwrap()
        .unwrap();

    // Claim at NOW + 100 (after the code expired).
    let claim = store
        .claim_code(&ClaimRequest {
            code_id: &found.id,
            subject: &subject(1),
            now: NOW + 100,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(
        claim,
        ClaimOutcome::Lost,
        "claim after expiry must return Lost"
    );
}

#[tokio::test]
async fn wrong_scope_does_not_revoke() {
    let store = MemCodeStore::new();
    let lk = code_lookup("SCOPEDCD");
    store
        .insert_code(CodeRecord {
            id: code_id(1),
            lookup_key: lk.clone(),
            key_version: kv(),
            purpose: None,
            scope: Some("community-A".to_string()),
            grant: None,
            created_at: NOW,
            expires_at: LATER,
        })
        .await
        .unwrap();
    // Attempt to revoke with the wrong scope.
    store
        .revoke_code(&code_id(1), Some("community-B"), NOW)
        .await
        .unwrap();
    // Must still be found.
    let found = store
        .find_redeemable(&[lk], NOW, Some("community-A"))
        .await
        .unwrap();
    assert!(
        found.is_some(),
        "wrong-scope revoke must not affect the record"
    );
}
