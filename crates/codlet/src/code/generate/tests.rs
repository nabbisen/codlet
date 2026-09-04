//! Unit tests for the `generate` module.
use super::*;
use crate::code::alphabet::Alphabet;
use crate::rng::{AlwaysFailRandom, FixedBytesRandom, SystemRandom};
use core::time::Duration;
use proptest::prelude::*;

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

// ── RFC-041: INV-4 properties P-2 (issue/redeem agreement) and P-6 (rejection) ─

/// A subsequence of uppercase ASCII letters and digits: normalization
/// fixed-points, so `Alphabet::new` accepts them. P-2 checks "for any
/// [safe] policy"; P-3 (`code::alphabet::tests::p3_*`) is the general form,
/// checking every symbol `Alphabet::new` can accept at all -- which, since
/// RFC-043, is guaranteed to be a normalization fixed-point by construction.
fn safe_alphabet_symbols() -> impl Strategy<Value = Vec<u8>> {
    let pool: Vec<u8> = (b'A'..=b'Z').chain(b'0'..=b'9').collect();
    proptest::sample::subsequence(pool, 2..=36)
}

proptest! {
    #[test]
    fn p2_generated_code_is_a_normalization_fixed_point_under_safe_policies(
        symbols in safe_alphabet_symbols(),
        length in 1usize..=16,
    ) {
        // P-2: normalize(generate_code(p)) == generate_code(p), for any
        // policy built from a normalization-safe alphabet.
        let alphabet = Alphabet::new(&symbols).unwrap();
        #[allow(deprecated)]
        let policy =
            CodePolicy::short_compat(alphabet, length, Duration::from_secs(3600)).unwrap();
        let mut rng = SystemRandom::new();
        let code = generate_code(&policy, &mut rng).unwrap();
        let plain = code.expose().to_string();
        prop_assert_eq!(normalize(&plain), plain);
    }
}

#[test]
fn p6_every_byte_at_or_above_ceiling_is_rejected_never_mapped() {
    // P-6: exhaustive over the default alphabet's rejected byte range
    // (248..256). Each rejected byte is fed first, then a distinguishable
    // accepted byte -- distinguishable so a broken (non-rejecting)
    // implementation would produce an observably different symbol, not one
    // that coincidentally matches.
    let alphabet = Alphabet::unambiguous();
    let ceiling = alphabet.unbiased_ceiling();
    let len = alphabet.len();
    #[allow(deprecated)]
    let policy = CodePolicy::short_compat(alphabet.clone(), 1, Duration::from_secs(3600)).unwrap();

    for rejected in ceiling..256 {
        let rejected = rejected as u8;
        let rejected_residue = rejected as usize % len;
        let accepted = ((rejected_residue + 1) % len) as u8;
        assert!(
            (accepted as usize) < ceiling,
            "test construction error: accepted byte must itself be < ceiling"
        );

        let mut rng = FixedBytesRandom::new(vec![rejected, accepted]);
        let code = generate_code(&policy, &mut rng).unwrap();
        let expected = alphabet.symbol_for_byte(accepted) as char;
        assert_eq!(
            code.expose(),
            &expected.to_string(),
            "byte {rejected} (>= ceiling {ceiling}) was not rejected"
        );
    }
}
