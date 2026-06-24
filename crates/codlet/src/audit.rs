//! Security audit events and the `AuditSink` trait (RFC-012).
//!
//! [`CodeAuthEvent`] represents every notable security event codlet can emit.
//! All variants are **redacted by construction**: no plaintext code, token,
//! session secret, raw lookup key, HMAC key, or raw IP address appears in any
//! variant (RFC-012 §10.3).
//!
//! The host application provides an [`AuditSink`] implementation and maps
//! codlet events into its own audit schema, logging backend, or metrics
//! pipeline. codlet never makes logging decisions for the host.
//!
//! ## Forbidden content
//!
//! The following must never appear in any event field:
//! - plaintext code, token, or session secret;
//! - raw HMAC lookup key or key bytes;
//! - display name, email, or other personally identifiable free text;
//! - raw IP address (use a stable fingerprint / hashed value instead).

use crate::secret::{CodeId, SessionId, SubjectId};

/// A notable security event emitted by codlet (RFC-012 §10.2).
///
/// Variants use stable string names following `noun.verb.outcome` convention.
/// All fields are opaque identifiers or redacted fingerprints — no secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeAuthEvent {
    /// A one-time code was successfully issued and a record inserted.
    ///
    /// Event key: `code.issue.succeeded`
    CodeIssued {
        /// Opaque record ID (not the plaintext code or lookup key).
        code_id: CodeId,
        /// Optional host-provided purpose label.
        purpose: Option<String>,
    },

    /// A one-time code was successfully claimed (atomic winner).
    ///
    /// Event key: `code.redeem.succeeded`
    CodeRedeemed {
        /// The record that was claimed.
        code_id: CodeId,
        /// The subject that claimed it.
        subject_id: SubjectId,
    },

    /// A code redemption attempt failed.
    ///
    /// Event key: `code.redeem.failed`
    RedemptionFailed {
        /// Stable internal classification (safe for logs; not for users).
        reason: crate::error::RedemptionFailReason,
    },

    /// A code was administratively revoked.
    ///
    /// Event key: `code.revoke.succeeded`
    CodeRevoked {
        /// The record that was revoked.
        code_id: CodeId,
        /// Optional scope at which the revocation was scoped.
        scope: Option<String>,
    },

    /// A session was successfully issued.
    ///
    /// Event key: `session.issue.succeeded`
    SessionIssued {
        /// Opaque session record ID (not the bearer secret).
        session_id: SessionId,
        /// The authenticated subject.
        subject_id: SubjectId,
    },

    /// A session validation attempt found no valid session.
    ///
    /// Event key: `session.validate.failed`
    ///
    /// Emitted only when the host opts in; not emitted on every anonymous
    /// request to avoid log noise.
    SessionValidateFailed,

    /// A session was explicitly revoked (logout or incident response).
    ///
    /// Event key: `session.revoke.succeeded`
    SessionRevoked {
        /// The revoked session record ID.
        session_id: SessionId,
    },

    /// A form-token consume returned `Replay` (idempotent second submit).
    ///
    /// Event key: `form_token.consume.replay`
    FormTokenReplay {
        /// The purpose label of the token that was replayed.
        purpose: String,
    },

    /// A rate-limit threshold was exceeded.
    ///
    /// Event key: `rate_limit.blocked`
    RateLimitHit {
        /// A stable, privacy-safe fingerprint of the rate-limit key.
        /// Must not be the raw IP or raw user identifier (RFC-012 §10.3).
        key_fingerprint: String,
        /// The purpose or action class that was limited.
        purpose: Option<String>,
    },

    /// A key version was requested but not found in the provider.
    ///
    /// Event key: `key_provider.missing_version`
    KeyVersionMissing {
        /// The version label that was requested.
        version: crate::hashing::KeyVersion,
    },
}

impl CodeAuthEvent {
    /// A stable, machine-readable event key for this variant.
    ///
    /// Suitable for structured logging, metrics labels, and audit schemas.
    /// Keys follow the `noun.verb.outcome` convention from RFC-012 §10.2.
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            Self::CodeIssued { .. } => "code.issue.succeeded",
            Self::CodeRedeemed { .. } => "code.redeem.succeeded",
            Self::RedemptionFailed { .. } => "code.redeem.failed",
            Self::CodeRevoked { .. } => "code.revoke.succeeded",
            Self::SessionIssued { .. } => "session.issue.succeeded",
            Self::SessionValidateFailed => "session.validate.failed",
            Self::SessionRevoked { .. } => "session.revoke.succeeded",
            Self::FormTokenReplay { .. } => "form_token.consume.replay",
            Self::RateLimitHit { .. } => "rate_limit.blocked",
            Self::KeyVersionMissing { .. } => "key_provider.missing_version",
        }
    }
}

/// A recipient of security audit events (RFC-012 §3).
///
/// Implement this trait to connect codlet events to a logging backend, an
/// audit database, or a metrics pipeline. The implementation must not block the
/// calling thread for extended periods; use a background channel if the backend
/// is slow.
///
/// The implementation must not log the event in a way that violates the
/// redaction contract — i.e., it must not attempt to extract or store
/// plaintext secrets from the event fields.
pub trait AuditSink {
    /// Receive a security event. Called synchronously in the hot path; must
    /// return quickly. Fire-and-forget semantics: codlet does not retry on
    /// failure.
    fn record(&self, event: CodeAuthEvent);
}

/// A no-op audit sink that discards every event. Useful as a default when the
/// host has not configured a sink, and for unit tests that do not care about
/// events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&self, _event: CodeAuthEvent) {}
}

/// An audit sink that accumulates events in a `Vec` for inspection in tests.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Default)]
pub struct CollectingAuditSink {
    events: std::sync::Mutex<Vec<CodeAuthEvent>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl CollectingAuditSink {
    /// Construct an empty collecting sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain and return all collected events.
    pub fn drain(&self) -> Vec<CodeAuthEvent> {
        self.events.lock().unwrap().drain(..).collect()
    }

    /// Number of events collected so far.
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Whether any events have been collected.
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl AuditSink for CollectingAuditSink {
    fn record(&self, event: CodeAuthEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests;
