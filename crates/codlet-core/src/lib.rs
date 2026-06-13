#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
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
//! This is the Phase 0 skeleton. The modules below are introduced by their
//! respective RFCs as implementation lands:
//!
//! - `code`    — code policy, generation, normalization, validation (RFC-003)
//! - `hashing` — HMAC lookup-key derivation, key providers, domain separation,
//!               key versioning (RFC-004)
//! - `state`   — pure lifecycle classifiers: claim / token-consume / session
//!               validation (RFC-005/006/007)
//! - `store`   — `CodeStore`, `SessionStore`, `FormTokenStore`,
//!               `RateLimitStore` traits (RFC-005..008)
//! - `error`   — internal vs public-safe error model (RFC-012/021)
//!
//! Until those RFCs are accepted and implemented, this crate exposes only the
//! crate-level documentation and version constant below.

/// The codlet wire/format version embedded in domain-separated HMAC inputs.
///
/// Bumping this is a breaking change to every stored lookup key and MUST be
/// accompanied by a key-version migration (RFC-004).
pub const FORMAT_VERSION: &str = "codlet/v1";

// Modules are added here as their RFCs are implemented. Keeping them out of the
// skeleton avoids shipping placeholder security code, which would be worse than
// an honest absence.
//
// pub mod code;     // RFC-003
// pub mod hashing;  // RFC-004
// pub mod state;    // RFC-005/006/007
// pub mod store;    // RFC-005..008
// pub mod error;    // RFC-012/021

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
