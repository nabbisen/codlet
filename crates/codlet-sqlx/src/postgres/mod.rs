//! PostgreSQL storage adapter for codlet (RFC-034).
//!
//! ## Atomicity
//!
//! Code claim and form-token consume use a conditional `UPDATE … WHERE … AND
//! <guard>` with `rows_affected()`. Under PostgreSQL's default `READ
//! COMMITTED` isolation level, the UPDATE acquires a row-level lock on the
//! matched row. Concurrent transactions targeting the same row serialise:
//! exactly one commits with `rows_affected() == 1`; the rest see `0`.
//!
//! This is equivalent to SQLite's serialised write mode and D1's per-row
//! locking. No `SERIALIZABLE` isolation or `FOR UPDATE` pre-lock is needed.
//!
//! ## Why no `RETURNING` clause
//!
//! RFC-034 §7 explicitly rejects `RETURNING`: the `CodeStore` trait is
//! designed around `classify_claim(rows_affected())`. Adding `RETURNING`
//! would require a different code path for no security gain — the conditional
//! WHERE clause already provides the single-winner guarantee.

pub mod code;
pub mod session;
pub mod token;

pub use code::PostgresCodeStore;
pub use session::PostgresSessionStore;
pub use token::PostgresFormTokenStore;

/// PostgreSQL storage adapter for codlet (RFC-034).
///
/// Wraps a [`sqlx::PgPool`]. Clone is cheap — the pool is reference-counted.
///
/// Implements [`codlet_core::store::code::CodeStore`],
/// [`codlet_core::store::session::SessionStore`],
/// [`codlet_core::store::token::FormTokenStore`], and
/// [`codlet_core::admin::CodeAdminStore`].
#[derive(Debug, Clone)]
pub struct PostgresStore {
    pub(crate) pool: sqlx::PgPool,
}

impl PostgresStore {
    /// Construct from an existing connection pool.
    ///
    /// Call [`run_postgres_migrations`] on the pool before issuing any
    /// store operations.
    #[must_use]
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool.
    #[must_use]
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

/// Run codlet's PostgreSQL migrations against `pool` (RFC-034 §9).
///
/// Applies `migrations/0002_postgres.sql` statement by statement. Uses
/// `IF NOT EXISTS` throughout — safe to call on every startup.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if any statement fails.
pub async fn run_postgres_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let migration_sql = include_str!("../../migrations/0002_postgres.sql");
    for stmt in migration_sql.split(';') {
        let code_lines: String = stmt
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = code_lines.trim();
        if trimmed.is_empty() {
            continue;
        }
        sqlx::query(trimmed).execute(pool).await?;
    }
    Ok(())
}
