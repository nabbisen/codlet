# Implementation Handoff — RFC-038 Migration Runner Must Not Parse SQL

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M4 (`ROADMAP.md`, items M4-6)
- **Governing RFC:** [`../../accepted/038-migration-runner-must-not-parse-sql.md`](../../accepted/038-migration-runner-must-not-parse-sql.md)
- **Priority:** ahead of the RFC-035 directory migration (owner decision, 2026-09-03). RFC-036 is merged at `7142f72`; branch from there or later.

Read RFC-038 in full before starting. It is the authority; this document directs
execution only. If execution conflicts with the RFC, **stop and escalate** — a
handoff may not override an RFC decision.

## 1. Purpose

Repair `run_postgres_migrations`, which fails for every consumer of the
published `codlet-sqlx` `postgres` feature, by removing the hand-rolled SQL
splitter from both migration runners.

## 2. Background

`postgres.rs:69` splits the migration file on `';'` and only then filters `--`
comment lines. `migrations/0002_postgres.sql:11` contains a semicolon inside a
comment, so the split cuts the comment in half and the tail
(` SQLx maps i64 to BIGINT natively.`) is executed as a statement. PostgreSQL
returns `42601 syntax error at or near "SQLx"` at position 1. Every PostgreSQL
conformance test dies at `tests/conformance.rs:321`.

Broken since RFC-034 shipped at v0.12.0. `migration.rs:31` has the identical
logic and works only because `0001_initial.sql` happens to contain no semicolon
in a comment.

Read RFC-038 §2.1 for why the second-order impact matters more than the first:
the PostgreSQL conformance suite, including the INV-5 concurrent-claim test,
has never executed.

## 3. Change scope

- `crates/codlet-sqlx/src/postgres.rs` — `run_postgres_migrations`
- `crates/codlet-sqlx/src/migration.rs` — `run_migrations`
- `crates/codlet-sqlx/src/migration/tests.rs` — **new**, the regression test
- `docs/src/adapter-matrix-and-config.md` — the `PostgresStore` row
- `CHANGELOG.md` — `[Unreleased]` entry

## 4. Non-change scope — do not touch

- **`crates/codlet-sqlx/migrations/*.sql`.** In particular do **not** reword
  `0002_postgres.sql:11` to remove its semicolon. That comment is now a live
  fixture; editing it would hide the defect instead of fixing it (RFC-038 §3.4).
- **Any store implementation** — `code.rs`, `session.rs`, `token.rs`,
  `admin.rs`, `postgres/*.rs`. This handoff changes how migrations are
  *executed*, not what any adapter does.
- The emitted schema. Not one byte of DDL changes.
- `crates/codlet-worker/` — its D1 runner does not use this pattern.
- `rfcs/` — including RFC-038's own status. It stays in `accepted/`; all four M4
  RFCs move to `done/` during the RFC-035 migration, which runs after you.
- `.github/workflows/ci.yml` — RFC-036 is merged and correct; no CI change is
  needed for this work.

## 5. Required implementation

### 5.1 PostgreSQL runner

Replace the loop body of `run_postgres_migrations` (`postgres.rs:67`) with a
single call:

```rust
pub async fn run_postgres_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let migration_sql = include_str!("../migrations/0002_postgres.sql");
    // The server parses the script, including `--` comments. codlet does not
    // parse SQL (RFC-038 §3). PostgreSQL wraps a multi-statement script in an
    // implicit transaction, so a failure rolls the whole migration back.
    sqlx::raw_sql(sqlx::AssertSqlSafe(migration_sql))
        .execute(pool)
        .await?;
    Ok(())
}
```

Delete the splitter and the comment filter. Do not keep them behind a flag or a
comment. Update the doc comment, which currently says "Applies
`migrations/0002_postgres.sql` statement by statement" — that stops being true.

### 5.2 SQLite runner

Same change in `run_migrations` (`migration.rs:16`). **Keep the two `PRAGMA`
calls that precede the script** — they are connection settings, not part of the
migration.

RFC-038 §3.3 requires verification here, with a pre-authorized fallback:

1. Make the change, run the SQLite conformance suite
   (`cargo test -p codlet-sqlx --no-default-features --features sqlite`).
2. **If it passes**, you are done — report that `raw_sql` handles the SQLite
   path.
3. **If it fails** because the driver will not accept a multi-statement script,
   the fallback for **SQLite only** is to strip `--` comment lines from the
   whole file *before* splitting on `';'`, with a comment recording why that
   runner differs from the PostgreSQL one. Do not take this path without first
   showing `raw_sql` fails; include the failure output in your review request.

Report which path you took either way. Do not silently choose the fallback.

### 5.3 Testability seam for the regression test

`run_migrations` reads a fixed file via `include_str!`, so it cannot be handed a
test script. Introduce a crate-internal seam:

```rust
pub(crate) async fn apply_sqlite_script(
    pool: &SqlitePool,
    sql: &str,
) -> Result<(), sqlx::Error> { … }
```

`run_migrations` calls it with the `include_str!` contents after the PRAGMAs.
Keep it `pub(crate)` — this is a test seam, not new public API, and adding
public surface is outside this handoff's scope.

### 5.4 Regression test — mandatory

New file `crates/codlet-sqlx/src/migration/tests.rs`, per the project's Rust
convention (unit tests as a `<module>/tests.rs` sibling, never inline
`#[cfg(test)]` in the module). Use an in-memory SQLite pool
(`sqlite::memory:`) so the test needs no Docker; `tokio` is already a dev-dependency.

