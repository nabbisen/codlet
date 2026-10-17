# Developer Handoff — RFC-054: The D1 Adapter Must Execute In a Test

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **RFC:** [`../../accepted/054-the-d1-adapter-must-execute-in-a-test.md`](../../accepted/054-the-d1-adapter-must-execute-in-a-test.md)
- **Baseline:** `5c1c2f9`
- **This is the largest piece of work outstanding, and it is staged.** Stage A is small and independently valuable. Stop after any stage and report; a partial delivery that says where it stopped is a good outcome here.

## 1. Purpose

Make the D1 stores execute in a test. Today nothing runs them: the Miniflare
suite runs a JavaScript reimplementation of their SQL, and the Rust code is only
type-checked by a `wasm32` build.

You found this and stated it precisely. Everything below is your finding turned
into work.

## 2. Stage A — the harness must run the shipped migration

Do this first, commit it alone, and treat it as deliverable on its own.

`crates/codlet-worker/tests/worker.js` defines its own `MIGRATION_SQL`. It
does not match what codlet ships:

| | timestamp columns |
|---|---|
| `crates/codlet-worker/migrations/0001_initial.sql` | **11 × `INTEGER`** |
| `worker.js`'s `MIGRATION_SQL` | **11 × `REAL`** |

Zero overlap. Delete the inline copy and have the harness execute the shipped
file. Wrangler/vitest can read it from disk at setup; if the Workers runtime
makes that awkward, import it as a build-time asset rather than retyping it —
**do not** copy its text into `worker.js`, because the copy is the defect.

Then make recurrence impossible or detectable: if the duplication is genuinely
gone, criterion 2 is satisfied structurally; if any copy must remain, add a
check that fails when the two diverge.

Expect tests to fail or change behaviour when the schema changes. SQLite's type
affinity is forgiving, so they may not — but if any test's result shifts, that
shift is a real finding about the shipped schema and I want it reported, not
smoothed over.

## 3. Stage B — let the suite accept a `!Send` store

`codlet-conformance`'s entry points require:

```rust
S: CodeStore + Send + Sync + 'static,     // src/code.rs:18
```

`D1CodeStore` holds `Rc<worker::d1::D1Database>` (`d1/code.rs:19`), so it is
`!Send + !Sync`. That bound is what excludes D1, and for the sequential tests it
buys nothing — they `await` in order and never cross a thread.

Drop `Send + Sync + 'static` from the sequential tests and their entry points.

**`test_exactly_one_claim_winner` is the exception** (`src/code.rs:201`). It
uses `tokio`'s `LocalSet` and `Barrier`, which do not exist in a Worker. Carve
it into its own entry point that keeps whatever bounds it needs.

**The real obstacle is the manifest, not the bounds.** `codlet-conformance`
depends on `tokio` unconditionally with `features = ["rt", "macros", "sync",
"time"]`, and those do not build for `wasm32-unknown-unknown`. So the
concurrency test and its `tokio` dependency must become **optional**, behind a
feature the wasm consumer does not enable. `futures` 0.3 should be fine on
wasm; confirm rather than assume.

Verify the split by compiling `codlet-conformance` for `wasm32-unknown-unknown`
with the concurrency feature off. If it does not compile, the carve-out is
incomplete — and tell me if the dependency graph makes this impractical rather
than forcing it.

## 4. Stage C — run the real stores in Miniflare

Add a **test-only crate** — not `codlet-worker` — with a `#[event(fetch)]`
handler that builds the real `D1CodeStore`, `D1SessionStore` and
`D1FormTokenStore` from the Miniflare D1 binding and runs Stage B's sequential
suite. Point the existing vitest job at it; `wrangler.toml`'s `main` becomes
the built worker.

Keeping a fetch handler out of the published crate was the original correct
instinct — the error was concluding that a JS reimplementation was therefore
the right architecture. A separate test crate gets both properties.

Report results so a failure names the test. A single pass/fail for the whole
suite would make this stage much less useful than it should be.

**If Stage C proves impractical** — the `worker` crate's test ergonomics, the
wasm build in CI, async plumbing — stop and report with what you tried. Stages A
and B stand on their own, and I would rather know the shape of the obstacle than
receive a workaround that re-creates a mirror.

## 5. Stage D — the harness's npm advisories

`npm install` in `crates/codlet-worker/tests` reports 19 advisories (3
critical). Run `npm audit fix`, record what remains and why, and state in the
directory that this tree is test-only and unreachable by consumers — the same
treatment RFC-052 gave the Rust side.

Do not `npm audit fix --force` into a major-version bump that breaks the
harness. If the remainder needs one, report it instead.

## 6. Non-change scope

- No fetch handler in `codlet-worker`. It stays a library.
- No change to the D1 adapters' SQL, behaviour, or the `Rc` handle. `!Send` is
  correct for Workers.
- Do not delete the JS harness. It still covers D1 binding behaviour and the
  concurrency route; the goal is that it stops being the only evidence.
- `test_exactly_one_claim_winner` must not be weakened for the in-memory,
  SQLite or PostgreSQL adapters.
- No attempt to run `codlet-sqlx` under Miniflare.

## 7. Required verification

| Check | Expectation |
|---|---|
| `cargo test --workspace` | green, no count regression |
| `cargo test -p codlet-sqlx --no-default-features --features postgres-test` | green (CI if no Docker) |
| `cargo check -p codlet-worker --target wasm32-unknown-unknown` | clean |
| `cargo check -p codlet-conformance --target wasm32-unknown-unknown --no-default-features` | clean after Stage B |
| `cd crates/codlet-worker/tests && npx vitest run` | green, against the shipped schema |
| `grep -c 'REAL' crates/codlet-worker/tests/worker.js` | 0 |
| `grep -c 'CREATE TABLE' crates/codlet-worker/tests/worker.js` | 0 after Stage A |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |

## 8. Acceptance criteria

Binding. Criteria 3–5 apply only to the stages you complete; say which.

1. The harness executes the shipped migration; no schema SQL duplicated in
   `worker.js`.
2. Divergence is impossible (duplication gone) or detected by a check.
3. The sequential conformance entry points compile for a `!Send + !Sync` store,
   and `codlet-conformance` compiles for `wasm32-unknown-unknown` without the
   concurrency feature.
4. A test-only wasm crate runs those tests against the real `D1*Store` types in
   Miniflare, passing in CI, with per-test reporting.
5. `test_exactly_one_claim_winner` still runs at full strength for the other
   three adapters.
6. `docs/src/adapter-matrix-and-config.md` states exactly what is proven for D1
   and by what — including that single-winner remains covered by the JS route if
   that is still the case. No implied parity.
7. npm findings fixed or recorded with a reason.
8. All blocking CI jobs green.

## 9. Required evidence

Per stage: `git diff --stat`; the §7 checks with exit status; for Stage A the
before/after of how the schema is obtained; for Stage C the vitest output
showing named D1 tests passing against real Rust; the npm audit summary; CI run
URL. Say plainly which stages you completed and which you did not.

## 10. Review request

`.git-exclude/review-request/054-d1-execution-evidence.md`; result returns at
`.git-exclude/reviewed/054-d1-execution-evidence.md`.
