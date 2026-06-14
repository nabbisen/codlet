//! Rate-limit policy and storage trait (RFC-008).
//!
//! Short human-friendly codes must be protected against online guessing.
//! codlet's rate-limit model is:
//!
//! 1. The **host** computes a [`RateLimitKey`] from a trustworthy source
//!    (e.g. a verified client IP from a trusted proxy header, or a
//!    scope+purpose combination).
//! 2. codlet checks the key **before** the expensive lookup.
//! 3. On a failed redemption, codlet records the failure.
//! 4. On a successful redemption, the caller may clear the failures.
//!
//! codlet never parses network headers. Trustworthiness of the key is the
//! host's responsibility (RFC-008 §6).

use std::future::Future;
use std::time::Duration;

use crate::store::error::StoreError;

/// A rate-limit dimension key supplied by the host (RFC-008 §4).
///
/// The key should be derived from a trustworthy, non-spoofable signal.
/// It must never be the raw plaintext code or a user-display identifier.
/// The recommended shape is `HMAC(purpose || 0x00 || ip_or_scope)` or a
/// stable fingerprint that the host can compute without codlet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RateLimitKey(String);

impl RateLimitKey {
    /// Wrap a pre-computed key string.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Borrow the key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A privacy-safe fingerprint of the key, safe to include in audit events
    /// and metrics labels (RFC-012 §10.3). Currently the first 8 characters
    /// of the key; adapters may override with a hashed prefix.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        let end = self
            .0
            .char_indices()
            .nth(8)
            .map(|(i, _)| i)
            .unwrap_or(self.0.len());
        &self.0[..end]
    }
}

/// Behaviour when the rate-limit store is unavailable (RFC-008 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateLimitUnavailable {
    /// Allow the operation to proceed; log the store error internally.
    /// Appropriate when rate limiting is a defence-in-depth layer and
    /// availability is preferred over strict enforcement.
    #[default]
    FailOpen,
    /// Deny the operation. Appropriate when rate limiting is a hard
    /// requirement and availability is secondary.
    FailClosed,
    /// Allow until the counter reaches `n` above the normal threshold,
    /// then deny. A compromise for services with intermittent store issues.
    SoftDenyAfterThreshold(u32),
}

/// Rate-limit policy (RFC-008 §4).
#[derive(Debug, Clone)]
pub struct RateLimitPolicy {
    /// Maximum number of recorded failures within `window` before blocking.
    pub max_failures: u32,
    /// Rolling window over which failures are counted.
    pub window: Duration,
    /// What to do when the rate-limit store is unreachable.
    pub unavailable: RateLimitUnavailable,
}

impl RateLimitPolicy {
    /// Sensible default: 10 failures in 5 minutes, fail-open.
    /// Matches the source service's `10 failures / 5 min / IP` policy.
    #[must_use]
    pub fn default_invite() -> Self {
        Self {
            max_failures: 10,
            window: Duration::from_secs(5 * 60),
            unavailable: RateLimitUnavailable::FailOpen,
        }
    }

    /// Whether a given failure count is at or over the threshold.
    #[must_use]
    pub fn is_exceeded(&self, failures: u32) -> bool {
        failures >= self.max_failures
    }
}

/// The result of a rate-limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitOutcome {
    /// The key is within the policy limit; proceed with the operation.
    Allow,
    /// The key has exceeded the policy limit; deny the operation.
    Deny,
}

/// Rate-limit storage (RFC-008 §4).
///
/// Implementations record failure counts within a rolling window keyed by
/// [`RateLimitKey`]. All methods are infallible from the caller's perspective;
/// backend errors are handled per [`RateLimitUnavailable`].
pub trait RateLimitStore {
    /// Check whether the key is within the policy limit **before** an
    /// operation. Does not mutate state.
    fn check(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> impl Future<Output = Result<RateLimitOutcome, StoreError>>;

    /// Record a failure for the given key within the current window.
    fn record_failure(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> impl Future<Output = Result<(), StoreError>>;

    /// Clear all failure counters for the given key (called after a
    /// successful redemption so legitimate users are not locked out).
    fn clear_failures(&self, key: &RateLimitKey) -> impl Future<Output = Result<(), StoreError>>;
}

#[cfg(test)]
mod tests {
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
}
