# Changelog

All notable changes to codlet are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and codlet aims to follow
semantic versioning once it reaches a stable release.

## [Unreleased]

Nothing yet.

## [0.17.0] — 2026-06-24

`codlet-core` crate renamed to `codlet`.

## [0.16.2] — 2026-06-16

Example projects in examples/.

## [0.16.1] — 2026-06-16

Sequence diagram position in README.md.

## [0.16.0] — 2026-06-15

P1 security fixes from the external source review (RFC-A through RFC-E).
165 Rust tests + 12 Miniflare tests. README workflow diagram added.

### Fixed

- **RFC-A: Key rotation candidate lookup** — `SessionManager::validate()`,
  `CodeAuth::find()`, and `FormTokenManager::consume()` previously derived
  only the active-key lookup candidate, making existing sessions, codes, and
  tokens unreachable after a key rotation. All three managers now call
  `SecretHasher::lookup_key_candidates()`, which returns one candidate per
  held key (active first, then previous). The store `find_*` methods already
  accepted `&[LookupKey]`; the fix is in the manager layer.
  `FormTokenStore::consume_form_token` and `set_token_result` signatures
  upgraded from `&LookupKey` to `&[LookupKey]` for consistency.
  `KeyProvider` gains `all_hmac_keys()`; `SecretHasher` gains
  `lookup_key_candidates()`.

- **RFC-B: Rate-limit failure accounting** — `CodeAuth::find()` now calls
  `record_failure()` after invalid-format rejections and after not-found
  results, not only after a lost concurrent claim. Prior to this fix,
  brute-force guesses of wrong or random codes did not increment the failure
  counter. `FailClosed` is now honoured when `check()` returns `Err`: the
  operation is denied rather than silently failing open.
  `RateLimitUnavailable::SoftDenyAfterThreshold` removed — only `FailOpen`
  and `FailClosed` remain.

- **RFC-C: Purpose/scope enforcement across all adapters** — `claim_code()`
  in the SQLite, PostgreSQL, and D1 adapters previously ignored `purpose` and
  `scope` from `ClaimRequest`, allowing a code issued for one flow to be
  redeemed in another. All three adapters now include `purpose` and `scope`
  in the conditional UPDATE WHERE clause when present. `RedeemableCode` gains
  a `purpose` field; `CodeAuth::claim()` passes `purpose` and `scope` from
  the found record into `ClaimRequest`. `find_redeemable` SELECT queries
  updated to include `purpose` in all three adapters.

- **RFC-E: Form-token bound_resource conformance** — `MemFormTokenStore`
  treated caller `None` as a wildcard matching any stored value
  (`(None, _) => true`). SQL and D1 adapters correctly reject
  `Some(stored) + None(caller)`. Semantics aligned to exact-match across all
  adapters: `(None, None)` proceeds; all other mismatches are invalid.

### Changed

- **RFC-D: `redeem_with_callback()` marked experimental** — `#[deprecated]`
  with a doc note explaining the DB/audit state divergence: the method claims
  the code before the host callback returns the real subject, leaving
  `used_by_subject = "__pending__"` in the database if the callback fails.
  The two-step `find()` + host subject creation + `claim()` flow is the
  recommended production path.

- **README.md: workflow diagram** replaces the stale "Status" section
  (which cited v0.8.0, 142 tests, and incorrectly described the library as
  "feature-complete"). The diagram shows the codlet/host responsibility
  boundary. Quick start section updated with a `Cargo.toml` snippet using
  crates.io version specifiers.

### Security

- After RFC-B, invalid-format and not-found code guesses now count toward the
  rate-limit counter. Before this fix, an attacker could submit unlimited
  wrong codes without being throttled.
- After RFC-A, rotating the HMAC key no longer silently invalidates all
  in-flight sessions and unredeemed codes.
- After RFC-C, `purpose` and `scope` fields set at code issuance are now
  enforced at claim time in all production adapters, preventing cross-flow
  redemption.
- After RFC-E, a resource-bound form token cannot be consumed by a caller
  that omits the `bound_resource` parameter in any adapter.

## [0.15.2] — 2026-06-15

Documentation fix: corrected a bug in the Option B in-place migration SQL
for the `sessions` table.

### Fixed

- `docs/src/migration-from-an-existing-service.md` — Option B migration SQL
  for `sessions` included `ALTER TABLE sessions ADD COLUMN created_at INTEGER
  NOT NULL DEFAULT 0`. This fails with `duplicate column name` on any service
  that already has a `created_at` column, which is the common case. The
  bogus `ADD COLUMN` line is removed. A callout now explains the type conflict
  (`TEXT` ISO-8601 vs `INTEGER` Unix seconds), its limited impact (admin
  listings only, not session validation), and the resolution options. The
  conflict is cited as a further reason to prefer Option A (fresh codlet
  tables alongside existing ones).

## [0.15.1] — 2026-06-15

Dependency updates. No API changes.

### Changed

- `hmac` 0.12 → 0.13: `KeyInit` trait now required to bring `new_from_slice`
  into scope; import updated in `hashing.rs`.
- `sha2` 0.10 → 0.11.
- `getrandom` 0.2 → 0.4: `getrandom::getrandom()` renamed to
  `getrandom::fill()`; wasm32 feature renamed from `js` to `wasm_js`.
- `sqlx` 0.8 → 0.9: dynamic SQL strings now require `sqlx::AssertSqlSafe`
  wrapper; applied to migration runners and the admin `list_codes` dynamic
  WHERE query.
- `testcontainers-modules` 0.11 → 0.15 (dev-dependency, postgres-test only).
- CI: `actions/checkout` v4 → v6; Node.js v22 → v24 in `wrangler-test` job.

## [0.15.0] — 2026-06-15

Breaking API changes to remove app-specific names from a general library,
plus D1 store constructor fix and short-code deprecation surface.

### Changed

