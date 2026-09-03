//! Migration runner for codlet SQLite tables (RFC-011 §10.4).
//!
//! The SQL is embedded at compile time via `include_str!`. Host applications
//! own the migration *application order* — this function is idempotent and
//! safe to call on startup, but the host decides when and how to run it
//! relative to its own migrations (RFC-011 §10.4).

use sqlx::SqlitePool;

/// Run codlet's embedded SQLite migrations against `pool`.
///
/// Uses `IF NOT EXISTS` semantics; safe to call on every startup.
///
/// # Errors
/// Returns a [`sqlx::Error`] if the SQL execution fails.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // WAL mode gives better concurrent read/write performance and is
    // recommended for codlet's workload.
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;
    // Enforce foreign key constraints if the host schema uses them.
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    let migration_sql = include_str!("../migrations/0001_initial.sql");
    apply_sqlite_script(pool, migration_sql).await
}

/// Execute a multi-statement SQL script against `pool` as a single call.
///
/// codlet does not parse SQL (RFC-038 §3): a hand-rolled split-on-`;` splitter
/// cuts a semicolon inside a `--` comment in half and submits the tail as a
/// statement. The driver already parses this correctly.
///
/// `pub(crate)` — this is a testability seam for the regression test in
/// `migration/tests.rs`, not public API.
pub(crate) async fn apply_sqlite_script(pool: &SqlitePool, sql: &str) -> Result<(), sqlx::Error> {
    // Safety: SQL comes from our own static migration files, not user input.
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
