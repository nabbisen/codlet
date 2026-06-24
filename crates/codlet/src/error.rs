//! Error types for codlet.
//!
//! This is the internal error layer (RFC-012/021): structured, useful for
//! developers and operators, and safe to log because no variant carries a
//! plaintext secret. The public, enumeration-resistant error layer
//! (`PublicAuthFailure`) is introduced with the redemption flow (RFC-012) once
//! the store traits exist.

use thiserror::Error;

/// Randomness could not be obtained. Generation fails closed on this error;
/// codlet never substitutes a deterministic value (INV-3, SR-29-adjacent).
#[derive(Debug, Error, PartialEq, Eq)]
#[error("secure randomness unavailable")]
pub struct RandomError;

/// A key provider could not supply usable key material.
///
/// Carries no key bytes. Missing material is fatal to the operation; there is
/// no fallback key (INV-2, SR-29).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyError {
    /// No active key is configured.
    #[error("no active HMAC key configured")]
    MissingActiveKey,
    /// The requested historical key version is not available. Validation fails
    /// closed for that candidate rather than falling back.
    #[error("HMAC key version not available")]
    MissingKeyVersion,
    /// Key material was present but unusable (e.g. empty).
    #[error("HMAC key material is invalid")]
    InvalidKeyMaterial,
}

/// A [`crate::code::CodePolicy`] was constructed with an impossible or unsafe
/// shape (RFC-003 §11.1).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    /// Alphabet has fewer than two distinct symbols.
    #[error("alphabet must contain at least 2 symbols")]
    AlphabetTooSmall,
    /// Alphabet contains a duplicate symbol, which would bias generation.
    #[error("alphabet contains duplicate symbols")]
    AlphabetNotUnique,
    /// Alphabet contains a non-ASCII or otherwise unsupported byte.
    #[error("alphabet contains an unsupported (non-ASCII) symbol")]
    AlphabetNotAscii,
    /// Requested code length is below the secure minimum and no explicit
    /// short-code opt-in was used.
    #[error("code length {got} is below the secure minimum of {min}")]
    LengthBelowMinimum {
        /// Requested length.
        got: usize,
        /// Enforced minimum.
        min: usize,
    },
    /// Requested code length is zero.
    #[error("code length must be non-zero")]
    ZeroLength,
}

/// Rejection of user-supplied code input during validation (RFC-003 FR-2).
///
/// All variants map to the same generic public message; the distinction here
/// exists only for internal diagnostics and metrics, never for user display
/// (INV-8).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodeInputError {
    /// Input was empty after trimming.
    #[error("code input is empty")]
    Empty,
    /// Raw input exceeded the maximum accepted length before normalization.
    #[error("code input exceeds maximum raw length")]
    TooLongRaw,
    /// Normalized input length does not match the configured code length.
    #[error("normalized code length does not match policy")]
    WrongLength,
    /// Normalized input contains a character outside the accepted set.
    #[error("code input contains unsupported characters")]
    UnsupportedCharacters,
}

// ── RFC-012/021: two-layer error model ──────────────────────────────────────

/// Internal reason a code redemption failed. Rich enough for logs and metrics;
/// never shown to the user (INV-8, RFC-012 §10.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedemptionFailReason {
    /// Code input was malformed (too long, wrong length, unsupported chars).
    InvalidFormat,
    /// No redeemable record matched the lookup key(s).
    NotFound,
    /// A matching record exists but `expires_at` has passed.
    Expired,
    /// A matching record exists but it was explicitly revoked.
    Revoked,
    /// A matching record exists but was already claimed.
    AlreadyUsed,
    /// The rate-limit threshold was exceeded before the lookup.
    RateLimited,
    /// The store could not be reached; the operation was not attempted.
    StoreUnavailable,
    /// Key material was unavailable or invalid.
    KeyFailure,
}

/// Public-safe redemption failure (RFC-012 §4, RFC-021).
///
/// All enumeration-sensitive reasons (`NotFound`, `Expired`, `Revoked`,
/// `AlreadyUsed`, `InvalidFormat`) collapse to `InvalidOrExpired`. The caller
/// must not expose the internal [`RedemptionFailReason`] to end users.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PublicRedemptionError {
    /// The code was not accepted. Reason intentionally omitted.
    #[error("invalid or expired code")]
    InvalidOrExpired,
    /// The caller has exceeded the rate limit. Safe to surface as a throttle
    /// hint (does not reveal code existence).
    #[error("too many attempts — please wait and try again")]
    RateLimited,
    /// A transient problem prevented the check. The code was not consumed.
    #[error("service temporarily unavailable")]
    TemporarilyUnavailable,
}

impl PublicRedemptionError {
    /// Map an internal reason to its public-safe equivalent (RFC-012 §4).
    #[must_use]
    pub fn from_reason(reason: &RedemptionFailReason) -> Self {
        match reason {
            RedemptionFailReason::InvalidFormat
            | RedemptionFailReason::NotFound
            | RedemptionFailReason::Expired
            | RedemptionFailReason::Revoked
            | RedemptionFailReason::AlreadyUsed => Self::InvalidOrExpired,
            RedemptionFailReason::RateLimited => Self::RateLimited,
            RedemptionFailReason::StoreUnavailable | RedemptionFailReason::KeyFailure => {
                Self::TemporarilyUnavailable
            }
        }
    }
}

/// Public-safe form-token / CSRF failure (RFC-012, RFC-021).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PublicFormError {
    /// The form could not be submitted. The token was missing, expired, or
    /// already consumed. No distinction is made between these states.
    #[error("form expired or invalid — please reload the page and try again")]
    ExpiredOrInvalid,
    /// A transient problem prevented the check.
    #[error("service temporarily unavailable")]
    TemporarilyUnavailable,
}

/// Public-safe session failure (RFC-012, RFC-021).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PublicSessionError {
    /// No valid session — missing cookie, expired, or revoked. No distinction.
    #[error("session missing or expired — please sign in again")]
    MissingOrExpired,
    /// A transient problem prevented the check.
    #[error("service temporarily unavailable")]
    TemporarilyUnavailable,
}
