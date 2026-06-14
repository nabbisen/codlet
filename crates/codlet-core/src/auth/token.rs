//! Form-token manager (RFC-013 §3).
//!
//! [`FormTokenManager`] wraps the primitives needed to issue single-use form
//! tokens (CSRF protection + idempotency replay) and consume them atomically.

use crate::audit::{AuditSink, CodeAuthEvent};
use crate::clock::Clock;
use crate::hashing::{KeyProvider, SecretDomain, SecretHasher};
use crate::rng::RandomSource;
use crate::secret::FormTokenSecret;
use crate::state::TokenConsumeOutcome;
use crate::store::code::expires_at_from_ttl;
use crate::store::token::{FormTokenRecord, FormTokenStore, TokenSubject};

use super::error::FormTokenError;

/// Manages form-token issuance and consumption (RFC-013 §3).
pub struct FormTokenManager<TS, K, C, A> {
    store: TS,
    hasher: SecretHasher<K>,
    clock: C,
    audit: A,
    ttl: std::time::Duration,
}

impl<TS, K, C, A> FormTokenManager<TS, K, C, A>
where
    TS: FormTokenStore,
    K: KeyProvider,
    C: Clock,
    A: AuditSink,
{
    /// Construct a form-token manager with the given token TTL.
    ///
    /// A TTL of one hour matches the source service's `FORM_TOKEN_TTL_SECONDS`.
    #[must_use]
    pub fn new(
        store: TS,
        hasher: SecretHasher<K>,
        clock: C,
        audit: A,
        ttl: std::time::Duration,
    ) -> Self {
        Self {
            store,
            hasher,
            clock,
            audit,
            ttl,
        }
    }

    /// Issue a new form token for `subject` and `purpose`.
    ///
    /// Returns a [`FormTokenSecret`] (plaintext) to embed in the form or
    /// a short-lived cookie. The secret is never persisted; only its HMAC
    /// lookup key is stored (INV-1).
    ///
    /// # Errors
    /// Returns [`FormTokenError::Internal`] on RNG, hasher, or store failure.
    pub async fn issue<R: RandomSource>(
        &self,
        rng: &mut R,
        subject: TokenSubject,
        purpose: impl Into<String>,
        bound_resource: Option<String>,
    ) -> Result<FormTokenSecret, FormTokenError> {
        // 32 random bytes, hex-encoded → 64-char URL-safe token.
        let mut raw = [0u8; 32];
        rng.fill_bytes(&mut raw)
            .map_err(|e| FormTokenError::Internal {
                cause: format!("rng: {e}"),
                public: crate::error::PublicFormError::TemporarilyUnavailable,
            })?;
        let secret_hex = hex_lower(&raw);
        let secret = FormTokenSecret::new(secret_hex.clone());

        let (lookup_key, key_version) = self
            .hasher
            .lookup_key(SecretDomain::FormToken, secret.expose())
            .map_err(FormTokenError::from_key)?;

        let now = self.clock.unix_now();
        let purpose = purpose.into();

        self.store
            .insert_form_token(FormTokenRecord {
                lookup_key,
                key_version,
                subject,
                purpose,
                bound_resource,
                issued_at: now,
                expires_at: expires_at_from_ttl(now, self.ttl),
            })
            .await
            .map_err(FormTokenError::from_store)?;

        Ok(secret)
    }

    /// Consume a form token submitted by the client.
    ///
    /// Returns `Ok(None)` on `Proceed` (first winner), `Ok(Some(result_ref))`
    /// on `Replay` (idempotent second submit), or [`FormTokenError::Invalid`]
    /// on any rejection.
    ///
    /// Emits [`CodeAuthEvent::FormTokenReplay`] on replay.
    ///
    /// # Errors
    /// Returns [`FormTokenError::Invalid`] when the token is not accepted.
    /// Returns [`FormTokenError::Internal`] on store/key failure.
    pub async fn consume(
        &self,
        raw_token: &str,
        subject: &TokenSubject,
        purpose: &str,
        bound_resource: Option<&str>,
    ) -> Result<Option<String>, FormTokenError> {
        let (lookup_key, _) = self
            .hasher
            .lookup_key(SecretDomain::FormToken, raw_token)
            .map_err(FormTokenError::from_key)?;

        let now = self.clock.unix_now();
        let (outcome, result_ref) = self
            .store
            .consume_form_token(&lookup_key, subject, purpose, bound_resource, now)
            .await
            .map_err(FormTokenError::from_store)?;

        match outcome {
            TokenConsumeOutcome::Proceed => Ok(None),
            TokenConsumeOutcome::Replay => {
                self.audit.record(CodeAuthEvent::FormTokenReplay {
                    purpose: purpose.to_string(),
                });
                Ok(result_ref)
            }
            TokenConsumeOutcome::Invalid => Err(FormTokenError::Invalid {
                public: crate::error::PublicFormError::ExpiredOrInvalid,
            }),
        }
    }

    /// Store a result reference on a consumed token for idempotency replay.
    ///
    /// # Errors
    /// Returns [`FormTokenError::Internal`] on store failure.
    pub async fn set_result(
        &self,
        raw_token: &str,
        result_ref: &str,
    ) -> Result<(), FormTokenError> {
        let (lookup_key, _) = self
            .hasher
            .lookup_key(SecretDomain::FormToken, raw_token)
            .map_err(FormTokenError::from_key)?;
        self.store
            .set_token_result(&lookup_key, result_ref)
            .await
            .map_err(FormTokenError::from_store)
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}
