//! SQLite and PostgreSQL storage adapters for codlet (RFC-011, RFC-034).
//!
//! ## Backends
//!
//! | Feature | Type | Connection | Use case |
//! |---------|------|-----------|----------|
//! | `sqlite` (default) | `SqliteStore` | `"sqlite::memory:"`, `"sqlite:path/to/db"` | Local dev, single-server |
//! | `postgres` | `PostgresStore` | `postgres://…` | Multi-instance production |
//!
//! Enable exactly one (or both) at build time:
//!
//! ```toml
//! # SQLite only (default):
//! codlet-sqlx = { version = "…" }
//!
//! # PostgreSQL only — no SQLite code compiled:
//! codlet-sqlx = { version = "…", default-features = false, features = ["postgres"] }
//!
//! # Both:
//! codlet-sqlx = { version = "…", features = ["sqlite", "postgres"] }
//! ```
//!
//! ## Atomicity guarantee
//!
//! Every one-time transition (code claim, form-token consume) uses a single
//! `UPDATE … WHERE … AND <guard>` followed by `rows_affected()`. For
//! PostgreSQL, `READ COMMITTED` + row-level locking means concurrent updates
//! serialise at the row — exactly one wins. No `SERIALIZABLE` or `RETURNING`
//! needed (RFC-034 §7).
//!
//! ## Conformance
//!
//! All stores pass the `codlet-conformance` suite including the concurrent
//! claim race test (RFC-022, RFC-023).

#![forbid(unsafe_code)]

// SQLite modules: compiled only when the `sqlite` feature is active.
#[cfg(feature = "sqlite")]
pub mod admin;
#[cfg(feature = "sqlite")]
pub mod code;
#[cfg(feature = "sqlite")]
pub mod migration;
#[cfg(feature = "sqlite")]
pub mod session;
#[cfg(feature = "sqlite")]
pub mod token;

// PostgreSQL modules: compiled only when the `postgres` feature is active.
#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub use migration::run_migrations;

#[cfg(feature = "postgres")]
pub use postgres::{PostgresStore, run_postgres_migrations};

/// A handle wrapping a [`sqlx::SqlitePool`] that implements all codlet
/// store traits (RFC-011).
///
/// Clone is cheap (the pool is reference-counted internally).
///
/// Requires the `sqlite` Cargo feature (enabled by default).
#[cfg(feature = "sqlite")]
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pub(crate) pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteStore {
    /// Construct from an existing pool.
    ///
    /// Call [`run_migrations`] on the pool before any store operations.
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
