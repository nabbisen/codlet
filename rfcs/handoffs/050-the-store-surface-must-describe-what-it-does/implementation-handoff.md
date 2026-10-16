# Developer Handoff — RFC-050: The Store Surface Must Describe What It Does

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-17
- **RFC:** [`../../accepted/050-the-store-surface-must-describe-what-it-does.md`](../../accepted/050-the-store-surface-must-describe-what-it-does.md) — Accepted by the owner, 2026-10-17
- **Baseline:** `084aca3`
- **Prerequisite:** the RFC-016 accuracy sweep must land first — see §2.

## 1. Purpose

RFC-047 moved expiry, revocation and use decisions out of the store and into the
classifiers. The names, the types and the signatures still describe the removed
behaviour. This change makes the surface say what the code does.

**There is no behavioural change in this handoff.** Not a small one, not a
"while we're here" one. Every decision stays exactly where RFC-047 put it. If
you find yourself changing an outcome, a predicate, or a SQL `WHERE` clause,
stop and report — you have found either a bug in the current tree or a
misunderstanding in this handoff, and both are worth more than a silent fix.

## 2. Sequencing — do not start until the sweep has landed

`rfcs/handoffs/016-documentation-examples-non-technical-ux-guidance/follow-up-accuracy-sweep-handoff.md`
rewrites two passages that this change then renames: the migration guide's
session example and the adapter matrix's guarantee row. It uses the **old**
names deliberately.

Land the sweep, then start here. If you have been handed both at once, say so
and do the sweep first.

## 3. Change scope — indicative, not authoritative

Occurrence counts at `084aca3`, excluding `rfcs/` and `.git-exclude/`:

| Area | Files |
|---|---|
| Trait + type definitions | `crates/codlet/src/store/code.rs`, `store/session.rs`, `store.rs` |
| Classifiers | `crates/codlet/src/state/code.rs`, `state/code/tests.rs`, `state/session.rs`, `state.rs` |
| Re-exports | `crates/codlet/src/lib.rs` |
| Flow managers | `crates/codlet/src/auth/code.rs`, `auth/session.rs`, `admin.rs` |
| In-memory adapter | `crates/codlet/src/mem/code.rs`, `mem/session.rs` |
| SQLite + PostgreSQL | `crates/codlet-sqlx/src/code.rs`, `session.rs`, `postgres/code.rs`, `postgres/session.rs` |
| D1 | `crates/codlet-worker/src/d1/code.rs`, `d1/session.rs` |
| Conformance | `crates/codlet-conformance/src/code.rs`, `session.rs`; `crates/codlet-sqlx/tests/conformance.rs` |
| Acceptance tests | `rfc_005`, `rfc_006`, `rfc_009_compile`, `rfc_013`, `rfc_044`, `rfc_045`, `rfc_046`, `rfc_security` |
| Docs | `docs/src/migration-from-an-existing-service.md`, `docs/src/adapter-matrix-and-config.md` |
| Changelog | `CHANGELOG.md` — one `[Unreleased]` entry |

**This table is a map, not a checklist.** I built it from grep and I have been
wrong about scope lists before. The binding statement is §8, and the authority
is the compiler plus the §7 greps. Where the two disagree, the compiler wins and
the table is the thing that was wrong.

## 4. Required implementation — Step 1: the renames, alone

One commit, four identifiers, nothing else:

| From | To |
|---|---|
| `CodeStore::find_redeemable` | `CodeStore::lookup_code` |
| `RedeemableCode` | `StoredCode` |
| `SessionStore::find_active_session` | `SessionStore::lookup_session` |
| `ActiveSessionRecord` | `StoredSession` |

Keep this commit a pure rename: no signature change, no doc rewording beyond
the identifiers themselves, no field changes. It should be reviewable as
"nothing happened except spelling", and `cargo test --workspace` green is then
near-proof of that.

Update the rustdoc **only** where it names the old identifier. The explanatory
paragraphs about RFC-047 are correct and stay — Step 2 is where they change.

## 5. Required implementation — Step 2: remove the clock, fix the footgun

A second commit. Three changes.

### 5.1 Drop `now` from both lookups

```rust
fn lookup_code(
    &self,
    candidates: &[LookupKey],
    scope: Option<&str>,
) -> impl Future<Output = Result<Option<StoredCode>, StoreError>>;

fn lookup_session(
    &self,
    candidates: &[LookupKey],
) -> impl Future<Output = Result<Option<StoredSession>, StoreError>>;
```

