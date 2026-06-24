//! Unit tests for the `session` module.
use super::*;

fn subject() -> SubjectId {
    SubjectId::new("user-42".to_string())
}

fn sid() -> crate::secret::SessionId {
    crate::secret::SessionId::new("sess-abc".to_string())
}

#[test]
fn some_record_authenticates() {
    let out = classify_session(Some((subject(), sid(), 9_999_999)));
    assert!(out.is_authenticated());
    assert_eq!(out.subject().unwrap().as_str(), "user-42");
}

#[test]
fn none_is_unauthenticated() {
    assert_eq!(
        classify_session(None),
        SessionValidationOutcome::Unauthenticated
    );
    assert!(!classify_session(None).is_authenticated());
    assert!(classify_session(None).subject().is_none());
}

#[test]
fn authenticated_carries_session_id_and_expiry() {
    let out = classify_session(Some((subject(), sid(), 12_345)));
    if let SessionValidationOutcome::Authenticated {
        session_id,
        expires_at,
        ..
    } = out
    {
        assert_eq!(session_id.as_str(), "sess-abc");
        assert_eq!(expires_at, 12_345);
    } else {
        panic!("expected Authenticated");
    }
}
