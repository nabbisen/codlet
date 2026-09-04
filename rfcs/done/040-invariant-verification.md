# RFC-040: Invariant Verification — Every Guard Observed Failing

- **Status:** Implemented (Unreleased)
- **Target milestone:** M5
- **Primary crate(s):** `xtask`, `codlet`, workspace CI
- **Source basis:** RFC-036 §3.4; M4 post-release evaluation; `ROADMAP.md` M5-5 (revised)

## 1. Summary

Establish that each of INV-1 through INV-8 is guarded by a named test or gate,
and that each guard has been **observed failing** against a deliberately
introduced violation — as an automated, repeatable check, not a one-off manual
trial.

Then apply that standard to the five `xtask` release gates, which have never
been seen to fail and which, as currently written, **all fail open on an empty
input corpus**.

## 2. Motivation

M5-5 originally asked that each invariant name a test. M4 demonstrated that a
mapping is not evidence:

- `core-deps` **had** a mapping to RFC-002 §10.5 and still reported success for
  two releases while inspecting an empty string.
- The PostgreSQL conformance suite **had** tests for INV-5 that had never once
  executed, while `docs/src/adapter-matrix-and-config.md` presented the adapter
  as conformant.

In both cases a table said the invariant was covered, and the table was
truthful about the test's existence and silent about whether it ran or could
fail. A guard that cannot fail is indistinguishable from no guard, and is worse
than none, because it also produces a green tick that suppresses inquiry.

### 2.1 The `xtask` gates have the same defect, unfixed

All five gates share this shape:

```rust
fn gate_no_fallback_key() -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in library_sources() { … }
    if hits.is_empty() { Ok(()) } else { Err(hits.join("; ")) }
}
```

`library_sources()` walks `crates/` and returns whatever it finds. **If it
returns nothing, every gate returns `Ok(())`.** No gate asserts that its corpus
is non-empty, or that it covers the crates it claims to guard. A directory
rename, a crate moved outside `crates/`, or any change to the traversal would
turn all five security gates green and silent — simultaneously.

This is precisely the `core-deps` failure, still live, in the checks that
enforce the project's most security-critical invariants. It has not caused an
incident only because `CARGO_MANIFEST_DIR` is resolved at compile time and the
layout has not moved.

Additionally, these gates are grep-based. Each searches for a *known* bad
pattern; a violation expressed differently passes. That is an accepted limit of
the technique, but it means the gates' value depends entirely on their patterns
matching real violations — which has never been tested.

## 3. Decision

### 3.1 A gate must fail when it cannot perform its check

Every gate asserts its own preconditions before drawing a conclusion:

- `library_sources()` returns an error, not an empty vector, when it finds no
  sources.
- Each gate asserts the corpus contains the crates it claims to guard — at
  minimum that `crates/codlet/src/` is represented — and fails loudly otherwise.

This generalises RFC-036 §3.5 from CI shell scripts to the `xtask` gates.
**Absence of evidence is never reported as evidence of absence.**

### 3.2 Gate self-test — automated, not a manual trial

RFC-036 required the `core-deps` gate to be *seen* failing, verified by a manual
trial. A manual trial proves the gate worked once, on one day, and rots
immediately.

Add `cargo run -p xtask -- self-test`: for each gate, run it against a fixture
tree containing a deliberate violation of exactly the pattern that gate exists
to catch, and assert the gate **fails**. A gate that passes its violation
fixture is itself a failure.

Fixtures live beside `xtask` and are never compiled into the workspace build.
The self-test runs in CI alongside `release-check`.

This converts "observed failing" from an event into a property.

### 3.3 The invariant map

Each invariant names its guard **and** the negative test proving that guard can
fail. Both columns are required; a row with an empty third column is not
complete.

