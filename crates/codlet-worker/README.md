# codlet-worker

Cloudflare Workers adapter for [codlet](https://github.com/nabbisen/codlet)
(RFC-033).

Provides D1-backed `CodeStore`, `SessionStore`, `FormTokenStore`, and
`CodeAdminStore` implementations, plus a KV-backed `RateLimitStore` and a
`WorkerKeyProvider` that loads HMAC key material from `Env` secrets.

## Status

Pre-release. The crate compiles for `wasm32-unknown-unknown` (verified in
CI). Miniflare integration tests are in `tests/` — see [Running
tests](#running-tests).

## Adapter guarantee matrix

| Property | This adapter |
|----------|-------------|
| Atomic code claim | ✓ conditional UPDATE + `meta().changes` |
| Atomic token consume | ✓ conditional UPDATE + `meta().changes` |
| `changes > 1` → `InvariantViolation` | ✓ |
| Multi-process safe | ✓ (D1 global per-row locking) |
| Rate-limit consistency | ⚠ KV is eventually consistent — see below |

## KV rate-limit caveat

`KvRateLimitStore` uses Cloudflare KV, which is **eventually consistent**.
Under a high-concurrency or distributed attack, failure counters may be read
stale and under-count actual attempts. This store is suitable for friction
reduction against unsophisticated bots.

For stronger guarantees, combine with Cloudflare WAF rules, Turnstile, or a
D1-backed counter (not yet implemented).

## Usage

```rust,ignore
use codlet_worker::{
    D1CodeStore, D1SessionStore, D1FormTokenStore,
    KvRateLimitStore, WorkerKeyProvider, D1TableConfig,
    run_d1_migrations,
};

#[worker::event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let db     = env.d1("DB")?;
    let kv     = env.kv("CODLET_RL")?;
    let tables = D1TableConfig::default();

    // Run migrations on every deploy (idempotent — IF NOT EXISTS).
    run_d1_migrations(&db).await?;

    // Load HMAC keys from Wrangler secrets. Fails closed if missing (INV-2).
    let keys = WorkerKeyProvider::from_env(&env, "v1", "CODLET_HMAC_KEY_V1", &[])?;

    // D1Database is not Clone; share it via Rc (Workers are single-threaded).
    let db = std::rc::Rc::new(db);
    let code_store    = D1CodeStore::new(std::rc::Rc::clone(&db), tables.clone());
    let session_store = D1SessionStore::new(std::rc::Rc::clone(&db), tables.clone());
    let token_store   = D1FormTokenStore::new(db, tables);
    let rl_store      = KvRateLimitStore::new(kv);

    // Wire into codlet_core managers …
    todo!()
}
```

## Migrating from zinnias-ciao

If your existing service uses different table names, use `D1TableConfig::zinnias_ciao_compat()`:

```rust,ignore
let tables = D1TableConfig::zinnias_ciao_compat();
// codes → "invite_codes", sessions → "sessions", form_tokens → "form_tokens"
// Column names must match the codlet schema; add key_version columns first.
```

See `docs/src/migration-from-zinnias-ciao.md` for the full checklist.

## wrangler.toml

```toml
[[d1_databases]]
binding       = "DB"
database_name = "my-codlet-db"
database_id   = "<your-d1-id>"

[[kv_namespaces]]
binding = "CODLET_RL"
id      = "<your-kv-id>"

[vars]
# Use `wrangler secret put CODLET_HMAC_KEY_V1` for production.
# CODLET_HMAC_KEY_V1 = "..."  # do not commit real secrets
```

## Running tests

Integration tests use Miniflare (no Cloudflare account required):

```sh
cd crates/codlet-worker/tests
npm install
npx vitest run
```

## Building

```sh
cargo build -p codlet-worker --target wasm32-unknown-unknown
```

## License

Apache-2.0
