# RFC-033: Cloudflare Workers / D1 / KV Adapter (`codlet-worker`)

- **Status:** Implemented (v0.11.0)
- **Target milestone:** M6
- **Primary crate(s):** `codlet-worker` (new)
- **Depends on:** RFC-010 (accepted design), RFC-022 (atomicity contract),
  RFC-023 (conformance suite), RFC-009 (`?Send` trait strategy)
- **Source basis:** zinnias-ciao v0.36.1 + `worker` crate v0.5.0

## 1. Summary

Implement `codlet-worker`: a `wasm32-unknown-unknown` adapter crate for
Cloudflare Workers that provides D1-backed `CodeStore`, `SessionStore`, and
`FormTokenStore` implementations, plus a KV-backed `RateLimitStore` and
`WorkerKeyProvider` that loads HMAC key material from `Env` secrets.

This RFC supersedes RFC-010's design section with concrete, implementable
detail. RFC-010 stated intent; this RFC specifies types, SQL, binding API
calls, timestamp representation, error mapping, migration strategy, local
test approach, and the full acceptance checklist.

## 2. Motivation

The source service (`zinnias-ciao`) runs on Cloudflare Workers with D1 and
KV. The D1 query API is SQLite-compatible (same SQL dialect, same
conditional-UPDATE atomicity model). The implementation delta from
`codlet-sqlx` is the binding layer: `D1PreparedStatement` instead of SQLx,
`D1Type` instead of `.bind()`, and `meta().changes` instead of
`rows_affected()`. The SQL itself is reused verbatim.

## 3. Decision

Ship `codlet-worker` as a workspace crate compiled only for
`wasm32-unknown-unknown`. It implements all three store traits and
`RateLimitStore` via D1 and KV respectively. Tests run under Miniflare
without production Cloudflare credentials.

## 4. Crate structure

```
crates/codlet-worker/
  Cargo.toml           (target: wasm32-unknown-unknown only)
  README.md
  src/
    lib.rs
    key_provider.rs    WorkerKeyProvider
    d1/
      mod.rs
      code.rs          D1CodeStore  → CodeStore + CodeAdminStore
      session.rs       D1SessionStore → SessionStore
      token.rs         D1FormTokenStore → FormTokenStore
    kv/
      rate_limit.rs    KvRateLimitStore → RateLimitStore
    http/
      cookies.rs       cookie extraction from Request headers
      identity.rs      rate-limit key from CF-Connecting-IP / trusted header
    migration.rs       run_d1_migrations(&D1Database)
  tests/               integration tests (run under miniflare via wrangler)
```

## 5. Cargo.toml

```toml
[package]
name = "codlet-worker"
# ... workspace fields ...
publish = false  # until v1.0 API stabilises

[lib]
crate-type = ["cdylib"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
codlet-core = { path = "../codlet-core" }
worker       = { version = "0.5", features = ["d1"] }

[dev-dependencies]
# No wasm-pack/miniflare dep here; tests are run via `wrangler dev --test`
# or a miniflare harness outside Cargo.
```

The `codlet-core` dep brings no native I/O; it compiles to wasm32 as
verified by RFC-009 / v0.10.0.

## 6. Timestamp representation

D1 is SQLite. SQLite has no native timestamp type. This RFC mandates
**Unix seconds stored as `REAL` (f64)**:

```rust
// Bind a u64 timestamp to a D1 statement.
fn ts(t: u64) -> D1Type<'static> {
    D1Type::Real(t as f64)
    // f64 has 53-bit integer precision.
    // Unix seconds fit until year 285,616,414 — no 2038 issue.
}
```

Comparison in SQL uses `>` and `<` against the same bound value:

```sql
WHERE expires_at > ?   -- same f64 value passed as `now`
```

This matches `codlet-sqlx`'s semantics (which uses `i64`); the schema DDL
is identical. Adapters must use the same comparison direction and never
mix integer and real representations within one table.

## 7. SQL

