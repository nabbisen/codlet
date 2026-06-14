//! Cookie extraction and response header construction (RFC-033 §12).

use codlet_core::cookie::CookiePolicy;

/// Extract a named cookie value from a Workers [`worker::Request`].
///
/// Parses the `Cookie` header, finds the first `name=value` pair matching
/// `name`, and returns the raw value without URL-decoding. Returns `None`
/// if the header is absent or the named cookie is not present.
pub fn extract_cookie(req: &worker::Request, name: &str) -> Option<String> {
    let cookie_header = req.headers().get("Cookie").ok().flatten()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == name {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Build a [`worker::Headers`] object containing the `Set-Cookie` header
/// produced by [`CookiePolicy::build_set_cookie`] for `secret`.
///
/// Use the returned headers in a Workers `Response`:
///
/// ```rust,ignore
/// let headers = set_cookie_header(&cookie_policy, session_secret);
/// Response::ok("ok")?.with_headers(headers)
/// ```
///
/// # Panics
///
/// Panics if the `Set-Cookie` header value cannot be set — this indicates a
/// bug in the cookie builder, not a runtime condition.
pub fn set_cookie_header(policy: &CookiePolicy, secret: &str) -> worker::Headers {
    let headers = worker::Headers::new();
    let value = policy.build_set_cookie(secret);
    headers
        .set("Set-Cookie", &value)
        .expect("valid Set-Cookie header");
    headers
}

/// Build a [`worker::Headers`] object containing a `Set-Cookie` header that
/// clears the session cookie (produced by [`CookiePolicy::build_clear_cookie`]).
pub fn clear_cookie_header(policy: &CookiePolicy) -> worker::Headers {
    let headers = worker::Headers::new();
    let value = policy.build_clear_cookie();
    headers
        .set("Set-Cookie", &value)
        .expect("valid clear-cookie header");
    headers
}
