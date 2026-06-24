//! Form-token storage trait (RFC-007).

use std::future::Future;

use crate::hashing::{KeyVersion, LookupKey};
use crate::state::TokenConsumeOutcome;

use super::error::StoreError;

/// The subject binding for a form token (RFC-007 §13.3).
///
/// Explicit variants prevent the "empty string for anonymous" anti-pattern
/// identified in RFC-007 §13.3. Bindings are persisted as part of the token
/// record and checked on consume.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenSubject {
    /// Token issued before authentication (e.g. a join form).
    Anonymous,
    /// Token issued for an authenticated subject.
    Authenticated(crate::secret::SubjectId),
    /// Token bound to a transient flow ID (e.g. a multi-step join ticket).
    ///
    /// ## Intended pattern for two-step join flows
    ///
    /// The host generates a random flow ID at the start of the flow and
    /// stores it in a short-lived bearer cookie (the "join ticket"):
    ///
    /// ```rust,ignore
    /// // Step 1 — POST /join: validate the invite code, issue a join ticket.
    /// let flow_id = CodeId::new(generate_random_hex(&mut rng));
    /// // Set `__join_ticket` cookie to flow_id.expose() (the plaintext).
    /// // The cookie is HttpOnly, Secure, SameSite=Strict, short TTL.
    ///
    /// // Also issue a CSRF form token bound to this flow:
    /// let token = form_token_mgr.issue(
    ///     &mut rng,
    ///     TokenSubject::Flow(flow_id.clone()),
    ///     "join_profile",          // purpose
    ///     None,                    // bound_resource (or Some(community_id))
    /// ).await?;
    /// // Embed token.expose() in the profile form as a hidden field.
    ///
    /// // Step 2 — POST /join/profile: consume the form token.
    /// // Read flow_id from the `__join_ticket` cookie.
    /// let flow_id = CodeId::new(join_ticket_cookie_value.into());
    /// let result = form_token_mgr.consume(
    ///     raw_form_token,
    ///     &TokenSubject::Flow(flow_id),
    ///     "join_profile",
    ///     None,
    /// ).await?;
    /// // Ok(None) → Proceed; Ok(Some(_)) → Replay; Err → Invalid/expired.
    /// ```
    ///
    /// `SecretDomain::FlowTicket` is used when the host wants to hash the
    /// join-ticket cookie value itself (making it a codlet-managed HMAC
    /// lookup rather than a raw random string). This is optional — the host
    /// may manage the ticket cookie independently and use `TokenSubject::Flow`
    /// only for the CSRF form token layer.
    Flow(crate::secret::CodeId),
}

impl TokenSubject {
    /// A stable string representation persisted in the store. This is not a
    /// security boundary on its own; the store's consume WHERE clause enforces
    /// the binding.
    #[must_use]
    pub fn as_binding_str(&self) -> String {
        match self {
            TokenSubject::Anonymous => "anon".to_string(),
            TokenSubject::Authenticated(s) => format!("auth:{}", s.as_str()),
            TokenSubject::Flow(f) => format!("flow:{}", f.as_str()),
        }
    }
}

/// A consumed token record with an optional replay reference.
#[derive(Debug, Clone)]
pub struct ConsumedTokenRecord {
    /// Whether the token has been consumed.
    pub consumed: bool,
    /// Optional result reference for idempotency replay (RFC-007 §4,
    /// `set_result`). `None` if the result was not yet stored.
    pub result_ref: Option<String>,
    /// Whether the binding checked in the consume WHERE clause matched.
    pub binding_ok: bool,
}

/// Parameters for inserting a new form token.
pub struct FormTokenRecord {
    /// Domain-separated HMAC of the token secret.
    pub lookup_key: LookupKey,
    /// Key version that produced `lookup_key`.
    pub key_version: KeyVersion,
    /// Subject binding (never an empty string).
    pub subject: TokenSubject,
    /// Purpose label, stable across the token's lifetime.
    pub purpose: String,
    /// Optional bound resource (HMAC of a domain object, not plaintext).
    pub bound_resource: Option<String>,
    /// Issuance time as Unix seconds (UTC).
    pub issued_at: u64,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
}

/// Form-token storage (RFC-007).
///
/// The consume operation must be atomic: a conditional UPDATE sets `consumed_at`
/// only when the token is unconsumed, unexpired, and bindings match. The
/// affected-row count drives [`TokenConsumeOutcome`] via
/// [`crate::state::classify_token_consume`].
pub trait FormTokenStore {
    /// Insert a new form token record.
    fn insert_form_token(
        &self,
        record: FormTokenRecord,
    ) -> impl Future<Output = Result<(), StoreError>>;

    /// Attempt to atomically consume a form token.
    ///
    /// The adapter must:
    /// 1. Run the conditional UPDATE (sets `consumed_at`).
    /// 2. If `changed == 0`, run a follow-up SELECT to classify why.
    /// 3. Call [`crate::state::classify_token_consume`] with the results.
    /// 4. Return the outcome plus any stored `result_ref` for replays.
    ///
    /// Try each candidate lookup key in order; consume on the first match.
    /// Accepting multiple candidates supports rotation grace periods (RFC-A).
    fn consume_form_token(
        &self,
        candidates: &[LookupKey],
        subject: &TokenSubject,
        purpose: &str,
        bound_resource: Option<&str>,
        now: u64,
    ) -> impl Future<Output = Result<(TokenConsumeOutcome, Option<String>), StoreError>>;

    /// Store a result reference on a consumed token for idempotency replay.
    fn set_token_result(
        &self,
        candidates: &[LookupKey],
        result_ref: &str,
    ) -> impl Future<Output = Result<(), StoreError>>;
}
