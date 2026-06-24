//! Acceptance tests for RFC-007: form-token lifecycle and binding semantics.
use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemFormTokenStore;
use codlet::secret::SubjectId;
use codlet::state::TokenConsumeOutcome;
use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;
const EXPIRED: u64 = NOW - 1;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap())
}

fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

fn token_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::FormToken, val).unwrap().0
}

fn auth_subject(n: u8) -> TokenSubject {
    TokenSubject::Authenticated(SubjectId::new(format!("user-{n}")))
}

// ── RFC-007: Form-token lifecycle ────────────────────────────────────────────

async fn insert_token(
    store: &MemFormTokenStore,
    secret: &str,
    subject: TokenSubject,
    purpose: &str,
    bound: Option<&str>,
    expires_at: u64,
) -> codlet::LookupKey {
    let lk = token_lookup(secret);
    store
        .insert_form_token(FormTokenRecord {
            lookup_key: lk.clone(),
            key_version: kv(),
            subject,
            purpose: purpose.to_string(),
            bound_resource: bound.map(str::to_string),
            issued_at: NOW,
            expires_at,
        })
        .await
        .unwrap();
    lk
}

#[tokio::test]
async fn winner_proceeds() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok1", auth_subject(1), "logout", None, LATER).await;
    let (outcome, rr) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "logout",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(outcome, TokenConsumeOutcome::Proceed);
    assert!(rr.is_none());
}

#[tokio::test]
async fn loser_sees_replay() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok2", auth_subject(1), "save_note", None, LATER).await;
    // First consume wins.
    let (r1, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "save_note",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(r1, TokenConsumeOutcome::Proceed);
    // Second consume must replay.
    let (r2, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "save_note",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(r2, TokenConsumeOutcome::Replay);
}

#[tokio::test]
async fn replay_returns_stored_result_ref() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok3", auth_subject(1), "create_event", None, LATER).await;
    store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "create_event",
            None,
            NOW,
        )
        .await
        .unwrap();
    store
        .set_token_result(std::slice::from_ref(&lk), "/events/42")
        .await
        .unwrap();
    let (outcome, rr) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "create_event",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(outcome, TokenConsumeOutcome::Replay);
    assert_eq!(rr.as_deref(), Some("/events/42"));
}

#[tokio::test]
async fn unknown_token_is_invalid() {
    let store = MemFormTokenStore::new();
    let lk = token_lookup("nonexistent");
    let (outcome, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "logout",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(outcome, TokenConsumeOutcome::Invalid);
}

#[tokio::test]
async fn binding_mismatch_is_invalid() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(
        &store,
        "tok4",
        auth_subject(1),
        "edit_event",
        Some("event-1"),
        LATER,
    )
    .await;
    let (outcome, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "edit_event",
            Some("event-2"),
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "wrong bound_resource must be Invalid"
    );
}

#[tokio::test]
async fn purpose_mismatch_is_invalid() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok5", auth_subject(1), "save_note", None, LATER).await;
    let (outcome, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "delete_note",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "wrong purpose must be Invalid"
    );
}

#[tokio::test]
async fn expired_unconsumed_is_invalid() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok6", auth_subject(1), "logout", None, EXPIRED).await;
    let (outcome, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "logout",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "expired unconsumed must be Invalid"
    );
}

#[tokio::test]
async fn changed_zero_never_proceeds() {
    // Cross-crate enforcement: all changed==0 classifier paths return non-Proceed.
    use codlet::classify_token_consume;
    for found in [false, true] {
        for consumed in [false, true] {
            for binding in [false, true] {
                let out = classify_token_consume(0, found, consumed, binding);
                assert_ne!(
                    out,
                    TokenConsumeOutcome::Proceed,
                    "changed==0 must never Proceed (found={found} consumed={consumed} binding={binding})"
                );
            }
        }
    }
}

#[tokio::test]
async fn anonymous_token_subject_distinct_from_authenticated() {
    let store = MemFormTokenStore::new();
    let lk = insert_token(&store, "tok7", TokenSubject::Anonymous, "join", None, LATER).await;
    // Authenticated subject must not consume an anonymous token.
    let (outcome, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &auth_subject(1),
            "join",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        outcome,
        TokenConsumeOutcome::Invalid,
        "auth subject must not consume anon token"
    );
    // Anonymous subject can.
    let (outcome2, _) = store
        .consume_form_token(
            std::slice::from_ref(&lk),
            &TokenSubject::Anonymous,
            "join",
            None,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(outcome2, TokenConsumeOutcome::Proceed);
}
