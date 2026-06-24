//! Code authentication manager (RFC-013).
//!
//! [`CodeAuth`] composes the primitives from `code`, `hashing`, `rng`,
//! `store`, `audit`, and `clock` into the safe redemption flow described in
//! RFC-013 §10.3:
//!
//! 1. rate-limit check;
//! 2. input normalization + validation;
//! 3. code lookup (`find_redeemable`);
//! 4. atomic claim (`claim_code`);
//! 5. host callback (creates / resolves subject);
//! 6. audit event;
//! 7. return [`RedeemSuccess`].
//!
//! Steps 1–3 can fail without consuming the code.  Only step 4 is
//! irreversible.  Session issuance requires the [`RedeemSuccess`] proof, which
//! is only constructible when the claim returns `Won`.

use std::future::Future;

use crate::audit::{AuditSink, CodeAuthEvent};
use crate::clock::Clock;
use crate::code::{CodePolicy, validate_code_input};
use crate::error::PublicRedemptionError;
use crate::error::RedemptionFailReason;
use crate::hashing::{KeyProvider, SecretDomain, SecretHasher};
use crate::secret::{CodeId, SubjectId};
use crate::store::code::{
    ClaimRequest, CodeRecord, CodeStore, RedeemableCode, expires_at_from_ttl,
};
use crate::store::ratelimit::{RateLimitKey, RateLimitOutcome, RateLimitPolicy, RateLimitStore};

use super::error::{ClaimProof, RedeemError, RedeemSuccess};

/// Manages one-time code issuance, validation, and redemption (RFC-013 §3).
///
/// Generic over:
/// - `CS` — the [`CodeStore`] backend;
/// - `RL` — the [`RateLimitStore`] backend (use `()` to opt out);
/// - `K` — the [`KeyProvider`];
/// - `C` — the [`Clock`];
/// - `A` — the [`AuditSink`].
pub struct CodeAuth<CS, RL, K, C, A> {
    store: CS,
    rate_limit_store: RL,
    hasher: SecretHasher<K>,
    clock: C,
    audit: A,
    policy: CodePolicy,
    rate_limit_policy: Option<RateLimitPolicy>,
}

