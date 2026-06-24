//! Unit tests for the `hashing` module.
use super::*;

fn hasher() -> SecretHasher<StaticKeyProvider> {
    let kp = StaticKeyProvider::single("v1", b"super-secret-key-material".to_vec()).unwrap();
    SecretHasher::new(kp)
}

#[test]
fn deterministic_same_inputs_same_key() {
    let h = hasher();
    let (a, va) = h.lookup_key(SecretDomain::Code, "ABCD2345").unwrap();
    let (b, vb) = h.lookup_key(SecretDomain::Code, "ABCD2345").unwrap();
    assert_eq!(a, b);
    assert_eq!(va, vb);
    assert_eq!(va.as_str(), "v1");
    // 32-byte digest → 64 hex chars.
    assert_eq!(a.as_str().len(), 64);
    assert!(a.as_str().bytes().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn different_value_different_key() {
    let h = hasher();
    let (a, _) = h.lookup_key(SecretDomain::Code, "AAAAAAAA").unwrap();
    let (b, _) = h.lookup_key(SecretDomain::Code, "BBBBBBBB").unwrap();
    assert_ne!(a, b);
}

#[test]
fn domain_separation_distinguishes_same_value() {
    let h = hasher();
    let (code, _) = h.lookup_key(SecretDomain::Code, "SAME").unwrap();
    let (sess, _) = h.lookup_key(SecretDomain::Session, "SAME").unwrap();
    let (form, _) = h.lookup_key(SecretDomain::FormToken, "SAME").unwrap();
    let (flow, _) = h.lookup_key(SecretDomain::FlowTicket, "SAME").unwrap();
    // All four must differ pairwise.
    let all = [&code, &sess, &form, &flow];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "domains {i},{j} collided");
        }
    }
}

#[test]
fn different_key_different_output() {
    let h1 = SecretHasher::new(StaticKeyProvider::single("v1", b"key-one".to_vec()).unwrap());
    let h2 = SecretHasher::new(StaticKeyProvider::single("v1", b"key-two".to_vec()).unwrap());
    let (a, _) = h1.lookup_key(SecretDomain::Code, "X").unwrap();
    let (b, _) = h2.lookup_key(SecretDomain::Code, "X").unwrap();
    assert_ne!(a, b);
}

#[test]
fn missing_active_key_fails_closed() {
    // A provider whose active version points at no stored key.
    let kp = StaticKeyProvider {
        active_version: KeyVersion::new("missing"),
        keys: vec![(KeyVersion::new("v1"), b"k".to_vec())],
    };
    let h = SecretHasher::new(kp);
    assert_eq!(
        h.lookup_key(SecretDomain::Code, "X").unwrap_err(),
        KeyError::MissingActiveKey
    );
}

#[test]
fn empty_key_rejected_at_construction() {
    assert_eq!(
        StaticKeyProvider::single("v1", Vec::new()).unwrap_err(),
        KeyError::InvalidKeyMaterial
    );
}

#[test]
fn key_version_round_trip_validation() {
    // Derive under v1, rotate active to v2, re-derive the v1 candidate.
    let kp = StaticKeyProvider::new(
        "v2",
        b"key-two".to_vec(),
        vec![(KeyVersion::new("v1"), b"key-one".to_vec())],
    )
    .unwrap();
    let h = SecretHasher::new(kp);
    let (active, av) = h.lookup_key(SecretDomain::Session, "tok").unwrap();
    assert_eq!(av.as_str(), "v2");
    let v1 = KeyVersion::new("v1");
    let prev = h
        .lookup_key_with_version(SecretDomain::Session, "tok", &v1)
        .unwrap();
    // v1 and v2 derivations differ; the active is v2.
    assert_ne!(active, prev);
    // Unknown version fails closed, not fallback.
    let missing = KeyVersion::new("v9");
    assert_eq!(
        h.lookup_key_with_version(SecretDomain::Session, "tok", &missing)
            .unwrap_err(),
        KeyError::MissingKeyVersion
    );
}

#[test]
fn lookup_key_ct_eq_matches_value_eq() {
    let h = hasher();
    let (a, _) = h.lookup_key(SecretDomain::Code, "ABCD2345").unwrap();
    let (b, _) = h.lookup_key(SecretDomain::Code, "ABCD2345").unwrap();
    let (c, _) = h.lookup_key(SecretDomain::Code, "DIFFEREN").unwrap();
    assert!(a.ct_eq(&b));
    assert!(!a.ct_eq(&c));
}

#[test]
fn key_material_redacted_in_debug() {
    let kp = StaticKeyProvider::single("v1", b"secret-bytes".to_vec()).unwrap();
    let dbg = format!("{kp:?}");
    assert!(!dbg.contains("secret-bytes"), "key bytes leaked: {dbg}");
    assert!(dbg.contains("<redacted>"));
    let key = kp.active_hmac_key().unwrap();
    let kdbg = format!("{key:?}");
    assert!(!kdbg.contains("secret-bytes"));
}
