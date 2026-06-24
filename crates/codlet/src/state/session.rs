//! Session validation state machine (RFC-006).
//!
//! Pure, storage-free. The store is responsible for querying and providing the
//! record state; this module classifies the outcome without any I/O.

use crate::secret::SubjectId;

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
    /// No valid session: cookie absent, not found, expired, or revoked.
    /// All cases collapse to one response type to prevent enumeration
    /// (INV-8, RFC-006 §13.5).
    Unauthenticated,
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
            Self::Unauthenticated => None,
        }
    }
}

/// Classify a session lookup from the store's query result.
///
/// `record` is `None` when the store found no active row for the given lookup
/// key (expired, revoked, or never issued). When `Some`, the tuple is
/// `(subject_id, session_id, expires_at_unix_secs)`.
#[must_use]
pub fn classify_session(
    record: Option<(SubjectId, crate::secret::SessionId, u64)>,
) -> SessionValidationOutcome {
    match record {
        Some((subject, session_id, expires_at)) => SessionValidationOutcome::Authenticated {
            subject,
            session_id,
            expires_at,
        },
        None => SessionValidationOutcome::Unauthenticated,
    }
}

#[cfg(test)]
mod tests;
