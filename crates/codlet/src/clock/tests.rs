//! Unit tests for the `clock` module.
use super::*;

#[test]
fn system_clock_is_positive() {
    let t = SystemClock::new().unix_now();
    // 2024-01-01T00:00:00Z = 1_704_067_200
    assert!(t > 1_704_067_200, "clock looks wrong: {t}");
}

#[test]
fn fixed_clock_is_deterministic() {
    let c = FixedClock::at(1_000_000);
    assert_eq!(c.unix_now(), 1_000_000);
    assert_eq!(c.unix_now(), 1_000_000);
}

#[test]
fn unix_now_plus_offsets_correctly() {
    let c = FixedClock::at(1_000);
    assert_eq!(c.unix_now_plus(Duration::from_secs(100)), 1_100);
}

#[test]
fn advance_produces_later_clock() {
    let c = FixedClock::at(1_000).advance(60);
    assert_eq!(c.unix_now(), 1_060);
}
