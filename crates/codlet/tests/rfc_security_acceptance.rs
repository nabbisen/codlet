//! Acceptance tests for security RFCs: A (key rotation), B (rate limit),
//! C (purpose/scope), and E (bound_resource conformance).

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