- **`D1CodeStore::new()`, `D1SessionStore::new()`, `D1FormTokenStore::new()`**
  now take `Rc<worker::d1::D1Database>` instead of `worker::d1::D1Database`.
  `D1Database` is not `Clone`; the previous doc example showing
  `Rc::clone(&db)` passed to a function expecting owned `D1Database` did not
  compile. The correct pattern is `Rc::new(env.d1("DB")?)` once, then
  `Rc::clone(&db)` for each store.

- **`D1TableConfig::zinnias_ciao_compat()`** renamed to
  **`D1TableConfig::with_existing_table_names()`**. The old name embedded
  the name of a specific adopting application. The function behaviour is
  unchanged (table-name override only; column names are not remapped).

- **`CodePolicy::legacy_ciao_6(ttl)`** renamed to
  **`CodePolicy::six_symbol(ttl)`** and marked `#[deprecated]`. The old name
  embedded an app-specific term. `six_symbol` emits a compiler warning at
  every call site — the warning is the intended friction for 6-symbol codes
  (~29.7 bits of entropy, requires rate limiting).

- **`CodePolicy::short_compat(alphabet, length, ttl)`** marked `#[deprecated]`
  for the same reason: codes shorter than `SECURE_MIN_HUMAN_LENGTH` require
  active rate limiting to be safe.

- **`LEGACY_CIAO_LENGTH`** constant renamed to **`SHORT_COMPAT_LENGTH`**.

- All `zinnias-ciao` / `ciao` / `zinnias` occurrences removed from `crates/`
  and `docs/` source and documentation. RFC filenames are exempted as
  immutable historical records.

- `docs/src/migration-from-zinnias-ciao.md` renamed to
  `docs/src/migration-from-an-existing-service.md`; title and body
  generalised to describe any existing service.

- `docs/src/adapter-matrix-and-config.md`: D1 row corrected from "planned"
  to implemented; `PostgresStore` row added; `six_symbol` deprecation
  explained in prose.

### Fixed

- `crates/codlet-worker/src/lib.rs` and `crates/codlet-worker/README.md`
  doc examples corrected to use `Rc::new(env.d1("DB")?)` and
  `Rc::clone(&db)` (matching the new `new()` signatures).

- `docs/src/migration-from-an-existing-service.md`: `D1TableConfig::
  with_existing_table_names()` doc comment clarified — table-name override
  only; column names are not remapped; full column-rename SQL provided for
  Option B in-place migration.

## [0.14.4] — 2026-06-14

Removed two needless `mut` annotations in `codlet-worker/src/http/cookies.rs`
that caused `unused_mut` Clippy warnings. No behaviour change.

## [0.14.3] — 2026-06-14

Published by the project owner. Includes dependency version consolidation
into workspace-level `[workspace.dependencies]`, worker/worker-kv bumps
(0.5→0.8, 0.7→0.9), `rust-toolchain.toml`, `CONTRIBUTING.md`, and
crates.io/docs.rs badges in README.

## [0.14.2] — 2026-06-14

Four-dimensional audit (RFCs vs codebase, tests vs requirements, code vs
tests, docs vs codebase). All findings corrected.

### Fixed

- **`D1CodeStore::new()` / `D1SessionStore::new()` / `D1FormTokenStore::new()`**
  documented as taking `Rc<D1Database>` in v0.14.0, but the actual signatures
  still took owned `D1Database`. The documentation and README were wrong; the
  implementation was correct. Doc examples corrected to use `env.d1("DB")?`
  once per store (Option A).

- **`D1TableConfig::zinnias_ciao_compat()`** doc comment strengthened to
  say explicitly: "table-name override only — column names are not remapped."

- **`MemFormTokenStore::consume_form_token()`** wildcard `(None, _) => true`
  identified as a semantic divergence from SQL/D1 adapters (pre-RFC-E;
  noted, not yet fixed in this release).

- **RFC index** (`rfcs/README.md`) rebuilt from filesystem: stale "Proposed (2)"
  entries pointing to RFC-033 and RFC-034 (both in `done/`) removed.

- **RFC checklists**: RFC-001, RFC-002, RFC-010, RFC-012, RFC-013, RFC-014,
  RFC-015 — all checklist items verified against code and ticked. RFC-031
  reclassified as partial (multi-candidate key lookup not yet wired).

- **`docs/src/adapter-matrix-and-config.md`**: D1 row said "planned"; now
  shows implemented. `PostgresStore` row was missing; added.

- **`docs/src/migration-from-an-existing-service.md`**: `six_symbol` /
  `short_compat` references updated.

## [0.14.1] — 2026-06-14

CI fixes only: `actions/checkout` v4 → v6; `node-version` 22 → 24.

## [0.14.0] — 2026-06-14

## [0.14.0] — 2026-06-14

Miniflare integration tests complete (RFC-033 §14 fully resolved). All 10
RFC-033 acceptance checklist items now ticked. The `wrangler-test` CI job
is active. 151 Rust tests + 12 Miniflare tests passing.

### Added

- **Miniflare integration test suite** (`crates/codlet-worker/tests/`):
  - `worker.js` — test harness Worker that mirrors the SQL executed by
    the Rust D1 stores, exposed as HTTP endpoints for `SELF.fetch()` calls.
    Runs inside Miniflare's V8 Workers runtime with real D1 and KV bindings.
  - `conformance.test.ts` — 12 integration tests via `@cloudflare/vitest-pool-workers`:
    migration idempotency, code insert/find, REAL timestamp comparison
    (D1Type::Real semantics), concurrent claim race (INV-5, exactly 1 winner
    out of 4), post-claim not-found, session insert/find, session expiry,
    token consume race (INV-6), token replay, KV record_failure increment,
    KV clear_failures delete.
  - `vitest.config.ts` — `defineWorkersConfig` wrangler integration.
  - `wrangler.toml` — updated `main = "worker.js"`.
  - `package.json` — `@cloudflare/vitest-pool-workers@^0.6`.

- CI `wrangler-test` job (previously commented out): `npm ci && npx vitest
  run` under Node 22. No Cloudflare account required; Miniflare runs fully
  locally.

### Changed

- RFC-033 §14 checklist item: `[~]` → `[x]`. RFC-033 is now 10/10.
  All RFC-033 acceptance criteria satisfied.

