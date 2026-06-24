//! KV-backed [`RateLimitStore`] implementation (RFC-033 §11).
//!
//! ## Consistency caveat (required by RFC-033 §11 / RFC-010 §12.3)
//!
//! Cloudflare KV is **eventually consistent**. Under a high-concurrency or
//! distributed attack, failure counters may be read stale and therefore
//! under-count actual attempts. This implementation is suitable for friction
//! reduction against unsophisticated bots, not as the sole defence against
//! determined attackers.
//!
//! For stronger rate-limit guarantees, combine this store with:
//! - Cloudflare WAF custom rules (IP-based blocking at the edge)
//! - Cloudflare Turnstile (challenge-based friction)
//! - D1-backed counters with serialised writes (higher latency, stronger
//!   consistency)

use codlet::store::error::StoreError;
use codlet::store::ratelimit::{
    RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore, RateLimitUnavailable,
};

/// Rate-limit store backed by Cloudflare KV (RFC-033 §11).
///
/// Key format: `"codlet:rl:{fingerprint}"` where `fingerprint` is the first
/// 8 characters of the [`RateLimitKey`] value — safe for logging, does not
/// expose a full IP or user identifier.
///
/// Value format: a JSON-encoded `u32` failure count.
///
/// TTL: set to `policy.window.as_secs()` on every `record_failure` write,
/// so the counter expires automatically after one window with no failures.
///
/// ## Caveat
///
/// KV is eventually consistent. See module-level docs for details.
pub struct KvRateLimitStore {
    kv: worker_kv::KvStore,
}

impl KvRateLimitStore {
    /// Construct from a bound KV namespace.
    pub fn new(kv: worker_kv::KvStore) -> Self {
        Self { kv }
    }

    fn kv_key(&self, key: &RateLimitKey) -> String {
        format!("codlet:rl:{}", key.fingerprint())
    }
}

impl RateLimitStore for KvRateLimitStore {
    async fn check(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitOutcome, StoreError> {
        let kv_key = self.kv_key(key);
        match self.kv.get(&kv_key).text().await {
            Ok(None) => Ok(RateLimitOutcome::Allow),
            Ok(Some(json)) => {
                let count: u32 = serde_json::from_str(&json).unwrap_or(0);
                Ok(if policy.is_exceeded(count) {
                    RateLimitOutcome::Deny
                } else {
                    RateLimitOutcome::Allow
                })
            }
            Err(e) => match policy.unavailable {
                RateLimitUnavailable::FailOpen => Ok(RateLimitOutcome::Allow),
                RateLimitUnavailable::FailClosed => {
                    Err(StoreError::Backend(format!("KV check failed: {e}")))
                }
            },
        }
    }

    async fn record_failure(
        &self,
        key: &RateLimitKey,
        policy: &RateLimitPolicy,
    ) -> Result<(), StoreError> {
        let kv_key = self.kv_key(key);
        // GET → increment → PUT with TTL.
        // KV is eventually consistent; under concurrency, increments may be
        // lost (RFC-033 §11 / RFC-010 §12.3).
        let current: u32 = self
            .kv
            .get(&kv_key)
            .text()
            .await
            .ok()
            .flatten()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or(0);

        let next = current.saturating_add(1);
        let ttl = policy.window.as_secs().max(60); // minimum 60 s per KV limits

        let json = serde_json::to_string(&next).map_err(|e| StoreError::Backend(e.to_string()))?;

        self.kv
            .put(&kv_key, json)
            .map_err(|e| StoreError::Backend(e.to_string()))?
            .expiration_ttl(ttl)
            .execute()
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn clear_failures(&self, key: &RateLimitKey) -> Result<(), StoreError> {
        let kv_key = self.kv_key(key);
        self.kv
            .delete(&kv_key)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))
    }
}
