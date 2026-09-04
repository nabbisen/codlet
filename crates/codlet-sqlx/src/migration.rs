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
    apply_sqlite_script(pool, migration_sql).await?;

    // RFC-044: `last_seen_at` was added to `codlet_sessions` after this
    // table's initial release. `CREATE TABLE IF NOT EXISTS` is a no-op on a
    // table that already exists, so a database created before RFC-044 needs
    // this column added explicitly. SQLite has no `ADD COLUMN IF NOT EXISTS`
    // (verified: it is a parse error), so this checks column presence first
    // via `PRAGMA table_info`, staying additive with no backfill either way
    // (RFC-044 §5).
    ensure_session_last_seen_at_column(pool).await
}

/// Add `codlet_sessions.last_seen_at` if the table exists without it.
/// Idempotent: a no-op if the column is already present.
async fn ensure_session_last_seen_at_column(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let columns: Vec<(i64, String)> =
        sqlx::query_as("SELECT cid, name FROM pragma_table_info('codlet_sessions')")
            .fetch_all(pool)
            .await?;
    let has_column = columns.iter().any(|(_, name)| name == "last_seen_at");
    if !has_column {
        sqlx::query("ALTER TABLE codlet_sessions ADD COLUMN last_seen_at INTEGER")
            .execute(pool)
            .await?;
    }
    Ok(())
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
