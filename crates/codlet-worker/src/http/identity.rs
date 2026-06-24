//! Rate-limit key extraction from Cloudflare Workers requests (RFC-033 §13).
//!
//! ## Trust model
//!
//! `CF-Connecting-IP` is set by Cloudflare's edge and cannot be spoofed by
//! the client (assuming the Worker is only reachable via Cloudflare, not
//! directly). It is the preferred source for a rate-limit key.
//!
//! Custom headers (e.g. `X-Real-IP`, `X-Forwarded-For`) are under the
//! caller's control and **must not be trusted** unless the Worker sits behind
//! a reverse proxy whose IP is verified. If you pass a `trusted_header`,
//! ensure you have validated that the proxy setting this header is
//! trustworthy.
//!
//! If neither header is available, this function returns `None`. Callers
//! decide whether to fail-open (allow) or fail-closed (deny) when no key
//! can be derived. Defaulting to `"unknown"` as a shared key is explicitly
//! rejected (RFC-033 §13 / RFC-010 §12.4): all requests would share one
//! counter, causing false positives.

use codlet::store::ratelimit::RateLimitKey;

/// Extract a rate-limit key from a Workers [`worker::Request`].
///
/// Strategy (in order):
/// 1. `CF-Connecting-IP` — set by Cloudflare's edge; not spoofable.
/// 2. `trusted_header` (if `Some`) — a custom header from a trusted proxy.
/// 3. Returns `None` — caller must decide how to handle the missing key.
///
/// # Security
///
/// Do **not** pass `trusted_header = Some("X-Forwarded-For")` unless you
/// have confirmed that only a trusted proxy can set that header for your
/// Worker. See module-level docs.
pub fn extract_rate_limit_key(
    req: &worker::Request,
    trusted_header: Option<&str>,
) -> Option<RateLimitKey> {
    let headers = req.headers();

    // CF-Connecting-IP is the authoritative source.
    if let Ok(Some(ip)) = headers.get("CF-Connecting-IP") {
        let ip = ip.trim().to_string();
        if !ip.is_empty() {
            return Some(RateLimitKey::new(ip));
        }
    }

    // Fall back to caller-specified trusted header.
    if let Some(header_name) = trusted_header {
        if let Ok(Some(val)) = headers.get(header_name) {
            // For X-Forwarded-For, take the leftmost (client) IP.
            let ip = val.split(',').next().unwrap_or("").trim().to_string();
            if !ip.is_empty() {
                return Some(RateLimitKey::new(ip));
            }
        }
    }

    // No trustworthy IP available. Caller decides: fail-open or fail-closed.
    // Do NOT fall back to "unknown" — see module-level docs.
    None
}
