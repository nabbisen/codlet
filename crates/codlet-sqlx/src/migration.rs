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

    // Split on statement boundaries and execute each statement separately,
    // since SQLx's `execute` does not support multiple statements in one call.
    for stmt in migration_sql.split(';') {
        // Strip leading comment lines and whitespace from each segment, then
        // execute only non-empty segments. A segment that is entirely comments
        // (e.g. the preamble before the first real statement) is silently
        // skipped; a segment that starts with comments but contains SQL is
        // executed with the comments stripped.
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
