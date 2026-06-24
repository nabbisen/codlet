//! Structured, redacted observability hooks (RFC-024).
//!
//! [`MetricsObserver`] is an optional, no-op-by-default trait for counters and
//! outcome tracking. Implementations must never include plaintext secrets,
//! lookup keys, subject IDs, or IP addresses in metric labels (RFC-024 §redaction).
//!
//! The recommended metric names follow a `codlet_<noun>_<verb>_total` pattern
//! (RFC-024 §metrics). High-cardinality labels (code IDs, user IDs, raw
//! scopes) must not be used as metric dimensions.
//!
//! ## Usage
//!
//! ```rust
//! use codlet::metrics::{MetricsObserver, NoopMetrics, Outcome};
//!
//! struct MyMetrics;
//! impl MetricsObserver for MyMetrics {
//!     fn increment(&self, counter: &'static str, outcome: Option<Outcome>) {
//!         // forward to your metrics backend (prometheus, statsd, …)
//!         let _ = (counter, outcome);
//!     }
//! }
//! ```

/// Outcome label for metrics that distinguish result categories.
///
/// Uses stable string values so metric dimensions don't change between
/// codlet versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// Code issue, claim won, session issue, form-token Proceed.
    Success,
    /// Claim lost, session missing/expired/revoked.
    Miss,
    /// Rate-limit threshold exceeded.
    RateLimited,
    /// Invalid input, wrong binding, expired token.
    Invalid,
    /// Replay detected on form-token or idempotency path.
    Replay,
    /// Transient store or key error.
    Error,
}

impl Outcome {
    /// Stable string label for use in metric dimensions.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Miss => "miss",
            Outcome::RateLimited => "rate_limited",
            Outcome::Invalid => "invalid",
            Outcome::Replay => "replay",
            Outcome::Error => "error",
        }
    }
}

/// Recommended counter names (RFC-024 §metrics).
///
/// Use these constants as the `counter` argument to
/// [`MetricsObserver::increment`] so metric names stay consistent across
/// adapters and host integrations.
pub mod counter {
    /// A one-time code was successfully issued.
    pub const CODE_ISSUED: &str = "codlet_code_issue_total";
    /// A code redemption was attempted (normalised and looked up).
    pub const CODE_REDEEM_ATTEMPT: &str = "codlet_code_redeem_attempt_total";
    /// The atomic claim succeeded (exactly one winner).
    pub const CODE_CLAIM_WON: &str = "codlet_code_claim_won_total";
    /// The atomic claim was lost to a concurrent caller.
    pub const CODE_CLAIM_LOST: &str = "codlet_code_claim_lost_total";
    /// A form-token consume call completed (use `outcome` to distinguish).
    pub const FORM_TOKEN_CONSUME: &str = "codlet_form_token_consume_total";
    /// A session was successfully issued.
    pub const SESSION_ISSUED: &str = "codlet_session_issue_total";
    /// A session validation attempt completed.
    pub const SESSION_VALIDATE: &str = "codlet_session_validate_total";
    /// A rate-limit check blocked an operation.
    pub const RATE_LIMIT_BLOCKED: &str = "codlet_rate_limit_block_total";
}

/// Optional observability sink for metrics and counters (RFC-024).
///
/// All implementations must be no-op by default (see [`NoopMetrics`]).
/// Implementations must not include high-cardinality or sensitive values in
/// metric labels — no code IDs, subject IDs, IP addresses, lookup keys, or
/// raw scopes.
pub trait MetricsObserver {
    /// Increment `counter` by 1, optionally tagging with `outcome`.
    ///
    /// Counter names should come from the [`counter`] module constants.
    /// This method is called in hot paths; it must not block.
    fn increment(&self, counter: &'static str, outcome: Option<Outcome>);
}

/// A no-op metrics observer. Compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMetrics;

impl MetricsObserver for NoopMetrics {
    #[inline]
    fn increment(&self, _counter: &'static str, _outcome: Option<Outcome>) {}
}

/// A metrics observer that records increments in a `Vec` for inspection
/// in tests. Available under the `test-utils` feature.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Default)]
pub struct CapturingMetrics {
    records: std::sync::Mutex<Vec<(&'static str, Option<Outcome>)>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl CapturingMetrics {
    /// Construct an empty capturing observer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all captured `(counter, outcome)` pairs.
    pub fn drain(&self) -> Vec<(&'static str, Option<Outcome>)> {
        self.records.lock().unwrap().drain(..).collect()
    }

    /// Count how many times `counter` was incremented.
    pub fn count(&self, counter: &'static str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| *c == counter)
            .count()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl MetricsObserver for CapturingMetrics {
    fn increment(&self, counter: &'static str, outcome: Option<Outcome>) {
        self.records.lock().unwrap().push((counter, outcome));
    }
}

#[cfg(test)]
mod tests;
