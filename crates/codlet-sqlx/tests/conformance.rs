//! Full conformance suite for the SQLite adapter (RFC-023, RFC-022).
//!
//! Each test function wires up a fresh SQLite in-memory pool, runs migrations,
//! and passes the pool to the `codlet-conformance` parameterised runner.  The
//! concurrent-claim race test verifies the single-winner guarantee under real
//! async task concurrency (RFC-022).

use codlet_sqlx::{SqliteStore, run_migrations};

async fn fresh_store() -> SqliteStore {
    // Each test gets its own in-memory database. We use a single-connection
    // pool so all queries share the same SQLite in-memory database — multiple
    // connections would each get an independent (empty) database.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    SqliteStore::new(pool)
}

// ── Code store ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn sqlite_code_store_conformance() {
    codlet_conformance::run_code_store_conformance(fresh_store).await;
}

// ── Session store ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn sqlite_session_store_conformance() {
    codlet_conformance::run_session_store_conformance(fresh_store).await;
}

// ── Form-token store ──────────────────────────────────────────────────────────

#[tokio::test]
async fn sqlite_form_token_store_conformance() {
    codlet_conformance::run_form_token_store_conformance(fresh_store).await;
}

// ── Migration smoke test ──────────────────────────────────────────────────────

#[tokio::test]
async fn migrations_are_idempotent() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    // Running twice must not error (IF NOT EXISTS semantics).
    run_migrations(&pool).await.unwrap();
    run_migrations(&pool).await.unwrap();
}

#[tokio::test]
async fn schema_includes_key_version_columns() {
    // RFC-011 §10.5: "Schema includes key version columns from first migration."
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();

    // Verify key_version exists in each table by attempting a SELECT.
    for table in ["codlet_codes", "codlet_sessions", "codlet_form_tokens"] {
        let result: Result<Vec<(String,)>, _> =
            sqlx::query_as(&format!("SELECT key_version FROM {table} LIMIT 0"))
                .fetch_all(&pool)
                .await;
        assert!(
            result.is_ok(),
            "table {table} must have a key_version column"
        );
    }
}
