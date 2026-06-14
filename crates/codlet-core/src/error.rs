//! Error types for codlet-core.
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
