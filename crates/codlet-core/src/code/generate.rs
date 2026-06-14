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
mod tests {
    use super::*;
    use crate::code::alphabet::Alphabet;
    use crate::rng::{AlwaysFailRandom, FixedBytesRandom, SystemRandom};
    use core::time::Duration;

    fn human() -> CodePolicy {
        CodePolicy::default_human(Duration::from_secs(3600)).unwrap()
    }

    #[test]
    fn generated_code_matches_policy_length_and_alphabet() {
        let policy = human();
        let mut rng = SystemRandom::new();
        let code = generate_code(&policy, &mut rng).unwrap();
        assert_eq!(code.expose().chars().count(), policy.length());
        let alpha = policy.alphabet();
        assert!(code.expose().bytes().all(|b| alpha.contains(b)));
    }

    #[test]
    fn rng_failure_fails_closed() {
        // Acceptance (RFC-003 §11.5): RNG that always errors yields no code.
        let policy = human();
        let mut rng = AlwaysFailRandom;
        assert_eq!(generate_code(&policy, &mut rng), Err(RandomError));
    }

    #[test]
    fn rejection_sampling_discards_bytes_at_or_above_ceiling() {
        // Alphabet len 31 → ceiling 248. Feed 248 (rejected) then 0 (accepted →
        // first symbol). A biased modulo-only generator would have used 248.
        #[allow(deprecated)]
        let policy = CodePolicy::six_symbol(Duration::from_secs(3600)).unwrap();
        let alpha = Alphabet::unambiguous();
        assert_eq!(alpha.unbiased_ceiling(), 248);
        // Sequence: 248 rejected, then 0,0,0,0,0,0 accepted → six of symbol[0].
        let mut rng = FixedBytesRandom::new(vec![248, 0]);
        let code = generate_code(&policy, &mut rng).unwrap();
        let first = alpha.symbols()[0] as char;
        assert_eq!(code.expose(), &first.to_string().repeat(6));
    }

    #[test]
    fn validate_accepts_normalizes_and_rejects() {
        let policy = human(); // length 8
        // Build a valid 8-char code from the alphabet with separators/lowercase.
        assert_eq!(
            validate_code_input("abcd-2345", &policy).unwrap(),
            "ABCD2345"
        );
        assert_eq!(validate_code_input("", &policy), Err(CodeInputError::Empty));
        assert_eq!(
            validate_code_input("ABCD234", &policy),
            Err(CodeInputError::WrongLength)
        );
        // '0' is not in the alphabet → unsupported (length is right at 8).
        assert_eq!(
            validate_code_input("ABCD2340", &policy),
            Err(CodeInputError::UnsupportedCharacters)
        );
        // Over the raw max.
        let long = "A".repeat(policy.max_raw_len() + 1);
        assert_eq!(
            validate_code_input(&long, &policy),
            Err(CodeInputError::TooLongRaw)
        );
    }
}
