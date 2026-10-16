# Follow-Up Handoff — RFC-016: Documentation Accuracy Sweep

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-17
- **Governing RFC:** [`../../done/016-documentation-examples-non-technical-ux-guidance.md`](../../done/016-documentation-examples-non-technical-ux-guidance.md) — RFC-016 governs documentation; this restores its accuracy against as-built rather than changing its intent, so it lives beside that RFC rather than acquiring one of its own.
- **Audit:** [`../../../.git-exclude/reviewed/AUDIT-documentation-accuracy-2026-10-17.md`](../../../.git-exclude/reviewed/AUDIT-documentation-accuracy-2026-10-17.md) — read it first; it has the evidence for every item here.

## 1. Purpose

The owner found one stale version string. The audit found ten issues, one of
which hands readers code that authenticates expired sessions.

**Amended 2026-10-17.** An eleventh was found afterwards, while reviewing the
rename decision: `docs/src/adapter-matrix-and-config.md` states a guarantee that
was removed by RFC-047 (§4.9). My audit had listed that file as current, and it
is not. It is the same defect class as §4.1 — a guarantee statement that outlived
the behaviour — so it belongs in this sweep.

## 2. Do §4 first

**§4.1 is a correctness fix to an authentication example.** Everything else is
version numbers and prose. If you only get one thing right, get that one.

## 3. Change scope

- `docs/src/migration-from-an-existing-service.md` — §4.1
- `crates/codlet/README.md`, `crates/codlet-sqlx/README.md` — §4.2
- `docs/src/threat-model.md` — §4.3
- `README.md` — §4.4, §4.5
- `docs/src/rfc-process.md` — §4.6
- `examples/README.md` — §4.7
- `CONTRIBUTING.md` — §4.8
- `docs/src/adapter-matrix-and-config.md` — §4.9
- `CHANGELOG.md` — one `[Unreleased]` entry for the lot

## 4. Required implementation

### 4.1 The migration guide's session example — correctness

Option B's example branches on `find_active_session(...).is_none()`. Since
RFC-047 step 2 that function returns expired and revoked records, so `is_some()`
does not mean valid, and the example as written authenticates them.

Rewrite it to pass the record to `classify_session` and branch on the outcome.
Keep it pseudocode — it is illustrating the parallel-lookup shape, not compiling
— but the shape must be correct.

Add one sentence making the contract explicit for a reader who skims: the store
returns the record, the classifier decides. That is the thing the old example
got wrong and the thing a migrating host most needs to know.

### 4.2 The two crate READMEs — these are the published front pages

Both are on crates.io; `codlet`'s is also its docs.rs rustdoc.

**`crates/codlet/README.md`:** drop the "pre-release (v0.4.0)" status block and
the "Production adapters … are the next step" claim. Write what is true at
0.20.0: the primitive, orchestration, and adapter layers are all implemented;
SQLite, PostgreSQL, D1 and in-memory adapters exist and pass a shared
conformance suite.

**Do not pin a version number in the prose.** That is what made it stale — a
status line naming a version is guaranteed to rot. Describe the state, and point
at `CHANGELOG.md` for the version.

**`crates/codlet-sqlx/README.md`:** the title says "SQLite storage adapters" and
the body says PostgreSQL "is planned for a later release". PostgreSQL shipped at
v0.12.0, its conformance passes in CI, and **this crate's MSRV is 1.94 because
of that `sqlx` dependency path** — the README denies the feature that sets its
own toolchain floor. Fix the title, the status, and the PostgreSQL sentence; same
no-version-in-prose rule.

While there: its usage example's comment says the store implements
`CodeStore, SessionStore, FormTokenStore`. Check whether `CodeAdminStore` belongs
in that list and add it if so — do not assume either way.

### 4.3 Gate count — pick the true one

`SECURITY.md` says five static gates. `docs/src/threat-model.md` says four.
**Five is correct** — `cookie-attrs-present` was retired (RFC-042) and
`no-interpolated-sql-values` added (RFC-048); the threat model recorded the
first and missed the second.

Fix the threat model. Verify against `cargo run -p xtask -- release-check`
rather than against my count.

### 4.4 `README.md` — the Axum claim contradicts its own §Official crates

Design notes say runtime support for "Cloudflare Workers/D1/KV, SQLx, **Axum**"
lives in separate adapter crates. Thirty lines later, "Official crates" says
`codlet-axum` was withdrawn and never built (RFC-037). Remove Axum from the
design-notes list.

### 4.5 `README.md` — the reported version strings

Four occurrences of `0.17` in the dependency examples (`codlet`,
`codlet-worker`, and `codlet-sqlx` twice). Current minor is `0.20`.

Note `codlet-worker` is `publish = false` and is not on crates.io — a `0.20`
version requirement for it in a `[dependencies]` example cannot resolve. Either
show it as a git/path dependency or say plainly that it is unpublished. **Decide
and say which you did**; do not leave an example that cannot work.

### 4.6 `rfc-process.md` — wrong three ways

1. "uses the 4-folder variant: `proposed/`, `done/`, and `archive/`" — the
   project adopted the **5-folder** variant (RFC-035 §3.1). Actual folders:
   `proposed/`, `accepted/`, `done/`, `archive/`, plus `handoffs/`.
2. "RFC-001 and RFC-002 are accepted" — both are Implemented, in `done/`.
3. "RFC-003 onward are proposed and being implemented in milestone order" — all
   49 are Implemented; Proposed is 0.

Rewrite from `rfcs/README.md` and the actual directory listing. **Do not restate
the RFC count in prose** — it will be wrong by next milestone. Point at the
index, which is maintained.

### 4.7 `examples/README.md`

