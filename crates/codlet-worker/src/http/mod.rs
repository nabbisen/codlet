//! HTTP request helpers for cookie extraction and rate-limit key derivation
//! (RFC-033 §12, §13).

pub mod cookies;
pub mod identity;

pub use cookies::{extract_cookie, set_cookie_header};
pub use identity::extract_rate_limit_key;
