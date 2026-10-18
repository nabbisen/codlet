# Developer Handoff — RFC-057: The D1 Migration Runner Does Not Work

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **RFC:** [`../../accepted/057-the-d1-migration-runner-does-not-work.md`](../../accepted/057-the-d1-migration-runner-does-not-work.md)
- **Baseline:** `3a7d24c`
- **Priority: ahead of other M7 work.** These are published defects in the documented setup path.

## 1. Purpose

Fix the two defects RFC-054 uncovered: `run_d1_migrations` cannot succeed, and
a pre-RFC-044 D1 database never gains `codlet_sessions.last_seen_at`.

You found both. You already have working code for the first one — RFC-054's
Miniflare crate applies the same file correctly. This is largely moving that
knowledge into the shipped function.

## 2. Change scope — indicative, not authoritative

`crates/codlet-worker/src/migration.rs`,
`crates/codlet-worker/migrations/0001_initial.sql` (comment only, if at all),
`crates/codlet-worker/README.md`, the two `it.fails` tests,
`CHANGELOG.md`. §7 is binding.

## 3. Defect 1 — apply statements individually, comments stripped first

Strip `--` comments, **then** split on `;`, then run each non-empty statement.
That order is RFC-038's finding: the shipped SQL has a `;` inside a comment, so
splitting first corrupts the parse. Do not reintroduce it.

**Check whether `D1Database::batch` exists in the pinned `worker` version** and
prefer it over a loop of `exec` calls: one round trip, and the binding's own
statement handling rather than ours. If it is unavailable or awkward, a loop is
acceptable — say which you used and why.

Reuse the logic RFC-054's `codlet-worker-miniflare` crate already uses rather
than writing a third copy. If that means lifting it into a shared helper in
`codlet-worker`, do that and have the test crate call it: **one** splitter in
the codebase, not two. Three copies of a splitter is how RFC-038 happened.

## 4. Defect 2 — the upgrade path the comment already promises

Detect whether `codlet_sessions` has `last_seen_at` via `pragma_table_info`,
and `ALTER TABLE … ADD COLUMN` only when absent. Mirror
`crates/codlet-sqlx/src/migration.rs:42-49` so the two SQLite-family adapters
read the same way.

A bare unconditional `ALTER` is wrong: SQLite has no
`ADD COLUMN IF NOT EXISTS`, so it fails with "duplicate column name" on an
already-migrated database and breaks the idempotence the rustdoc promises.

**Do not resolve the discrepancy by deleting the comment.** The schema says
`migration.rs` has an idempotent ALTER; implement it so the comment becomes
true. Deleting it would resolve the inconsistency in the wrong direction and
lose the only record that an upgrade path was ever intended.

## 5. The rustdoc has to stop asserting things

`migration.rs`'s module and function docs currently say `exec` "is designed
for multi-statement migration scripts" and that the function is "safe to call
on every Worker startup". The first is false and the second only becomes true
after §4. Rewrite both to describe what the code does after your change.

`crates/codlet-worker/README.md` shows `run_d1_migrations(&db).await?` as the
setup step. After this it is correct, so leave the call — just make sure
nothing around it still describes the old mechanism.

## 6. The two `it.fails` tests flip, and gain a reference

Both become plain passing tests. That is the acceptance evidence: executable
before the fix, executable after.

While you are there, add the RFC reference to each — per the RFC-054 review, an
`it.fails` must name the RFC that will remove it so it cannot become permanent
furniture. These two are being removed, so the reference goes on whatever
replaces them.

Add one test that does not exist yet: **a `;` inside a `--` comment**,
asserting the statement split is not fooled. That is the RFC-038 trap and it
should have a test of its own rather than being covered incidentally by the
real file.

## 7. Acceptance criteria

Binding; §2 yields to these.

1. `run_d1_migrations` succeeds against a real D1 binding in Miniflare, called
   as a host calls it.
2. Two consecutive calls both succeed, including against a database that
   already has `last_seen_at`.
3. A pre-RFC-044 `codlet_sessions` gains the column, and the full session
   conformance suite passes against that upgraded database.
4. A dedicated test covers a `;` inside a `--` comment.
5. Both `it.fails` tests are plain passing tests.
6. Only one statement-splitting implementation exists in the workspace.
7. Rustdoc and README describe the actual mechanism.
8. §8's audit table is delivered.
9. All blocking CI jobs green, including `rust-d1`.

## 8. Also required: the audit RFC-038 never got

RFC-038 fixed one runner and its reasoning was not carried to the others, which
is why this RFC exists. Produce a short table covering **all three** migration
paths — SQLite, PostgreSQL, D1 — with two columns:

- are comments stripped before splitting, or is splitting avoided entirely?
- is there a column-upgrade path for every column added after the initial
  release, and is it idempotent?

Report it even if everything but D1 is already correct. **If it finds a defect,
report before changing anything** — I will decide whether it belongs here or in
its own RFC.

## 9. Non-change scope

- No change to the schema's column types. `INTEGER` is correct.
- No change to any `D1*Store` query, or to the `Rc` handle.
- No new migration file, no migration-versioning framework.
- No change to `codlet-sqlx`'s runners unless §8 finds something — and then
  report first.
- Do not touch the JS harness beyond what the flipped tests require.

## 10. Required evidence

`git diff --stat`; the new `run_d1_migrations` quoted; whether you used `batch`
or a loop and why; the vitest output showing the two former `it.fails` tests
passing; the idempotence and pre-RFC-044 results; the comment-semicolon test;
§8's table; CI run URL.

## 11. Review request

`.git-exclude/review-request/057-d1-migration-runner.md`; result returns at
`.git-exclude/reviewed/057-d1-migration-runner.md`.
