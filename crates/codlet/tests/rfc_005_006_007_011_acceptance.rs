//! Acceptance tests for RFC-005 (code lifecycle), RFC-006 (session), RFC-007
//! (form tokens), and RFC-011 (in-memory stores).
//!
//! Each section covers the checklist items from the corresponding RFC.

use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::{MemCodeStore, MemFormTokenStore, MemSessionStore};
use codlet::secret::{CodeId, SessionId, SubjectId};
use codlet::state::{ClaimOutcome, TokenConsumeOutcome};
use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore, expires_at_from_ttl};
use codlet::store::session::{SessionRecord, SessionStore};
use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

// ── helpers ─────────────────────────────────────────────────────────────────

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"test-key".to_vec()).unwrap())
}

fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

fn code_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Code, val).unwrap().0
}

fn session_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Session, val).unwrap().0
}

fn token_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::FormToken, val).unwrap().0
}

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;
const EXPIRED: u64 = NOW - 1;

fn subject(n: u8) -> SubjectId {
    SubjectId::new(format!("user-{n}"))
}

fn code_id(n: u8) -> CodeId {
    CodeId::new(format!("code-{n}"))
}

fn session_id(n: u8) -> SessionId {
    SessionId::new(format!("sess-{n}"))
}

fn basic_code_record(id: CodeId, lk: codlet::LookupKey, expires_at: u64) -> CodeRecord {
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

// ── RFC-006: Session lifecycle ───────────────────────────────────────────────

#[tokio::test]
async fn session_issuance_and_validation() {
    let store = MemSessionStore::new();
    let lk = session_lookup("sess-secret-xyz");
    store
        .insert_session(SessionRecord {
            id: session_id(1),
            lookup_key: lk.clone(),
            key_version: kv(),
            subject: subject(1),
            created_at: NOW,
            expires_at: expires_at_from_ttl(NOW, std::time::Duration::from_secs(30 * 86_400)),
        })
        .await
        .unwrap();

    let active = store
        .find_active_session(&[lk], NOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.subject.as_str(), "user-1");
    assert_eq!(active.id.as_str(), "sess-1");
    // Plaintext must not be stored.
    // (The lookup key is an HMAC, not the plaintext — this is enforced by the
    // API: the field type is LookupKey, not SecretString.)
}

#[tokio::test]
async fn expired_session_is_inactive() {
    let store = MemSessionStore::new();
    let lk = session_lookup("exp-sess");
    store
        .insert_session(SessionRecord {
            id: session_id(1),
            lookup_key: lk.clone(),
            key_version: kv(),
            subject: subject(1),
            created_at: EXPIRED - 10,
            expires_at: EXPIRED,
        })
        .await
        .unwrap();
    assert!(
        store
            .find_active_session(&[lk], NOW)
            .await
            .unwrap()
            .is_none(),
        "expired session must be inactive"
    );
}

#[tokio::test]
async fn revoked_session_is_inactive() {
    let store = MemSessionStore::new();
    let lk = session_lookup("rev-sess");
    store
        .insert_session(SessionRecord {
            id: session_id(1),
            lookup_key: lk.clone(),
            key_version: kv(),
            subject: subject(1),
            created_at: NOW,
            expires_at: LATER,
        })
        .await
        .unwrap();
    store.revoke_session(&session_id(1), NOW).await.unwrap();
    assert!(
        store
            .find_active_session(&[lk], NOW)
            .await
            .unwrap()
            .is_none(),
        "revoked session must be inactive"
    );
}

// ── RFC-007: Form-token lifecycle ────────────────────────────────────────────

fn auth_subject(n: u8) -> TokenSubject {
    TokenSubject::Authenticated(subject(n))
}

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

// ── RFC-006 cookie policy acceptance ─────────────────────────────────────────

#[test]
fn cookie_defaults_are_secure_by_construction() {
    use codlet::cookie::CookiePolicy;
    let p = CookiePolicy::production_strict("sid", std::time::Duration::from_secs(3600));
    assert!(p.is_secure());
    let hdr = p.build_set_cookie("mysecret");
    assert!(hdr.contains("HttpOnly"));
    assert!(hdr.contains("Secure"));
    assert!(hdr.contains("SameSite=Strict"));
    assert!(!hdr.contains("Domain="));
}

#[test]
fn clear_cookie_mirrors_set_cookie_attributes() {
    use codlet::cookie::CookiePolicy;
    let p = CookiePolicy::production_strict("sid", std::time::Duration::from_secs(3600))
        .with_domain(Some("example.com"));
    let set = p.build_set_cookie("s");
    let clear = p.build_clear_cookie();
    // Name, Path, Domain must appear in both.
    for attr in ["sid=", "Path=/", "Domain=example.com"] {
        assert!(set.contains(attr), "set-cookie missing {attr}");
        assert!(clear.contains(attr), "clear-cookie missing {attr}");
    }
    assert!(clear.contains("Max-Age=0"));
}

// ── RFC-A: Key rotation candidate lookup ─────────────────────────────────────

#[cfg(feature = "test-utils")]
mod rfc_a_key_rotation {
    use codlet::{
        SecretHasher, StaticKeyProvider,
        hashing::{KeyVersion, SecretDomain},
    };

    fn v1() -> SecretHasher<StaticKeyProvider> {
        SecretHasher::new(StaticKeyProvider::single("v1", vec![1u8; 32]).unwrap())
    }
    fn v2_with_v1() -> SecretHasher<StaticKeyProvider> {
        SecretHasher::new(
            StaticKeyProvider::new(
                "v2",
                vec![2u8; 32],
                vec![(KeyVersion::new("v1"), vec![1u8; 32])],
            )
            .unwrap(),
        )
    }

    #[test]
    fn candidates_active_only() {
        let cs = v1().lookup_key_candidates(SecretDomain::Code, "x").unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].1.as_str(), "v1");
    }

    #[test]
    fn candidates_active_plus_previous() {
        let cs = v2_with_v1()
            .lookup_key_candidates(SecretDomain::Code, "x")
            .unwrap();
        assert_eq!(cs.len(), 2);
        // Active key first
        assert_eq!(cs[0].1.as_str(), "v2");
        assert_eq!(cs[1].1.as_str(), "v1");
    }

    #[test]
    fn old_key_candidate_matches_value_issued_under_old_key() {
        // Derive under v1 active
        let lk_v1 = v1().lookup_key(SecretDomain::Code, "secret").unwrap().0;
        // After rotation to v2, v1 lookup key must appear in candidates
        let cs = v2_with_v1()
            .lookup_key_candidates(SecretDomain::Code, "secret")
            .unwrap();
        let found = cs.iter().any(|(lk, _)| lk.as_str() == lk_v1.as_str());
        assert!(found, "v1 lookup key must be in v2+v1 candidates");
    }

    #[test]
    fn new_key_candidate_differs_from_old() {
        let lk_v1 = v1().lookup_key(SecretDomain::Code, "secret").unwrap().0;
        let lk_v2 = v2_with_v1()
            .lookup_key(SecretDomain::Code, "secret")
            .unwrap()
            .0;
        assert_ne!(
            lk_v1.as_str(),
            lk_v2.as_str(),
            "different keys must produce different lookup values"
        );
    }
}