The schema DDL is `crates/codlet-sqlx/migrations/0001_initial.sql` verbatim.
D1's SQL dialect is SQLite; the same `CREATE TABLE`, `CREATE INDEX`, and
`UPDATE … WHERE … AND` statements are valid without modification.

Migration is applied via `D1Database::exec`:

```rust
pub async fn run_d1_migrations(db: &worker::d1::D1Database) -> worker::Result<()> {
    // exec() accepts multiple statements separated by semicolons.
    // It is intended for maintenance tasks; migration runs once at deploy time.
    db.exec(include_str!("../migrations/0001_initial.sql")).await?;
    Ok(())
}
```

## 8. Affected-row count (atomicity)

D1's `run()` returns `D1Result`; `meta().changes` gives the affected count:

```rust
let result = stmt.run().await?;
let changed = result
    .meta()?
    .and_then(|m| m.changes)
    .unwrap_or(0);
// changed == 0 → Lost / Invalid / already-consumed
// changed == 1 → Won / Proceed
// changed > 1 → InvariantViolation (impossible by UNIQUE constraint, but guard it)
```

This is the exact counterpart of `result.rows_affected() as usize` in
`codlet-sqlx`. The security contract is identical.

## 9. `WorkerKeyProvider`

```rust
pub struct WorkerKeyProvider {
    active: (KeyVersion, Vec<u8>),
    previous: Vec<(KeyVersion, Vec<u8>)>,
}

impl WorkerKeyProvider {
    /// Load key material from Cloudflare Worker `Env` secrets.
    ///
    /// `active_binding` is the Wrangler secret name for the active key,
    /// e.g. `"CODLET_HMAC_KEY_V2"`.  `active_version` is the label that
    /// will appear in the `key_version` column of every new record.
    ///
    /// `previous` is a list of `(version_label, secret_binding_name)` pairs
    /// for keys still accepted for existing records.
    ///
    /// Fails closed: if any binding is missing or empty, returns `Err`.
    pub fn from_env(
        env: &worker::Env,
        active_version: &str,
        active_binding: &str,
        previous: &[(&str, &str)],
    ) -> worker::Result<Self> { ... }
}

impl KeyProvider for WorkerKeyProvider { ... }
```

The secret is loaded via `env.secret("CODLET_HMAC_KEY_V2")?.to_string()`.
An empty or missing binding returns `KeyError::InvalidKeyMaterial` (INV-2).

## 10. `D1TableConfig` — zinnias-ciao compatibility

The existing service uses different table and column names. The adapter
accepts an optional config to map them:

```rust
/// Production default: codlet's own table names (new deployments).
pub struct D1TableConfig {
    /// Table name for codes. Default: `"codlet_codes"`.
    pub codes: &'static str,
    /// Table name for sessions. Default: `"codlet_sessions"`.
    pub sessions: &'static str,
    /// Table name for form tokens. Default: `"codlet_form_tokens"`.
    pub form_tokens: &'static str,
}

impl Default for D1TableConfig {
    fn default() -> Self {
        Self {
            codes:       "codlet_codes",
            sessions:    "codlet_sessions",
            form_tokens: "codlet_form_tokens",
        }
    }
}
```

Column names are fixed per the migration schema. If the host service has
different column names, a view or a separate migration that renames columns
is the host's responsibility; codlet does not parameterise column names.

## 11. `KvRateLimitStore`

KV stores a JSON counter with expiry:

```rust
// Key format: "codlet:rl:{fingerprint}"
// Value: u32 failure count, JSON-encoded
// TTL: set to policy.window.as_secs() on every write
```

```rust
impl RateLimitStore for KvRateLimitStore {
    async fn check(&self, key: &RateLimitKey, policy: &RateLimitPolicy)
        -> Result<RateLimitOutcome, StoreError>
    {
        match self.kv.get(self.kv_key(key)).text().await? {
            None => Ok(RateLimitOutcome::Allow),
            Some(json) => {
                let count: u32 = serde_json::from_str(&json).unwrap_or(0);
                Ok(if policy.is_exceeded(count) {
                    RateLimitOutcome::Deny
                } else {
                    RateLimitOutcome::Allow
                })
            }
        }
    }
    // record_failure: GET → increment → PUT with TTL
    // clear_failures: DELETE
}
```

