# codlet-worker Miniflare tests

**Test-only, and unreachable by consumers.** Nothing in this directory is part of
any published crate (`codlet-worker` is `publish = false`, and `tests/` is never
compiled into a library). The npm tree here exists to run the tests; a host that
depends on codlet never receives it. The same reasoning RFC-052 applied to the
Rust dev-dependencies applies here, so `npm audit` findings are triaged for the
**harness**, not for users.

## What runs

`npx vitest run` runs two vitest projects (see `vitest.workspace.ts`):

| Project | Config | Runs |
|---|---|---|
| `js-harness` | `vitest.config.ts`, `wrangler.toml` → `worker.js` | D1 binding behaviour, KV, and the concurrent-claim route. `worker.js` re-implements the store SQL in JavaScript; the **schema** is the shipped `crates/codlet-worker/migrations/0001_initial.sql`, read from disk (RFC-054 stage A). |
| `rust-d1` | `vitest.rust.config.ts`, `wrangler.rust.toml` → `rust-build/worker/shim.mjs` | The **real Rust** `D1CodeStore` / `D1SessionStore` / `D1FormTokenStore`, driven one test at a time by `codlet-conformance` (RFC-054 stage C). |

The `rust-d1` Worker is built from the test-only crate
`crates/codlet-worker-miniflare` and must be built first (needs the
`wasm32-unknown-unknown` target and `worker-build`):

```sh
cargo install worker-build --version 0.8.4 --locked
npm run build:rust      # writes ./rust-build (git-ignored)
npx vitest run
```

(`npm test` does both steps.)

## What the migration tests pin (RFC-057)

- `rust_d1.test.ts` runs the shipped `run_d1_migrations` for real: it applies, it is
  idempotent, and it upgrades a pre-RFC-044 `codlet_sessions` (the whole session
  suite is also run against such an upgraded database).
- `conformance.test.ts` pins *why* the runner splits the file: D1's `exec`
  rejects the shipped multi-line file, and this test says so if D1 ever changes.
- The statement splitter lives in one place, `crates/codlet-worker/src/sql_statements.rs`,
  with native unit tests (including a `;` inside a `--` comment). Nothing else in the
  workspace splits SQL: the JS harness migrates by calling the shipped Rust runner.

## npm advisories

`npm audit fix` (without `--force`) was applied on 2026-10-18: 19 → 16 findings.
The 16 that remain (3 critical: `vitest`, `tinypool`, `@cloudflare/vitest-pool-workers`;
the rest in `wrangler`, `miniflare`, `sharp`, `undici`, `ws`, `vite`, `esbuild` and
friends) all sit under `@cloudflare/vitest-pool-workers` ^0.6 and `vitest` ^2, and
every fix is a major-version bump (`@cloudflare/vitest-pool-workers` 0.23, `vitest`
5). `--force` was not run: it would move the harness across several major versions
at once, which needs its own change and its own verification. They affect a local
test runner on a developer or CI machine, not anything a consumer installs.