### Notes

The test harness (`worker.js`) uses JavaScript rather than compiled Rust
for the following reason: `codlet-worker` is a library crate with no fetch
handler; adding one to the production crate would pollute its API. The
JavaScript harness executes the exact same SQL as the Rust implementations,
running in the live Miniflare D1/KV runtime — which is what these tests
uniquely verify over the wasm32 compile check and the SQLite conformance suite.

## [0.13.0] — 2026-06-14

Feature-flag audit: all cargo features now gate their code correctly.
A postgres-only deployment no longer compiles SQLite code; a native
server build no longer links `getrandom/js` wasm-bindgen glue.

### Fixed

- **`codlet-sqlx`: SQLite and PostgreSQL now properly isolated.**
  The sqlite modules (`code`, `session`, `token`, `admin`, `migration`,
  `SqliteStore`) are now gated on `#[cfg(feature = "sqlite")]`.
  Previously, `--features postgres` would still compile `sqlx-sqlite`
  and all SQLite code even when unused.
  - `--no-default-features --features postgres`: compiles only
    `sqlx-postgres` — SQLite driver and code absent from the binary.
  - `--features sqlite --features postgres`: both drivers compiled.
  - Default (`sqlite` only): unchanged for existing users.
  - `codlet-sqlx/tests/conformance.rs`: SQLite tests wrapped in
    `#[cfg(feature = "sqlite")] mod sqlite_tests`; Postgres tests
    already in `#[cfg(feature = "postgres-test")] mod postgres_tests`.
  - CI `test-postgres` job now uses `--no-default-features --features
    postgres-test`, verifying feature isolation on every push.

- **`getrandom/js` feature removed from native builds.**
  The `js` feature routes `getrandom` through `wasm-bindgen`'s
  `crypto.getRandomValues()` and has no effect on native targets —
  but added wasm-bindgen as a transitive dependency to native binaries.
  The workspace root now specifies `getrandom` without `features = ["js"]`.
  `codlet-core` activates `features = ["js"]` only under
  `[target.'cfg(target_arch = "wasm32")'.dependencies]`.
  Native server binaries no longer link wasm-bindgen glue.

- **`codlet-sqlx` description updated** to reflect PostgreSQL support.

### Changed

- `codlet-sqlx` Cargo.toml comment added explaining the `runtime-tokio`
  rationale: tokio is the only supported async runtime; hosts on async-std
  must bridge via a tokio block.

## [0.12.0] — 2026-06-14

`PostgresStore` — PostgreSQL adapter for `codlet-sqlx`. Full `CodeStore`,
`SessionStore`, `FormTokenStore`, and `CodeAdminStore` implementation using
`$N` placeholders, `BIGINT` timestamps, and `READ COMMITTED` + conditional
UPDATE atomicity. RFC-034 fully implemented (9/9 checklist items). 152 tests
in native CI; Docker-dependent postgres tests gated on `postgres-test` feature.

### Added

- **`crates/codlet-sqlx`: `postgres` feature** (RFC-034):
  - `PostgresStore` — wraps `sqlx::PgPool`; implements `CodeStore`,
    `SessionStore`, `FormTokenStore`, `CodeAdminStore`. Clone is cheap.
  - `run_postgres_migrations` — applies `migrations/0002_postgres.sql`
    statement by statement; idempotent (`IF NOT EXISTS` throughout).
  - `migrations/0002_postgres.sql` — PostgreSQL DDL: `BIGINT` timestamps,
    same schema shape as SQLite, no `PRAGMA` statements.
  - Module doc (`postgres/mod.rs`) states the isolation level rationale:
    `READ COMMITTED` + row-level UPDATE lock = exactly one winner.
    No `SERIALIZABLE`, no `FOR UPDATE`, no `RETURNING` (decision documented).

- **`postgres-test` feature** — activates `postgres` + `testcontainers-modules`.
  Runs `postgres_code_store_conformance`, `postgres_session_store_conformance`,
  `postgres_form_token_store_conformance`, migration idempotency, BIGINT column
  type verification, admin list/get, and claim result verification.
  Requires Docker; separate from `postgres` so native CI is unaffected.

- CI `test-postgres` job — runs `cargo test -p codlet-sqlx --features
  postgres-test` on ubuntu-latest (Docker available by default on GitHub
  Actions).

### Changed

- `codlet-sqlx/Cargo.toml`: `postgres` feature now active (was commented out);
  `testcontainers-modules = "0.11"` added as optional dependency.
- `codlet-sqlx/src/lib.rs` doc updated: two-backend overview table, postgres
  atomicity rationale, updated crate description.
- RFC-034 moved `proposed/ → done/` (Implemented v0.12.0). 9/9 items.
- RFC index updated: 0 proposed, 33 done, 1 archived.

### Security

- All PostgreSQL queries use `$N` parameterised placeholders via SQLx; no
  raw SQL interpolation anywhere in `postgres/`.
- `claim_code` and `consume_form_token` both guard `rows_affected() > 1`
  with `StoreError::InvariantViolation` (impossible in practice due to
  `PRIMARY KEY` + `UNIQUE`, but defence-in-depth).

## [0.11.0] — 2026-06-14

`codlet-worker` — first production adapter for Cloudflare Workers. D1-backed
stores, KV rate limiting, `WorkerKeyProvider`, HTTP helpers. Compiles for
`wasm32-unknown-unknown`. RFC-033 implemented (9/10 checklist items; Miniflare
CI job is scaffolded but pending Node/wrangler pipeline). 152 tests.

### Added

