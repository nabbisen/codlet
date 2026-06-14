//! D1 migration runner (RFC-033 §7).
//!
//! Applies `migrations/0001_initial.sql` to a D1 database using
//! [`D1Database::exec`], which is intended for maintenance/migration tasks.
//! The SQL uses `IF NOT EXISTS` throughout, so `run_d1_migrations` is safe
//! to call on every Worker startup.

/// Apply codlet's schema migrations to a D1 database.
///
/// Uses `D1Database::exec` (batch DDL execution) rather than `prepare` +
/// `run` because `exec` is designed for multi-statement migration scripts.
/// Safe to call on every deploy; all statements use `IF NOT EXISTS`.
///
/// # Errors
///
/// Returns `worker::Error` if D1 reports an execution failure.
pub async fn run_d1_migrations(db: &worker::d1::D1Database) -> worker::Result<()> {
    let sql = include_str!("../migrations/0001_initial.sql");
    db.exec(sql).await?;
    Ok(())
}
