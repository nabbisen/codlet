//! Unit tests for the `session` module.
use super::*;

fn subject() -> SubjectId {
    SubjectId::new("user-42".to_string())
}

fn sid() -> crate::secret::SessionId {
    crate::secret::SessionId::new("sess-abc".to_string())
}

fn record(created_at: u64, expires_at: u64, last_seen_at: Option<u64>) -> ActiveSessionRecord {
    record_with_revocation(created_at, expires_at, last_seen_at, None)
}

fn record_with_revocation(
    created_at: u64,
    expires_at: u64,
    last_seen_at: Option<u64>,
    revoked_at: Option<u64>,
) -> ActiveSessionRecord {
    ActiveSessionRecord {
        id: sid(),
        subject: subject(),
        created_at,
        expires_at,
        last_seen_at,
        revoked_at,
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
fn a_record_with_an_unreached_expiry_and_recent_idle_activity_authenticates() {
    // Positive control: neither absolute expiry nor idle timeout has fired,
    // so the record authenticates. (Superseded RFC-044-era framing: this
    // used to say "the store would never return this record" and "absolute
    // expiry enforcement lives in the store, not here" -- both are false
    // since RFC-047 step 2 moved absolute-expiry enforcement into this
    // function too. See `absolute_expiry_wins_even_when_idle_check_would_pass`
    // below for the case that actually exercises the boundary this test's
    // old comment described.)
    let idle_timeout = Duration::from_secs(1_800);
    let out = classify_session(Some(record(0, 100, Some(50))), Some(idle_timeout), 60);
    assert!(out.is_authenticated());
}

// ── RFC-047 step 2: classify_session owns expiry and revocation too ────────

#[test]
fn revoked_alone_is_unauthenticated_revoked() {
    let out = classify_session(
        Some(record_with_revocation(0, 9_999_999, None, Some(5))),
        None,
        10,
    );
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );
}

#[test]
fn expired_alone_is_unauthenticated_expired() {
    let out = classify_session(Some(record(0, 100, None)), None, 100);
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Expired
        }
    );
    // Strictly before the boundary: still authenticated.
    let out = classify_session(Some(record(0, 100, None)), None, 99);
    assert!(out.is_authenticated());
}

#[test]
fn revoked_and_expired_classifies_revoked() {
    // RFC-047 §8.1 (restated for sessions): revoked, then expired, then
    // idle, then authenticated. A record satisfying both must classify as
    // Revoked, not Expired.
    let out = classify_session(
        Some(record_with_revocation(0, 100, None, Some(50))),
        None,
        100, // also past expiry
    );
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        },
        "revoked must win over expired"
    );
}

#[test]
fn absolute_expiry_wins_even_when_idle_check_would_pass() {
    // Expiry is checked before idle timeout: a record whose idle-activity
    // check would pass (last_seen_at is recent) must still classify as
    // Expired if its absolute expiry has passed. Proves the decision order
    // is enforced, not just each condition individually.
    let idle_timeout = Duration::from_secs(1_800);
    let out = classify_session(
        Some(record(0, 100, Some(99))), // last_seen_at = 99, well within idle_timeout of now = 100
        Some(idle_timeout),
        100, // exactly at absolute expiry
    );
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Expired
        },
        "absolute expiry must win over a passing idle-timeout check"
    );
}

#[test]
fn revoked_wins_over_idle_timeout_too() {
    let idle_timeout = Duration::from_secs(1_800);
    let out = classify_session(
        Some(record_with_revocation(0, 9_999_999, Some(99), Some(5))),
        Some(idle_timeout),
        100,
    );
    assert_eq!(
        out,
        SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::Revoked
        }
    );
}