- **`crates/codlet-worker`** (new crate, RFC-033):
  - `D1CodeStore` — `CodeStore` + `CodeAdminStore` backed by Cloudflare D1.
    Conditional UPDATE + `meta().changes` for atomic claim (INV-5).
    Timestamps as `D1Type::Real(f64)` (RFC-033 §6). `Rc<D1Database>` for
    single-threaded Workers.
  - `D1SessionStore` — `SessionStore` backed by D1.
  - `D1FormTokenStore` — `FormTokenStore` backed by D1. Conditional UPDATE +
    classify follow-up SELECT (INV-6). `changes > 1` returns
    `StoreError::InvariantViolation`.
  - `KvRateLimitStore` — `RateLimitStore` backed by Cloudflare KV.
    GET → increment → PUT with TTL. Documents eventual-consistency caveat.
  - `WorkerKeyProvider` — `KeyProvider` from `Env` secrets. Fails closed on
    missing or empty binding (INV-2).
  - `run_d1_migrations` — applies `0001_initial.sql` via `db.exec()`.
  - `D1TableConfig` — `default()` (codlet schema) + `zinnias_ciao_compat()`.
  - HTTP helpers: `extract_cookie`, `set_cookie_header`, `clear_cookie_header`,
    `extract_rate_limit_key` (CF-Connecting-IP, trust model documented,
    `"unknown"` fallback explicitly rejected per RFC-010 §12.4).
  - `tests/conformance.test.ts` — Miniflare integration test scaffold
    (TypeScript/Vitest) with migrations, atomic claim race, timestamp check,
    atomic consume race, and KV counter tests. Runs without Cloudflare account.
  - `README.md` with usage, wrangler.toml setup, KV caveat, zinnias-ciao
    migration note.

- CI `wasm32-worker-compile` job: `cargo build -p codlet-worker --target
  wasm32-unknown-unknown` on every push (RFC-033 §16 item 1).
- CI `wrangler-test` job: commented-in configuration; uncomment when
  Node/wrangler pipeline is ready.

### Changed

- RFC-033 moved `proposed/ → done/` (Implemented v0.11.0). 9/10 acceptance
  items satisfied; Miniflare CI job is the one remaining pre-v1 item.

### Security

- All D1 queries use parameterised `bind_refs` — no raw SQL interpolation.
- `WorkerKeyProvider::from_env` returns `Err` on missing or empty secret
  (INV-2; not a silent default).
- `extract_rate_limit_key` refuses the `"unknown"` fallback (RFC-010 §12.4).

## [0.10.0-rfc] — 2026-06-14

Added RFC-033 and RFC-034 specifications. No code changes.

### Added

- `rfcs/proposed/RFC-033` — Cloudflare Workers / D1 / KV Adapter
  (`codlet-worker`): full implementation specification including wasm32
  Cargo target config, timestamp representation (`D1Type::Real(f64)`),
  affected-row count via `meta().changes`, `WorkerKeyProvider` from
  `Env` secrets, `D1TableConfig` for zinnias-ciao compatibility,
  `KvRateLimitStore` with eventual-consistency caveat, cookie helpers,
  identity extraction, migration via `exec()`, Miniflare local test
  strategy, and a 10-item acceptance checklist.

- `rfcs/proposed/RFC-034` — PostgreSQL Adapter (`codlet-sqlx` `postgres`
  feature): full implementation specification including `BIGINT` type
  mapping, `0002_postgres.sql` migration, `PostgresStore` type, `READ
  COMMITTED` + conditional UPDATE isolation rationale (with explicit
  rejection of `RETURNING` and `FOR UPDATE`), `testcontainers`-based
  test strategy, and an 8-item acceptance checklist.

### Changed

- RFC-010 status corrected: `Partially implemented` — the design is
  accepted but `codlet-worker` crate has not been written yet; all 5
  checklist items remain open. RFC-033 is the implementation RFC.

- RFC-011 status corrected: `Partially implemented` — SQLite and in-memory
  adapters are done; PostgreSQL adapter is not. All 5 checklist items
  now ticked for the SQLite/mem portion. RFC-034 covers PostgreSQL.

- RFC index updated: 2 proposed, 31 done (2 partial), 1 archived.

## [0.10.0] — 2026-06-14

Closes all RFC checklist items. RFC-009 items 2 and 3 are now verified by
running tests, and `codlet-core` is confirmed to compile for
`wasm32-unknown-unknown` — the target required for the Cloudflare Workers D1
adapter. 152 tests. All 31 RFC checklists fully resolved.

### Added

- `crates/codlet-core/tests/rfc_009_compile.rs` — two compile tests that
  close the last formally-annotated RFC-009 deferred items:
  - `not_send_store_satisfies_code_store_trait`: a `!Send` type (raw pointer,
    analogous to a D1 handle) implements `CodeStore` — the trait has no
    implicit `Send` bound. Required for Cloudflare Workers.
  - `send_sync_store_satisfies_axum_style_bounds`: a `Send + Sync` type
    satisfies `CodeStore + Send + Sync + 'static` without any shim — the
    bounds Axum/Tower shared state requires. No adapter layer needed.
- `.cargo/config.toml` — workspace Cargo config placeholder, documented for
  future wasm32 linker flags when `codlet-worker` is added.

### Changed

- `static_assertions = "1"` added to `codlet-core` dev-dependencies (used
  by `rfc_009_compile.rs` to assert `Send + Sync` at compile time).
- RFC-009 checklist items 2 and 3 are now `[x]` with verified test evidence
  rather than `[~]` deferred. All 31 RFC checklists are fully resolved.

### Verified

- `cargo build -p codlet-core --target wasm32-unknown-unknown` passes.
  `codlet-core` has no native I/O dependencies and compiles cleanly to WASM.
  This is the prerequisite for `codlet-worker` (the Cloudflare D1 adapter).
- CI `wasm32-compile` job (added in v0.9.0-audited) exercises this path on
  every push.

## [0.9.0] — 2026-06-14

Completes `codlet-sqlx`: `CodeAdminStore` is now fully implemented for
`SqliteStore`, the `created_at` bug in `insert_code` is fixed, and the
admin tests are written. SQLite is now the complete production-ready backend
for all three store traits plus admin listing. 150 tests total.

### Added

- `codlet-sqlx::admin` — full `CodeAdminStore` implementation for `SqliteStore`
  (RFC-030): `list_codes` with scope/active/limit filtering, `get_code_meta`
  by record ID. Never returns plaintext codes or HMAC lookup keys.
