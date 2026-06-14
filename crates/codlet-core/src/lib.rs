#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

//! # codlet-core
//!
//! Runtime-neutral authentication primitives. This crate contains pure types,
//! policy objects, cryptographic lookup-key derivation, lifecycle state
//! machines, storage traits, and audit events. It deliberately contains no web
//! framework, database, or async-executor dependencies (RFC-002).
//!
//! ## Boundary
//!
//! codlet authenticates a subject. The host application authorizes that
//! subject (RFC-001). Nothing in this crate decides community membership,
//! roles, permissions, or resource access.
//!
//! ## Status
//!
//! This release completes the M3 primitive layer:
//!
//! - [`code`]    — code policy, generation, normalization, validation (RFC-003)
//! - [`hashing`] — HMAC lookup-key derivation, key providers, domain
//!                 separation, key versioning (RFC-004)
//! - [`rng`]     — fail-closed randomness abstraction (RFC-020)
//! - [`secret`]  — redacted secret newtypes and opaque IDs (RFC-019 foundation)
//! - [`clock`]   — `Clock` trait for testable time (RFC-020)
//! - [`state`]   — pure lifecycle classifiers: claim, session, form-token
//!                 consume (RFC-005/006/007)
//! - [`store`]   — `CodeStore`, `SessionStore`, `FormTokenStore`,
//!                 `RateLimitStore` traits (RFC-005/006/007/008)
//! - [`cookie`]  — secure cookie policy and builder (RFC-006)
//! - [`audit`]   — `CodeAuthEvent` vocabulary and `AuditSink` trait (RFC-012)
//! - [`error`]   — two-layer error model: internal causes + public-safe
//!                 failures (RFC-012/021)
//! - `mem`       — in-memory stores (`test-utils` feature only, RFC-011/008)

/// The codlet wire/format version embedded in domain-separated HMAC inputs.
///
/// Bumping this is a breaking change to every stored lookup key and MUST be
/// accompanied by a key-version migration (RFC-004).
pub const FORMAT_VERSION: &str = "codlet/v1";

pub mod audit;
pub mod clock;
pub mod code;
pub mod cookie;
pub mod error;
pub mod hashing;
pub mod rng;
pub mod secret;
pub mod state;
pub mod store;

/// In-memory store implementations for tests and local development.
///
/// **Not for production.** Gated behind the `test-utils` feature.
#[cfg(any(test, feature = "test-utils"))]
pub mod mem;

// Convenience re-exports for the most common types.
pub use audit::{AuditSink, CodeAuthEvent, NoopAuditSink};
pub use clock::{Clock, SystemClock};
pub use code::{Alphabet, CodePolicy, generate_code, normalize, validate_code_input};
pub use cookie::{CookiePolicy, CookieProfile, SameSitePolicy};
pub use error::{
    CodeInputError, KeyError, PolicyError, PublicFormError, PublicRedemptionError,
    PublicSessionError, RandomError, RedemptionFailReason,
};
pub use hashing::{
    HmacKeyRef, KeyProvider, KeyVersion, LookupKey, SecretDomain, SecretHasher, StaticKeyProvider,
};
pub use rng::{RandomSource, SystemRandom};
pub use secret::{
    CodeId, FormTokenSecret, PlainCode, SecretString, SessionId, SessionSecret, SubjectId,
};
pub use state::{
    ClaimOutcome, SessionValidationOutcome, TokenConsumeOutcome, classify_claim, classify_session,
    classify_token_consume,
};
pub use store::{
    error::{PublicAuthError, StoreError},
    ratelimit::{
        RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore, RateLimitUnavailable,
    },
    token::TokenSubject,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_version_is_stable() {
        // Guard against an accidental format bump. Changing this string is a
        // breaking change requiring a key-version migration (RFC-004).
        assert_eq!(FORMAT_VERSION, "codlet/v1");
    }
}
