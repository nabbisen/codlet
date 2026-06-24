//! Time abstraction (RFC-020 clock contract).
//!
//! All expiry checks go through [`Clock`] so production code is testable with
//! a fixed time without system-clock dependencies. The clock is always
//! wall-time monotonic in production; only `FixedClock` (under `test-utils`) is non-monotonic.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A source of the current wall-clock time, expressed as seconds since the
/// Unix epoch (UTC). Implementations must be infallible and must return a
/// non-decreasing value in production.
pub trait Clock {
    /// Current time as seconds since the Unix epoch (UTC).
    fn unix_now(&self) -> u64;

    /// Convenience: current time plus `offset`.
    fn unix_now_plus(&self, offset: Duration) -> u64 {
        self.unix_now().saturating_add(offset.as_secs())
    }
}

/// Production clock backed by [`SystemTime`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl SystemClock {
    /// Construct the system clock.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn unix_now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Deterministic clock that always returns the same instant. Available under
/// `test-utils` and in this crate's own tests. Useful for expiry boundary tests.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(pub u64);

#[cfg(any(test, feature = "test-utils"))]
impl FixedClock {
    /// A clock pinned to `unix_secs`.
    #[must_use]
    pub fn at(unix_secs: u64) -> Self {
        Self(unix_secs)
    }

    /// Advance the fixed clock by `secs`, returning a new `FixedClock`.
    #[must_use]
    pub fn advance(self, secs: u64) -> Self {
        Self(self.0.saturating_add(secs))
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Clock for FixedClock {
    fn unix_now(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests;
