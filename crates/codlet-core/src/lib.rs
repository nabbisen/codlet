#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

//! # codlet-core
//!
//! Runtime-neutral authentication primitives. This crate contains pure types,
//! policy objects, cryptographic lookup-key derivation, lifecycle state
//! machines, and storage *traits*. It deliberately contains no web framework,
//! database, or async-executor dependencies (RFC-002).
//!
//! ## Boundary
//!
//! codlet authenticates a subject. The host application authorizes that
//! subject (RFC-001). Nothing in this crate decides community membership,
//! roles, permissions, or resource access.
//!
//! ## Status
//!
//! This release implements the first cryptographic primitives:
//!
//! - [`code`]    — code policy, generation, normalization, validation (RFC-003)
//! - [`hashing`] — HMAC lookup-key derivation, key providers, domain
//!                 separation, key versioning (RFC-004)
//! - [`rng`]     — fail-closed randomness abstraction (RFC-020)
//! - [`secret`]  — redacted secret newtypes and opaque IDs (RFC-019 foundation)
//! - [`error`]   — internal error layer (RFC-021)
//!
//! Forthcoming modules (added with their RFCs):
//!
//! - `state` — pure lifecycle classifiers (RFC-005/006/007)
//! - `store` — storage traits (RFC-005..008)
//!
//! ## Example
//!
//! Generate a code and derive the value that would be stored (never the
//! plaintext). End-to-end redemption needs the storage traits, still to come.
//!
//! ```
//! use codlet_core::{CodePolicy, SecretDomain, SecretHasher, StaticKeyProvider};
//! use codlet_core::{generate_code, validate_code_input};
//! use codlet_core::rng::SystemRandom;
//! use std::time::Duration;
//!
//! let policy = CodePolicy::default_human(Duration::from_secs(24 * 3600)).unwrap();
//!
//! let mut rng = SystemRandom::new();
//! let code = generate_code(&policy, &mut rng).unwrap();
//!
//! let hasher = SecretHasher::new(
//!     StaticKeyProvider::single("v1", b"real-key-from-secret-manager".to_vec()).unwrap(),
//! );
//! let normalized = validate_code_input(code.expose(), &policy).unwrap();
//! let (lookup_key, key_version) =
//!     hasher.lookup_key(SecretDomain::Code, &normalized).unwrap();
//! assert_eq!(key_version.as_str(), "v1");
//! assert_eq!(lookup_key.as_str().len(), 64);
//! // Persist `lookup_key` + `key_version`; never persist `code`.
//! ```

/// The codlet wire/format version embedded in domain-separated HMAC inputs.
///
/// Bumping this is a breaking change to every stored lookup key and MUST be
/// accompanied by a key-version migration (RFC-004).
pub const FORMAT_VERSION: &str = "codlet/v1";

pub mod code;
pub mod error;
pub mod hashing;
pub mod rng;
pub mod secret;

// Convenience re-exports for the most common types.
pub use code::{Alphabet, CodePolicy, generate_code, normalize, validate_code_input};
pub use error::{CodeInputError, KeyError, PolicyError, RandomError};
pub use hashing::{
    HmacKeyRef, KeyProvider, KeyVersion, LookupKey, SecretDomain, SecretHasher, StaticKeyProvider,
};
pub use rng::{RandomSource, SystemRandom};
pub use secret::{
    CodeId, FormTokenSecret, PlainCode, SecretString, SessionId, SessionSecret, SubjectId,
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
