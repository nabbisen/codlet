//! Unit tests for the `secret` module.
use super::*;

#[test]
fn secret_string_redacts_debug_and_display() {
    let s = SecretString::new("hunter2".to_string());
    assert_eq!(format!("{s:?}"), "SecretString(<redacted>)");
    assert_eq!(format!("{s}"), "<redacted>");
    assert!(!format!("{s:?}").contains("hunter2"));
    assert!(!format!("{s}").contains("hunter2"));
    assert_eq!(s.expose(), "hunter2");
}

#[test]
fn secret_newtypes_redact_debug() {
    let c = PlainCode::new("ABCD2345".to_string());
    let dbg = format!("{c:?}");
    assert!(
        !dbg.contains("ABCD2345"),
        "PlainCode Debug leaked plaintext: {dbg}"
    );
    assert!(dbg.contains("<redacted>"));
    assert_eq!(c.expose(), "ABCD2345");
}

#[test]
fn id_newtype_displays_and_roundtrips() {
    let id = CodeId::new("abc123".to_string());
    assert_eq!(id.as_str(), "abc123");
    assert_eq!(format!("{id}"), "abc123");
    assert_eq!(CodeId::from("x".to_string()).as_str(), "x");
}

#[cfg(feature = "serde")]
#[test]
fn secret_serializes_redacted() {
    let c = SessionSecret::new("supersecret".to_string());
    let json = serde_json::to_string(&c).unwrap();
    assert_eq!(json, "\"<redacted>\"");
    assert!(!json.contains("supersecret"));
}

// ── RFC-019 typed wrapper tests ───────────────────────────────────────────────

#[test]
fn normalized_code_displays_plainly() {
    let n = NormalizedCode::new("ABCD2345".into());
    assert_eq!(format!("{n}"), "ABCD2345");
    assert_eq!(n.as_str(), "ABCD2345");
}

#[test]
fn purpose_rejects_empty() {
    assert!(Purpose::new("").is_none());
    assert!(Purpose::new("logout").is_some());
}

#[test]
fn scope_key_roundtrip() {
    let s = ScopeKey::new("community-42");
    assert_eq!(s.as_str(), "community-42");
    assert_eq!(format!("{s}"), "community-42");
}