impl<CS, RL, K, C, A> CodeAuth<CS, RL, K, C, A>
where
    CS: CodeStore,
    RL: RateLimitStore,
    K: KeyProvider,
    C: Clock,
    A: AuditSink,
{
    /// Construct a `CodeAuth` with a rate-limit store and policy.
    #[must_use]
    pub fn new(
        store: CS,
        rate_limit_store: RL,
        hasher: SecretHasher<K>,
        clock: C,
        audit: A,
        policy: CodePolicy,
        rate_limit_policy: RateLimitPolicy,
    ) -> Self {
        Self {
            store,
            rate_limit_store,
            hasher,
            clock,
            audit,
            policy,
            rate_limit_policy: Some(rate_limit_policy),
        }
    }

    // ── Issue ────────────────────────────────────────────────────────────────

    /// Issue a new one-time code and insert it into the store.
    ///
    /// Returns the [`CodeId`] (for audit/admin) and the plaintext code (for
    /// delivery to the recipient). The plaintext must not be logged or stored.
    ///
    /// `rng` must be a fresh CSPRNG; `ttl` overrides the policy TTL if needed.
    /// `scope` and `grant` are host-owned and not interpreted by codlet.
    ///
    /// # Errors
    /// Returns [`RedeemError::Internal`] if the RNG or store fails.
    pub async fn issue_code<R: crate::rng::RandomSource>(
        &self,
        rng: &mut R,
        id: CodeId,
        purpose: Option<String>,
        scope: Option<String>,
        grant: Option<String>,
    ) -> Result<(CodeId, crate::secret::PlainCode), RedeemError> {
        let plain =
            crate::code::generate_code(&self.policy, rng).map_err(|e| RedeemError::Internal {
                cause: format!("rng: {e}"),
                public: PublicRedemptionError::TemporarilyUnavailable,
            })?;

        let normalized = plain.expose().to_string(); // already in canonical form
        let (lookup_key, key_version) = self
            .hasher
            .lookup_key(SecretDomain::Code, &normalized)
            .map_err(RedeemError::from_key)?;

        let now = self.clock.unix_now();
        let expires_at = expires_at_from_ttl(now, self.policy.ttl());

        let record = CodeRecord {
            id: id.clone(),
            lookup_key,
            key_version,
            purpose,
            scope,
            grant,
            created_at: now,
            expires_at,
        };
        self.store
            .insert_code(record)
            .await
            .map_err(RedeemError::from_store)?;

        self.audit.record(CodeAuthEvent::CodeIssued {
            code_id: id.clone(),
            purpose: None,
        });

        Ok((id, plain))
    }

    // ── Two-step redemption ──────────────────────────────────────────────────

    /// Step 1: validate and look up a submitted code without claiming it.
    ///
    /// Returns a [`RedeemableCode`] that the caller can inspect (e.g. to
    /// display a confirmation or collect additional user input) before
    /// committing the claim in [`Self::claim`].
    ///
    /// Rate limiting is applied here if configured.
    ///
    /// # Errors
    /// Returns [`RedeemError`] on validation failure, rate limit, or lookup miss.
    pub async fn find(
        &self,
        raw_input: &str,
        rate_key: Option<&RateLimitKey>,
    ) -> Result<RedeemableCode, RedeemError> {
        // Step 1: rate-limit check. Honour unavailable policy on store error.
        if let (Some(key), Some(rl_policy)) = (rate_key, &self.rate_limit_policy) {
            match self.rate_limit_store.check(key, rl_policy).await {
                Ok(RateLimitOutcome::Deny) => {
                    self.audit.record(CodeAuthEvent::RateLimitHit {
                        key_fingerprint: key.fingerprint().to_string(),
                        purpose: None,
                    });
                    return Err(RedeemError::RateLimited {
                        public: PublicRedemptionError::RateLimited,
                    });
                }
                Ok(RateLimitOutcome::Allow) => {}
                Err(_) => {
                    // Rate-limit store unavailable: apply configured policy.
                    match rl_policy.unavailable {
                        crate::store::ratelimit::RateLimitUnavailable::FailClosed => {
                            self.audit.record(CodeAuthEvent::RateLimitHit {
                                key_fingerprint: key.fingerprint().to_string(),
                                purpose: None,
                            });
                            return Err(RedeemError::RateLimited {
                                public: PublicRedemptionError::RateLimited,
                            });
                        }
                        crate::store::ratelimit::RateLimitUnavailable::FailOpen => {}
                    }
                }
            }
        }

        // Step 2: input normalization + validation.
        let normalized = match validate_code_input(raw_input, &self.policy) {
            Ok(n) => n,
            Err(_) => {
                self.audit.record(CodeAuthEvent::RedemptionFailed {
                    reason: RedemptionFailReason::InvalidFormat,
                });
                // Invalid-format guesses count toward the rate limit (RFC-B).
                if let (Some(key), Some(rl_policy)) = (rate_key, &self.rate_limit_policy) {
                    let _ = self.rate_limit_store.record_failure(key, rl_policy).await;
                }
                return Err(RedeemError::InvalidInput {
                    reason: RedemptionFailReason::InvalidFormat,
                    public: PublicRedemptionError::from_reason(
                        &RedemptionFailReason::InvalidFormat,
                    ),
                });
            }
        };

        // Step 3: derive one candidate per held key (RFC-A) and find the record.
        let candidates: Vec<_> = self
            .hasher
            .lookup_key_candidates(SecretDomain::Code, &normalized)
            .map_err(RedeemError::from_key)?
            .into_iter()
            .map(|(lk, _)| lk)
            .collect();

        let now = self.clock.unix_now();
        let record = self
            .store
            .find_redeemable(&candidates, now, None)
            .await
            .map_err(RedeemError::from_store)?
            .ok_or_else(|| {
                self.audit.record(CodeAuthEvent::RedemptionFailed {
                    reason: RedemptionFailReason::NotFound,
                });
                RedeemError::NotRedeemable {
                    reason: RedemptionFailReason::NotFound,
                    public: PublicRedemptionError::InvalidOrExpired,
                }
            });

        // Not-found guesses count toward the rate limit (RFC-B).
        if record.is_err() {
            if let (Some(key), Some(rl_policy)) = (rate_key, &self.rate_limit_policy) {
                let _ = self.rate_limit_store.record_failure(key, rl_policy).await;
            }
        }
        let record = record?;

        Ok(record)
    }

    /// Step 2: atomically claim a [`RedeemableCode`] found by [`Self::find`].
    ///
    /// Returns a [`RedeemSuccess`] proof only if `claim_code` returns `Won`.
    /// A `Lost` result means a concurrent caller already claimed the code.
    ///
    /// Rate-limit failures are recorded on a failed claim, and cleared on a
    /// successful one, when a `rate_key` is provided.
    ///
    /// # Errors
    /// Returns [`RedeemError::ClaimLost`] if the atomic claim was lost, or
    /// [`RedeemError::Internal`] on store failure.
    pub async fn claim(
        &self,
        record: &RedeemableCode,
        subject: SubjectId,
        rate_key: Option<&RateLimitKey>,
    ) -> Result<RedeemSuccess, RedeemError> {
        let now = self.clock.unix_now();
        let outcome = self
            .store
            .claim_code(&ClaimRequest {
                code_id: &record.id,
                subject: &subject,
                now,
                // Pass purpose/scope from the found record so adapters can
                // enforce cross-flow isolation in the UPDATE WHERE (RFC-C).
                purpose: record.purpose.as_deref(),
                scope: record.scope.as_deref(),
            })
            .await
            .map_err(RedeemError::from_store)?;

        match ClaimProof::new(outcome) {
            Some(proof) => {
                // Clear rate-limit counter on success.
                if let Some(key) = rate_key {
                    if self.rate_limit_policy.is_some() {
                        let _ = self.rate_limit_store.clear_failures(key).await;
                    }
                }
                self.audit.record(CodeAuthEvent::CodeRedeemed {
                    code_id: record.id.clone(),
                    subject_id: subject.clone(),
                });
                Ok(RedeemSuccess {
                    subject,
                    grant: record.grant.clone(),
                    _claim_proof: proof,
                })
            }
            None => {
                // Record failure in rate limiter for a lost claim too.
                if let (Some(key), Some(rl_policy)) = (rate_key, &self.rate_limit_policy) {
                    let _ = self.rate_limit_store.record_failure(key, rl_policy).await;
                }
                self.audit.record(CodeAuthEvent::RedemptionFailed {
                    reason: RedemptionFailReason::AlreadyUsed,
                });
                Err(RedeemError::ClaimLost {
                    public: PublicRedemptionError::InvalidOrExpired,
                })
            }
        }
    }

    // ── Single-call callback flow (RFC-013 §4) ───────────────────────────────

    /// Validate, look up, and claim a code in one call, invoking `on_won` as
    /// the host callback that creates or resolves the subject.
    ///
    /// Enforces RFC-013 §10.3 step order. `on_won` is called only after a
    /// confirmed won claim; its error aborts the flow without a session.
    ///
    /// # Errors
    /// Returns [`RedeemError`] on any failure. If `on_won` fails, returns
    /// [`RedeemError::Internal`] and the claim is already consumed (the host
    /// must decide on compensation if needed — RFC-013 §5).
    ///
    /// # Production warning
    ///
    /// **Experimental (RFC-D).** This method claims the code before the host
    /// callback returns the real subject, leaving `used_by_subject = "__pending__"`
    /// in the database until the callback completes. If the callback fails, the
    /// code is permanently consumed with no subject recorded, and the audit event
    /// and database state disagree on who claimed it.
    ///
    /// For production audit-sensitive deployments, use the explicit two-step
    /// flow: [`Self::find`] → host creates/resolves subject → [`Self::claim`].
    #[deprecated(
        note = "experimental: DB and audit state diverge if callback fails.                 Use find() + host subject creation + claim() for production."
    )]
    pub async fn redeem_with_callback<F, Fut, E>(
        &self,
        raw_input: &str,
        rate_key: Option<&RateLimitKey>,
        on_won: F,
    ) -> Result<RedeemSuccess, RedeemError>
    where
        F: FnOnce(&RedeemableCode) -> Fut,
        Fut: Future<Output = Result<SubjectId, E>>,
        E: std::fmt::Display,
    {
        let record = self.find(raw_input, rate_key).await?;
        let now = self.clock.unix_now();

        // Attempt claim before invoking host callback (fail-fast on race).
        // WARNING: redeem_with_callback() is experimental (RFC-D). The DB record
        // will store the real subject once the callback returns, but the interim
        // state is a won claim with no subject yet. Use find()+claim() for
        // production audit-sensitive deployments.
        let outcome = self
            .store
            .claim_code(&ClaimRequest {
                code_id: &record.id,
                subject: &SubjectId::new("__pending__".into()),
                now,
                purpose: record.purpose.as_deref(),
                scope: record.scope.as_deref(),
            })
            .await
            .map_err(RedeemError::from_store)?;

        let proof = ClaimProof::new(outcome).ok_or_else(|| {
            self.audit.record(CodeAuthEvent::RedemptionFailed {
                reason: RedemptionFailReason::AlreadyUsed,
            });
            RedeemError::ClaimLost {
                public: PublicRedemptionError::InvalidOrExpired,
            }
        })?;

        // Claim won — now invoke host callback.
        let subject = on_won(&record).await.map_err(|e| RedeemError::Internal {
            cause: format!("host callback failed: {e}"),
            public: PublicRedemptionError::TemporarilyUnavailable,
        })?;

        if let Some(key) = rate_key {
            if self.rate_limit_policy.is_some() {
                let _ = self.rate_limit_store.clear_failures(key).await;
            }
        }
        self.audit.record(CodeAuthEvent::CodeRedeemed {
            code_id: record.id.clone(),
            subject_id: subject.clone(),
        });

        Ok(RedeemSuccess {
            subject,
            grant: record.grant.clone(),
            _claim_proof: proof,
        })
    }

    /// Revoke a code by its record ID. Scoped to `scope` when provided.
    ///
    /// # Errors
    /// Returns [`RedeemError::Internal`] on store failure.
    pub async fn revoke_code(
        &self,
        code_id: &CodeId,
        scope: Option<&str>,
    ) -> Result<(), RedeemError> {
        let now = self.clock.unix_now();
        self.store
            .revoke_code(code_id, scope, now)
            .await
            .map_err(RedeemError::from_store)?;
        self.audit.record(CodeAuthEvent::CodeRevoked {
            code_id: code_id.clone(),
            scope: scope.map(str::to_string),
        });
        Ok(())
    }
}

/// Convenience impl: construct a [`CodeAuth`] with no rate-limit store.
///
/// Uses `NoRateLimit` as the `RL` type parameter so callers don't need to
/// spell out the full generic signature when rate limiting is handled elsewhere.
impl<CS, K, C, A> CodeAuth<CS, super::norate::NoRateLimit, K, C, A>
where
    CS: CodeStore,
    K: KeyProvider,
    C: Clock,
    A: AuditSink,
{
    /// Construct without a rate-limit store. Equivalent to passing
    /// `NoRateLimit` explicitly.
    #[must_use]
    pub fn without_rate_limit(
        store: CS,
        hasher: SecretHasher<K>,
        clock: C,
        audit: A,
        policy: CodePolicy,
    ) -> Self {
        Self {
            store,
            rate_limit_store: super::norate::NoRateLimit,
            hasher,
            clock,
            audit,
            policy,
            rate_limit_policy: None,
        }
    }
}
