//! Unit tests for the `code` lookup classifier.
use super::*;

const NOW: u64 = 1_000;

#[test]
fn redeemable_when_nothing_is_set() {
    assert_eq!(
        classify_code_lookup(None, None, NOW + 1, NOW),
        CodeLookupOutcome::Redeemable
    );
}

#[test]
fn revoked_alone() {
    assert_eq!(
        classify_code_lookup(Some(NOW), None, NOW + 1, NOW),
        CodeLookupOutcome::Revoked
    );
}

#[test]
fn expired_alone() {
    // expires_at <= now.
    assert_eq!(
        classify_code_lookup(None, None, NOW, NOW),
        CodeLookupOutcome::Expired
    );
    assert_eq!(
        classify_code_lookup(None, None, NOW - 1, NOW),
        CodeLookupOutcome::Expired
    );
}

#[test]
fn used_alone() {
    assert_eq!(
        classify_code_lookup(None, Some(NOW), NOW + 1, NOW),
        CodeLookupOutcome::Used
    );
}

// ── RFC-047 §8.1: fixed decision order — revoked, expired, used, redeemable ─

#[test]
fn revoked_and_expired_classifies_revoked() {
    assert_eq!(
        classify_code_lookup(Some(NOW), None, NOW, NOW),
        CodeLookupOutcome::Revoked,
        "revoked must win over expired"
    );
}

#[test]
fn revoked_and_used_classifies_revoked() {
    assert_eq!(
        classify_code_lookup(Some(NOW), Some(NOW), NOW + 1, NOW),
        CodeLookupOutcome::Revoked,
        "revoked must win over used"
    );
}

#[test]
fn expired_and_used_classifies_expired() {
    assert_eq!(
        classify_code_lookup(None, Some(NOW), NOW, NOW),
        CodeLookupOutcome::Expired,
        "expired must win over used when not revoked"
    );
}

#[test]
fn revoked_expired_and_used_classifies_revoked() {
    assert_eq!(
        classify_code_lookup(Some(NOW), Some(NOW), NOW, NOW),
        CodeLookupOutcome::Revoked,
        "revoked must win over every other combination"
    );
}
