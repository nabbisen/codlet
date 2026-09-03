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
