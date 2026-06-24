//! Typed errors and outcomes for the orchestration layer (RFC-013).
//!
//! Every manager operation returns a structured result that carries both the
//! internal cause (for logs and metrics) and its public-safe mapping.  Callers
//! must not expose the internal cause to end users (INV-8).

use crate::error::{
    PublicFormError, PublicRedemptionError, PublicSessionError, RedemptionFailReason,
};
use crate::secret::{SessionId, SubjectId};
use crate::state::ClaimOutcome;
use crate::store::error::StoreError;

// ── Code redemption ──────────────────────────────────────────────────────────

/// Why a code redemption flow failed (RFC-013 §10.3, RFC-012).
///
/// Carries the internal reason alongside the public-safe error so callers can
/// log the internal cause without showing it to users.
#[derive(Debug)]
pub enum RedeemError {
    /// Input validation or normalization failed before any store access.
    InvalidInput {
        /// Internal reason (log, do not display).
        reason: RedemptionFailReason,
        /// Public-safe mapping.
        public: PublicRedemptionError,
    },
    /// Rate-limit threshold exceeded before lookup.
    RateLimited {
        /// Public-safe mapping.
        public: PublicRedemptionError,
    },
    /// No redeemable record found, or the record was expired/used/revoked.
    NotRedeemable {
        /// Internal reason (log, do not display).
        reason: RedemptionFailReason,
        /// Public-safe mapping (always `InvalidOrExpired`).
        public: PublicRedemptionError,
    },
    /// The atomic claim was lost to a concurrent caller.
    ClaimLost {
        /// Public-safe mapping.
        public: PublicRedemptionError,
    },
    /// A transient store or key failure prevented the operation.
    Internal {
        /// Internal diagnostic (log, do not display).
        cause: String,
        /// Public-safe mapping.
        public: PublicRedemptionError,
    },
}

impl RedeemError {
    /// The public-safe error to return to callers / map to HTTP responses.
    #[must_use]
    pub fn public(&self) -> &PublicRedemptionError {
        match self {
            Self::InvalidInput { public, .. }
            | Self::NotRedeemable { public, .. }
            | Self::ClaimLost { public }
            | Self::Internal { public, .. }
            | Self::RateLimited { public } => public,
        }
    }

    /// Convenience: construct from a [`StoreError`].
    pub(crate) fn from_store(e: StoreError) -> Self {
        Self::Internal {
            cause: format!("{e}"),
            public: PublicRedemptionError::TemporarilyUnavailable,
        }
    }

    /// Convenience: construct from a key / hashing error.
    pub(crate) fn from_key(e: crate::error::KeyError) -> Self {
        Self::Internal {
            cause: format!("key error: {e}"),
            public: PublicRedemptionError::TemporarilyUnavailable,
        }
    }
}

impl std::fmt::Display for RedeemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display the public-safe message only. The internal cause stays in Debug.
        write!(f, "{}", self.public())
    }
}

impl std::error::Error for RedeemError {}

/// A successfully completed code redemption.
///
/// The `claim` proof certifies that exactly one concurrent caller won the
/// atomic race. The caller must not issue a session or perform host-side
/// effects without this proof (RFC-013 §5, RFC-005 §14.5).
#[derive(Debug)]
pub struct RedeemSuccess {
    /// The authenticated subject returned by the host callback (or passed
    /// directly in the two-step flow).
    pub subject: SubjectId,
    /// Opaque grant payload from the code record. Passed to the host callback;
    /// not interpreted by codlet.
    pub grant: Option<String>,
    /// Proof that `claim_code` returned `Won`.  Structurally prevents issuing
    /// a session without going through the claim path.
    pub(crate) _claim_proof: ClaimProof,
}

/// Zero-size proof token that `claim_code` returned [`ClaimOutcome::Won`].
/// Not constructible outside this module; prevents session issuance without
/// a confirmed claim.
#[derive(Debug)]
pub(crate) struct ClaimProof(());

impl ClaimProof {
    pub(crate) fn new(outcome: ClaimOutcome) -> Option<Self> {
        match outcome {
            ClaimOutcome::Won => Some(Self(())),
            ClaimOutcome::Lost => None,
        }
    }
}

// ── Session ──────────────────────────────────────────────────────────────────

/// Why a session operation failed.
#[derive(Debug)]
pub enum SessionError {
    /// No valid session matched the bearer credential.
    NotFound {
        /// Public-safe mapping.
        public: PublicSessionError,
    },
    /// Transient store or key failure.
    Internal {
        /// Internal diagnostic (log, do not display).
        cause: String,
        /// Public-safe mapping.
        public: PublicSessionError,
    },
}

impl SessionError {
    /// The public-safe error.
    #[must_use]
    pub fn public(&self) -> &PublicSessionError {
        match self {
            Self::NotFound { public } | Self::Internal { public, .. } => public,
        }
    }

    pub(crate) fn from_store(e: StoreError) -> Self {
        Self::Internal {
            cause: format!("{e}"),
            public: PublicSessionError::TemporarilyUnavailable,
        }
    }

    pub(crate) fn from_key(e: crate::error::KeyError) -> Self {
        Self::Internal {
            cause: format!("key error: {e}"),
            public: PublicSessionError::TemporarilyUnavailable,
        }
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.public())
    }
}

impl std::error::Error for SessionError {}

/// A successfully issued session.
#[derive(Debug)]
pub struct IssuedSession {
    /// The opaque session record identifier.  Not a bearer credential.
    pub session_id: SessionId,
    /// The `Set-Cookie` header value to send to the client. Contains the
    /// plaintext bearer secret; must not be logged.
    pub set_cookie: String,
}

// ── Form token ───────────────────────────────────────────────────────────────

/// Why a form-token operation failed.
#[derive(Debug)]
pub enum FormTokenError {
    /// Token invalid, expired, or binding mismatch.
    Invalid {
        /// Public-safe mapping.
        public: PublicFormError,
    },
    /// Transient store or key failure.
    Internal {
        /// Internal diagnostic (log, do not display).
        cause: String,
        /// Public-safe mapping.
        public: PublicFormError,
    },
}

impl FormTokenError {
    /// The public-safe error.
    #[must_use]
    pub fn public(&self) -> &PublicFormError {
        match self {
            Self::Invalid { public } | Self::Internal { public, .. } => public,
        }
    }

    pub(crate) fn from_store(e: StoreError) -> Self {
        Self::Internal {
            cause: format!("{e}"),
            public: PublicFormError::TemporarilyUnavailable,
        }
    }

    pub(crate) fn from_key(e: crate::error::KeyError) -> Self {
        Self::Internal {
            cause: format!("key error: {e}"),
            public: PublicFormError::TemporarilyUnavailable,
        }
    }
}

impl std::fmt::Display for FormTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.public())
    }
}

impl std::error::Error for FormTokenError {}
