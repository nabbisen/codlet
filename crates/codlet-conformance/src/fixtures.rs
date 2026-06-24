//! Shared test fixtures and helper functions for the conformance suite.

// Re-export all types needed by the conformance sub-modules so they can use
// `use crate::fixtures::*;` for a clean import.
pub use std::future::Future;
pub use std::sync::Arc;

pub use codlet::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};
pub use codlet::secret::{CodeId, SessionId, SubjectId};
pub use codlet::state::{ClaimOutcome, TokenConsumeOutcome};
pub use codlet::store::code::{ClaimRequest, CodeRecord, CodeStore};
pub use codlet::store::session::{SessionRecord, SessionStore};
pub use codlet::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

/// Fixed "now" timestamp used in all conformance tests.
pub const NOW: u64 = 1_700_000_000;
/// A timestamp one hour after [`NOW`]: codes/sessions with this expiry are redeemable.
pub const LATER: u64 = NOW + 3_600;
/// A timestamp one second before [`NOW`]: codes/sessions with this expiry are stale.
pub const EXPIRED: u64 = NOW - 1;

/// A deterministic hasher for use in conformance tests.
pub fn hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("v1", b"conformance-test-key".to_vec()).unwrap())
}

/// The standard key version used in conformance fixtures.
pub fn kv() -> KeyVersion {
    KeyVersion::new("v1")
}

/// Derive a code domain lookup key from a test value.
pub fn code_lk(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Code, val).unwrap().0
}

/// Derive a session domain lookup key from a test value.
pub fn session_lk(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::Session, val).unwrap().0
}

/// Derive a form-token domain lookup key from a test value.
pub fn token_lk(val: &str) -> codlet::LookupKey {
    hasher().lookup_key(SecretDomain::FormToken, val).unwrap().0
}

/// Build a code record for insertion in conformance tests.
pub fn code_record(id: &str, secret: &str, expires_at: u64, scope: Option<&str>) -> CodeRecord {
    CodeRecord {
        id: CodeId::new(id.into()),
        lookup_key: code_lk(secret),
        key_version: kv(),
        purpose: None,
        scope: scope.map(str::to_string),
        grant: Some(format!("grant-{id}")),
        created_at: NOW,
        expires_at,
    }
}

/// Build a session record for insertion in conformance tests.
pub fn session_record(id: &str, secret: &str, expires_at: u64) -> SessionRecord {
    SessionRecord {
        id: SessionId::new(id.into()),
        lookup_key: session_lk(secret),
        key_version: kv(),
        subject: SubjectId::new(format!("user-{id}")),
        created_at: NOW,
        expires_at,
    }
}

/// Build a form-token record for insertion in conformance tests.
pub fn token_record(
    secret: &str,
    subject: TokenSubject,
    purpose: &str,
    bound: Option<&str>,
    expires_at: u64,
) -> FormTokenRecord {
    FormTokenRecord {
        lookup_key: token_lk(secret),
        key_version: kv(),
        subject,
        purpose: purpose.into(),
        bound_resource: bound.map(str::to_string),
        issued_at: NOW,
        expires_at,
    }
}

/// Build an authenticated [`TokenSubject`] for test user `n`.
pub fn auth(n: u8) -> TokenSubject {
    TokenSubject::Authenticated(SubjectId::new(format!("user-{n}")))
}