`scope` stays on `lookup_code` — it is a lookup criterion, not a state
judgement. No adapter reads `now` today, so no adapter body changes except to
drop the parameter.

Replace the rustdoc's "accepted for symmetry with other store methods and
possible future use" with the real reason it is gone, stated as a contract
rather than an apology: **the lookup makes no time-based decision, so it is not
given the time.**

Be precise about the scope of that claim. "The store does not consult a clock"
would be a **new false statement** — and planting one is the defect this RFC
exists to remove. These legitimately keep their `now` and are out of scope:

| Keeps `now` | Why |
|---|---|
| `revoke_code`, `revoke_session` | records `revoked_at` |
| `touch_session` | records `last_seen_at` (RFC-044) |
| `ClaimRequest.now` | written as `used_at`, and the expiry guard of the conditional UPDATE (INV-5) |
| `expires_at_from_ttl` | a pure helper computing from a caller-supplied now |

The distinction to write down: a **write** is told the time so it can record it;
a **lookup** is not, because it judges nothing. That sentence is the whole point
of the change — get it right and the rest is mechanical.

### 5.2 `classify_code_lookup` takes the record

```rust
pub fn classify_code_lookup(record: &StoredCode, now: u64) -> CodeLookupOutcome
```

The body keeps the RFC-047 §8.1 decision order exactly: revoked → expired →
used → redeemable. Read the fields off `record`; change no comparison.

This removes a real hazard: `revoked_at` and `used_at` are both
`Option<u64>` and adjacent, so a caller transposing them compiles silently and
logs the wrong reason. It also makes the two classifiers symmetric with
`classify_session`, which already takes its record.

### 5.3 The nine classifier unit tests

`crates/codlet/src/state/code/tests.rs` calls the classifier with positional
scalars. They now need a `StoredCode`. Add a local helper in that test module
that builds one from the three state fields and fills the rest with fixed dummy
values.

**Preserve every case, including the precedence pairs** — revoked-and-expired,
revoked-and-used, used-and-expired. Those tests exist to pin the decision
*order*, which is the part of RFC-047 that a careless refactor would silently
reorder. Do not collapse, parameterise, or drop any of them.

## 6. Non-change scope

- No behavioural change anywhere. See §1.
- `claim_code` is untouched — its conditional UPDATE is the INV-5 enforcement
  point. `lookup_code`'s state fields remain a pre-filter, not a guard.
- `classify_session`'s signature, semantics and decision order: untouched.
- `insert_*`, `revoke_*`, `touch_session`, `consume_form_token`: untouched.
- Public error types and the `SessionFailure` surface: untouched.
- The write-side `CodeRecord` / `SessionRecord` keep their names. The pairing
  is deliberate: `Record` goes in, `Stored` comes out.
- `CodeLookupOutcome` and `SessionValidationOutcome` keep their shapes —
  RFC-050 §9 defers that question to M7's API audit. Do not start it here.
- Nothing in `rfcs/done/`, `rfcs/archive/`, or `CHANGELOG.md` history. Those
  record what was true when written.

## 7. Required verification

| Check | Expectation |
|---|---|
| `cargo test --workspace` | green |
| `cargo test -p codlet-sqlx --features postgres` | green — PostgreSQL conformance included |
| `cargo check -p codlet-worker --target wasm32-unknown-unknown` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean — catches a dangling intra-doc link from a rename |
| `cargo run -p xtask -- release-check` | 5 gates pass |
| the two greps in §7.1 | no hits |

### 7.1 The two residue greps

Run these as written, from the repository root. They are in a block rather than
the table above because a markdown table cannot hold an unescaped `|`, and an
alternation escaped as `\|` is a *literal pipe* in an ERE — it matches nothing
and passes unconditionally. Do not retype them from the table.

```sh
# 1. No old identifier survives outside the historical record.
grep -rnE 'find_redeemable|RedeemableCode|find_active_session|ActiveSessionRecord' \
  --include='*.rs' --include='*.md' . \
  | grep -v '^./rfcs/done/' | grep -v '^./rfcs/archive/' \
  | grep -v '^./.git-exclude/' | grep -v '^./CHANGELOG.md' | grep -v '^./target/'

# 2. Which declarations still take a timestamp.
grep -nE 'now: u64' crates/codlet/src/store/code.rs crates/codlet/src/store/session.rs
```

