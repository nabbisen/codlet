//! Form-token consume state machine (RFC-007).
//!
//! This is a direct port of `zinnias_ciao_contracts::auth::classify_token_consume`
//! and its six tests, lifted into codlet-core so the logic is a pure,
//! storage-free primitive. The function signature and all invariants are
//! preserved exactly; adapters supply the inputs from their query results.

/// Outcome of a single-use form-token consume attempt (RFC-007 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenConsumeOutcome {
    /// This call won the atomic race (UPDATE changed exactly one row). Execute
    /// the operation.
    Proceed,
    /// Token already consumed — idempotent replay. Return the prior result
    /// reference if one was stored; do not re-execute the operation.
    Replay,
    /// Token not found, expired, or binding mismatch. Reject the request.
    Invalid,
}

/// Classify a consume attempt from the atomic UPDATE and a follow-up SELECT.
///
/// `changed` is the affected-row count from:
///
/// ```sql
/// UPDATE codlet_form_tokens
/// SET consumed_at = :now
/// WHERE lookup_key = :key
///   AND purpose    = :purpose
///   AND subject    = :subject         -- binding
///   AND expires_at > :now
///   AND consumed_at IS NULL
/// ```
///
/// When `changed == 0`, the follow-up SELECT provides:
///
/// - `found`           — a row matching the lookup key + purpose + subject exists.
/// - `already_consumed`— that row has `consumed_at IS NOT NULL`.
/// - `binding_ok`      — the row's bound resource matches the caller's.
///
/// The single rule that must never be violated: **`changed == 0` never
/// produces [`TokenConsumeOutcome::Proceed`]** (RFC-007 §5, §13.5,
/// acceptance checklist item "changed == 0 never proceeds").
#[must_use]
pub fn classify_token_consume(
    changed: usize,
    found: bool,
    already_consumed: bool,
    binding_ok: bool,
) -> TokenConsumeOutcome {
    if changed == 1 {
        return TokenConsumeOutcome::Proceed;
    }
    // changed == 0: the conditional UPDATE matched nothing — classify why.
    if !found || !binding_ok {
        return TokenConsumeOutcome::Invalid;
    }
    if already_consumed {
        return TokenConsumeOutcome::Replay;
    }
    // Row exists, unconsumed, binding ok, but UPDATE still missed →
    // the expiry guard fired. Treat as invalid.
    TokenConsumeOutcome::Invalid
}

#[cfg(test)]
mod tests {
    use super::*;

    // The six tests from zinnias-ciao contracts/src/auth.rs, preserved verbatim
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
}
