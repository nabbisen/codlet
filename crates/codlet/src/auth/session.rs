//! Session manager (RFC-013 §3, RFC-044, RFC-046).
//!
//! [`SessionManager`] composes [`SessionStore`], [`SecretHasher`], [`Clock`],
//! [`CookiePolicy`], and [`AuditSink`] into the three session operations:
//! issue (after a won claim), validate (on every authenticated request), and
//! revoke (on logout or incident response).

use std::time::Duration;

use crate::audit::{AuditSink, CodeAuthEvent};
use crate::clock::Clock;
use crate::cookie::CookiePolicy;
use crate::hashing::{KeyProvider, SecretDomain, SecretHasher};
use crate::rng::RandomSource;
use crate::secret::{SessionId, SessionSecret};
use crate::state::{SessionFailure, SessionValidationOutcome, classify_session};
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
    /// Idle timeout (RFC-044). `None` — the default — means no idle-timeout
    /// checking, no `last_seen_at` write path, and no behavioural difference
    /// from before this feature existed (RFC-044 §4.1).
    idle_timeout: Option<Duration>,
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
            idle_timeout: None,
        }
    }

    /// Enable idle-timeout expiry (RFC-044): a session becomes invalid after
    /// `idle_timeout` without use, independently of its absolute lifetime.
    ///
    /// Opt-in only. Enabling this adds a throttled write to the validation
    /// path (`touch_session`, at most once per `max(idle_timeout / 20, 30s)`
    /// of continuous activity — RFC-044 §4.2), which is why it is not the
    /// default.
    #[must_use]
    pub fn with_idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = Some(idle_timeout);
        self
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
    /// `cookie_value` is `None` when the host found no session cookie on the
    /// request — pass that through rather than pre-filtering, so codlet can
    /// distinguish "no cookie" from "cookie present but invalid"
    /// ([`SessionFailure`], RFC-046). Derives the lookup key when a value is
    /// present, queries the store for an active session, and returns the
    /// authentication outcome. The end-user-visible response is identical for
    /// every failure reason (INV-8, RFC-006 §13.5); `reason` is for the host
    /// only.
    ///
    /// If an idle timeout is configured ([`Self::with_idle_timeout`]) and the
    /// session is still active, this may perform a throttled `touch_session`
    /// write (RFC-044 §4.2). A `touch_session` failure does not affect the
    /// returned outcome — the request stays authenticated and an audit event
    /// is recorded instead (RFC-044 §4.5).
    ///
    /// # Errors
    /// Returns [`SessionError::Internal`] only on store/key failure. A
    /// missing, malformed, or expired session returns `Ok(Unauthenticated)`,
    /// not an error.
    pub async fn validate(
        &self,
        cookie_value: Option<&str>,
    ) -> Result<SessionValidationOutcome, SessionError> {
        let outcome = match cookie_value {
            None => SessionValidationOutcome::Unauthenticated {
                reason: SessionFailure::NoCookie,
            },
            Some(cookie_value) if !is_well_formed_session_secret(cookie_value) => {
                SessionValidationOutcome::Unauthenticated {
                    reason: SessionFailure::Malformed,
                }
            }
            Some(cookie_value) => {
                // Derive one candidate per held key so records written under
                // previous keys remain reachable during the rotation grace
                // period (RFC-A).
                let candidates: Vec<_> = self
                    .hasher
                    .lookup_key_candidates(SecretDomain::Session, cookie_value)
                    .map_err(SessionError::from_key)?
                    .into_iter()
                    .map(|(lk, _)| lk)
                    .collect();

                let now = self.clock.unix_now();
                let record = self
                    .store
                    .find_active_session(&candidates, now)
                    .await
                    .map_err(SessionError::from_store)?;

                // Capture what the throttled touch needs before `record` is
                // consumed by `classify_session`.
                let touch_target = record
                    .as_ref()
                    .map(|r| (r.id.clone(), r.last_seen_at.unwrap_or(r.created_at)));

                let outcome = classify_session(record, self.idle_timeout, now);

                if outcome.is_authenticated() {
                    if let (Some(idle_timeout), Some((session_id, last_seen))) =
                        (self.idle_timeout, touch_target)
                    {
                        let granularity = touch_granularity(idle_timeout);
                        if now.saturating_sub(last_seen) >= granularity.as_secs()
                            && self.store.touch_session(&session_id, now).await.is_err()
                        {
                            // A bookkeeping-write failure must never
                            // invalidate an otherwise-valid session
                            // (RFC-044 §4.5) — `outcome` is untouched.
                            self.audit
                                .record(CodeAuthEvent::SessionTouchFailed { session_id });
                        }
                    }
                }

                outcome
            }
        };

        // `NoCookie` is the common case for an anonymous request and must not
        // become log noise (this event's original purpose: an opt-in signal
        // for an actual failed validation attempt, not every page view).
        let is_no_cookie = matches!(
            outcome,
            SessionValidationOutcome::Unauthenticated {
                reason: SessionFailure::NoCookie
            }
        );
        if !outcome.is_authenticated() && !is_no_cookie {
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

/// Session secrets are 32 random bytes, hex-encoded by [`hex_lower`] at issue
/// time (RFC-006 §4): exactly 64 lowercase ASCII hex digits. Anything else —
/// truncated, uppercase, non-hex — did not come from `issue` and cannot match
/// a stored lookup key; rejecting it here (RFC-046's [`SessionFailure::Malformed`])
/// avoids spending a key derivation and a store round trip on it.
fn is_well_formed_session_secret(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The touch throttle granularity (RFC-044 §4.2): a fraction of the idle
/// timeout, defaulting to one twentieth, floored at 30 seconds. A session is
/// touched at most once per granularity of continuous activity, not once per
/// request.
fn touch_granularity(idle_timeout: Duration) -> Duration {
    (idle_timeout / 20).max(Duration::from_secs(30))
}