The docs must state: KV is eventually consistent. Under distributed attack,
counters may under-count. For stronger guarantees use D1-backed counters or
Cloudflare WAF rules. This was required by RFC-010 §12.3 and remains a
documentation obligation.

## 12. Cookie helpers

```rust
/// Extract a named cookie value from a Workers `Request`.
pub fn extract_cookie(req: &worker::Request, name: &str) -> Option<String>;

/// Construct a `Headers` object containing a `Set-Cookie` header value
/// produced by `CookiePolicy::build_set_cookie`.
pub fn set_cookie_header(policy: &CookiePolicy, secret: &str) -> worker::Headers;
```

## 13. Rate-limit key extraction

```rust
/// Extract a rate-limit key from a Workers `Request`.
///
/// Tries `CF-Connecting-IP` first (set by Cloudflare's edge, not
/// spoofable from the internet).  Falls back to `X-Real-IP` if configured.
/// Returns `None` if no trustworthy IP is available — callers must decide
/// whether to fail-open or fail-closed.
pub fn extract_rate_limit_key(
    req: &worker::Request,
    trusted_header: Option<&str>,
) -> Option<RateLimitKey>;
```

## 14. Local test strategy (no production credentials required)

Tests use **Miniflare** via `wrangler dev --test` or the
`@cloudflare/vitest-pool-workers` harness, which provides a local D1 and
KV implementation without a Cloudflare account. The conformance suite
(`codlet-conformance`) is invoked from a Workers test script:

```typescript
// tests/conformance.ts (run by wrangler)
import { runConformance } from "./run_conformance_wasm";
test("d1 code store conforms", async () => {
    await runConformance(env.DB);
});
```

The Rust conformance runner is compiled to WASM and called from the test
script. This satisfies RFC-010 §12.5 without requiring production
credentials.

CI runs `wrangler dev --test` in the CI job gated on the `codlet-worker`
crate. This job is separate from native test jobs.

## 15. Security considerations

All invariants from RFC-022 apply:
- `claim_code` uses a conditional UPDATE; `meta().changes` is checked.
- `consume_form_token` uses a conditional UPDATE; `meta().changes` is checked.
- `changes > 1` returns `StoreError::InvariantViolation`.
- `WorkerKeyProvider::from_env` fails closed on missing or empty secret
  binding (INV-2).
- KV rate limiting is documented as eventually consistent (RFC-010 §12.3).

## 16. Concrete acceptance checklist

- [x] `cargo build -p codlet-worker --target wasm32-unknown-unknown` succeeds.
- [x] `D1CodeStore::claim_code` uses conditional UPDATE and checks `meta().changes`.
- [x] `D1FormTokenStore::consume_form_token` uses conditional UPDATE and
      checks `meta().changes`.
- [x] `changes > 1` in either operation returns `StoreError::InvariantViolation`.
- [x] Timestamps are bound as `D1Type::Real(t as f64)` throughout.
- [x] `WorkerKeyProvider::from_env` returns `Err` if any secret binding is
      missing or empty.
- [x] `KvRateLimitStore` docs mention eventual consistency.
- [x] `extract_rate_limit_key` documents which headers are trusted and warns
      about spoofing risk.
- [~] D1 conformance suite passes under Miniflare without production credentials. *(Note: test scaffold in tests/conformance.test.ts; CI job commented out pending Node/wrangler pipeline setup — pre-v1 task.)*
- [x] `D1TableConfig` defaults match the migration schema names; custom names
      are documented as a host responsibility.

## 17. Open questions

None. The design decisions in RFC-010 §12 are adopted verbatim; this RFC
fills the gaps.
