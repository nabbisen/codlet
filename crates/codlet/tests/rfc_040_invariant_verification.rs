//! Negative tests for RFC-040 §3.3: each row's guard proven capable of
//! failing, not merely present. INV-4's guard is deferred to RFC-041 and is
//! deliberately absent here (RFC-040 §4). INV-7 is a compile-failure test
//! and lives in `rfc_040_inv7_compile_fail.rs`, wired via `trybuild`.

use codlet::error::{PublicRedemptionError, RedemptionFailReason};
use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::{MemCodeStore, MemFormTokenStore};
use codlet::secret::{CodeId, SubjectId};
use codlet::state::{ClaimOutcome, TokenConsumeOutcome};
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore};
use codlet::store::error::StoreError;
use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap())
}

fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

fn code_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Code, val).unwrap().0
}

fn subject(n: u8) -> SubjectId {
    SubjectId::new(format!("user-{n}"))
}

fn code_id(n: u8) -> CodeId {
    CodeId::new(format!("code-{n}"))
}

fn code_record(id: CodeId, lk: codlet::LookupKey) -> CodeRecord {
    CodeRecord {
        id,
        lookup_key: lk,
        key_version: kv(),
        purpose: None,
        scope: None,
        grant: Some("grant".to_string()),
        created_at: NOW,
        expires_at: LATER,
    }
}

// ── INV-5: claim_code, changed == 0 never proceeds; changed > 1 is an error ─

