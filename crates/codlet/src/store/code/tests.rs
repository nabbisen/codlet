//! Unit tests for the `code` module.
use super::*;
use std::time::Duration;

#[test]
fn expires_at_from_ttl_adds_correctly() {
    assert_eq!(expires_at_from_ttl(1_000, Duration::from_secs(3600)), 4_600);
    assert_eq!(
        expires_at_from_ttl(u64::MAX, Duration::from_secs(1)),
        u64::MAX
    );
}
