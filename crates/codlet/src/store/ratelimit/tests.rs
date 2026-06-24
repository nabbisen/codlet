//! Unit tests for the `ratelimit` module.
use super::*;

#[test]
fn default_policy_thresholds() {
    let p = RateLimitPolicy::default_invite();
    assert_eq!(p.max_failures, 10);
    assert!(!p.is_exceeded(9));
    assert!(p.is_exceeded(10));
    assert!(p.is_exceeded(11));
}

#[test]
fn fingerprint_is_prefix_not_full_key() {
    let k = RateLimitKey::new("abcdefghijklmnop");
    assert_eq!(k.fingerprint(), "abcdefgh");
    let short = RateLimitKey::new("ab");
    assert_eq!(short.fingerprint(), "ab");
}

#[test]
fn key_roundtrips() {
    let k = RateLimitKey::new("test-key");
    assert_eq!(k.as_str(), "test-key");
}
