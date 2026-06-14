//! SQLite storage adapters for codlet (RFC-011).
//!
//! `SqliteStore` wraps a [`sqlx::SqlitePool`] and implements all three
//! core store traits plus the admin extension:
//!
//! - [`codlet_core::store::code::CodeStore`] — code issue, lookup, claim, revoke
//! - [`codlet_core::store::session::SessionStore`] — session issue, validate, revoke
//! - [`codlet_core::store::token::FormTokenStore`] — form-token issue, consume, replay
//! - [`codlet_core::admin::CodeAdminStore`] — metadata listing and lookup (RFC-030)
//!
//! ## Backend options
//!
//! SQLx supports three SQLite connection strings:
//!
//! ```text
//! "sqlite::memory:"          — ephemeral, in-process only (tests / local dev)
//! "sqlite:path/to/codlet.db" — persistent file on disk (single-server production)
//! "sqlite::memory:?cache=shared&uri=true" — named shared memory (advanced)
//! ```
//!
//! For production use a file-backed database and set WAL mode (applied
//! automatically by [`run_migrations`]).
//!
//! ## Atomicity guarantee
//!
//! Every one-time transition (code claim, form-token consume) uses a single
//! `UPDATE … WHERE … AND <guard condition>` followed by an affected-row count
//! check. SQLite's serialised write mode ensures these are atomic under
//! concurrent access from multiple threads within the same process. For
//! multi-process deployments, WAL mode and an appropriate busy-timeout are
//! recommended.
//!
//! ## Conformance
//!
//! All stores pass the full `codlet-conformance` suite, including the
//! concurrent-claim race test (RFC-022, RFC-023).

#![forbid(unsafe_code)]

pub mod admin;
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