Its practices list names `SessionManager::issue`, `validate`, `revoke`. Add
`rotate` (RFC-045). Note that `validate` now takes `Option<&str>` if the
examples show otherwise — check the example sources rather than assuming.

M6's features have no example. **Do not write one here** — that is a larger
piece of work and §6 covers why it is out of scope.

### 4.8 `CONTRIBUTING.md`

The "non-negotiable invariants" list has seven entries; the threat model has
nine. Missing: session issuance requires a won-claim proof (INV-7), and rotation
requires an unforgeable `Authenticated` outcome (INV-9).

Add both. Keep the list's voice — it is a short "do not weaken these", not a
copy of the table.

### 4.9 The adapter matrix states a guarantee that no longer exists

`docs/src/adapter-matrix-and-config.md` line 13, in the guarantee-definitions
table:

> | **Expiry filter** | `find_redeemable` and `find_active_session` exclude expired records server-side. |

Since RFC-047 that is **false for every adapter**. Both lookups return records
regardless of expiry, use or revocation; the classifiers decide. The adapters'
rustdoc says so explicitly, and `codlet-conformance` asserts it — an adapter
that still filtered would fail the suite.

Note also that **"Expiry filter" is an orphan**: the adapters table below it has
no such column, so nothing in the document ever claims a specific adapter
satisfies it. It is a definition of a property that is not merely unclaimed but
now contradicted.

Replace that row with one that states the actual contract — that lookups return
any matching record and the classifier decides state. Keep it in the same voice
and the same one-line table form.

Do not add a column to the adapters table. The property is uniform across
adapters by design; a column of identical cells says less than the definition
does.

## 5. Non-change scope

- **No rename of `find_active_session`, `find_redeemable`, or their record
  types.** The owner accepted RFC-050 on 2026-10-17, which renames all four and
  removes the dead `now` parameter. That is a code change with its own handoff
  and it is not yours here.

  **This sweep runs first, and RFC-050 follows it.** Both touch §4.1's example
  and §4.9's table row, so write them with the names as they are **today**.
  RFC-050 then renames the identifiers in whatever text you leave. Do not try to
  pre-empt it by using the new names — they do not exist yet and the docs are
  doctested.
- **No new `docs/src/` chapter** for M6's features — §6.
- No code changes at all. If a doc fix seems to require one, stop and report.
- Nothing in `rfcs/done/` or `rfcs/archive/`. Those are historical records.
- `docs/src/key-rotation.md`, `introduction.md`, `SECURITY.md`,
  `crates/codlet-worker/README.md` — audited and current. Leave them.
  (`adapter-matrix-and-config.md` was on this list and has been moved to §4.9 —
  if you find anything else on it that looks stale, say so in the review request
  rather than assuming the list is right.)

## 6. Deliberately out of scope: documenting M6

Idle timeout, session rotation, and host-visible failure reasons are shipped and
have no user-facing documentation. That is a real gap and it is **not** this
handoff's job: it is new chapters plus a `SUMMARY.md` entry, which is authoring
work, not an accuracy sweep. Mixing it in would bury §4.1 in a large diff.

Recorded in the audit; it will get its own handoff.

## 7. Required verification

Prose has no test suite, so the check is mechanical and specific:

| Check | Expectation |
|---|---|
| `grep -rn '0\.17' README.md` | no dependency-example hits |
| `grep -rnE 'v0\.4\.0\|v0\.5\.0' crates/*/README.md` | no hits |
| `grep -rn 'codlet-axum' README.md` | only inside "Official crates" |
| `grep -rn 'four static gates' docs/src/` | no hits |
| `grep -rn '4-folder' docs/src/rfc-process.md` | no hits |
| `grep -c '^- ' <CONTRIBUTING invariants block>` | 9 |
| `grep -n 'Expiry filter' docs/src/adapter-matrix-and-config.md` | no hits |
| `cargo run -p xtask -- release-check` | 5 gates — confirms §4.3's number |
| `cargo test --workspace` | green (the `codlet` README is doctested; do not break it) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean |

**Also re-read the migration guide's rewritten example against
`classify_session`'s actual signature** before submitting. An authentication
example that is wrong in a new way is worse than the one being replaced.

## 8. Acceptance criteria

1. §4.1's example branches on `classify_session`, not `is_none()`.
2. Neither crate README names a version in prose; both describe the 0.20.0
   state truthfully.
3. `codlet-sqlx`'s README states PostgreSQL support as shipped.
4. Threat model says five gates, verified against `release-check`.
5. Axum gone from `README.md`'s design notes.
6. No `0.17` in dependency examples; `codlet-worker`'s unpublished status
   handled explicitly and stated.
7. `rfc-process.md` correct on folders and states, with no hardcoded RFC count.
8. `examples/README.md` includes `rotate`.
9. `CONTRIBUTING.md` lists nine invariants.
10. The adapter matrix no longer claims server-side expiry filtering, and
    states the post-RFC-047 contract instead.
11. No code change; full CI green.

## 9. Prohibited shortcuts

- Do not pin a version in any prose status line. That is the defect, not a fix.
- Do not rename the store API.
- Do not add M6 chapters.
- Do not restate the RFC count in prose.
- Do not leave a dependency example that cannot resolve (§4.5).
- Do not "fix" `docs/src/` files the audit lists as current — but do report one
  that looks wrong (see §5).
- Do not use RFC-050's new names anywhere. They do not exist yet.

## 10. Required evidence

Diff; each §7 check's output; the rewritten migration example quoted in your
review request so I can read it against `classify_session` without a checkout;
CI run URL.

## 11. Review request

`.git-exclude/review-request/016-followup-accuracy-sweep.md`; my result returns
at `.git-exclude/reviewed/016-followup-accuracy-sweep.md`.
