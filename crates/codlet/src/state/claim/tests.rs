//! Unit tests for the `claim` module.
use super::*;

#[test]
fn one_row_changed_wins() {
    assert_eq!(classify_claim(1), ClaimOutcome::Won);
}

#[test]
fn zero_rows_lost() {
    assert_eq!(classify_claim(0), ClaimOutcome::Lost);
}

#[test]
fn invariant_violation_returns_lost_conservatively() {
    // >1 is a storage bug; we must never return Won.
    for bad in [2usize, 100] {
        assert_eq!(
            classify_claim(bad),
            ClaimOutcome::Lost,
            "changed={bad} must be Lost"
        );
    }
}

#[test]
fn only_exactly_one_produces_won() {
    // Property: the only way to get Won is changed == 1.
    for n in 0usize..=10 {
        let outcome = classify_claim(n);
        if n == 1 {
            assert_eq!(outcome, ClaimOutcome::Won);
        } else {
            assert_eq!(outcome, ClaimOutcome::Lost);
        }
    }
}
