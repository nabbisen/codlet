//! `NoRateLimit` — a no-op [`RateLimitStore`] for hosts that opt out of
//! codlet-managed rate limiting.
//!
//! Satisfies the [`RateLimitStore`] bound without performing any I/O. Use this
//! as the `RL` type parameter of [`super::code::CodeAuth`] when the host
//! provides its own rate limiting at the network or application layer.

use crate::store::error::StoreError;
use crate::store::ratelimit::{RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore};

/// A no-op rate-limit store. Every `check` returns `Allow`; every
/// `record_failure` and `clear_failures` is a no-op.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoRateLimit;

impl RateLimitStore for NoRateLimit {
    async fn check(
        &self,
        _key: &RateLimitKey,
        _policy: &RateLimitPolicy,
    ) -> Result<RateLimitOutcome, StoreError> {
        Ok(RateLimitOutcome::Allow)
    }

    async fn record_failure(
        &self,
        _key: &RateLimitKey,
        _policy: &RateLimitPolicy,
    ) -> Result<(), StoreError> {
        Ok(())
    }

    async fn clear_failures(&self, _key: &RateLimitKey) -> Result<(), StoreError> {
        Ok(())
    }
}