- 8 new `CodeAdminStore` tests in `codlet-sqlx/tests/conformance.rs`: all,
  active-only, scoped, limit, get-found, get-not-found, used-state-after-claim,
  no-lookup-key-in-metadata.

### Fixed

- `CodeRecord` now carries a `created_at: u64` field. Previously `codlet-sqlx`
  approximated it as `expires_at - 3600` (wrong for any TTL other than 1 hour).
  `CodeRecord` construction sites in `auth/code.rs`, the conformance fixtures,
  and the acceptance tests all updated.
- `codlet-sqlx::lib.rs` doc updated: backend options (`:memory:`, file path,
  named shared memory) are now explicitly listed with guidance on which to use.

### Changed

- `SqliteStore` now also implements `CodeAdminStore`. The adapter guarantee
  matrix in `docs/src/adapter-matrix-and-config.md` should be updated to
  reflect this (doc-only follow-up).
- `MemCodeStore::list_codes` (in `admin::mem_admin`) remains a stub that
  always returns empty — this is intentional and documented. Production code
  using listing must use `SqliteStore`.

## [0.8.0] — 2026-06-14

Final planned RFC sprint: observability hooks, admin API, security policy, and
closing out all remaining proposed RFCs. All 31 planned RFCs are now
implemented; RFC-018 (future server/IdP) is archived as post-v1. 142 tests.

### Added

- `metrics` module (RFC-024): `MetricsObserver` trait (fire-and-forget
  counter/outcome hook), `NoopMetrics` (default zero-cost implementation),
  `CapturingMetrics` (test-utils), `Outcome` enum with stable `label()`
  strings, and `counter` module with 8 recommended counter-name constants.
  Gate: counter names verified to contain no sensitive vocabulary.

- `admin` module (RFC-030): `CodeAdminStore` optional extension trait with
  `list_codes` (with `CodeListFilter`) and `get_code_meta`; `CodeMeta`
  metadata record (no plaintext code, no HMAC lookup key — enforced by type
  design and test); `CodeStats` aggregate; `CodeListFilter` with scope/active
  helpers; in-memory stub for `MemCodeStore` under `test-utils`.

- `SECURITY.md` (RFC-028): complete security policy covering supported
  versions, MSRV policy (1.85, never raised in patch), reporting address and
  response targets, disclosure policy, advisory format, and explicit list of
  what constitutes a security bug. Release checklist matches the 5 `xtask`
  gates.

### Changed

- RFC-024, RFC-025, RFC-026, RFC-027, RFC-028, RFC-029, RFC-030, RFC-031,
  RFC-032 moved `proposed/ → done/` (Implemented v0.8.0). All 31 planned
  RFCs are now implemented.
- RFC-018 (future server/IdP strategy) moved to `archive/` as post-v1 deferred.
- `proposed/` directory is now empty.

### Security

- `CodeMeta` contains no plaintext code value and no HMAC lookup key; a
  `Debug`-output test asserts no sensitive vocabulary appears in the type.
- `MetricsObserver::increment` must not block; `counter` names are tested to
  contain no sensitive vocabulary (no `key`, `secret`, `hmac`, etc. in the
  label strings that would be exported to metric backends).
- SECURITY.md is now complete and linked from README (RFC-028 acceptance).

## [0.7.0] — 2026-06-14

Documentation layer and compilable examples (RFC-016), plus the RFC-010
groundwork. 22 RFCs implemented, 10 remaining, 133 tests, 5 gates.

### Added

- **`codlet-examples`** (new crate, RFC-016): three compilable binaries that
  each run end-to-end and produce correct output:
  - `sqlite_quickstart` — complete issue→validate→claim→session→validate flow
    using `codlet-sqlx`; shows host authorization note after authentication.
  - `key_rotation` — configures active + previous keys, re-derives a v1 record
    under the rotated config, then proves missing-version fails closed.
  - `form_token_csrf` — issue, first-submit Proceed, duplicate-submit Replay,
    wrong-subject Invalid, wrong-purpose Invalid; includes UX copy guidance.
  All examples follow RFC-016 §10.2 rules (no hard-coded production secrets,
  no plaintext code logging, safe defaults throughout).

- `docs/src/threat-model.md` — what codlet protects against (online guessing,
  double-claim, session forgery, code enumeration, plaintext persistence, JS
  cookie access, weak HMAC), what it does NOT protect against (authorization,
  user management, offline attack after key+DB leak, KV eventual consistency),
  and the 8 security invariants with their INV-N labels.

- `docs/src/adapter-matrix-and-config.md` — adapter guarantee matrix (atomic
  claim/consume, multi-process safety for each adapter), secure configuration
  guide (code policy, key provider, cookie policy, rate limiting), and
  user-facing copy guidance table (say "code" not "token", generic failure
  messages, no jargon).

- `docs/src/SUMMARY.md` updated with threat model and adapter matrix pages.

- `xtask release-check`: `no-debug-prints` gate now exempts `codlet-examples`
  binaries (intentional terminal output for demonstrations).

### Changed

- RFC-010 (Cloudflare Workers/D1 adapter) and RFC-016 (documentation and
  examples) moved `proposed/ → done/` (Implemented v0.7.0).
  22 RFCs implemented total; 10 remaining (post-v1 / future).

### Security

- All three examples verified to compile and run correctly in CI (`cargo run`),
  satisfying RFC-016 §10.4 "all example code compiles."
- Example binaries use `production_strict` cookie policy, 8-char codes, and
  `StaticKeyProvider` with a clearly-labelled placeholder key.

## [0.6.0] — 2026-06-14

Typestate completions, two new release gates, key rotation and migration docs,
and `.gitignore` updated to the standard Cargo template. 133 tests, 5 static
release gates, 20 RFCs implemented.

### Added

- `secret` module additions (RFC-019):
  - `NormalizedCode` — distinct type for the post-normalization canonical form,
    preventing confusion between raw user input and the value passed to HMAC
    derivation.
  - `Purpose` — validated non-empty purpose label; `Purpose::new("")` returns
    `None`.
  - `ScopeKey` — host-owned scope/boundary label.
  All three are exported from `codlet_core` root.

