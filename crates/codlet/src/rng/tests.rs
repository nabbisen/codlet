//! Unit tests for the `rng` module.
use super::*;

#[test]
fn system_random_fills_distinct_buffers() {
    let mut r = SystemRandom::new();
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    r.fill_bytes(&mut a).unwrap();
    r.fill_bytes(&mut b).unwrap();
    // Astronomically unlikely to be equal if real entropy is used.
    assert_ne!(a, b);
    assert!(a.iter().any(|&x| x != 0));
}

#[test]
fn always_fail_random_errors() {
    let mut r = AlwaysFailRandom;
    let mut buf = [0u8; 4];
    assert_eq!(r.fill_bytes(&mut buf), Err(RandomError));
}

#[test]
fn fixed_bytes_random_is_deterministic() {
    let mut r = FixedBytesRandom::new(vec![1, 2, 3]);
    let mut buf = [0u8; 5];
    r.fill_bytes(&mut buf).unwrap();
    assert_eq!(buf, [1, 2, 3, 1, 2]);
}