**Grep 1 must print nothing.** Check its exit status, not just the absence of
output — `grep` exits 1 on no match, and a pipeline that errors early also
prints nothing. Report the exit code.

**Grep 2 must print exactly five lines**, and that is the check — not "no
output". The five are §5.1's write-side parameters:

| File | Declaration |
|---|---|
| `store/code.rs` | `ClaimRequest.now` |
| `store/code.rs` | `revoke_code` |
| `store/code.rs` | `expires_at_from_ttl` |
| `store/session.rs` | `revoke_session` |
| `store/session.rs` | `touch_session` |

It prints seven at baseline `10c4c84`. Fewer than five means you removed a
`now` that a write needs; more than five means a lookup still has one. Quote the
output and name which declaration each line is, so neither of us is counting
lines alone. The pattern matches declarations only, not the word "now", which
still appears legitimately in the prose of both files.

**Verify in both environments.** Run the greps on a clean checkout as well as
your working tree — a stale `target/` or an uncommitted file has produced a
false pass on this project before.

**Confirm the rename commit is behaviour-free** by reporting
`git diff --stat` for Step 1 and confirming the suite was green at that commit
before Step 2 was written.

## 8. Acceptance criteria

These are binding. Where §3's table disagrees, these win.

1. The four old identifiers appear nowhere outside `rfcs/done/`,
   `rfcs/archive/`, `.git-exclude/`, and `CHANGELOG.md` history.
2. Neither `lookup_code` nor `lookup_session` takes a timestamp, and no
   adapter implementation of either reads a clock by any means — including a
   SQL-level one. The five write-side `now` parameters in §5.1 are unchanged.
3. `classify_code_lookup` takes `&StoredCode` and `now`.
4. The RFC-047 §8.1 decision order is unchanged for both entities, and the nine
   precedence tests still prove it.
5. **No test assertion was altered except for renamed identifiers and the
   record-construction in §5.3.** If an assertion had to change, the change is
   not behaviour-free and you must stop and report instead.
6. `codlet-conformance`'s RFC-047 inverted assertions are intact and still
   fail against a filtering adapter — see §9.
7. Conformance green on all four adapters, PostgreSQL and D1 included.
8. The two doc occurrences use the new names; the migration guide's example
   still classifies rather than branching on `is_some()`.
9. Full CI green, all 22 jobs.

## 9. Prohibited shortcuts

- **Do not delete or weaken the conformance tests that prove the store returns
  expired and revoked records.** There is a tempting wrong argument here: "the
  lookup has no `now` any more, so an adapter cannot filter, so the test is
  redundant." It is false. SQLite has `unixepoch()`, PostgreSQL has `now()`,
  D1 has both — an adapter can reach a clock without being handed one. Removing
  `now` makes accidental filtering less likely, not impossible, and those
  inverted assertions are the only thing that would catch it.
- Do not combine Steps 1 and 2 into one commit. The rename's value as a
  reviewable no-op depends on its being alone.
- Do not "improve" the classifiers' logic, outcome enums, or decision order.
- Do not rename anything not listed in §4. In particular not `CodeRecord`,
  `SessionRecord`, `LookupKey`, or the `classify_*` functions.
- Do not add a compatibility alias or a deprecated re-export. Breaking changes
  are accepted; a half-migration leaves both names in the docs, which is the
  defect this RFC exists to remove.
- Do not touch `claim_code`'s SQL. If a rename appears to require it, stop.

## 10. Required evidence

- `git diff --stat` for each step separately, and confirmation that CI was
  green at Step 1.
- Each §7 check's output, from a clean checkout.
- The new `lookup_session` and `lookup_code` signatures and rustdoc, quoted in
  the review request, so I can read the contract wording without a checkout.
- The §5.3 test helper and one precedence test, quoted.
- CI run URL.

## 11. Review request

File at `.git-exclude/review-request/050-store-surface-rename.md`; my result
returns at `.git-exclude/reviewed/050-store-surface-rename.md`.

If anything in §4 or §5 turns out to be impossible or to require a behavioural
change, report it in the review request rather than working around it. This
handoff asserts that the whole change is mechanical; if that is wrong, I want to
know where.
