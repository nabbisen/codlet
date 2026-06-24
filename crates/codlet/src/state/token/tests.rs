//! Unit tests for the `token` module.
use super::*;

// Six exhaustive cases covering all combinations of found/consumed/binding_ok.
// as a compatibility / regression suite.

#[test]
fn consume_winner_proceeds() {
    assert_eq!(
        classify_token_consume(1, true, false, true),
        TokenConsumeOutcome::Proceed
    );
}

#[test]
fn consume_loser_of_race_sees_replay() {
    // Concurrent double-submit: second call's UPDATE changes 0 rows because
    // consumed_at is set. Must replay, not re-execute.
    assert_eq!(
        classify_token_consume(0, true, true, true),
        TokenConsumeOutcome::Replay
    );
}

#[test]
fn consume_unknown_token_is_invalid() {
    assert_eq!(
        classify_token_consume(0, false, false, false),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_binding_mismatch_is_invalid() {
    // Right token, wrong bound_resource → rejection, not replay.
    assert_eq!(
        classify_token_consume(0, true, false, false),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_expired_unconsumed_is_invalid() {
    // Found, unconsumed, binding ok, but UPDATE missed (expiry guard).
    assert_eq!(
        classify_token_consume(0, true, false, true),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_never_double_proceeds() {
    // Exhaustive: across all changed==0 states, Proceed is impossible.
    // Acceptance checklist RFC-007 §13.5: "changed == 0 never proceeds".
    for found in [false, true] {
        for consumed in [false, true] {
            for binding in [false, true] {
                assert_ne!(
                    classify_token_consume(0, found, consumed, binding),
                    TokenConsumeOutcome::Proceed,
                    "changed==0 must never proceed \
                     (found={found} consumed={consumed} binding={binding})"
                );
            }
        }
    }
}

// Additional codlet-specific tests.

#[test]
fn changed_greater_than_one_is_conservatively_invalid() {
    // >1 is a storage invariant violation. We must never Proceed.
    for bad in [2usize, 100] {
        assert_ne!(
            classify_token_consume(bad, true, false, true),
            TokenConsumeOutcome::Proceed,
            "changed={bad} must not Proceed"
        );
    }
}

#[test]
fn not_found_always_invalid_regardless_of_other_flags() {
    for consumed in [false, true] {
        for binding in [false, true] {
            assert_eq!(
                classify_token_consume(0, false, consumed, binding),
                TokenConsumeOutcome::Invalid,
                "not found must always be Invalid"
            );
        }
    }
}
