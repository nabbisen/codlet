//! Form-token consume state machine (RFC-007).
//!
//! Pure classifier for atomic form-token consume operations (RFC-007).
//! and its six tests, lifted into codlet so the logic is a pure,
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
/// The single rule that must never be violated (INV-6): **`changed == 0` never
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
mod tests;
