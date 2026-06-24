//! Code-claim state machine (RFC-005).
//!
//! Pure, storage-free logic: given the result of an atomic conditional
//! `UPDATE … WHERE … AND used_at IS NULL AND expires_at > :now`, classify the
//! outcome. No I/O, no `async`. Tested exhaustively.

/// Outcome of a `claim_code` attempt (RFC-005 §3).
///
/// Only `Won` may advance the host to session creation or any other
/// side-effecting operation. `Lost` is definitive; there is no retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// This caller won the atomic race: the conditional UPDATE changed exactly
    /// one row. Proceed with session issuance and any host-side effects.
    Won,
    /// The conditional UPDATE changed zero rows: the code was already claimed,
    /// revoked, or expired when this call ran. Do not proceed.
    Lost,
}

/// Classify an atomic claim attempt from the affected-row count (INV-5).
///
/// `changed` is the number of rows the conditional UPDATE reported modifying:
///
/// - `1` → [`ClaimOutcome::Won`]
/// - `0` → [`ClaimOutcome::Lost`]
/// - anything else → storage invariant violation; returns `Lost` conservatively.
///   Adapters should log an internal error when `changed > 1`.
#[must_use]
pub fn classify_claim(changed: usize) -> ClaimOutcome {
    if changed == 1 {
        ClaimOutcome::Won
    } else {
        // changed == 0 (normal lost) or > 1 (invariant violation).
        // Either way: do not proceed. RFC-005 §14.1: `changed > 1` is a store
        // invariant violation and must be surfaced by the adapter as an error
        // rather than silently returning Lost; this classifier handles it
        // conservatively so even a misbehaving adapter cannot produce a Won.
        ClaimOutcome::Lost
    }
}

#[cfg(test)]
mod tests;
