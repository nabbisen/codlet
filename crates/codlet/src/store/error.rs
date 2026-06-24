//! Public-safe and internal error types (RFC-012/021).

use thiserror::Error;

/// The single public authentication failure response (INV-8, RFC-012 §14.3).
///
/// All internal failure states — not found, expired, revoked, already used,
/// purpose mismatch, binding mismatch, scope mismatch — collapse to
/// `InvalidOrExpiredCode`. This prevents enumeration attacks by ensuring the
/// caller cannot distinguish record existence from expiry from prior use.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum PublicAuthError {
    /// The credential (code, token, session) was not accepted. The reason is
    /// intentionally omitted from this type; internal diagnostics use the richer
    /// internal error layer.
    #[error("invalid or expired")]
    InvalidOrExpired,
    /// A transient storage failure prevented the operation. The credential may
    /// or may not have been consumed; the host should not retry automatically.
    #[error("service temporarily unavailable")]
    TemporaryProblem,
}

/// Internal store failure, not for public display.
#[derive(Debug, Error)]
pub enum StoreError {
    /// The underlying store returned an error.
    #[error("store error: {0}")]
    Backend(String),
    /// A storage invariant was violated (e.g. `changed > 1` after a claim).
    #[error("store invariant violated: {0}")]
    InvariantViolation(String),
}
