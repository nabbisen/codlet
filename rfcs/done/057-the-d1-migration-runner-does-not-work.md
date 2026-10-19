# RFC-057: The D1 Migration Runner Does Not Work

- **Status:** Implemented (v0.21.0) — reviewed and approved 2026-10-18
- **Target milestone:** M7 — **these are published defects; ahead of other M7 work**
- **Primary crate(s):** `codlet-worker`
- **Source basis:** RFC-054 stages A and C; both reproduced and extended 2026-10-18

## 1. Summary

Two defects in `codlet-worker`'s migration path, both found the day D1 code
first executed, both affecting the flow the crate's own README tells hosts to
use:

1. **`run_d1_migrations` fails on every call.** D1 rejects the script.
2. **A pre-RFC-044 D1 database never gains `codlet_sessions.last_seen_at`**, so
   `D1SessionStore` cannot work against it — and the schema file claims a
   mechanism that was never written.

Neither is a regression. Both have been true since the code was added, and
nothing could have caught them, because until RFC-054 nothing ran them.

## 2. Defect 1: `run_d1_migrations` is rejected by D1

`crates/codlet-worker/src/migration.rs` is 21 lines:

```rust
pub async fn run_d1_migrations(db: &worker::d1::D1Database) -> worker::Result<()> {
    let sql = include_str!("../migrations/0001_initial.sql");
    db.exec(sql).await?;
    Ok(())
}
```

Observed, on Miniflare, from both the JS harness and the shipped function
called as a host would call it:

```
D1_EXEC_ERROR: Error in line 1: -- codlet SQLite migration 0001:
SQL code did not contain a statement.
```

D1's `exec` treats each **line** as a statement. Line 1 of the migration is a
comment, and the `CREATE TABLE` statements span many lines. The function cannot
succeed with this file.

Its rustdoc asserts the opposite twice — that `exec` "is designed for
multi-statement migration scripts" and that the function is "safe to call on
every Worker startup". `crates/codlet-worker/README.md` and the crate rustdoc
both show `run_d1_migrations(&db).await?` as the setup step.

A second trap sits behind it: the SQL contains a `;` inside a `--` comment, so
a naive splitter also breaks — **the exact defect RFC-038 fixed for
PostgreSQL.** Comments must be stripped before splitting, not after.

Observed on Miniflare only. Production D1 is not known to differ, and must not
be assumed to.

### 2.1 This is RFC-038 recurring

RFC-038 found `run_postgres_migrations` broken since v0.12.0 by a hand-rolled
SQL splitter, which meant PostgreSQL conformance — including INV-5 — had never
executed. Same crate family, same function role, same class, and it survived
the RFC that fixed its sibling because the D1 path had no execution evidence.

The lesson is not "write better migration runners". It is that RFC-038's fix
was applied to one adapter and its *reasoning* was not applied to the others.

## 3. Defect 2: no upgrade path, and a comment asserting one

`CREATE TABLE IF NOT EXISTS` is a no-op against an existing table, so a D1
database created before RFC-044 never gains `codlet_sessions.last_seen_at`.
`D1SessionStore` SELECTs and UPDATEs that column, so it fails against such a
database.

`migrations/0001_initial.sql` contains exactly one occurrence of the word
`ALTER`, and it is inside a comment on that column:

> `-- added by migration.rs's idempotent ALTER for databases created before RFC-044`

**There is no such ALTER.** The schema documents a mechanism that was never
implemented. This is the false-comment class: a comment asserting a guard that
does not exist, which is worse than silence because it answers the question a
reviewer would otherwise ask.

Both SQL adapters have the mechanism. D1 is the only one without it:

| Adapter | Upgrade path |
|---|---|
| SQLite | `PRAGMA table_info`, then conditional `ALTER` (`codlet-sqlx/src/migration.rs:42-49`) — SQLite has no `ADD COLUMN IF NOT EXISTS` |
| PostgreSQL | `ALTER TABLE … ADD COLUMN IF NOT EXISTS` (`migrations/0002_postgres.sql:49`) |
| D1 | none |