#[tokio::test]
async fn inv5_claim_with_changed_zero_reports_lost_not_won() {
    // The straightforward changed == 0 case: no matching record at all.
    let store = MemCodeStore::new();
    let missing = code_id(1);
    let subj = subject(1);
    let outcome = store
        .claim_code(&ClaimRequest {
            code_id: &missing,
            subject: &subj,
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(
        outcome,
        ClaimOutcome::Lost,
        "INV-5: a claim that matches zero rows must never report Won"
    );
}

#[tokio::test]
async fn inv5_second_claim_of_an_already_won_code_also_reports_lost() {
    // "No effect follows" a lost claim: winning once, then losing on retry,
    // and the second loser must not have altered anything a caller could
    // observe as a second win.
    let store = MemCodeStore::new();
    let id = code_id(2);
    let lk = code_lookup("ABCD2345");
    store
        .insert_code(code_record(id.clone(), lk))
        .await
        .unwrap();

    let winner = subject(1);
    let first = store
        .claim_code(&ClaimRequest {
            code_id: &id,
            subject: &winner,
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(first, ClaimOutcome::Won);

    let loser = subject(2);
    let second = store
        .claim_code(&ClaimRequest {
            code_id: &id,
            subject: &loser,
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await
        .unwrap();
    assert_eq!(
        second,
        ClaimOutcome::Lost,
        "INV-5: changed == 0 on the second attempt (already used) must never report Won"
    );
}

#[tokio::test]
async fn inv5_changed_greater_than_one_surfaces_as_invariant_violation_not_lost() {
    // Construct a genuine changed > 1: two records sharing one code_id but
    // distinct lookup keys. `insert_code` only rejects duplicate lookup
    // keys, not duplicate ids, so this is reachable through the public API,
    // not a test-only backdoor into store internals.
    let store = MemCodeStore::new();
    let shared_id = code_id(3);
    store
        .insert_code(code_record(shared_id.clone(), code_lookup("AAAA1111")))
        .await
        .unwrap();
    store
        .insert_code(code_record(shared_id.clone(), code_lookup("BBBB2222")))
        .await
        .unwrap();

    let subj = subject(1);
    let result = store
        .claim_code(&ClaimRequest {
            code_id: &shared_id,
            subject: &subj,
            now: NOW,
            purpose: None,
            scope: None,
        })
        .await;

    match result {
        Err(StoreError::InvariantViolation(_)) => {}
        other => panic!(
            "INV-5: changed > 1 must surface as StoreError::InvariantViolation, \
             not be folded into a Lost/Won outcome; got {other:?}"
        ),
    }
}

// ── INV-6: consume_form_token, changed == 0 never proceeds; changed > 1 errors ─

fn token_record(lk: codlet::LookupKey) -> FormTokenRecord {
    FormTokenRecord {
        lookup_key: lk,
        key_version: kv(),
        subject: TokenSubject::Flow(CodeId::new("checkout-flow".to_string())),
        purpose: "checkout".to_string(),
        bound_resource: None,
        issued_at: NOW,
        expires_at: LATER,
    }
}

#[tokio::test]
async fn inv6_consume_with_changed_zero_reports_invalid_not_proceed() {
    let store = MemFormTokenStore::new();
    let lk = code_lookup("unknown-token");
    let outcome = store
        .consume_form_token(
            &[lk],
            &TokenSubject::Flow(CodeId::new("checkout-flow".to_string())),
            "checkout",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        outcome.0,
        TokenConsumeOutcome::Invalid,
        "INV-6: a consume that matches zero rows must never report Proceed"
    );
}

#[tokio::test]
async fn inv6_second_consume_of_an_already_consumed_token_replays_not_proceeds() {
    let store = MemFormTokenStore::new();
    let lk = code_lookup("CCCC3333");
    store
        .insert_form_token(token_record(lk.clone()))
        .await
        .unwrap();

    let subj = TokenSubject::Flow(CodeId::new("checkout-flow".to_string()));
    let first = store
        .consume_form_token(std::slice::from_ref(&lk), &subj, "checkout", None, NOW)
        .await
        .unwrap();
    assert_eq!(first.0, TokenConsumeOutcome::Proceed);

    let second = store
        .consume_form_token(&[lk], &subj, "checkout", None, NOW)
        .await
        .unwrap();
    assert_eq!(
        second.0,
        TokenConsumeOutcome::Replay,
        "INV-6: changed == 0 on the second attempt (already consumed) must never report Proceed"
    );
}

#[tokio::test]
async fn inv6_changed_greater_than_one_surfaces_as_invariant_violation_not_replay() {
    // Two rows sharing lookup key + subject + purpose: `insert_form_token`
    // has no uniqueness check at all, so both are insertable through the
    // public API.
    let store = MemFormTokenStore::new();
    let lk = code_lookup("DDDD4444");
    store
        .insert_form_token(token_record(lk.clone()))
        .await
        .unwrap();
    store
        .insert_form_token(token_record(lk.clone()))
        .await
        .unwrap();

    let subj = TokenSubject::Flow(CodeId::new("checkout-flow".to_string()));
    let result = store
        .consume_form_token(&[lk], &subj, "checkout", None, NOW)
        .await;

    match result {
        Err(StoreError::InvariantViolation(_)) => {}
        other => panic!(
            "INV-6: changed > 1 must surface as StoreError::InvariantViolation, \
             not be folded into a Proceed/Replay outcome; got {other:?}"
        ),
    }
}

// ── INV-8: every RedemptionFailReason maps to one of the generic public errors,
//    via an exhaustive match with no wildcard arm ──────────────────────────

#[test]
fn inv8_every_redemption_fail_reason_is_classified_by_an_exhaustive_match() {
    // The existing rfc_008_012_acceptance.rs tests cover every variant
    // through array literals, but an array does not force the compiler to
    // reject a new, unhandled variant — it would simply not be added to any
    // array, and the suite would keep passing. This match has no `_` arm:
    // adding a RedemptionFailReason variant without extending this match
    // fails to compile.
    for reason in [
        RedemptionFailReason::InvalidFormat,
        RedemptionFailReason::NotFound,
        RedemptionFailReason::Expired,
        RedemptionFailReason::Revoked,
        RedemptionFailReason::AlreadyUsed,
        RedemptionFailReason::RateLimited,
        RedemptionFailReason::StoreUnavailable,
        RedemptionFailReason::KeyFailure,
    ] {
        let expected = match reason {
            RedemptionFailReason::InvalidFormat
            | RedemptionFailReason::NotFound
            | RedemptionFailReason::Expired
            | RedemptionFailReason::Revoked
            | RedemptionFailReason::AlreadyUsed => PublicRedemptionError::InvalidOrExpired,
            RedemptionFailReason::RateLimited => PublicRedemptionError::RateLimited,
            RedemptionFailReason::StoreUnavailable | RedemptionFailReason::KeyFailure => {
                PublicRedemptionError::TemporarilyUnavailable
            }
        };
        assert_eq!(
            PublicRedemptionError::from_reason(&reason),
            expected,
            "reason {reason:?} disagreed with the exhaustive classification"
        );
    }
}
