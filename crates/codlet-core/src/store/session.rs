//! Session storage trait (RFC-006).

use std::future::Future;

use crate::hashing::{KeyVersion, LookupKey};
use crate::secret::{SessionId, SubjectId};

use super::error::StoreError;

/// An active session record returned by validation.
#[derive(Debug, Clone)]
pub struct ActiveSessionRecord {
    /// Opaque session record identifier (not a bearer credential).
    pub id: SessionId,
    /// The subject this session authenticates.
    pub subject: SubjectId,
    /// Expiry as Unix seconds (UTC).
    pub expires_at: u64,
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
    /// Look up an active session by HMAC lookup key candidates.
    ///
    /// Returns the first record matching any candidate that is not expired and
    /// not revoked at `now`. Returns `Ok(None)` if no such session exists.
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
}
