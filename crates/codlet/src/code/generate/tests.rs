//! Unit tests for the `generate` module.
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