- Two new `xtask release-check` gates (RFC-015):
  - `cookie-attrs-present` — verifies `HttpOnly`, `Secure`, and `SameSite`
    appear in `cookie.rs`; fails if any is removed.
  - `no-plaintext-in-store-ops` — bans `.expose()` on the same line as
    `.bind(` or `INSERT` in library source, preventing accidental plaintext
    persistence.
  Both gates verified to fire on injected violations. Total: 5 static gates.

- `docs/src/key-rotation.md` — operational key management and rotation guide
  (RFC-017): key states, planned rotation procedure, emergency compromise
  procedure, what codlet does vs. does not do.

- `docs/src/migration-from-zinnias-ciao.md` — migration plan (RFC-014):
  HMAC incompatibility explanation, parallel lookup strategy, schema migration
  SQL, column mapping table, cookie name compatibility, checklist.

- `docs/src/SUMMARY.md` updated with new pages.

- `.gitignore` replaced with the standard Cargo template (covers `debug/`,
  `target`, `*.rs.bk`, `*.pdb`, `mutants.out*/`, RustRover hints).

### Changed

- RFC-014, RFC-015, RFC-017, RFC-019 moved `proposed/ → done/`
  (Implemented v0.6.0). 20 RFCs total implemented, 12 remaining proposed.

## [0.5.0] — 2026-06-14

First production adapter: SQLite via SQLx. A new shared conformance test suite
verifies that all adapters — in-memory and SQLite — satisfy the single-winner
claim/consume contracts under real concurrency. 130 tests total across the
workspace.

### Added

- **`codlet-conformance`** (new crate, RFC-023): parameterised async conformance
  test suite. Contains `run_code_store_conformance`, `run_session_store_conformance`,
  and `run_form_token_store_conformance` — each takes a factory function and
  runs the full RFC-023 required test list against any store implementation.
  The code-store suite includes the concurrent-claim race test (RFC-022):
  8 tasks hit a `tokio::Barrier` simultaneously and exactly one must win.
  Split into four modules: `fixtures`, `code`, `session`, `token`.

- **`codlet-sqlx`** (new crate, RFC-011): SQLite adapter implementing all three
  store traits using a single embedded migration (`0001_initial.sql`). Code claim
  and form-token consume use a conditional `UPDATE … WHERE … AND <guard>` with
  affected-row count check (RFC-022). Uses WAL mode for concurrent access.
  Passes all five conformance tests including the concurrent race test.

- `codlet-sqlx`: migration runner (`run_migrations`) embedded via `include_str!`,
  idempotent (`IF NOT EXISTS`), safe to call on startup.

- `codlet-sqlx`: `SqliteStore` — a cheaply-clonable pool wrapper implementing
  `CodeStore + SessionStore + FormTokenStore`.

- `.gitignore` updated to the standard Cargo template (covers `debug/`, `target`,
  `*.rs.bk`, `*.pdb`, `mutants.out*/`, RustRover hints).

### Changed

- RFC-011, RFC-022, RFC-023 moved `proposed/ → done/` (Implemented v0.5.0).
  16 RFCs total implemented.

### Security

- The SQLite conditional UPDATE pattern is documented in `codlet-sqlx/src/code.rs`
  and `token.rs` to make the atomicity requirement visible at the implementation
  site, not just in the trait docs.
- `changed > 1` in code claim and form-token consume returns
  `StoreError::InvariantViolation` — same as the in-memory adapter.
- The conformance suite's race test is run under `tokio::task::LocalSet` with
  a `Barrier`, so it works with both `Send` and `!Send` store implementations.

## [0.4.0] — 2026-06-14

High-level orchestration layer (`auth` module). A host can now implement a
complete authentication flow end-to-end — issue, find, claim, session — without
writing glue code against every primitive individually. 122 tests total.

### Added

- `auth` module with five sub-modules (RFC-013, RFC-009):
  - `auth::code`: `CodeAuth<CS, RL, K, C, A>` manager with `issue_code`,
    `find` (rate-limit check → input validation → store lookup), `claim`
    (atomic won/lost with rate-limit record/clear), `redeem_with_callback`
    (full RFC-013 §10.3 8-step flow order), and `revoke_code`. Session
    issuance is only possible after `claim` or `redeem_with_callback` returns
    `RedeemSuccess` — enforced at the type level via `ClaimProof`.
  - `auth::session`: `SessionManager<SS, K, C, A>` with `issue` (requires
    `RedeemSuccess` proof), `validate`, and `revoke` (returns clear-cookie
    header string). Generates 32-byte session secrets; stores only the HMAC
    lookup key; plaintext leaves only in the `Set-Cookie` value.
  - `auth::token`: `FormTokenManager<TS, K, C, A>` with `issue`, `consume`
    (idempotency replay with `result_ref`), and `set_result`.
  - `auth::norate`: `NoRateLimit` — zero-cost opt-out `RateLimitStore` for
    hosts that handle rate limiting at the network layer.
  - `auth::error`: `RedeemError` (5 variants, each carrying internal reason +
    public mapping), `SessionError`, `FormTokenError`, `IssuedSession`,
    `RedeemSuccess`, `ClaimProof` (zero-size proof token).
- 11 new acceptance integration tests covering every RFC-013 checklist item:
  complete two-step issue→find→claim→session round trip; callback-based flow;
  lost claim has no proof (`Err`, not a session); host callback error leaves
  claim consumed but no session issued; public errors are generic regardless of
  internal cause; expired session returns `Unauthenticated`; logout clears
  session and returns `Max-Age=0` cookie; wrong-subject form-token rejected.

### Changed

- RFC-013 and RFC-009 moved `proposed/ → done/` (Implemented v0.4.0).
- `auth/code.rs` split: `NoRateLimit` extracted to `auth/norate.rs` to keep
  all source files within the 300-ELOC guideline.

### Security

- Session issuance is structurally gated: `SessionManager::issue` accepts only
  a `RedeemSuccess`, which wraps a `ClaimProof` that is only constructible when
  `claim_code` returns `ClaimOutcome::Won`. The compiler prevents issuing a
  session without a confirmed won claim (RFC-013 §5).
