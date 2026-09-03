# RFC-034: PostgreSQL Adapter (`codlet-sqlx` postgres feature)

- **Status:** Implemented (v0.12.0)
- **Target milestone:** M6
- **Primary crate(s):** `codlet-sqlx` (existing crate, new `postgres` feature)
- **Depends on:** RFC-011 (SQLx adapter scope), RFC-022 (atomicity contract),
  RFC-023 (conformance suite)
- **Source basis:** zinnias-ciao v0.36.1, PostgreSQL 14+

## 1. Summary

Add a PostgreSQL backend to `codlet-sqlx` behind a `postgres` feature flag.
`SqliteStore` and a new `PostgresStore` share the same migration schema
(with minor dialect adjustments) and the same conformance suite. PostgreSQL
is the recommended production backend for conventional multi-instance server
deployments.

## 2. Motivation

`SqliteStore` is suitable for single-process or single-server deployments.
Multi-instance deployments (multiple app servers sharing one database) need
a backend with row-level locking and cross-process atomic writes.
PostgreSQL's `UPDATE … WHERE … AND … RETURNING changes_count` (or affected
row count) satisfies the same single-winner contract as SQLite's serialised
writes. RFC-011 §10.1 explicitly deferred Postgres; the trait contract is
now stable (v0.9.0+), making this the right time.

## 3. Decision

Add a `postgres` feature to `codlet-sqlx`. Behind it, implement
`PostgresStore` with:
- a `PgPool` (from `sqlx::postgres`);
- a separate migration file (`0001_initial_pg.sql`) that maps SQLite types
  to their PostgreSQL equivalents;
- `CodeStore`, `SessionStore`, `FormTokenStore`, and `CodeAdminStore`
  implementations identical in SQL logic to `SqliteStore`, with three
  dialect differences documented below;
- conformance tests that spin up a PostgreSQL instance via `testcontainers`
  (no manual Postgres install required).

## 4. Feature flag

```toml
# codlet-sqlx/Cargo.toml
[features]
default  = ["sqlite"]
sqlite   = ["sqlx/sqlite"]
postgres = ["sqlx/postgres", "dep:testcontainers-modules"]

[dependencies]
sqlx = { workspace = true, features = ["runtime-tokio", "macros"] }

[dev-dependencies]
testcontainers-modules = { version = "0.11", features = ["postgres"], optional = true }
tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
```

`SqliteStore` and `PostgresStore` are independent types; both can be
enabled simultaneously. The conformance suite is parameterised over either.

## 5. Type mapping — SQLite vs PostgreSQL

| Concept | SQLite (`SqliteStore`) | PostgreSQL (`PostgresStore`) |
|---------|----------------------|------------------------------|
| Lookup key | `TEXT NOT NULL UNIQUE` | `TEXT NOT NULL UNIQUE` |
| Timestamps | `INTEGER` (Unix seconds, `i64`) | `BIGINT` (Unix seconds) |
| Nullable text | `TEXT` | `TEXT` |
| Subject kind | `TEXT` | `TEXT` |
| Primary key | `TEXT` | `TEXT` |
| Conditional UPDATE result | `result.rows_affected()` | `result.rows_affected()` |

PostgreSQL's `BIGINT` is 8-byte signed integer — identical semantics to
SQLite's `INTEGER` when storing Unix seconds. The Rust binding uses `i64`
in both cases via SQLx's native type mapping. No `as f64` conversion is
needed (unlike D1 in RFC-033).

## 6. Migration — `0002_postgres.sql`

```sql
-- codlet PostgreSQL migration 0001
-- Run this instead of 0001_initial.sql for PostgreSQL deployments.

CREATE TABLE IF NOT EXISTS codlet_codes (
    id              TEXT    NOT NULL PRIMARY KEY,
    lookup_key      TEXT    NOT NULL UNIQUE,
    key_version     TEXT    NOT NULL,
    purpose         TEXT,
    scope           TEXT,
    grant_payload   TEXT,
    created_at      BIGINT  NOT NULL,
    expires_at      BIGINT  NOT NULL,
    used_at         BIGINT,
    used_by_subject TEXT,
    revoked_at      BIGINT
);

CREATE INDEX IF NOT EXISTS idx_codlet_codes_lookup
    ON codlet_codes (lookup_key, used_at, revoked_at, expires_at);

CREATE INDEX IF NOT EXISTS idx_codlet_codes_scope
    ON codlet_codes (scope, used_at, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_sessions (
    id          TEXT    NOT NULL PRIMARY KEY,
    lookup_key  TEXT    NOT NULL UNIQUE,
    key_version TEXT    NOT NULL,
    subject     TEXT    NOT NULL,
    created_at  BIGINT  NOT NULL,
    expires_at  BIGINT  NOT NULL,
    revoked_at  BIGINT
);

CREATE INDEX IF NOT EXISTS idx_codlet_sessions_lookup
    ON codlet_sessions (lookup_key, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_form_tokens (
    lookup_key      TEXT    NOT NULL PRIMARY KEY,
    key_version     TEXT    NOT NULL,
    subject_kind    TEXT    NOT NULL,
    purpose         TEXT    NOT NULL,
    bound_resource  TEXT,
    issued_at       BIGINT  NOT NULL,
    expires_at      BIGINT  NOT NULL,
    consumed_at     BIGINT,
    result_ref      TEXT
);

CREATE INDEX IF NOT EXISTS idx_codlet_form_tokens_lookup
    ON codlet_form_tokens (lookup_key, consumed_at, expires_at);
```