D1 is SQLite-backed, so it needs SQLite's shape: detect, then add. A bare
`ALTER TABLE … ADD COLUMN` would fail with "duplicate column name" on an
already-migrated database, which would break the idempotence the rustdoc
promises.

## 4. Decision

### 4.1 Execute statement by statement, stripping comments first

`run_d1_migrations` must apply the migration as individual statements: strip
`--` comments, then split on `;`, then run each non-empty statement. In that
order — RFC-038's finding was that splitting first lets a `;` inside a comment
break the parse.

The mechanism already exists and works: RFC-054's Miniflare crate applies the
same file this way via `include_str!`, and the JS harness does the same. The
fix is to put it where hosts call it.

**Prefer `D1Database::batch` with prepared statements over looping `exec`** if
it is available in the pinned `worker` version — one round trip and the
adapter's own statement handling rather than ours. Verify which is available;
do not assume.

### 4.2 Add the `last_seen_at` upgrade path that the comment already promises

Detect whether `codlet_sessions` has `last_seen_at` — `pragma_table_info` works
on D1 — and `ALTER TABLE … ADD COLUMN` only when it is absent. Mirror
`codlet-sqlx/src/migration.rs:42-49` so the two SQLite-family adapters read the
same way.

Then the comment in the schema becomes true. Do not delete the comment to
resolve the discrepancy — implement what it says.

### 4.3 Keep the rustdoc honest

The claims that `exec` suits multi-statement scripts and that the function is
startup-safe must match the implementation after §4.1 and §4.2. Idempotence
becomes a real property only once §4.2 handles the already-added column.

### 4.4 The two `it.fails` tests flip

RFC-054 recorded both defects as `it.fails` tests. Each must become a plain
passing test. That is the acceptance evidence: the defect was executable before
the fix and is executable after it.

### 4.5 Audit the other adapters for RFC-038's reasoning

RFC-038 fixed one runner. §2.1 is what that cost. Check every remaining
migration path — SQLite, PostgreSQL, D1 — for comment-before-split handling and
for a column-upgrade path per post-initial-release column, and record the result
as a table even where nothing is wrong. An audit that finds nothing is still
worth having written down; this RFC exists because one was never done.

## 5. Non-goals

- No change to the shipped schema's **column types**. They are `INTEGER` and
  correct; RFC-053 fixed the documentation that said otherwise.
- No change to any `D1*Store` query or to the `Rc` handle.
- No new migration file and no migration-versioning framework. One file applied
  idempotently is the design; this is about applying it correctly.
- No change to `codlet-sqlx`'s runners unless §4.5 finds a defect — in which
  case report before changing.

## 6. Security considerations

Both defects fail closed: a failed migration means an unusable store, and a
missing column means `D1SessionStore` errors rather than mis-authenticating.
Neither is a bypass.

The security-relevant part is what they imply about evidence. D1's row in the
adapter guarantee matrix has asserted atomic claim and atomic consume — INV-5
among them — on the strength of a JavaScript re-implementation, while the
shipped setup path could not run at all. A guarantee table whose subject has
never executed is not a guarantee table. RFC-054 closed the general case; this
closes the specific defects it exposed.

## 7. Acceptance criteria

1. `run_d1_migrations` succeeds against a real D1 binding in Miniflare, called
   as a host calls it.
2. It is idempotent: two consecutive calls both succeed, including on a
   database that already has `last_seen_at`.
3. A pre-RFC-044 `codlet_sessions` gains `last_seen_at`, and the full session
   conformance suite then passes against that upgraded database.
4. Comments are stripped before splitting; a test covers a `;` inside a `--`
   comment specifically.
5. Both RFC-054 `it.fails` tests are plain passing tests.
6. The rustdoc and `crates/codlet-worker/README.md` describe what the function
   now does.
7. §4.5's audit table exists, covering all three migration paths.
8. All blocking CI jobs green, including the `rust-d1` vitest project.