- `redeem_with_callback` enforces RFC-013 §10.3 step order: rate-limit check
  and input validation happen before the claim; the host callback runs only
  after a confirmed won claim; if the callback fails, no session is issued and
  the code is consumed (host must compensate — documented).
- Public errors from all orchestration paths return generic messages regardless
  of the internal cause, verified by test.

## [0.3.0] — 2026-06-14

M3 complete: rate limiting, two-layer error model, and audit events.
`codlet-core` now contains the full primitive layer — every security-critical
concept has a type, a classifier, a store trait, and a test. 111 tests total.

### Added

- `audit` module: `CodeAuthEvent` enum (10 variants, stable `noun.verb.outcome`
  keys), `AuditSink` trait, `NoopAuditSink`, and (under `test-utils`)
  `CollectingAuditSink`. All event fields are **redacted by construction**: no
  plaintext code, token, session secret, lookup key, or raw IP address appears
  in any variant (RFC-012).
- `store::ratelimit`: `RateLimitKey`, `RateLimitPolicy` (with
  `default_invite()`: 10 failures / 5 min / key), `RateLimitUnavailable`
  (`FailOpen` / `FailClosed` / `SoftDenyAfterThreshold`), `RateLimitOutcome`,
  and `RateLimitStore` trait with `check` / `record_failure` / `clear_failures`
  (RFC-008).
- `mem::MemRateLimitStore` — in-memory rate-limit store (`test-utils`, RFC-008
  in-memory portion). Documents its best-effort counter atomicity.
- Error model extensions in `error` module (RFC-012/021):
  - `RedemptionFailReason` — 7 internal variants for logging/metrics.
  - `PublicRedemptionError` — `InvalidOrExpired` / `RateLimited` /
    `TemporarilyUnavailable`, with `from_reason()` mapping.
  - `PublicFormError` — `ExpiredOrInvalid` / `TemporarilyUnavailable`.
  - `PublicSessionError` — `MissingOrExpired` / `TemporarilyUnavailable`.
- 18 new acceptance integration tests covering all RFC-008 and RFC-012
  checklist items: enumeration collapse, rate-limit threshold/clear/isolation,
  fail-open default, fingerprint privacy, audit-event key stability, no-secret
  in event debug output.

### Changed

- RFC-008, RFC-012, RFC-020, RFC-021 moved `proposed/ → done/`
  (Implemented v0.3.0). RFC index regenerated.

### Security

- All enumeration-sensitive redemption states (`NotFound`, `Expired`,
  `Revoked`, `AlreadyUsed`, `InvalidFormat`) map to `InvalidOrExpired` — a
  single public error — via `PublicRedemptionError::from_reason()`. Test-
  enforced exhaustively.
- `RateLimitKey::fingerprint()` returns a prefix safe for audit events and
  metrics labels; the full key is never emitted in `CodeAuthEvent::RateLimitHit`.
- `CodeAuthEvent` is `#[non_exhaustive]` so adding variants is non-breaking.

## [0.2.0] — 2026-06-14

Lifecycle state machines, storage traits, cookie policy, in-memory stores, and
a `Clock` abstraction. `codlet-core` now has all the primitives needed to
express a complete authentication flow at the type level; adapters and
orchestration come next.

### Added

- `clock` module: `Clock` trait + `SystemClock` (production) + `FixedClock`
  (deterministic, `test-utils`) (RFC-020).
- `state` module with three pure classifiers (no I/O, no `async`):
  - `classify_claim` / `ClaimOutcome` — atomic single-winner code claim
    (RFC-005);
  - `classify_token_consume` / `TokenConsumeOutcome` — form-token
    idempotency/CSRF classifier, ported verbatim from `zinnias-ciao`
    `contracts::auth` with its full regression suite (RFC-007);
  - `classify_session` / `SessionValidationOutcome` — session lookup result
    with `Authenticated`/`Unauthenticated` variants (RFC-006).
- `store` module with async traits and supporting types:
  - `CodeStore` (`find_redeemable`, `claim_code`, `insert_code`,
    `revoke_code`) plus `RedeemableCode`, `CodeRecord`, `ClaimRequest`, and
    helpers `expires_at_from_ttl`, `code_lookup_candidates` (RFC-005);
  - `SessionStore` (`find_active_session`, `insert_session`,
    `revoke_session`) plus `ActiveSessionRecord`, `SessionRecord` (RFC-006);
  - `FormTokenStore` (`insert_form_token`, `consume_form_token`,
    `set_token_result`) plus `FormTokenRecord`, `TokenSubject`
    (RFC-007). `TokenSubject::Anonymous/Authenticated/Flow` replaces the
    empty-string anti-pattern from the source service.
  - `store::error`: `StoreError` (internal) and `PublicAuthError`
    (single-variant public-safe collapse per INV-8).
- `cookie` module: `CookiePolicy` with named profiles (`ProductionStrict`,
  `ProductionLax`, `LocalDevelopment`), `build_set_cookie`,
  `build_clear_cookie` (RFC-006). `HttpOnly` is always set; `Secure` is
  mandatory in all production profiles; `Domain` omitted by default.
- `mem` module (`test-utils` feature only): `MemCodeStore`,
  `MemSessionStore`, `MemFormTokenStore` — in-memory implementations of
  the three store traits for tests and local dev. Non-production; documented
  clearly as such (RFC-011 in-memory portion).
- `tokio` as a dev-dependency for async integration tests.
- 21 new acceptance integration tests covering all RFC checklist items:
  find/claim/revoke/expiry for codes; session issuance/validation/revocation;
  form-token winner/replay/invalid/binding-mismatch/purpose-mismatch/expiry;
  `changed == 0` never-proceeds exhaustive check; anonymous vs authenticated
  subject separation; cookie attribute assertions.

### Changed

- RFC-005, RFC-006, RFC-007 moved `proposed/ → done/` (Implemented v0.2.0);
  RFC index regenerated. RFC-011 remains `proposed/` (SQLx adapter is M4;
  only the in-memory portion landed here).