| INV | Guard | Negative test |
|---|---|---|
| INV-1 secrets stored only as HMAC | `xtask` `no-plaintext-in-store-ops` | self-test fixture (§3.2) |
| INV-2 no fallback key | `xtask` `no-fallback-key` | self-test fixture |
| INV-3 RNG failure fails closed | `xtask` `rng-no-silent-fallback` | self-test fixture |
| INV-4 normalization identical and idempotent | property test (RFC-041) | *to be established by RFC-041* |
| INV-5 `claim_code` conditional UPDATE, `changed == 0` never proceeds | `codlet-conformance` concurrent claim test, per adapter | adapter fixture that returns `changed == 0`; assert `Lost`, never `Won` |
| INV-6 `consume_form_token` likewise | `codlet-conformance` token consume test | as INV-5 |
| INV-7 session issuance requires `RedeemSuccess` | compile-time — `RedeemSuccess` is unconstructible outside a won claim | `rfc_009_compile`-style trybuild negative case: constructing it must not compile |
| INV-8 all failures map to one public error | `rfc_008_012_acceptance` | exhaustive match over `RedemptionFailReason`, asserting a single public variant |

INV-4's guard is deferred to RFC-041 (property and fuzz testing) and is the one
row this RFC does not close. It is listed so the gap is visible rather than
absent.

INV-7 is notable: it is the only invariant enforced by the type system rather
than by a test. Its negative test must therefore assert a **compile failure**,
which is a different mechanism from every other row — and one that silently
stops protecting anything if the test harness stops running.

### 3.4 Where the map lives

`docs/src/threat-model.md`, extending the existing invariant table with the two
new columns. The threat model is where a reviewer looks; a map in a test file is
a map nobody reads.

## 4. Non-goals

- No new invariants. INV-1…8 are unchanged.
- Not a rewrite of the gates from grep to static analysis. Their pattern-matching
  limits are accepted; this RFC ensures they can fail, not that they are
  exhaustive.
- No property or fuzz testing — RFC-041.
- No change to `codlet-conformance`'s adapter contract.

## 5. Security considerations

This RFC strengthens no invariant and weakens none. It changes what the project
is entitled to *claim*.

The honest position after M4 is that INV-1, INV-2, INV-3, and INV-8 are guarded
by checks that have never been demonstrated capable of failing, and INV-5 was
demonstrated only for SQLite, in-memory, and D1 — PostgreSQL joined that list
during M4, on 2026-09-03. Until §3.2 lands, "the gate passes" means only that.

## 6. Testing and verification

The deliverable *is* the verification. Exit criteria:

1. `cargo run -p xtask -- self-test` exists, runs in CI, and passes.
2. Every gate fails its violation fixture — proven by the self-test itself,
   which fails if any gate passes one.
3. `library_sources()` returning empty is an error, with a test asserting it.
4. Every row of §3.3 except INV-4 has both a guard and a negative test that has
   been observed failing.
5. `docs/src/threat-model.md` carries the map.

## 7. Alternatives considered

1. **Keep manual trials per RFC-036 §3.4.** Rejected — proves a gate worked
   once. The self-test makes it a standing property.
2. **Rewrite the gates as lint rules or static analysis.** Rejected for M5:
   large, and orthogonal to whether a check can fail. Revisit if the grep
   patterns prove inadequate once the fixtures exist.
3. **Track the map in a test file rather than the threat model.** Rejected —
   §3.4.

## 8. Open questions

1. Should the self-test fixtures live under `xtask/fixtures/` or a dedicated
   `xtask/tests/`? Implementation detail; implementer's call, provided they are
   excluded from the workspace build.
2. ~~INV-7's compile-failure test needs a harness (`trybuild` or equivalent),
   a new dev-dependency.~~ **Resolved: approved by the owner, 2026-09-03**,
   accepting this RFC including the recommendation. `trybuild` (or equivalent)
   may be added as a **dev-dependency only** — it must not appear in any
   published crate's normal dependency tree, and the `core-deps` CI gate
   remains the check that enforces that boundary. Its version belongs in
   `[workspace.dependencies]` per DEC-012.

## 9. Acceptance criteria

- All five gates assert a non-empty, expected corpus and fail otherwise.
- `xtask self-test` implemented, wired into CI, green.
- Each gate demonstrably fails its fixture.
- §3.3 map complete except INV-4, recorded in the threat model.
- No invariant described as verified on the strength of a test's existence.
