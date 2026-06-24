//! Unit tests for the `policy` module.
use super::*;

const HOUR: Duration = Duration::from_secs(3600);

#[test]
fn default_human_is_8_and_unambiguous() {
    let p = CodePolicy::default_human(HOUR).unwrap();
    assert_eq!(p.length(), 8);
    assert_eq!(p.alphabet().len(), 31);
    assert!((p.approx_entropy_bits() - 39.6).abs() < 0.2);
}

#[test]
fn new_rejects_below_minimum() {
    let err = CodePolicy::new(Alphabet::unambiguous(), 6, HOUR).unwrap_err();
    assert_eq!(err, PolicyError::LengthBelowMinimum { got: 6, min: 8 });
}

#[test]
#[allow(deprecated)]
fn short_compat_allows_six() {
    #[allow(deprecated)]
    let p = CodePolicy::six_symbol(HOUR).unwrap();
    assert_eq!(p.length(), 6);
    assert!((p.approx_entropy_bits() - 29.7).abs() < 0.2);
}

#[test]
#[allow(deprecated)]
fn zero_length_and_zero_ttl_rejected() {
    assert_eq!(
        CodePolicy::short_compat(Alphabet::unambiguous(), 0, HOUR),
        Err(PolicyError::ZeroLength)
    );
    assert_eq!(
        CodePolicy::default_human(Duration::ZERO),
        Err(PolicyError::ZeroLength)
    );
}
