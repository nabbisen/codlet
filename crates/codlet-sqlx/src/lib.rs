//! SQLite storage adapters for codlet (RFC-011).
//!
//! Each adapter is a thin wrapper around a [`sqlx::SqlitePool`] that
//! implements the corresponding `codlet-core` store trait using SQLite's
//! conditional `UPDATE` semantics for atomic single-winner operations
//! (RFC-022).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codlet_sqlx::{SqliteStore, run_migrations};
//!
//! let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
//! run_migrations(&pool).await?;
//! let store = SqliteStore::new(pool);
//! // store implements CodeStore + SessionStore + FormTokenStore
//! ```
//!
//! ## Atomicity guarantee
//!
//! Every one-time transition (code claim, form-token consume) uses a single
//! `UPDATE … WHERE … AND <guard condition>` followed by an affected-row count
//! check. SQLite's serialised write mode ensures these are atomic under
//! concurrent access from multiple threads within the same process. For
//! multi-process deployments (rare for codlet's target use case), WAL mode
//! and appropriate busy-timeout settings are recommended.
//!
//! ## Conformance
//!
//! All three stores pass the full `codlet-conformance` suite, including the
//! concurrent-claim race test (RFC-022, RFC-023).

#![forbid(unsafe_code)]

pub mod code;
pub mod migration;
pub mod session;
pub mod token;

pub use migration::run_migrations;

/// A handle wrapping a [`sqlx::SqlitePool`] that implements all three
/// codlet store traits.
///
/// Clone is cheap (the pool is reference-counted internally).
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: sqlx::SqlitePool,
}

impl SqliteStore {
    /// Construct from an existing pool. The caller must have already run
    /// [`run_migrations`] on the pool before issuing any store operations.
    #[must_use]
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool (e.g. to run custom queries alongside codlet).
    #[must_use]
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}