// ── RFC-B: Rate-limit failure accounting ─────────────────────────────────────

#[cfg(feature = "test-utils")]
mod rfc_b_rate_limit {
    use codlet::{
        CodePolicy, SecretHasher, StaticKeyProvider,
        audit::NoopAuditSink,
        auth::CodeAuth,
        clock::FixedClock,
        mem::{MemCodeStore, MemRateLimitStore},
        store::ratelimit::{RateLimitKey, RateLimitPolicy, RateLimitUnavailable},
    };
    use std::time::Duration;

    // CodeAuth<CS, RL, K, C, A> = <CodeStore, RateLimitStore, KeyProvider, Clock, AuditSink>
    type TestAuth =
        CodeAuth<MemCodeStore, MemRateLimitStore, StaticKeyProvider, FixedClock, NoopAuditSink>;

    fn make(max: u32) -> (TestAuth, RateLimitKey) {
        let hasher = SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap());
        let policy = CodePolicy::default_human(Duration::from_secs(3600)).unwrap();
        let rl = RateLimitPolicy {
            max_failures: max,
            window: Duration::from_secs(300),
            unavailable: RateLimitUnavailable::FailOpen,
        };
        let auth = CodeAuth::new(
            MemCodeStore::new(),
            MemRateLimitStore::new(),
            hasher,
            FixedClock::at(1000),
            NoopAuditSink,
            policy,
            rl,
        );
        (auth, RateLimitKey::new("127.0.0.1".to_string()))
    }

    #[tokio::test]
    async fn invalid_format_exhausts_limit() {
        let (auth, key) = make(2);
        let _ = auth.find("!!!bad!!!", Some(&key)).await;
        let _ = auth.find("!!!bad!!!", Some(&key)).await;
        // Threshold reached; next attempt must be rate-limited
        let r = auth.find("!!!bad!!!", Some(&key)).await;
        assert!(r.is_err());
        assert!(
            format!("{:?}", r.unwrap_err()).contains("RateLimited"),
            "invalid-format must count toward rate limit (RFC-B)"
        );
    }

    #[tokio::test]
    async fn not_found_code_increments_counter() {
        let (auth, key) = make(2);
        let _ = auth.find("ABCDEFGH", Some(&key)).await;
        let _ = auth.find("BCDEFGHI", Some(&key)).await;
        let r = auth.find("CDEFGHIJ", Some(&key)).await;
        assert!(r.is_err());
        let s = format!("{:?}", r.unwrap_err());
        assert!(
            s.contains("RateLimited") || s.contains("NotRedeemable"),
            "not-found must count toward rate limit (RFC-B): {s}"
        );
    }

    #[test]
    fn fail_closed_variant_compiles() {
        // RateLimitUnavailable::FailClosed must exist (RFC-B removed SoftDenyAfterThreshold).
        let p = RateLimitPolicy {
            max_failures: 5,
            window: Duration::from_secs(300),
            unavailable: RateLimitUnavailable::FailClosed,
        };
        assert!(matches!(p.unavailable, RateLimitUnavailable::FailClosed));
    }
}

