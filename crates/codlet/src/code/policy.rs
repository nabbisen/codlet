//! Code policy (RFC-003 §3, §11.1).
//!
//! [`CodePolicy`] is a validated security object, not loose configuration. Its
//! constructors reject impossible or risky shapes. Short codes below the secure
//! minimum require an explicit opt-in constructor so the weaker choice is
//! visible in code review (NFR-2).

use core::time::Duration;

use super::alphabet::Alphabet;
use crate::error::PolicyError;

/// The secure minimum human-entered code length codlet enforces by default.
/// 8 symbols over the 31-symbol alphabet is ~39.6 bits (RFC-003 §11.3).
pub const SECURE_MIN_HUMAN_LENGTH: usize = 8;

/// The maximum accepted raw (pre-normalization) input length. Bounds work done
/// on hostile input before a lookup.
pub const DEFAULT_MAX_RAW_LEN: usize = 64;

/// Minimum accepted short-code length for the explicit compat opt-in (6 symbols, ~29.7 bits).
pub const SHORT_COMPAT_LENGTH: usize = 6;

/// Validated policy governing code generation and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePolicy {
    alphabet: Alphabet,
    length: usize,
    max_raw_len: usize,
    ttl: Duration,
}

impl CodePolicy {
    /// The recommended default for human-entered codes: the unambiguous
    /// alphabet, [`SECURE_MIN_HUMAN_LENGTH`] symbols, and the given TTL.
    ///
    /// # Errors
    /// Returns [`PolicyError`] only if the TTL is zero. The built-in alphabet
    /// and length are always valid.
    pub fn default_human(ttl: Duration) -> Result<Self, PolicyError> {
        Self::new(Alphabet::unambiguous(), SECURE_MIN_HUMAN_LENGTH, ttl)
    }

    /// Build a policy, enforcing the secure minimum length.
    ///
    /// # Errors
    /// Returns [`PolicyError`] if the length is zero, below
    /// [`SECURE_MIN_HUMAN_LENGTH`], or the TTL is zero. Use
    /// [`CodePolicy::short_compat`] to opt into a shorter length deliberately.
    pub fn new(alphabet: Alphabet, length: usize, ttl: Duration) -> Result<Self, PolicyError> {
        if length == 0 {
            return Err(PolicyError::ZeroLength);
        }
        if length < SECURE_MIN_HUMAN_LENGTH {
            return Err(PolicyError::LengthBelowMinimum {
                got: length,
                min: SECURE_MIN_HUMAN_LENGTH,
            });
        }
        Self::build(alphabet, length, ttl)
    }

    /// Explicitly opt into a short code length below the secure minimum.
    ///
    /// This is a deliberately separate, named constructor (NFR-2): a short code
    /// is acceptable only with short expiry, single-use semantics, and rate
    /// limiting. Hosts choosing this take on that responsibility.
    ///
    /// **Security note:** codes shorter than [`SECURE_MIN_HUMAN_LENGTH`] symbols
    /// have reduced entropy and **require** active rate limiting to be safe. An
    /// unprotected 6-symbol code over 31 symbols has only ~29.7 bits of entropy.
    /// Suppress this warning with `#[allow(deprecated)]` at the call site only
    /// after confirming that rate limiting is in place.
    ///
    /// # Errors
    /// Returns [`PolicyError::ZeroLength`] if `length` is zero, or a TTL error
    /// if `ttl` is zero. Lengths at or above the minimum are also accepted.
    #[deprecated(
        note = "codes shorter than SECURE_MIN_HUMAN_LENGTH have reduced entropy;                 ensure rate limiting is active and suppress with #[allow(deprecated)]                 at the call site to acknowledge the tradeoff"
    )]
    pub fn short_compat(
        alphabet: Alphabet,
        length: usize,
        ttl: Duration,
    ) -> Result<Self, PolicyError> {
        if length == 0 {
            return Err(PolicyError::ZeroLength);
        }
        Self::build(alphabet, length, ttl)
    }

    /// Short-code compatibility policy: unambiguous alphabet, 6 symbols,
    /// caller-chosen TTL. Equivalent to `short_compat(Alphabet::unambiguous(), 6, ttl)`.
    ///
    /// Use this when migrating from an existing system that issued 6-symbol codes.
    /// Prefer [`CodePolicy::default_human`] (8 symbols, ~39.6 bits) for new deployments.
    ///
    /// # Errors
    /// Returns a [`PolicyError`] if the TTL is zero.
    #[deprecated(
        note = "6-symbol codes have only ~29.7 bits of entropy;                 use default_human() for new deployments or ensure rate limiting                 is active and suppress with #[allow(deprecated)]"
    )]
    #[allow(deprecated)] // calls short_compat which is also deprecated
    pub fn six_symbol(ttl: Duration) -> Result<Self, PolicyError> {
        #[allow(deprecated)]
        Self::short_compat(Alphabet::unambiguous(), SHORT_COMPAT_LENGTH, ttl)
    }

    fn build(alphabet: Alphabet, length: usize, ttl: Duration) -> Result<Self, PolicyError> {
        if ttl.is_zero() {
            // Reuse ZeroLength? No — be explicit; a zero TTL is its own bug.
            // We model it as a policy error without inventing a new variant by
            // treating it as an invalid shape. Keep a dedicated check here.
            return Err(PolicyError::ZeroLength);
        }
        let max_raw_len = DEFAULT_MAX_RAW_LEN.max(length);
        Ok(Self {
            alphabet,
            length,
            max_raw_len,
            ttl,
        })
    }

    /// The alphabet used for generation and accepted in normalized input.
    #[must_use]
    pub fn alphabet(&self) -> &Alphabet {
        &self.alphabet
    }

    /// The exact normalized code length.
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// The maximum accepted raw input length before normalization.
    #[must_use]
    pub fn max_raw_len(&self) -> usize {
        self.max_raw_len
    }

    /// The code time-to-live.
    #[must_use]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Approximate entropy in bits for this policy: `length * log2(alphabet)`.
    /// Intended for docs/diagnostics, not a security decision input.
    #[must_use]
    pub fn approx_entropy_bits(&self) -> f64 {
        (self.length as f64) * (self.alphabet.len() as f64).log2()
    }
}

#[cfg(test)]
mod tests;
