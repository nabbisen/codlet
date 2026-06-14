//! Session manager (RFC-013 §3).
//!
//! [`SessionManager`] composes [`SessionStore`], [`SecretHasher`], [`Clock`],
//! [`CookiePolicy`], and [`AuditSink`] into the three session operations:
//! issue (after a won claim), validate (on every authenticated request), and
//! revoke (on logout or incident response).

use crate::audit::{AuditSink, CodeAuthEvent};
use crate::clock::Clock;
use crate::cookie::CookiePolicy;
use crate::hashing::{KeyProvider, SecretDomain, SecretHasher};
use crate::rng::RandomSource;
use crate::secret::{SessionId, SessionSecret};
use crate::state::{SessionValidationOutcome, classify_session};
use crate::store::code::expires_at_from_ttl;
use crate::store::session::{SessionRecord, SessionStore};

use super::error::{IssuedSession, RedeemSuccess, SessionError};

/// Manages session issuance, validation, and revocation (RFC-013 §3).
///
/// Session issuance requires a [`RedeemSuccess`] proof to enforce the
/// invariant that sessions can only be created after a confirmed won claim
/// (RFC-013 §5, acceptance checklist: "session issuance cannot occur before
/// claim success").
pub struct SessionManager<SS, K, C, A> {
    store: SS,
    hasher: SecretHasher<K>,
    clock: C,
    audit: A,
    cookie_policy: CookiePolicy,
}

impl<SS, K, C, A> SessionManager<SS, K, C, A>
where
    SS: SessionStore,
    K: KeyProvider,
    C: Clock,
    A: AuditSink,
{
    /// Construct a session manager.
    #[must_use]
    pub fn new(
        store: SS,
        hasher: SecretHasher<K>,
        clock: C,
        audit: A,
        cookie_policy: CookiePolicy,
    ) -> Self {
        Self {
            store,
            hasher,
            clock,
            audit,
            cookie_policy,
        }
    }

    /// Issue a new session for the authenticated subject.
    ///
    /// Requires a [`RedeemSuccess`] proof so this cannot be called without a
    /// prior confirmed won claim. Generates a high-entropy session secret,
    /// derives the HMAC lookup key, inserts the record, and returns the
    /// `Set-Cookie` header value.
    ///
    /// The plaintext session secret leaves this function only inside
    /// [`IssuedSession::set_cookie`]; it is never stored or logged by codlet.
    ///
    /// # Errors
    /// Returns [`SessionError::Internal`] if the RNG, hasher, or store fails.
    pub async fn issue<R: RandomSource>(
        &self,
        success: &RedeemSuccess,
        session_id: SessionId,
        rng: &mut R,
    ) -> Result<IssuedSession, SessionError> {
        // Generate a high-entropy session secret (256 bits / 32 bytes).
        let mut raw = [0u8; 32];
        rng.fill_bytes(&mut raw)
            .map_err(|e| SessionError::Internal {
                cause: format!("rng: {e}"),
                public: crate::error::PublicSessionError::TemporarilyUnavailable,
            })?;

        // Hex-encode for cookie transport (64 ASCII chars, URL-safe).
        let secret_hex = hex_lower(&raw);
        let secret = SessionSecret::new(secret_hex.clone());

        let (lookup_key, key_version) = self
            .hasher
            .lookup_key(SecretDomain::Session, secret.expose())
            .map_err(SessionError::from_key)?;

        let now = self.clock.unix_now();
        let expires_at = expires_at_from_ttl(now, self.cookie_policy.max_age_duration());

        self.store
            .insert_session(SessionRecord {
                id: session_id.clone(),
                lookup_key,
                key_version,
                subject: success.subject.clone(),
                created_at: now,
                expires_at,
            })
            .await
            .map_err(SessionError::from_store)?;

        self.audit.record(CodeAuthEvent::SessionIssued {
            session_id: session_id.clone(),
            subject_id: success.subject.clone(),
        });

        let set_cookie = self.cookie_policy.build_set_cookie(secret.expose());
        Ok(IssuedSession {
            session_id,
            set_cookie,
        })
    }

    /// Validate a session from the bearer credential in a cookie.
    ///
    /// Derives the lookup key from `cookie_value`, queries the store for an
    /// active (unexpired, unrevoked) session, and returns the authentication
    /// outcome. Expired and revoked sessions both collapse to
    /// `Unauthenticated` (INV-8).
    ///
    /// # Errors
    /// Returns [`SessionError::Internal`] only on store/key failure.
    /// A missing or invalid session returns `Ok(Unauthenticated)`, not an error.
    pub async fn validate(
        &self,
        cookie_value: &str,
    ) -> Result<SessionValidationOutcome, SessionError> {
        let (lookup_key, _) = self
            .hasher
            .lookup_key(SecretDomain::Session, cookie_value)
            .map_err(SessionError::from_key)?;

        let now = self.clock.unix_now();
        let record = self
            .store
            .find_active_session(&[lookup_key], now)
            .await
            .map_err(SessionError::from_store)?;

        let outcome = classify_session(record.map(|r| (r.subject, r.id, r.expires_at)));

        if !outcome.is_authenticated() {
            self.audit.record(CodeAuthEvent::SessionValidateFailed);
        }

        Ok(outcome)
    }

    /// Revoke a session (logout or incident response).
    ///
    /// Returns the `Set-Cookie` header value that clears the session cookie
    /// from the client.
    ///
    /// # Errors
    /// Returns [`SessionError::Internal`] on store failure.
    pub async fn revoke(&self, session_id: &SessionId) -> Result<String, SessionError> {
        let now = self.clock.unix_now();
        self.store
            .revoke_session(session_id, now)
            .await
            .map_err(SessionError::from_store)?;

        self.audit.record(CodeAuthEvent::SessionRevoked {
            session_id: session_id.clone(),
        });

        Ok(self.cookie_policy.build_clear_cookie())
    }

    /// Borrow the cookie policy (e.g. to build the initial `Set-Cookie` name
    /// for extraction on the next request).
    #[must_use]
    pub fn cookie_policy(&self) -> &CookiePolicy {
        &self.cookie_policy
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
