//! D1 migration runner (RFC-033 §7, RFC-057).
//!
//! Applies `migrations/0001_initial.sql` to a D1 database, then brings an older
//! `codlet_sessions` table up to date. It does **not** hand the file to
//! [`D1Database::exec`]: D1's `exec` treats each *line* as a statement, so it
//! rejects the file's first line (a comment) and cuts every multi-line
//! `CREATE TABLE` in half. Instead the file is split into statements (comments
//! stripped as it is scanned, by the one statement splitter in this crate) and the statements
//! are submitted together with [`D1Database::batch`].
//!
//! [`D1Database::exec`]: worker::d1::D1Database::exec
//! [`D1Database::batch`]: worker::d1::D1Database::batch

use serde::Deserialize;

use crate::sql_statements::split_statements;

#[derive(Deserialize)]
struct ColumnName {
    name: String,
}

/// Apply codlet's schema migrations to a D1 database.
///
/// 1. Every statement of the shipped migration (`CREATE … IF NOT EXISTS`) is
///    applied in one `D1Database::batch`, which D1 runs atomically.
/// 2. `codlet_sessions.last_seen_at` was added after the table's initial
///    release (RFC-044). `CREATE TABLE IF NOT EXISTS` does nothing to a table
///    that already exists, so a database created before then gets the column
///    here: `pragma_table_info` is checked first, and `ALTER TABLE … ADD COLUMN`
///    runs only when the column is absent — SQLite has no
///    `ADD COLUMN IF NOT EXISTS`, and a bare `ALTER` would fail with
///    "duplicate column name" on an already-migrated database. No backfill: an
///    existing row keeps `NULL`, meaning "never touched".
///
/// Idempotent: calling it again on a migrated database changes nothing, so it
/// is safe to call on every Worker startup. If two Worker instances race to add
/// the column, the loser's `ALTER` fails; the column is re-checked and, being
/// present, that is treated as success.
///
/// # Errors
///
/// Returns `worker::Error` if D1 reports a failure applying the migration or
/// reading the table's columns.
pub async fn run_d1_migrations(db: &worker::d1::D1Database) -> worker::Result<()> {
    let statements: Vec<_> = split_statements(include_str!("../migrations/0001_initial.sql"))
        .into_iter()
        .map(|sql| db.prepare(sql))
        .collect();
    db.batch(statements).await?;

    if has_session_last_seen_at(db).await? {
        return Ok(());
    }
    let added = db
        .prepare("ALTER TABLE codlet_sessions ADD COLUMN last_seen_at INTEGER")
        .run()
        .await;
    match added {
        Ok(_) => Ok(()),
        // Lost a race with another instance adding the same column.
        Err(_) if has_session_last_seen_at(db).await? => Ok(()),
        Err(e) => Err(e),
    }
}

/// Whether `codlet_sessions` already has its `last_seen_at` column.
async fn has_session_last_seen_at(db: &worker::d1::D1Database) -> worker::Result<bool> {
    let columns: Vec<ColumnName> = db
        .prepare("SELECT name FROM pragma_table_info('codlet_sessions')")
        .all()
        .await?
        .results()?;
    Ok(columns.iter().any(|c| c.name == "last_seen_at"))
}
