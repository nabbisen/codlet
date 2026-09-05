//! Code-lookup state machine (RFC-047).
//!
//! Pure, storage-free. `find_redeemable` returns any record matching the
//! lookup key (and scope), regardless of its use/expiry/revocation state —
//! this module is what decides what that record means. Adapters look records
//! up; they do not adjudicate (RFC-047 §4.1, extending the principle RFC-044
//! established for session idle timeout).

/// The current state of a looked-up code record (RFC-047).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeLookupOutcome {
    /// Available to claim. `claim_code`'s conditional UPDATE is still the
    /// actual enforcement point (RFC-005, INV-5) — this classifier is a
    /// pre-filter, not a second guard.
    Redeemable,
    /// Explicitly revoked.
    Revoked,
    /// Absolute expiry has passed.
    Expired,
    /// Already claimed by a prior, successful `claim_code`.
    Used,
}

/// Classify a looked-up code record's redeemability from its state fields.
///
/// A record can satisfy more than one condition at once (e.g. revoked *and*
/// expired). The decision order is fixed (RFC-047 §8.1, owner-resolved):
/// **revoked, then expired, then used, then redeemable.** Revoked is checked
/// first because it is the only state an operator caused deliberately, and
/// the most useful thing to see in a log during an incident.
#[must_use]
pub fn classify_code_lookup(
    revoked_at: Option<u64>,
    used_at: Option<u64>,
    expires_at: u64,
    now: u64,
) -> CodeLookupOutcome {
    if revoked_at.is_some() {
        CodeLookupOutcome::Revoked
    } else if expires_at <= now {
        CodeLookupOutcome::Expired
    } else if used_at.is_some() {
        CodeLookupOutcome::Used
    } else {
        CodeLookupOutcome::Redeemable
    }
}

#[cfg(test)]
mod tests;
