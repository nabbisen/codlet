# codlet-sqlx

SQLite and PostgreSQL storage adapters for
[codlet](https://github.com/nabbisen/codlet), backed by
[SQLx](https://github.com/launchbadge/sqlx) (RFC-011, RFC-034).

## Status

Pre-1.0; breaking changes are permitted between minor releases (see
`CHANGELOG.md` for the current version). Both backends are implemented and pass
the full `codlet-conformance` suite, including the concurrent-claim race test:

| Feature | Store | Use case |
|---------|-------|----------|
| `sqlite` (default) | `SqliteStore` | Local development, single-server |
| `postgres` | `PostgresStore` | Multi-instance production. Supported and tested on PostgreSQL **15–18**. **No TLS stack**: see below |
| `postgres-rustls` | `PostgresStore` + TLS | Implies `postgres`; adds rustls (ring, webpki roots) so `sslmode=require` works |

PostgreSQL 15 to 18 is the supported range: CI runs the conformance suite on
15 and 18, the two ends, and asserts the server version it reached. Older
servers are untested rather than unsupported by design. See the version note in
`docs/src/adapter-matrix-and-config.md`.

A remote PostgreSQL connection is production-safe only with `postgres-rustls`
**and** `sslmode=require` (or stronger) in the connection string; without the
feature the default `sslmode` silently connects in plaintext. See *Transport
security* in `docs/src/adapter-matrix-and-config.md`.

The crate's minimum supported Rust version is higher than `codlet`'s because
of its `sqlx` dependency; see `SECURITY.md` for the per-crate table.

## Usage

```rust,ignore
use codlet_sqlx::{run_migrations, SqliteStore};
use codlet::store::code::CodeStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = sqlx::SqlitePool::connect("sqlite:codlet.db").await?;
    run_migrations(&pool).await?;
    let store = SqliteStore::new(pool);
    // store implements CodeStore, CodeAdminStore, SessionStore, FormTokenStore
    Ok(())
}
```

## Atomicity guarantee

Code claim and form-token consume use a single conditional `UPDATE … WHERE …
AND <guard>` followed by an affected-row count check. SQLite's serialised write
mode makes these atomic within one process; PostgreSQL relies on `READ
COMMITTED` row locking, so concurrent updates serialise at the row. See RFC-022
for the full isolation requirements.

## Migration

`run_migrations` (SQLite) and `run_postgres_migrations` (PostgreSQL) embed the
SQL and are safe to call on every startup. Host applications own the migration
application order; codlet tables must never contain foreign keys into
host-domain tables (RFC-011 §10.4).

## Conformance

Every store passes the `codlet-conformance` suite:
`run_code_store_conformance`, `run_session_store_conformance`,
`run_form_token_store_conformance` — including the concurrent-claim winner test
(RFC-022, RFC-023).

## License

Apache-2.0
