//! Session validation state machine (RFC-006, RFC-044, RFC-046).
//!
//! Pure, storage-free. The store is responsible for querying and providing the
//! record state; this module classifies the outcome without any I/O.

use std::time::Duration;

use crate::secret::SubjectId;
use crate::store::session::ActiveSessionRecord;

/// Why a session failed to validate (RFC-046).
///
/// Returned to the **host application** alongside
/// [`SessionValidationOutcome::Unauthenticated`], never to the end user.
///
/// # Do not render these to end users
///
/// This type exists so the host can distinguish operational cases (an
/// idle-timeout spike vs. a revocation spike) and, at its own discretion, show
/// a more specific message than a bare login form. It is **not** safe to
/// forward verbatim to a client:
///
/// - [`Expired`](Self::Expired) and [`IdleTimeout`](Self::IdleTimeout) are safe
///   to surface as something like "your session ended, please sign in again"
///   — both are already inferable by a user who knows they were signed in.
/// - [`NotFound`](Self::NotFound) and [`Revoked`](Self::Revoked) distinguish
///   states an unauthenticated visitor should not learn (RFC-046 §3.2):
///   revealing "revoked" rather than a generic failure to someone presenting a
///   cookie they should not have tells them their guess at a valid session
///   shape was meaningfully close.
///
/// This distinction exists for the host, in-process, about a credential the
/// caller already presented — it does not weaken DEC-006 or INV-8, which
/// govern what an *attacker guessing a value they do not hold* can learn
/// (RFC-046 §3.1). [`SessionFailure`] must never be reachable from
/// [`crate::error::PublicSessionError`] — a test in this module's `tests`
/// asserts no conversion exists between the two types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFailure {
    /// No session cookie was presented.
    NoCookie,
    /// A cookie was presented but is not a well-formed session secret.
    Malformed,
    /// The cookie was well-formed but no matching record exists.
    ///
    /// **Currently also reported for `Expired` and `Revoked`** — see those
    /// variants' docs.
    NotFound,
    /// A matching record exists but its absolute expiry has passed.
    ///
    /// **Not currently produced.** [`SessionStore::find_active_session`]
    /// collapses "never issued", "expired", and "revoked" into a single
    /// `None`, so `classify_session` cannot distinguish this case from
    /// [`NotFound`](Self::NotFound) today — it reports `NotFound` for all
    /// three. This variant is defined for the API it is intended to reach,
    /// not one it reaches now; a follow-up RFC will decide whether
    /// `find_active_session`'s contract changes to make it reachable (the
    /// same decision governs [`Revoked`](Self::Revoked) and
    /// `RedemptionFailReason::Expired`, which has the identical gap for the
    /// same structural reason).
    ///
    /// [`SessionStore::find_active_session`]: crate::store::session::SessionStore::find_active_session
    Expired,
    /// A matching record exists but its idle timeout has passed (RFC-044).
    IdleTimeout,
    /// A matching record exists but was explicitly revoked.
    ///
    /// **Not currently produced**, for the same reason as
    /// [`Expired`](Self::Expired): `find_active_session` cannot distinguish
    /// a revoked record from one that was never issued or has expired.
    /// `NotFound` is reported instead until that contract changes.
    Revoked,
}

/// The result of validating a session secret against the store (RFC-006 §13.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionValidationOutcome {
    /// Session is valid. The host application must still check authorization
    /// (RFC-001: codlet authenticates; the host authorizes).
    Authenticated {
        /// The host-owned subject this session is bound to.
        subject: SubjectId,
        /// The opaque session record identifier (not a bearer credential).
        session_id: crate::secret::SessionId,
        /// Expiry as Unix seconds (UTC). For display / renewal decisions only;
        /// the store already filtered out expired sessions.
        expires_at: u64,
    },
    /// No valid session. The end-user-visible response is identical for every
    /// reason (INV-8, RFC-006 §13.5); `reason` is for the host only — see
    /// [`SessionFailure`]'s documentation before using it.
    Unauthenticated {
        /// Why validation failed. Host-visible only; never render verbatim.
        reason: SessionFailure,
    },
}

impl SessionValidationOutcome {
    /// Return `true` if the outcome is [`SessionValidationOutcome::Authenticated`].
    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated { .. })
    }

    /// Return the authenticated subject, if any.
    #[must_use]
    pub fn subject(&self) -> Option<&SubjectId> {
        match self {
            Self::Authenticated { subject, .. } => Some(subject),
            Self::Unauthenticated { .. } => None,
        }
    }
}

/// Classify a session lookup from the store's query result.
///
/// `record` is `None` when the store found no matching, active row for the
/// given lookup key. **Under the current [`crate::store::session::SessionStore`]
/// contract this collapses "never issued", "expired", and "revoked" into one
/// signal** — the store's own active-row filter does not tell the caller which
/// of the three excluded a row, so a `None` here classifies as
/// [`SessionFailure::NotFound`] rather than guessing. See RFC-046's review
/// request for the open question this raises about `Expired` and `Revoked`'s
/// reachability.
///
/// When `Some`, `idle_timeout` (if configured) is checked against the record's
/// effective last-seen time (`last_seen_at`, or `created_at` if the session has
/// never been touched — RFC-044 §5) to decide between
/// [`SessionValidationOutcome::Authenticated`] and
/// [`SessionFailure::IdleTimeout`].
#[must_use]
pub fn classify_session(
    record: Option<ActiveSessionRecord>,
    idle_timeout: Option<Duration>,
    now: u64,
) -> SessionValidationOutcome {
    match record {
        Some(r) => {
            if let Some(idle_timeout) = idle_timeout {
                let last_seen = r.last_seen_at.unwrap_or(r.created_at);
                if now.saturating_sub(last_seen) >= idle_timeout.as_secs() {
                    return SessionValidationOutcome::Unauthenticated {
                        reason: SessionFailure::IdleTimeout,
                    };
                }
            }
            SessionValidationOutcome::Authenticated {
                subject: r.subject,
                session_id: r.id,
                expires_at: r.expires_at,
            }
        }
        None => SessionValidationOutcome::Unauthenticated {
            reason: SessionFailure::NotFound,
        },
    }
}

#[cfg(test)]
mod tests;
