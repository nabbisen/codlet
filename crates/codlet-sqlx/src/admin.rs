//! [`CodeAdminStore`] implementation for [`SqliteStore`].
//!
//! Provides metadata listing and single-record lookup for admin tooling.
//! Never returns plaintext codes or HMAC lookup keys (RFC-030).

use codlet::admin::{CodeAdminStore, CodeListFilter, CodeMeta};
use codlet::hashing::KeyVersion;
use codlet::secret::{CodeId, SubjectId};
use codlet::store::error::StoreError;

use crate::SqliteStore;

/// Full row type returned by the admin SELECT.
/// (id, key_version, purpose, scope, grant_payload,
///  created_at, expires_at, used_at, used_by_subject, revoked_at)
type AdminRow = (
    String,         // id
    String,         // key_version
    Option<String>, // purpose
    Option<String>, // scope
    Option<String>, // grant_payload
    i64,            // created_at
    i64,            // expires_at
    Option<i64>,    // used_at
    Option<String>, // used_by_subject
    Option<i64>,    // revoked_at
);

fn row_to_meta(row: AdminRow) -> CodeMeta {
    let (id, kv, purpose, scope, grant, created_at, expires_at, used_at, used_by, revoked_at) = row;
    CodeMeta {
        id: CodeId::new(id),
        key_version: KeyVersion::new(kv),
        purpose,
        scope,
        grant,
        created_at: Some(created_at as u64),
        expires_at: expires_at as u64,
        used_at: used_at.map(|t| t as u64),
        used_by: used_by.map(SubjectId::new),
        revoked_at: revoked_at.map(|t| t as u64),
    }
}

impl CodeAdminStore for SqliteStore {
    async fn list_codes(
        &self,
        filter: &CodeListFilter,
        now: u64,
    ) -> Result<Vec<CodeMeta>, StoreError> {
        let now_i = now as i64;

        let rows: Vec<AdminRow> = match (&filter.scope, filter.active_only, filter.limit) {
            (Some(scope), true, limit) => {
                let mut rows: Vec<AdminRow> = sqlx::query_as(
                    "SELECT id, key_version, purpose, scope, grant_payload,
                            created_at, expires_at, used_at, used_by_subject, revoked_at
                     FROM codlet_codes
                     WHERE scope = ?
                       AND used_at    IS NULL
                       AND revoked_at IS NULL
                       AND expires_at  > ?
                     ORDER BY expires_at DESC",
                )
                .bind(scope.as_str())
                .bind(now_i)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                rows
            }
            (Some(scope), false, limit) => {
                let mut rows: Vec<AdminRow> = sqlx::query_as(
                    "SELECT id, key_version, purpose, scope, grant_payload,
                            created_at, expires_at, used_at, used_by_subject, revoked_at
                     FROM codlet_codes
                     WHERE scope = ?
                     ORDER BY expires_at DESC",
                )
                .bind(scope.as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                rows
            }
            (None, true, limit) => {
                let mut rows: Vec<AdminRow> = sqlx::query_as(
                    "SELECT id, key_version, purpose, scope, grant_payload,
                            created_at, expires_at, used_at, used_by_subject, revoked_at
                     FROM codlet_codes
                     WHERE used_at    IS NULL
                       AND revoked_at IS NULL
                       AND expires_at  > ?
                     ORDER BY expires_at DESC",
                )
                .bind(now_i)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                rows
            }
            (None, false, limit) => {
                let mut rows: Vec<AdminRow> = sqlx::query_as(
                    "SELECT id, key_version, purpose, scope, grant_payload,
                            created_at, expires_at, used_at, used_by_subject, revoked_at
                     FROM codlet_codes
                     ORDER BY expires_at DESC",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                rows
            }
        };

        Ok(rows.into_iter().map(row_to_meta).collect())
    }

    async fn get_code_meta(&self, code_id: &CodeId) -> Result<Option<CodeMeta>, StoreError> {
        let row: Option<AdminRow> = sqlx::query_as(
            "SELECT id, key_version, purpose, scope, grant_payload,
                    created_at, expires_at, used_at, used_by_subject, revoked_at
             FROM codlet_codes
             WHERE id = ?
             LIMIT 1",
        )
        .bind(code_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?;

        Ok(row.map(row_to_meta))
    }
}