The test must apply a script that contains **a semicolon inside a `--` comment**,
and assert it succeeds and produced the expected object. Carry the fixture
inline in the test — it must not depend on `0002_postgres.sql` keeping its
current wording (RFC-038 §5.1). For example, a script whose comment reads
`-- note (see RFC-033); this clause must survive` above a `CREATE TABLE`.

**Prove the test catches the defect.** Stash your fix, run the new test against
the unmodified pre-fix runner, and confirm it **fails**. Restore the fix and
confirm it passes. Record both outputs. This is the same standard RFC-036 §3.4
set for gates: a test nobody has seen fail is not a verified test.

### 5.5 Documentation

**`docs/src/adapter-matrix-and-config.md`.** The `PostgresStore` row currently
asserts conformance on atomic claim and single-use consume. Until the
PostgreSQL suite actually passes, that is unsupported. Correct the row to state
the verified position — and state it accurately according to how §6 lands, not
optimistically in advance.

**`CHANGELOG.md`** `[Unreleased]`: record the defect, that it affected published
versions from v0.12.0 onward, that the runners no longer parse SQL, and that
PostgreSQL migrations are now atomic (a failure rolls back rather than leaving
partial schema).

## 6. Required tests

| Test | Command | Must |
|---|---|---|
| Regression (§5.4) | `cargo test -p codlet-sqlx --no-default-features --features sqlite` | pass after fix, **fail before** |
| SQLite conformance | same command | pass |
| PostgreSQL conformance | `cargo test -p codlet-sqlx --no-default-features --features postgres-test` | **see below** |
| Workspace | `cargo test --workspace` | pass |
| Lint/format | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` | pass |

**The PostgreSQL suite is the point of this work, and it needs Docker.** If your
sandbox has no Docker daemon, say so plainly and do not infer a result — the
last round's Docker hypothesis was wrong precisely because it was inferred. If
you cannot run it, state that the fix is unverified against a real PostgreSQL
and hand that verification back to me; I can read the CI log.

## 7. Escalation — expected, not exceptional

RFC-038 §7 is explicit: fixing the runner is a precondition for learning whether
the PostgreSQL adapter is correct, **not evidence that it is**. When the suite
runs for the first time it may surface further defects, including in the INV-5
concurrent-claim path.

If that happens: **stop and escalate.** Do not fix an adapter defect inside this
handoff. Report the failing test, the assertion, and your reading of it. A
second defect gets its own RFC.

## 8. Acceptance criteria

1. Neither `migration.rs` nor `postgres.rs` contains a SQL splitter or comment
   filter (or, for SQLite only, contains the §5.2 fallback with its recorded
   justification and evidence).
2. `git diff crates/codlet-sqlx/migrations/` is empty.
3. The §5.4 regression test exists, passes after the fix, and was demonstrated
   to fail before it — both outputs recorded.
4. SQLite conformance passes.
5. PostgreSQL conformance passes, **or** its non-execution is reported honestly
   with the reason.
6. `cargo test --workspace`, `fmt`, and `clippy` all pass.
7. No public API added. `apply_sqlite_script` is `pub(crate)`.
8. `docs/src/adapter-matrix-and-config.md` states the verified position.

## 9. Prohibited shortcuts

- Do not reword the comment in `0002_postgres.sql` to dodge the bug.
- Do not "fix" the splitter by reordering strip-then-split for PostgreSQL. That
  was considered and rejected (RFC-038 §3.1, §9.1).
- Do not add a SQL-parsing dependency (RFC-038 §9.4).
- Do not adopt `sqlx::migrate!` (RFC-038 §4).
- Do not mark the PostgreSQL suite as passing on the strength of the SQLite
  suite passing.
- Do not fix any adapter defect the suite reveals — escalate (§7).
- Do not widen `apply_sqlite_script` to `pub` for convenience.

## 10. Security constraints

No security invariant changes. The threat-model position to preserve in your
wording: INV-5 for PostgreSQL is **unverified**, not violated. Do not write
anything in the CHANGELOG or the adapter matrix that claims more than the tests
demonstrate — overstating verification is the failure this whole milestone
exists to correct.

## 11. Compatibility constraints

Schema unchanged and still `IF NOT EXISTS` idempotent, so hosts that applied it
by hand are unaffected. The one behavioral change — a failed PostgreSQL
migration now rolls back instead of leaving partial schema — is strictly safer
and requires no host action.

## 12. Known risks

| Risk | Mitigation |
|---|---|
| `raw_sql` does not accept multi-statement scripts on SQLite | §5.2 fallback, pre-authorized, evidence required |
| The PostgreSQL suite reveals further adapter defects | Expected; §7 escalation, not in-scope repair |
| No Docker in your sandbox | Report honestly (§6); do not infer a result |
| The regression test passes against pre-fix code | Then it does not test the defect — redesign it before submitting |

## 13. Required evidence

- Full diff.
- Regression test output **before** and **after** the fix.
- SQLite conformance output; PostgreSQL conformance output, or a plain
  statement that it could not run and why.
- Which §5.2 path you took, with the evidence that justified it.
- `fmt`, `clippy`, `cargo test --workspace` results.

## 14. Required review-request format

Per §9.2 of `ai-multi-agent-software-development-organization-and-workflow.md`.
File it at
`.git-exclude/review-request/038-migration-runner-must-not-parse-sql.md`.
My review result is returned at
`.git-exclude/reviewed/038-migration-runner-must-not-parse-sql.md`.

Report to the owner only the path of this handoff and your review request.
