//! Regression test for RFC-038: the migration runner must not parse SQL.
use super::*;

/// A hand-rolled split-on-`;` splitter cuts a semicolon inside a `--` comment
/// in half and submits the tail as a statement — this is exactly what broke
/// `run_postgres_migrations` in production (RFC-038 §2). This fixture carries
/// its own script rather than reading `migrations/0002_postgres.sql`, so it
/// does not depend on that file keeping its current wording (RFC-038 §5.1).
const SCRIPT_WITH_SEMICOLON_IN_COMMENT: &str = "\
-- note (see RFC-033); this clause must survive
CREATE TABLE IF NOT EXISTS regression_probe (id INTEGER PRIMARY KEY);
";

#[tokio::test]
async fn semicolon_inside_a_comment_does_not_break_the_script() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    apply_sqlite_script(&pool, SCRIPT_WITH_SEMICOLON_IN_COMMENT)
        .await
        .expect("a semicolon inside a `--` comment must not break the script");

    let created: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'regression_probe'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created, 1, "expected table was not created");
}

// ── RFC-044: additive `last_seen_at` column on a pre-existing database ──────

/// A pre-RFC-044 `codlet_sessions` schema (before this migration's column
/// addition), used to simulate a database created before this feature
/// existed. `CREATE TABLE IF NOT EXISTS` in `run_migrations` cannot add a
/// column to a table that already exists this way -- that is exactly the case
/// `ensure_session_last_seen_at_column` must handle.
const PRE_RFC_044_SESSIONS_TABLE: &str = "\
CREATE TABLE codlet_sessions (
    id          TEXT    NOT NULL PRIMARY KEY,
    lookup_key  TEXT    NOT NULL UNIQUE,
    key_version TEXT    NOT NULL,
    subject     TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    expires_at  INTEGER NOT NULL,
    revoked_at  INTEGER
);
";

#[tokio::test]
async fn last_seen_at_is_added_to_a_pre_existing_sessions_table() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Simulate an existing, pre-RFC-044 database: create the table in its old
    // shape and insert a row, exactly as a real deployment would already have.
    apply_sqlite_script(&pool, PRE_RFC_044_SESSIONS_TABLE)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO codlet_sessions (id, lookup_key, key_version, subject, created_at, expires_at)
         VALUES ('s1', 'lk1', 'v1', 'user-1', 1000, 2000)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Run the real migration runner -- the one a host actually calls -- not
    // just the column-check helper, so this proves the full startup path.
    run_migrations(&pool).await.unwrap();

    let columns: Vec<(i64, String)> =
        sqlx::query_as("SELECT cid, name FROM pragma_table_info('codlet_sessions')")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        columns.iter().any(|(_, name)| name == "last_seen_at"),
        "last_seen_at column was not added"
    );

    // Additive, no backfill (RFC-044 §5): the pre-existing row must validate
    // unchanged, with last_seen_at reading back as NULL, not some invented value.
    let last_seen_at: Option<i64> =
        sqlx::query_scalar("SELECT last_seen_at FROM codlet_sessions WHERE id = 's1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        last_seen_at, None,
        "pre-existing row must read back NULL, not a backfilled value"
    );

    // Idempotent: calling again (as every host startup does) must not error.
    run_migrations(&pool)
        .await
        .expect("second run must be a no-op, not a duplicate-column error");
}

#[tokio::test]
async fn last_seen_at_present_on_a_fresh_database() {
    // A brand-new database goes through CREATE TABLE with the column already
    // in the schema (`0001_initial.sql`); this confirms that path also ends
    // with the column present, not just the ALTER-TABLE path above.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    run_migrations(&pool).await.unwrap();

    let columns: Vec<(i64, String)> =
        sqlx::query_as("SELECT cid, name FROM pragma_table_info('codlet_sessions')")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        columns.iter().any(|(_, name)| name == "last_seen_at"),
        "last_seen_at column missing from a fresh database"
    );
}
