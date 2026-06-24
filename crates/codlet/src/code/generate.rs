//! Code generation (RFC-003 §4, FR-1) and input validation (FR-2).
//!
//! Generation uses rejection sampling to avoid modulo bias and fails closed on
//! RNG error — never substituting a deterministic or partial value (INV-3).

use super::normalize::normalize;
use super::policy::CodePolicy;
use crate::error::{CodeInputError, RandomError};
use crate::rng::RandomSource;
use crate::secret::PlainCode;

/// Generate a fresh plaintext code under `policy`, drawing randomness from
/// `rng`.
///
/// The algorithm (RFC-003 §4): for each position, read one random byte; accept
/// it only if below the alphabet's [unbiased ceiling], then map
/// `alphabet[byte % len]`; otherwise discard and redraw.
///
/// [unbiased ceiling]: super::alphabet::Alphabet::unbiased_ceiling
///
/// # Errors
/// Returns [`RandomError`] if the RNG fails at any point. On error no code is
/// produced; there is no fallback (INV-3).
pub fn generate_code<R: RandomSource>(
    policy: &CodePolicy,
    rng: &mut R,
) -> Result<PlainCode, RandomError> {
    let alphabet = policy.alphabet();
    let ceiling = alphabet.unbiased_ceiling();
    let mut out = String::with_capacity(policy.length());

    while out.len() < policy.length() {
        let mut buf = [0u8; 1];
        // Propagate RNG failure immediately — fail closed.
        rng.fill_bytes(&mut buf)?;
        let b = buf[0];
        if (b as usize) < ceiling {
            out.push(alphabet.symbol_for_byte(b) as char);
        }
        // else: above ceiling, discard and redraw (rejection sampling).
    }

    Ok(PlainCode::new(out))
}

/// Validate and normalize raw user-supplied code input under `policy`.
///
/// Runs before any storage lookup so garbage never reaches the database
/// (RFC-003 FR-2). Returns the canonical normalized string on success.
///
/// # Errors
/// Returns [`CodeInputError`] for empty input, raw input over the policy
/// maximum, a normalized length mismatch, or characters outside the policy
/// alphabet. All variants are intended to collapse to one generic public
/// message (INV-8); the distinction is for internal diagnostics only.
pub fn validate_code_input(raw: &str, policy: &CodePolicy) -> Result<String, CodeInputError> {
    if raw.is_empty() {
        return Err(CodeInputError::Empty);
    }
    if raw.len() > policy.max_raw_len() {
        return Err(CodeInputError::TooLongRaw);
    }
    let normalized = normalize(raw);
    if normalized.is_empty() {
        return Err(CodeInputError::Empty);
    }
    // Length is counted in characters; the accepted alphabet is ASCII so a
    // char count equals the byte count for valid input.
    if normalized.chars().count() != policy.length() {
        return Err(CodeInputError::WrongLength);
    }
    let alphabet = policy.alphabet();
    if !normalized.bytes().all(|b| alphabet.contains(b)) {
        return Err(CodeInputError::UnsupportedCharacters);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests;
