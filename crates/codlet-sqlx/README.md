# codlet-sqlx

SQLite storage adapters for [codlet](https://github.com/nabbisen/codlet),
backed by [SQLx](https://github.com/launchbadge/sqlx) (RFC-011).

## Status

Pre-release (v0.5.0). SQLite support is implemented and passes the full
`codlet-conformance` suite, including the concurrent-claim race test. PostgreSQL
support is planned for a later release.

## Usage

```rust,ignore
use codlet_sqlx::{run_migrations, SqliteStore};
use codlet_core::store::code::CodeStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = sqlx::SqlitePool::connect("sqlite:codlet.db").await?;
    run_migrations(&pool).await?;
    let store = SqliteStore::new(pool);
    // store implements CodeStore, SessionStore, FormTokenStore
    Ok(())
}
```

## Atomicity guarantee

Code claim and form-token consume use a single conditional `UPDATE … WHERE …
AND <guard>` followed by an affected-row count check. SQLite's serialised write
mode makes these atomic within one process. See RFC-022 for the full isolation
requirements.

## Migration

`run_migrations` embeds the SQL and uses `IF NOT EXISTS`, so it is safe to call
on every startup. Host applications own the migration application order; codlet
tables must never contain foreign keys into host-domain tables (RFC-011 §10.4).

## Conformance

All three stores pass the `codlet-conformance` suite:
`run_code_store_conformance`, `run_session_store_conformance`,
`run_form_token_store_conformance` — including the concurrent-claim winner test
(RFC-022, RFC-023).

## License

Apache-2.0
