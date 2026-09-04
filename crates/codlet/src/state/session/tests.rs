//! Unit tests for the `session` module.
use super::*;

fn subject() -> SubjectId {
    SubjectId::new("user-42".to_string())
}

fn sid() -> crate::secret::SessionId {
    crate::secret::SessionId::new("sess-abc".to_string())
}

fn record(created_at: u64, expires_at: u64, last_seen_at: Option<u64>) -> ActiveSessionRecord {
    ActiveSessionRecord {
        id: sid(),
        subject: subject(),
        created_at,
        expires_at,
        last_seen_at,
    }
}

#[test]
fn some_record_authenticates() {
    let out = classify_session(Some(record(0, 9_999_999, None)), None, 0);
    assert!(out.is_authenticated());
    assert_eq!(out.subject().unwrap().as_str(), "user-42");
}

#[test]
fn none_is_unauthenticated_not_found() {
    let out = classify_session(None, None, 0);
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::NotFound
        }
    );
    assert!(!out.is_authenticated());
    assert!(out.subject().is_none());
}

#[test]
fn authenticated_carries_session_id_and_expiry() {
    let out = classify_session(Some(record(0, 12_345, None)), None, 0);
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

// ── RFC-044: idle timeout ───────────────────────────────────────────────────

#[test]
fn idle_timeout_none_ignores_last_seen() {
    // No idle_timeout configured: a session untouched since creation, long
    // ago, is still authenticated — absolute expiry is the only bound.
    let out = classify_session(
        Some(record(0, 9_999_999, None)),
        None,
        5_000_000, // far past created_at, would be idle-expired if enabled
    );
    assert!(out.is_authenticated());
}

#[test]
fn idle_timeout_enabled_expires_a_stale_session() {
    let idle_timeout = Duration::from_secs(1_800); // 30 minutes
    // created_at = 0, never touched (last_seen_at = None -> falls back to
    // created_at = 0), now = 1_800 -> exactly at the boundary, expired.
    let out = classify_session(Some(record(0, 9_999_999, None)), Some(idle_timeout), 1_800);
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::IdleTimeout
        }
    );
}

#[test]
fn idle_timeout_enabled_authenticates_a_recently_touched_session() {
    let idle_timeout = Duration::from_secs(1_800);
    // last_seen_at = 1_000, now = 1_799 -> 799s since last seen, under 1800s.
    let out = classify_session(
        Some(record(0, 9_999_999, Some(1_000))),
        Some(idle_timeout),
        1_799,
    );
    assert!(out.is_authenticated());
}

#[test]
fn idle_timeout_falls_back_to_created_at_when_never_touched() {
    let idle_timeout = Duration::from_secs(1_800);
    // last_seen_at = None (never touched) -> effective last-seen = created_at
    // = 100. now = 100 + 1800 - 1 = 1899 -> still within the window.
    let out = classify_session(
        Some(record(100, 9_999_999, None)),
        Some(idle_timeout),
        1_899,
    );
    assert!(out.is_authenticated());

    // now = 1900 -> exactly at the boundary from created_at, expired.
    let out = classify_session(
        Some(record(100, 9_999_999, None)),
        Some(idle_timeout),
        1_900,
    );
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::IdleTimeout
        }
    );
}

#[test]
fn absolute_expiry_independent_of_idle_timeout() {
    // The store would never return this record (it's past expires_at) --
    // but this proves classify_session doesn't second-guess idle-timeout
    // math into overriding what "Some" already means: a record the store
    // considers active. Absolute expiry enforcement lives in the store, not
    // here (RFC-044 §4.3); idle timeout is the only thing this function
    // decides. A record with an alarmingly close expires_at is still
    // authenticated if idle timeout doesn't fire.
    let idle_timeout = Duration::from_secs(1_800);
    let out = classify_session(Some(record(0, 100, Some(50))), Some(idle_timeout), 60);
    assert!(out.is_authenticated());
}
