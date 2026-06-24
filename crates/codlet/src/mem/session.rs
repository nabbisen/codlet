//! In-memory session store (RFC-011 §10.3). Non-production.

use std::sync::Mutex;

use crate::hashing::LookupKey;
use crate::secret::{SessionId, SubjectId};
use crate::store::error::StoreError;
use crate::store::session::{ActiveSessionRecord, SessionRecord, SessionStore};

#[derive(Debug, Clone)]
struct MemSessionRow {
    id: SessionId,
    lookup_key: LookupKey,
    subject: SubjectId,
    expires_at: u64,
    revoked_at: Option<u64>,
}

/// **Non-production** in-memory session store.
#[derive(Debug, Default)]
pub struct MemSessionStore {
    rows: Mutex<Vec<MemSessionRow>>,
}

impl MemSessionStore {
    /// Construct an empty in-memory session store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionStore for MemSessionStore {
    async fn find_active_session(
        &self,
        candidates: &[LookupKey],
        now: u64,
    ) -> Result<Option<ActiveSessionRecord>, StoreError> {
        let rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let found = rows
            .iter()
            .find(|r| {
                r.revoked_at.is_none()
                    && r.expires_at > now
                    && candidates.iter().any(|c| r.lookup_key.ct_eq(c))
            })
            .map(|r| ActiveSessionRecord {
                id: r.id.clone(),
                subject: r.subject.clone(),
                expires_at: r.expires_at,
            });
        Ok(found)
    }

    async fn insert_session(&self, record: SessionRecord) -> Result<(), StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        rows.push(MemSessionRow {
            id: record.id,
            lookup_key: record.lookup_key,
            subject: record.subject,
            expires_at: record.expires_at,
            revoked_at: None,
        });
        Ok(())
    }

    async fn revoke_session(&self, session_id: &SessionId, now: u64) -> Result<(), StoreError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        for row in rows.iter_mut() {
            if &row.id == session_id && row.revoked_at.is_none() {
                row.revoked_at = Some(now);
            }
        }
        Ok(())
    }
}
