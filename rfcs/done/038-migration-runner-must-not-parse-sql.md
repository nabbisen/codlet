# RFC-038: The Migration Runner Must Not Parse SQL

- **Status:** Implemented (v0.18.0)
- **Target milestone:** M4 (defect fix in a published crate; sequencing per §10)
- **Primary crate(s):** `codlet-sqlx`
- **Source basis:** RFC-036 review finding F-1, 2026-09-03; RFC-034 §9; RFC-023

## 1. Summary

`codlet-sqlx`'s migration runners split migration SQL on `';'` with a
hand-rolled splitter, then strip `--` comment lines from each fragment. The
order is wrong: a semicolon inside a comment cuts the comment in half, and the
tail of that comment is submitted to the database as a statement.

`migrations/0002_postgres.sql` contains such a comment. As a result
`run_postgres_migrations` fails for every consumer of the published
`codlet-sqlx` `postgres` feature, and has done since RFC-034 shipped at
v0.12.0.

The fix is not a better splitter. It is to stop parsing SQL in codlet at all.

## 2. The defect

`crates/codlet-sqlx/src/postgres.rs:69`:

```rust
for stmt in migration_sql.split(';') {          // (1) split first
    let trimmed: String = stmt.lines()
        .filter(|l| !l.trim_start().starts_with("--"))   // (2) strip comments after
        …
    sqlx::query(sqlx::AssertSqlSafe(trimmed.as_str())).execute(pool).await?;
}
```

`crates/codlet-sqlx/migrations/0002_postgres.sql:11`:

```sql
-- (unlike the D1/wasm32 adapter in RFC-033); SQLx maps i64 to BIGINT natively.
```

Step (1) cuts that line at the semicolon. The trailing fragment begins
` SQLx maps i64 to BIGINT natively.` — it no longer starts with `--`, so step
(2) does not remove it, and it is executed. PostgreSQL replies:

```
PgDatabaseError { code: "42601", message: "syntax error at or near \"SQLx\"",
                  position: Some(Original(1)) }
```

Every PostgreSQL conformance test fails at this point, in the fixture at
`crates/codlet-sqlx/tests/conformance.rs:321`.

### 2.1 Impact

**Direct.** The PostgreSQL adapter cannot create its own schema. A host calling
`run_postgres_migrations` — the documented startup path in RFC-034 §9 — receives
a syntax error. The adapter is unusable as published unless the operator applies
the schema by hand.

**Second-order, and worse.** Because migrations fail, the PostgreSQL conformance
suite has **never executed**. That suite carries the concurrent single-winner
claim test that proves **INV-5** for this adapter. RFC-023 makes passing the
suite the precondition for calling an adapter production-ready;
`docs/src/adapter-matrix-and-config.md` nonetheless presents `PostgresStore` as
conformant on atomic claim and single-use consume. That claim is not false
because the adapter is known broken — it is unsupported, which for a security
library is the same problem wearing a better suit.

**Why it survived two releases.** The job was red and nobody was reading CI.
This is the same blindness RFC-036 documents; F-1 is what that blindness was
hiding.

### 2.2 Latent twin

`crates/codlet-sqlx/src/migration.rs:31` (SQLite) has the identical
split-then-filter logic. `migrations/0001_initial.sql` currently contains no
semicolon inside a comment, so SQLite works — by luck. A single explanatory
comment added to that file would break the SQLite adapter the same way.

The `codlet-worker` D1 runner does not use this pattern and is unaffected.

## 3. Decision

**codlet does not parse SQL.** Both runners hand the migration file to the
driver whole, via `sqlx::raw_sql`, which accepts multi-statement scripts:

```rust
sqlx::raw_sql(sqlx::AssertSqlSafe(migration_sql)).execute(pool).await?;
```

The hand-rolled splitter and the comment filter are deleted, not repaired.

### 3.1 Why not fix the ordering

Stripping comments before splitting fixes this instance and leaves the class
intact. A semicolon inside a string literal or a quoted identifier breaks the
splitter just as thoroughly, and the next person to write a migration has no way
to know the constraint exists. The defect is not that the two steps are in the
wrong order; it is that a security library is hand-parsing a language it has a
parser for, sitting one call away.

The comment that guards the current code — "Safety: SQL comes from our own
static migration files, not user input" — is exactly the assumption that failed.
The input was our own static file, and it still produced malformed SQL. Trusted
input does not make a naive parser correct.

### 3.2 What is gained beyond the fix

`raw_sql` sends the script in one round trip, and on PostgreSQL the server wraps
it in an implicit transaction: if any statement fails, the whole script rolls
back. The current loop has no such property — a failure at statement five of
nine leaves a half-created schema behind, with no record that it is partial.
Migrations become atomic as a side effect of removing our parser.

### 3.3 SQLite

The same change applies to `migration.rs`. The `PRAGMA` calls it issues before
the file stay as they are — they are separate connection settings, not part of
the migration script.