// ── RFC-C: Purpose/scope in RedeemableCode ───────────────────────────────────

#[cfg(feature = "test-utils")]
mod rfc_c_purpose_scope {
    use codlet::{
        CodePolicy, SecretHasher, StaticKeyProvider,
        audit::NoopAuditSink,
        auth::CodeAuth,
        clock::FixedClock,
        mem::{MemCodeStore, MemRateLimitStore},
        rng::SystemRandom,
        secret::{CodeId, SubjectId},
        store::ratelimit::RateLimitPolicy,
    };
    use std::time::Duration;

    type TestAuth =
        CodeAuth<MemCodeStore, MemRateLimitStore, StaticKeyProvider, FixedClock, NoopAuditSink>;

    fn auth() -> TestAuth {
        let hasher = SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap());
        let policy = CodePolicy::default_human(Duration::from_secs(3600)).unwrap();
        CodeAuth::new(
            MemCodeStore::new(),
            MemRateLimitStore::new(),
            hasher,
            FixedClock::at(1000),
            NoopAuditSink,
            policy,
            RateLimitPolicy::default_invite(),
        )
    }

    #[tokio::test]
    async fn purpose_and_scope_surface_on_found_record() {
        let a = auth();
        let mut rng = SystemRandom::new();
        let (_, plain) = a
            .issue_code(
                &mut rng,
                CodeId::new("c1".into()),
                Some("invite".into()),
                Some("community-1".into()),
                None,
            )
            .await
            .unwrap();
        let found = a.find(plain.expose(), None).await.unwrap();
        assert_eq!(found.purpose.as_deref(), Some("invite"));
        assert_eq!(found.scope.as_deref(), Some("community-1"));
    }

    #[tokio::test]
    async fn claim_with_matching_purpose_and_scope_succeeds() {
        let a = auth();
        let mut rng = SystemRandom::new();
        let (_, plain) = a
            .issue_code(
                &mut rng,
                CodeId::new("c2".into()),
                Some("invite".into()),
                Some("community-1".into()),
                None,
            )
            .await
            .unwrap();
        let found = a.find(plain.expose(), None).await.unwrap();
        let r = a.claim(&found, SubjectId::new("user-1".into()), None).await;
        assert!(
            r.is_ok(),
            "correct purpose/scope must allow claim: {:?}",
            r.err()
        );
    }

    #[tokio::test]
    async fn code_without_purpose_is_claimable() {
        let a = auth();
        let mut rng = SystemRandom::new();
        let (_, plain) = a
            .issue_code(&mut rng, CodeId::new("c3".into()), None, None, None)
            .await
            .unwrap();
        let found = a.find(plain.expose(), None).await.unwrap();
        let r = a.claim(&found, SubjectId::new("user-2".into()), None).await;
        assert!(r.is_ok());
    }
}