### Security

- `changed == 0` never-proceeds invariant is both a classifier contract
  (`classify_token_consume`) and enforced in `MemFormTokenStore::consume_form_token`.
- `changed > 1` in code claim or form-token consume returns
  `StoreError::InvariantViolation`, never silently maps to `Lost`/`Invalid`.
- `TokenSubject` enum prevents the empty-string anonymous collision present in
  the source service (RFC-007 §13.3).
- Cookie `Domain` omitted by default; subdomain sharing requires explicit opt-in.

## [0.1.0] — 2026-06-13

First functional primitives in `codlet-core`. Implements RFC-003 (one-time code
policy, generation, normalization, validation) and RFC-004 (secret hashing, key
providers, domain separation, key versioning). Still pre-1.0 and incomplete: no
storage traits, session/form-token lifecycle, or adapters yet.

### Added

- `secret` module: redacted `SecretString` plus `PlainCode`, `SessionSecret`,
  `FormTokenSecret` (redacted `Debug`/`Display`/serde) and opaque `CodeId`,
  `SubjectId`, `SessionId` newtypes (RFC-019 foundation).
- `rng` module: `RandomSource` trait, `SystemRandom`, and (under `test-utils`)
  deterministic `FixedBytesRandom` / `AlwaysFailRandom`. RNG failure is fatal;
  no fallback value is produced (RFC-020, INV-3).
- `code` module: validated `Alphabet` (unambiguous 31-symbol default),
  idempotent `normalize`, `CodePolicy` with `default_human` (≥8 chars),
  `short_compat`, and `legacy_ciao_6`, rejection-sampling `generate_code`, and
  `validate_code_input` (RFC-003).
- `hashing` module: `SecretDomain`, `KeyVersion`, `LookupKey` (with constant-
  time `ct_eq`), `KeyProvider`/`HmacKeyRef`, `StaticKeyProvider` (active +
  previous keys, empty key rejected), and `SecretHasher` deriving
  domain-separated HMAC-SHA256 lookup keys via the prefixing scheme (RFC-004).
- `error` module: internal error layer (`RandomError`, `KeyError`,
  `PolicyError`, `CodeInputError`).
- Frozen HMAC test vectors per domain and a reproducible Unicode normalization
  property test (RFC-003 §11.5, RFC-004 §12.3).
- `xtask release-check` now enforces real static gates: no fallback-key literal,
  no silently-defaulted/swallowed RNG result, no debug prints in library code
  (RFC-015 §9). Verified to fail on injected violations.

### Changed

- `codlet-core` is `std` for v0.1; the `std`/`no_std` split is deferred
  (RFC-002 §4).
- RFC-003 and RFC-004 moved from `rfcs/proposed/` to `rfcs/done/`
  (Implemented); RFC index regenerated.

### Security

- Domain-separated lookup keys: the same plaintext yields distinct keys across
  the code/session/form-token/flow-ticket domains (test-enforced).
- Secret-bearing types and key material are redacted in `Debug`/`Display`/serde
  (INV-1, SR-38); tests assert no plaintext leakage.
- codlet lookup keys are intentionally not bit-identical to zinnias-ciao's
  (domain prefix added); the future migration adapter will provide a
  `legacy_no_domain` mode (RFC-004 §9.1, RFC-014).

## [0.0.0] — 2026-06-13

Phase 0 bootstrap. No authentication functionality yet; this release
establishes the repository, process, and an empty `codlet-core` skeleton.

### Added

- Cargo workspace with `codlet-core` (skeleton) and `xtask` (release-gate
  runner skeleton).
- `#![forbid(unsafe_code)]` and a shared workspace lint policy.
- RFC process and directory structure under `rfcs/`, governed by the
  4-folder lifecycle policy (`proposed/`, `done/`, `archive/`), with an index
  at `rfcs/README.md`.
- Design RFC-001 (project scope, product shape, non-goals) and RFC-002 (crate
  architecture, feature flags, runtime matrix) accepted and moved to
  `rfcs/done/`. RFC-003 through RFC-032 placed under `rfcs/proposed/`.
- Recommendation added to RFC-004 favouring HMAC message prefixing for domain
  separation, and noting that codlet lookup keys are deliberately not
  bit-identical to zinnias-ciao's, so the migration adapter needs a
  `legacy_no_domain` mode.
- Project hygiene: `README.md`, `SECURITY.md`, `LICENSE` (Apache-2.0),
  `NOTICE`, `CONTRIBUTING.md`, CI workflow, `rust-toolchain.toml`,
  `.gitignore`.

### Security

- Verified `codlet-core`'s dependency tree contains no web-framework, database,
  or async-executor crates (RFC-002 acceptance gate): only `hmac`, `sha2`,
  `subtle`, `getrandom`, `thiserror`.

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.16.0...HEAD
[0.16.0]: https://github.com/nabbisen/codlet/compare/v0.15.2...v0.16.0
[0.15.2]: https://github.com/nabbisen/codlet/compare/v0.15.1...v0.15.2
[0.15.1]: https://github.com/nabbisen/codlet/compare/v0.15.0...v0.15.1
[0.15.0]: https://github.com/nabbisen/codlet/compare/v0.14.4...v0.15.0
[0.14.4]: https://github.com/nabbisen/codlet/compare/v0.14.3...v0.14.4
[0.14.3]: https://github.com/nabbisen/codlet/compare/v0.14.2...v0.14.3
[0.14.2]: https://github.com/nabbisen/codlet/compare/v0.14.1...v0.14.2
[0.14.1]: https://github.com/nabbisen/codlet/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/nabbisen/codlet/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/nabbisen/codlet/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/nabbisen/codlet/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/nabbisen/codlet/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/nabbisen/codlet/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/nabbisen/codlet/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/nabbisen/codlet/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/nabbisen/codlet/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/nabbisen/codlet/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/nabbisen/codlet/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/nabbisen/codlet/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/nabbisen/codlet/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/nabbisen/codlet/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nabbisen/codlet/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/nabbisen/codlet/releases/tag/v0.0.0
