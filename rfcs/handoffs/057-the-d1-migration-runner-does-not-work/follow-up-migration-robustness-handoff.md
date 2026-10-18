# Follow-Up Handoff — RFC-057: Migration Robustness, Three Small Items

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **Governing RFC:** [`../../done/057-the-d1-migration-runner-does-not-work.md`](../../done/057-the-d1-migration-runner-does-not-work.md)
- **Review result:** `.git-exclude/reviewed/057-d1-migration-runner.md` §6
- **Baseline:** `084a10b`
- **Size:** small. Items 1 and 2 are a few lines each; item 3 is a test. Two of the three are yours.

RFC-057 is approved and merged. These close gaps it surfaced. None is a defect
in what you shipped.

## 1. Item 1 — the splitter should treat `"` as quoting

`crates/codlet-worker/src/sql_statements.rs` handles `--`, `/* */` and `'…'`.
It does not treat `"` or backticks as quoting, so a `;` or `--` inside a
double-quoted identifier would be read as a boundary or a comment.

**The current input is fine and I verified it.** The shipped migration contains
`"`, backticks and apostrophes, but every occurrence is inside a `--` comment,
which the scanner consumes first. So this is about a future edit, not a live
bug — and if it ever bit, D1 would reject the malformed statement, so it fails
closed.

Add `"` to the scanner, mirroring the `'` arm. SQL's standard escape inside a
double-quoted identifier is a doubled `""`, the same shape as `''`, so the arm
is nearly identical — consider whether one arm parameterised by the quote
character reads better than two near-copies, and use your judgement.

Backticks and `[…]` are SQLite/MySQL extensions. **Your call** whether to add
them; I would do `"` and state the remaining limitation in the module doc
rather than chase every dialect. Either way, the doc's "Anything else is passed
through unchanged" should name what is not handled, because as written it reads
more reassuring than it is.

Add a test per quoting form you support: a `;` and a `--` inside it, and the
doubled-quote escape.

**Why bother at all:** the module exists because a splitter that is wrong about
quoting is how RFC-038 happened, and that is your own sentence from its doc
comment. A component whose justification is "be right about quoting" should be
right about the standard identifier quote.

## 2. Item 2 — give `codlet-sqlx`'s SQLite runner your race tolerance

You found this and left it alone correctly. `crates/codlet-sqlx/src/migration.rs`
(around lines 42-49) checks `pragma_table_info` then runs
`ALTER TABLE codlet_sessions ADD COLUMN last_seen_at INTEGER`. Two processes
starting together can both see the column absent; the loser's `ALTER` fails
with "duplicate column name" and takes its startup with it.

Mirror what you did for D1: on `ALTER` failure, re-check the column and treat
"now present" as success. Key on the observed state, not on the error text —
that was the right instinct in the D1 version and it applies here.

You judged this less likely than on Workers, and that is true for a single
process at startup. It is not true for a multi-instance deployment where every
instance runs migrations on boot, which is a normal shape.

**PostgreSQL needs nothing**: `ADD COLUMN IF NOT EXISTS` is atomic server-side.

Test it if you can do so without contortion — simulating the race in SQLite may
not be worth the machinery. If a test would be artificial, say so and rely on
the code reading obviously; I will take that.

## 3. Item 3 — PostgreSQL has no pre-RFC-044 upgrade test

Your §4 audit found that PostgreSQL's `last_seen_at` upgrade rests entirely on
`ADD COLUMN IF NOT EXISTS` with no test that starts from a table lacking the
column, while SQLite and now D1 both have one.

Add it, mirroring SQLite's `last_seen_at_is_added_to_a_pre_existing_sessions_table`:
create the table in its pre-RFC-044 shape (or `ALTER … DROP COLUMN` the shipped
one, as you did for D1), assert the column is absent so the setup is proven,
run `run_postgres_migrations`, assert it is present, then run the session
conformance suite against the upgraded database.

Asserting absence first is the part that matters. A test that only checks the
column is present afterwards passes just as happily when the setup never
removed it.

This needs Docker, so it runs in CI. Commit and read the CI log as you have
been doing; if it fails there, report rather than iterating blind.

**Why this is worth a test for a server-guaranteed clause:** RFC-038's whole
lesson is that an untested migration path breaks silently for years. "The
server guarantees it" is exactly the reasoning that left the D1 path untested.

## 4. Non-change scope

- No change to the D1 runner, `batch`, or the upgrade path. Approved as
  committed.
- No change to the shipped SQL, including its comments.
- No second splitter anywhere, for any reason.
- No change to `run_postgres_migrations`'s `raw_sql` approach — splitting is
  correctly avoided there.
- Do not add `#[non_exhaustive]` to anything; that is for the M7 audit.

## 5. Required verification

| Check | Expectation |
|---|---|
| `cargo test -p codlet-worker` | the splitter tests pass, including the new quoting ones |
| `cargo test --workspace` | 259 or higher, nothing lost |
| `cargo test -p codlet-sqlx --no-default-features --features postgres-test` | green in CI, including the new upgrade test |
| `npm test` in `crates/codlet-worker/tests` | 61 or higher |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `grep -rn 'split_statements' crates/` | one implementation, one caller, plus tests |

## 6. Acceptance criteria

1. The splitter treats `"` as quoting, with tests for `;` and `--` inside it
   and the doubled-quote escape.
2. The module doc names what is not handled.
3. `codlet-sqlx`'s SQLite `ALTER` is race-tolerant, keyed on observed state.
4. A PostgreSQL test upgrades a pre-RFC-044 table, asserting absence first, and
   runs the session conformance suite against the result.
5. Still exactly one statement splitter in the workspace.
6. All blocking CI jobs green.

## 7. Required evidence

`git diff --stat`; the splitter's new arm quoted; the new tests quoted; the
SQLite race guard quoted; the PostgreSQL test's CI output; CI run URL.

If you decide against backticks or against a SQLite race test, say which and
why — both are explicitly your call.

## 8. Review request

`.git-exclude/review-request/057-followup-migration-robustness.md`; result
returns at `.git-exclude/reviewed/057-followup-migration-robustness.md`.