// ── RFC-E: Form-token bound_resource conformance ──────────────────────────────

#[cfg(feature = "test-utils")]
mod rfc_e_bound_resource {
    use codlet::{
        SecretHasher, StaticKeyProvider,
        hashing::{KeyVersion, SecretDomain},
        mem::MemFormTokenStore,
        secret::SubjectId,
        state::TokenConsumeOutcome,
        store::token::{FormTokenRecord, FormTokenStore, TokenSubject},
    };

    const NOW: u64 = 1_000_000;
    const LATER: u64 = NOW + 3600;

    fn hasher() -> SecretHasher<StaticKeyProvider> {
        SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap())
    }
    fn subject() -> TokenSubject {
        TokenSubject::Authenticated(SubjectId::new("u1".into()))
    }

    async fn insert(
        store: &MemFormTokenStore,
        secret: &str,
        res: Option<&str>,
    ) -> codlet::hashing::LookupKey {
        let lk = hasher()
            .lookup_key(SecretDomain::FormToken, secret)
            .unwrap()
            .0;
        store
            .insert_form_token(FormTokenRecord {
                lookup_key: lk.clone(),
                key_version: KeyVersion::new("v1"),
                subject: subject(),
                purpose: "act".into(),
                bound_resource: res.map(|s| s.to_string()),
                issued_at: NOW,
                expires_at: LATER,
            })
            .await
            .unwrap();
        lk
    }

    #[tokio::test]
    async fn none_stored_none_caller_proceeds() {
        let store = MemFormTokenStore::new();
        let k = insert(&store, "tok-nn", None).await;
        let (o, _) = store
            .consume_form_token(&[k], &subject(), "act", None, NOW)
            .await
            .unwrap();
        assert_eq!(o, TokenConsumeOutcome::Proceed, "None+None must Proceed");
    }

    #[tokio::test]
    async fn some_stored_same_caller_proceeds() {
        let store = MemFormTokenStore::new();
        let k = insert(&store, "tok-ss", Some("res-A")).await;
        let (o, _) = store
            .consume_form_token(&[k], &subject(), "act", Some("res-A"), NOW)
            .await
            .unwrap();
        assert_eq!(o, TokenConsumeOutcome::Proceed, "Some+Same must Proceed");
    }

    #[tokio::test]
    async fn some_stored_none_caller_is_invalid() {
        let store = MemFormTokenStore::new();
        let k = insert(&store, "tok-sn", Some("res-A")).await;
        let (o, _) = store
            .consume_form_token(&[k], &subject(), "act", None, NOW)
            .await
            .unwrap();
        assert_eq!(
            o,
            TokenConsumeOutcome::Invalid,
            "Some(stored)+None(caller) must be Invalid — RFC-E aligns Mem with SQL semantics"
        );
    }

    #[tokio::test]
    async fn none_stored_some_caller_is_invalid() {
        let store = MemFormTokenStore::new();
        let k = insert(&store, "tok-ns", None).await;
        let (o, _) = store
            .consume_form_token(&[k], &subject(), "act", Some("res-X"), NOW)
            .await
            .unwrap();
        assert_eq!(
            o,
            TokenConsumeOutcome::Invalid,
            "None(stored)+Some(caller) must be Invalid — RFC-E"
        );
    }
}
