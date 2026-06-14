//! SQLite and PostgreSQL storage adapters for codlet (RFC-011, RFC-034).
//!
//! ## Backends
//!
//! | Feature | Type | Connection | Use case |
//! |---------|------|-----------|----------|
//! | `sqlite` (default) | `SqliteStore` | `"sqlite::memory:"`, `"sqlite:path/to/db"` | Local dev, single-server |
//! | `postgres` | `PostgresStore` | `postgres://…` | Multi-instance production |
//!
//! Both stores implement `CodeStore + SessionStore + FormTokenStore +
//! CodeAdminStore` and pass the full `codlet-conformance` suite.
//!
//! ## Atomicity guarantee
//!
//! Every one-time transition (code claim, form-token consume) uses a single
//! `UPDATE … WHERE … AND <guard>` checked via `rows_affected()`. For
//! PostgreSQL, `READ COMMITTED` isolation + row-level locking means concurrent
//! updates serialise at the row — exactly one wins. No `SERIALIZABLE`
//! isolation or `RETURNING` clause is needed or used (RFC-034 §7).
//!
//! ## Conformance
//!
//! All stores pass the `codlet-conformance` suite including the concurrent
//! claim race test (RFC-022, RFC-023).

#![forbid(unsafe_code)]

pub mod admin;
pub mod code;
pub mod migration;
pub mod session;
pub mod token;

#[cfg(feature = "postgres")]
pub mod postgres;

pub use migration::run_migrations;

#[cfg(feature = "postgres")]
pub use postgres::{PostgresStore, run_postgres_migrations};

/// A handle wrapping a [`sqlx::SqlitePool`] that implements all codlet
/// store traits (RFC-011).
///
/// Clone is cheap (the pool is reference-counted internally).
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pub(crate) pool: sqlx::SqlitePool,
}

impl SqliteStore {
    /// Construct from an existing pool.
    ///
    /// Call [`run_migrations`] on the pool before issuing any store
    /// operations.
    #[must_use]
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool.
    #[must_use]
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}
