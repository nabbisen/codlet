//! Session storage trait (RFC-006).

use std::future::Future;

use crate::hashing::{KeyVersion, LookupKey};
use crate::secret::{SessionId, SubjectId};

use super::error::StoreError;

/// A session record returned by `find_active_session`.
///
/// Despite the name, this may **not** be active: `find_active_session`
/// returns any record matching the lookup key regardless of state (RFC-047
/// step 2), and `revoked_at`/`expires_at` are what
/// [`crate::state::classify_session`] uses to decide. There is no second
/// enforcement point downstream of this call — unlike codes' `claim_code`
/// conditional UPDATE (RFC-047 §3), this classifier's decision *is* the
/// guard.
#[derive(Debug, Clone)]
pub struct ActiveSessionRecord {
    /// Opaque session record identifier (not a bearer credential).
    pub id: SessionId,
    /// The subject this session authenticates.
    pub subject: SubjectId,
    /// Creation time as Unix seconds (UTC). Used as the idle-timeout
    /// fallback when `last_seen_at` is `NULL` (RFC-044 §5).
    pub created_at: u64,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
    /// Last-touched time as Unix seconds (UTC), or `None` if the session has
    /// never been touched. `None` must be treated as `created_at` by the
    /// caller (RFC-044 §5) — the store does not perform that substitution,
    /// so a `NULL` column reads back honestly as "never recorded".
    pub last_seen_at: Option<u64>,
    /// When this session was revoked, if it has been (RFC-047 step 2).
    pub revoked_at: Option<u64>,
}

/// Parameters for inserting a new session.
pub struct SessionRecord {
    /// Store-assigned identifier.
    pub id: SessionId,
    /// Domain-separated HMAC of the session secret.
    pub lookup_key: LookupKey,
    /// Key version that produced `lookup_key`.
    pub key_version: KeyVersion,
    /// The authenticated subject.
    pub subject: SubjectId,
    /// Creation time as Unix seconds (UTC).
    pub created_at: u64,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
}

/// Session storage (RFC-006).
///
/// Sessions are stored by their HMAC lookup key, never by the plaintext secret.
/// The plaintext lives only in the cookie.
pub trait SessionStore {
    /// Look up a session record by HMAC lookup key candidates.
    ///
    /// Returns the first record matching any candidate key **regardless of
    /// its expiry or revocation state** (RFC-047 step 2) — pass the result to
    /// [`crate::state::classify_session`] to decide whether it is actually
    /// authenticated. Returns `Ok(None)` only if no record matches the lookup
    /// criteria at all. `now` is accepted for symmetry with other store
    /// methods and possible future use; this method itself no longer
    /// compares against it.
    fn find_active_session(
        &self,
        candidates: &[LookupKey],
        now: u64,
    ) -> impl Future<Output = Result<Option<ActiveSessionRecord>, StoreError>>;

    /// Insert a new session record.
    fn insert_session(&self, record: SessionRecord)
    -> impl Future<Output = Result<(), StoreError>>;

    /// Revoke a session by its record ID (logout / incident response).
    /// Revocation is monotonic: a revoked session cannot be unrevoked.
    fn revoke_session(
        &self,
        session_id: &SessionId,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>>;

    /// Record that a session was used at `now` (RFC-044 idle timeout).
    ///
    /// Deliberately **not** part of [`find_active_session`](Self::find_active_session):
    /// keeping the read and the write separate means the read path can never be
    /// made conditional on a write succeeding, and a `touch_session` failure
    /// must never invalidate an otherwise-valid session (RFC-044 §4.5) — the
    /// caller decides that, not this method.
    ///
    /// Implementations should perform this as a single unconditional write
    /// (e.g. `UPDATE ... WHERE id = ?`, no existence check required); the
    /// caller is responsible for throttling how often this is called.
    fn touch_session(
        &self,
        session_id: &SessionId,
        now: u64,
    ) -> impl Future<Output = Result<(), StoreError>>;
}
