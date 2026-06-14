//! In-memory rate-limit store (RFC-008 §13.5, RFC-011 §10.3). Non-production.
//!
//! Uses a `Mutex<HashMap>` with wall-clock expiry. Counter atomicity is
//! best-effort within a single process; this mirrors the source service's KV
//! behaviour note ("KV read-modify-write counters can under-count under
//! concurrency" — RFC-008 §5) and is acceptable for testing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::store::error::StoreError;
use crate::store::ratelimit::{RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore};

#[derive(Debug)]
struct Window {
    failures: u32,
    window_start: Instant,
}

/// **Non-production** in-memory rate-limit store.
///
/// Uses `std::time::Instant` for the window. In production use the store
/// appropriate for your deployment (Workers KV, Redis, SQL with timestamps).
#[derive(Debug, Default)]
pub struct MemRateLimitStore {
    windows: Mutex<HashMap<String, Window>>,
}

impl MemRateLimitStore {
    /// Construct an empty in-memory rate-limit store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl RateLimitStore for MemRateLimitStore {
    async fn check(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitOutcome, StoreError> {
        let map = self
            .windows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let failures = map.get(key.as_str()).map_or(0, |w| {
            if w.window_start.elapsed() < policy.window {
                w.failures
            } else {
                0
            }
        });
        Ok(if policy.is_exceeded(failures) {
            RateLimitOutcome::Deny
        } else {
            RateLimitOutcome::Allow
        })
    }

    async fn record_failure(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<(), StoreError> {
        let mut map = self
            .windows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let entry = map
            .entry(key.as_str().to_string())
            .or_insert_with(|| Window {
                failures: 0,
                window_start: Instant::now(),
            });
        // Reset counter if window has elapsed.
        if entry.window_start.elapsed() >= policy.window {
            entry.failures = 0;
            entry.window_start = Instant::now();
        }
        entry.failures = entry.failures.saturating_add(1);
        Ok(())
    }

    async fn clear_failures(&self, key: &RateLimitKey) -> Result<(), StoreError> {
        let mut map = self
            .windows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        map.remove(key.as_str());
        Ok(())
    }
}