Differences from the SQLite migration: `BIGINT` instead of `INTEGER`.
`IF NOT EXISTS` is supported by PostgreSQL 9.1+ and is safe for idempotent
application. No `PRAGMA` statements (SQLite-only).

## 7. Atomic claim / consume — dialect differences

### PostgreSQL `claim_code`

The SQL is identical to SQLite:

```sql
UPDATE codlet_codes
SET used_at = $1, used_by_subject = $2
WHERE id = $3
  AND used_at    IS NULL
  AND revoked_at IS NULL
  AND expires_at  > $4
```

PostgreSQL uses `$N` placeholders instead of `?`. SQLx handles this
transparently — the same Rust `.bind()` call works for both drivers.
`result.rows_affected()` returns the affected count.

### Why no `RETURNING` clause

RFC-011 mentions `RETURNING` as an option for PostgreSQL. This RFC rejects
it for uniformity: the `CodeStore` trait is designed around the
affected-row count (`classify_claim(changed)`). Using `RETURNING` would
require a different code path for no security gain — the conditional WHERE
clause already provides the atomic single-winner guarantee. `RETURNING` is
not used.

### PostgreSQL isolation

PostgreSQL's default isolation level is `READ COMMITTED`. The conditional
UPDATE acquires a row-level lock on the matched row. Concurrent transactions
targeting the same row will serialise: exactly one will update and commit;
the rest will see `rows_affected() == 0`. This is equivalent to SQLite's
serialised write mode and D1's per-row locking.

No explicit `SERIALIZABLE` isolation or `FOR UPDATE` locking is needed.
The adapter documentation states this explicitly.

## 8. `PostgresStore` type

```rust
/// PostgreSQL storage adapter for codlet (RFC-034).
///
/// Clone is cheap (pool is reference-counted).
#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: sqlx::PgPool,
}

impl PostgresStore {
    pub fn new(pool: sqlx::PgPool) -> Self { Self { pool } }
    pub fn pool(&self) -> &sqlx::PgPool { &self.pool }
}
```

Implements `CodeStore`, `SessionStore`, `FormTokenStore`, `CodeAdminStore`.
All implementations are behind `#[cfg(feature = "postgres")]`.

## 9. Migration runner

```rust
#[cfg(feature = "postgres")]
pub async fn run_postgres_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let sql = include_str!("../migrations/0002_postgres.sql");
    for stmt in sql.split(';') {
        let trimmed = stmt.lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = trimmed.trim();
        if trimmed.is_empty() { continue; }
        sqlx::query(trimmed).execute(pool).await?;
    }
    Ok(())
}
```

Same splitter logic as `run_migrations` for SQLite.

## 10. Test strategy — `testcontainers`

Postgres integration tests spin up a real PostgreSQL 16 container:

```rust
#[cfg(all(test, feature = "postgres"))]
async fn fresh_pg_store() -> PostgresStore {
    use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};
    let container = postgres::Postgres::default().start().await.unwrap();
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container.get_host_port_ipv4(5432).await.unwrap()
    );
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    run_postgres_migrations(&pool).await.unwrap();
    PostgresStore::new(pool)
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn postgres_code_store_conformance() {
    codlet_conformance::run_code_store_conformance(fresh_pg_store).await;
}
```

The container is started per test and dropped on test completion. No
persistent Postgres installation is required. CI adds Docker to the
`test-postgres` job.

## 11. Security considerations

- Conditional UPDATE provides row-level atomic single-winner guarantee under
  `READ COMMITTED` isolation (see §7 above). Documented explicitly.
- `rows_affected() > 1` returns `StoreError::InvariantViolation` (same as
  SQLite). Impossible in practice due to `PRIMARY KEY` and `UNIQUE`
  constraints, but the guard is kept for defence in depth.
- Timestamps use `BIGINT` (i64). No overflow risk for Unix seconds.
- No raw SQL interpolation. All user-supplied values pass through SQLx's
  parameterised query layer.

## 12. Concrete acceptance checklist

- [x] `cargo test -p codlet-sqlx --features postgres-test` passes all
      conformance tests against a real PostgreSQL instance (requires Docker; CI job: test-postgres).
- [x] `PostgresStore::claim_code` uses conditional UPDATE and checks `rows_affected()`.
- [x] `PostgresStore::consume_form_token` uses conditional UPDATE and checks `rows_affected()`.
- [x] `rows_affected() > 1` returns `StoreError::InvariantViolation`.
- [x] `run_postgres_migrations` is idempotent (`IF NOT EXISTS` everywhere).
- [x] Adapter documentation states the isolation level and explains why `READ COMMITTED` + conditional UPDATE is sufficient (in `postgres/mod.rs` module doc).
- [x] `PostgresStore` passes the concurrent-claim race test from `codlet-conformance` via `postgres_code_store_conformance` (includes `test_exactly_one_claim_winner`).
- [x] `RETURNING` is not used; decision documented in `postgres/mod.rs` and `postgres/code.rs`.
- [x] `#[cfg(feature = "postgres")]` gates all Postgres code; `sqlite` feature works independently. Docker-requiring tests behind `postgres-test` feature.

## 13. Open questions

None. The dialect delta from SQLite is fully characterised above. The only
pending decision is whether `testcontainers` should be a mandatory dev
dependency or an optional one gated on a `"postgres-test"` feature; the
default should be mandatory to keep the test gate simple.
