//! Acceptance tests for RFC-006: session lifecycle and cookie policy.
use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
use codlet::mem::MemSessionStore;
use codlet::store::code::expires_at_from_ttl;
use codlet::store::session::{SessionRecord, SessionStore};

const NOW: u64 = 1_700_000_000;
const LATER: u64 = NOW + 3_600;
const EXPIRED: u64 = NOW - 1;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", vec![0u8; 32]).unwrap())
}

fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

fn subject(n: u8) -> codlet::secret::SubjectId {
    codlet::secret::SubjectId::new(format!("user-{n}"))
}

fn session_lookup(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Session, val).unwrap().0
}

fn session_id(n: u8) -> codlet::secret::SessionId {
    codlet::secret::SessionId::new(format!("sess-{n}"))
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