**Verification required, with a pre-authorized fallback.** `raw_sql`'s
multi-statement behavior is documented for the server-side protocol; the
implementer must confirm the SQLite driver executes `0001_initial.sql` correctly
through the same path, evidenced by the existing SQLite conformance suite
passing. If it does not, the fallback for **SQLite only** is to strip `--`
comment lines from the whole file *before* splitting, with the ordering defect
fixed and a comment recording why that runner differs. Do not adopt the fallback
without first demonstrating that `raw_sql` fails there; report which path was
taken and the evidence for it.

### 3.4 Migration file hygiene is not the fix

`0002_postgres.sql:11` could be reworded to avoid the semicolon. That would turn
the CI job green and leave the defect in place, armed for the next author. The
comment stays as it is — after the fix it is a live regression fixture (§5).

## 4. Non-goals

- No schema change. Not one byte of the emitted DDL differs.
- No adoption of `sqlx::migrate!`. codlet deliberately uses idempotent
  `IF NOT EXISTS` DDL against a host-owned database and does not want a
  `_sqlx_migrations` table or version-tracking semantics. RFC-034 §9 stands.
- No change to `codlet-worker`'s D1 runner.
- No change to any store implementation. **See §7 — this RFC does not claim the
  PostgreSQL adapter is correct.**

## 5. Tests and release gates

1. **Regression test, mandatory.** A test that runs a migration script
   containing a semicolon inside a `--` comment and asserts it applies cleanly.
   The existing comment at `0002_postgres.sql:11` supplies the real case; the
   test must not depend on that file keeping its current wording, so it should
   carry its own fixture script.
2. **PostgreSQL conformance passes** — `cargo test -p codlet-sqlx
   --no-default-features --features postgres-test`, in CI, with Docker. This is
   the first time it will have run.
3. **SQLite conformance still passes**, proving §3.3 either way.
4. `test-postgres` green on `main`, with the run URL recorded.

## 6. Compatibility and migration effects

None for hosts that already applied the schema by hand — the DDL is unchanged
and remains `IF NOT EXISTS` idempotent. Hosts blocked by the defect will find
`run_postgres_migrations` works for the first time.

The behavioral change is that a failed PostgreSQL migration now rolls back
instead of leaving partial schema. That is strictly safer and needs no host
action.

## 7. What this RFC does **not** establish

Fixing the runner is a precondition for learning whether the PostgreSQL adapter
is correct — not evidence that it is. Once migrations succeed, the conformance
suite runs against `PostgresStore` for the first time, including the INV-5
concurrent claim test. **It may fail.** Any such failure is a separate defect
requiring its own RFC, and must be escalated rather than folded into this fix.

Until that suite passes in CI, `docs/src/adapter-matrix-and-config.md` overstates
what has been verified. Correcting that documentation is in scope for this RFC's
handoff; the correction is to state the verified position accurately, whichever
way the suite lands.

## 8. Security considerations

No security invariant is modified. The threat-model position is that INV-5 for
PostgreSQL is currently **unverified**, not that it is violated: the conditional
`UPDATE … WHERE` logic in `postgres/code.rs` was written against RFC-022 and
reviewed, but has never been executed under the concurrency test.

`AssertSqlSafe` remains appropriate — the input is a compile-time
`include_str!` of a file in this repository. The lesson recorded here is that
*trusted* input and *correctly handled* input are different properties, and the
existing safety comment conflated them.

## 9. Alternatives considered

1. **Strip comments before splitting.** Rejected — §3.1. Fixes the instance,
   keeps the class.
2. **Reword the offending comment.** Rejected — §3.4. Turns CI green while
   leaving the trap armed, and would have to be remembered forever by every
   future author.
3. **Adopt `sqlx::migrate!`.** Rejected — §4. Changes operational semantics
   codlet deliberately does not want.
4. **Add a SQL-parsing dependency.** Rejected — a parser dependency to avoid
   using the parser already reachable through `sqlx` is unjustified weight in a
   crate whose dependency surface is governed by RFC-002.

## 10. Open question for the owner

**Sequencing and release.** This is a defect in a published crate. Options:

1. **Immediately, before the RFC-035 directory migration.** *(Recommended.)*
   RFC-035 is repository hygiene; this is a broken adapter. It also unblocks the
   first real execution of the PostgreSQL conformance suite, and if that surfaces
   further defects the owner wants to know early rather than after a
   documentation reshuffle.
2. After RFC-035, keeping M4's original order.

The fix ships in the same minor release as the accepted D-1 MSRV note unless the
owner prefers to separate them. Whether v0.17.1 warrants a yank remains the
owner's call and is deliberately not decided here — note that the affected path
fails closed and loudly, so no host can have been silently running on a
half-created schema.

## 11. Acceptance criteria

- Neither `migration.rs` nor `postgres.rs` contains a hand-rolled SQL splitter
  or comment filter.
- `0002_postgres.sql:11` is unchanged.
- The §5.1 regression test exists and fails against the pre-fix code.
- PostgreSQL and SQLite conformance suites both pass in CI.
- `docs/src/adapter-matrix-and-config.md` states the verified position.
- CHANGELOG records the defect, its duration (v0.12.0 → present), and the
  migration-atomicity improvement.
