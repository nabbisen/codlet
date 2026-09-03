# Implementation Handoff — RFC-040 Invariant Verification

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M5 (`ROADMAP.md`, M5-5 revised)
- **Governing RFC:** [`../../accepted/040-invariant-verification.md`](../../accepted/040-invariant-verification.md)
- **Priority:** may run in parallel with the RFC-039 handoff.

Read RFC-040 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Make every invariant guard capable of failing, and prove it — automatically and
repeatably, not by a one-off trial.

## 2. Background — the live defect

All five `xtask` gates share this shape:

```rust
fn gate_no_fallback_key() -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in library_sources() { … }
    if hits.is_empty() { Ok(()) } else { Err(hits.join("; ")) }
}
```

`library_sources()` walks `crates/` and returns what it finds. **If it returns
nothing, all five gates return `Ok(())` at once.** No gate asserts its corpus is
non-empty or covers the crates it claims to guard.

This is the `core-deps` failure — which reported success for two releases while
inspecting an empty string — still live, in the checks enforcing INV-1, INV-2,
and INV-3.

## 3. Change scope

- `xtask/src/main.rs` — `library_sources`, all five gates, new `self-test`
  subcommand
- `xtask/` fixtures — **new** (location your call, §5.3)
- `xtask/Cargo.toml` — only if the self-test needs a dev-dependency
- `crates/codlet/tests/` — negative tests for INV-5/6/7/8
- `Cargo.toml` — `[workspace.dependencies]` entry if `trybuild` is added
- `docs/src/threat-model.md` — the invariant map
- `.github/workflows/ci.yml` — run the self-test
- `CHANGELOG.md` — `[Unreleased]`

## 4. Non-change scope

- **No invariant changes.** INV-1…8 are unchanged in meaning.
- **No rewrite of the gates from grep to static analysis** (RFC-040 §4). Their
  pattern-matching limits are accepted; make them capable of failing, not
  exhaustive.
- No property or fuzz testing — that is RFC-041.
- No change to `codlet-conformance`'s adapter contract or to any store
  implementation.
- INV-4's guard is deliberately **out of scope** and stays an open row.

## 5. Required implementation

### 5.1 A gate must fail when it cannot check

`library_sources()` returns an error rather than an empty vector when it finds
no sources. Each gate additionally asserts its corpus covers what it claims to
guard — at minimum that `crates/codlet/src/` is represented — and fails loudly
otherwise.

This is RFC-036 §3.5 generalised from CI shell to `xtask`: **absence of evidence
is never reported as evidence of absence.**

Add a test asserting that an empty corpus produces an error, not a pass.

### 5.2 `xtask self-test`

New subcommand: `cargo run -p xtask -- self-test`. For each of the five gates,
run it against a fixture containing a deliberate violation of exactly the
pattern that gate exists to catch, and assert the gate **fails**.

**A gate that passes its violation fixture is itself a failure** and must make
`self-test` exit non-zero, naming the gate.

One fixture per gate, each violating only its own pattern:

| Gate | Fixture must contain |
|---|---|
| `no-fallback-key` | a `change-in-production`-style key literal in non-comment code |
| `rng-no-silent-fallback` | `unwrap_or_default()` / `.ok()` on a `fill_bytes`/`getrandom` call |
| `no-debug-prints` | a debug print of secret-bearing material |
| `cookie-attrs-present` | a cookie construction missing a required attribute |
| `no-plaintext-in-store-ops` | a store operation passing plaintext where a lookup key belongs |

Read each gate's current implementation to get its pattern exactly right — a
fixture that does not actually trip its gate proves nothing, and the self-test
will tell you, which is the point.

### 5.3 Fixtures must not be compiled into the workspace

Fixtures contain deliberately bad code. They must never be built as part of
`cargo build --workspace`, never be published, and never be scanned by the real
gates — a fixture picked up by `library_sources()` would make `release-check`
fail permanently.

Verify this explicitly: after adding the fixtures, `cargo run -p xtask --
release-check` must still pass. If it does not, your fixtures are inside the
scanned corpus and must move.

Location is yours (`xtask/fixtures/` is the obvious choice). Say what you chose.

### 5.4 Negative tests for the non-gate invariants

Per RFC-040 §3.3:

- **INV-5 / INV-6.** A store fixture whose `claim_code` / `consume_form_token`
  reports `changed == 0`; assert the outcome is `Lost` / not-consumed and that
  no session or effect follows. Assert also that `changed > 1` surfaces as
  `StoreError::InvariantViolation` rather than being folded into `Lost`.
- **INV-7.** A compile-failure test: constructing `RedeemSuccess` outside a won
  claim must **not** compile. `trybuild` is approved as a **dev-dependency
  only** (owner, 2026-09-03) — its version goes in `[workspace.dependencies]`
  per DEC-012, and it must not enter any published crate's normal dependency
  tree. The `core-deps` gate is what enforces that boundary; confirm it still
  passes afterwards.
- **INV-8.** An exhaustive match over `RedemptionFailReason` asserting every
  variant maps to the single generic public error. Exhaustive so that adding a
  variant later fails to compile rather than silently escaping the check.

**INV-7 deserves care.** It is the only invariant enforced by the type system
rather than a test, so its guard silently stops protecting anything if the
harness stops running. Make sure the trybuild test is wired into a CI job that
would go red if it vanished.

### 5.5 The map

Extend the existing invariant table in `docs/src/threat-model.md` with two
columns: **Guard** and **Negative test**. Fill every row per RFC-040 §3.3.

INV-4's Negative test cell reads that it is deferred to RFC-041 — **leave the
gap visible.** Do not write "n/a", and do not quietly omit the row.

### 5.6 CI

Run `cargo run -p xtask -- self-test` in CI, blocking, alongside the existing
`release-check` job.

## 6. Acceptance criteria

1. `library_sources()` errors on an empty corpus; a test asserts it.
2. All five gates assert their corpus covers `crates/codlet/src/`.
3. `xtask self-test` exists, runs all five gates against fixtures, and fails if
   any gate passes its fixture.
4. `cargo run -p xtask -- release-check` still passes with fixtures present
   (§5.3).
5. Negative tests exist and pass for INV-5, INV-6, INV-7, INV-8.
6. `docs/src/threat-model.md` carries the map, with INV-4 shown as open.
7. `core-deps` still passes — `trybuild` has not entered a published tree.
8. Full CI green, self-test included.

## 7. Prohibited shortcuts

- Do not write a fixture that trips a gate by accident rather than by the
  intended pattern — if you cannot make a gate fail on a genuine violation of
  its own rule, that is a finding about the gate, and you must report it rather
  than contriving a fixture that passes the self-test.
- Do not weaken a gate's pattern to make a fixture trip it.
- Do not close INV-4 by pointing at an existing test that was not designed to
  prove it.
- Do not add `trybuild` as a normal dependency.
- Do not silence `release-check` if fixtures trip it — move the fixtures.

## 8. Known risks

| Risk | Mitigation |
|---|---|
| A gate cannot be made to fail on a genuine violation | That is the most valuable possible finding — report it immediately; it means the gate never worked |
| Fixtures land inside the scanned corpus | Acceptance criterion 4 catches it |
| `trybuild` leaks into a published dependency tree | `core-deps` catches it; criterion 7 |

## 9. Required evidence

Diff; `xtask self-test` output showing all five gates failing their fixtures;
`release-check` still passing; the four negative tests passing; `core-deps`
output; CI run URL.

## 10. Review request

`.git-exclude/review-request/040-invariant-verification.md`; my result returns
at `.git-exclude/reviewed/040-invariant-verification.md`.
