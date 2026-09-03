# Implementation Handoff — RFC-041 Property and Distribution Testing

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M5, final item
- **Governing RFC:** [`../../accepted/041-property-and-distribution-testing.md`](../../accepted/041-property-and-distribution-testing.md)
- **Priority:** last in M5. RFC-039, 040, 042 are merged and `main` is 22/22 green — start from there.

Read RFC-041 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Close INV-4 — the only invariant in the threat model with no guard — and replace
example-based coverage of the rejection sampler with exact assertions.

## 2. Change scope

- `crates/codlet/src/code/normalize/tests.rs` — or a new sibling for the
  properties, your call on file split per the project's line-count convention
- `crates/codlet/src/code/generate/tests.rs`
- `crates/codlet/src/code/alphabet/tests.rs`
- `Cargo.toml` — `[workspace.dependencies]`, if `proptest` is used
- `crates/codlet/Cargo.toml` — `[dev-dependencies]`
- `docs/src/threat-model.md` — the INV-4 row
- `CHANGELOG.md` — `[Unreleased]`

## 3. Non-change scope

- **`normalize`, `generate_code`, `Alphabet`, `unbiased_ceiling`, or any
  behaviour.** This handoff observes; it does not change what the code does.
- **No fuzzing harness.** M5-4 is formally deferred (RFC-041 §4, §8.1). Do not
  add `cargo-fuzz`, a nightly job, or a corpus.
- No new invariants; INV-4's wording is unchanged.
- No repair of anything P-3 uncovers — see §6.

## 4. Required implementation

### 4.1 The seven properties

Implement P-1 through P-7 exactly as RFC-041 §3.1 and §3.2 state them. Two
deserve emphasis:

**P-3 (alphabet safety)** is the one with teeth. For any `Alphabet`, every
symbol must be a normalization fixed-point. `Alphabet::new` validates only
`len >= 2`, so it currently accepts lowercase letters and `-`. If P-3 fires,
that is the point of writing it.

**P-5 through P-7 need no property framework.** They are exhaustive over their
input domains — 256 byte values, and alphabet lengths 2..=256. Write them as
plain deterministic tests. The default alphabet has 31 symbols and
`256 % 31 == 8`, so the ceiling is 248 and each symbol is reachable from exactly
8 byte values; assert that equality, not a tolerance.

Use the existing deterministic test RNG to drive P-6 with known bytes.

### 4.2 Generator coverage assertions — mandatory

For P-1 through P-4, assert the input corpus actually contains the interesting
shapes: hyphens, ASCII whitespace, lowercase letters, non-ASCII characters, and
the empty string.

This is RFC-041 §3.3 and it is not optional. A property over a corpus of
`"ABC"`-alikes passes for the wrong reason and is indistinguishable from one
that passes for the right one. No property-testing framework will tell you
which you have — only an explicit assertion will.

### 4.3 Every property observed failing

Same standard as RFC-036 §3.4 and RFC-040 §3.2. For each property, temporarily
break the implementation it guards, confirm the property **fails**, restore, and
record both outputs. Suggested breakages:

| Property | Break |
|---|---|
| P-1, P-4 | `normalize` lowercases instead of uppercasing |
| P-2, P-3 | add `'-'` or a lowercase letter to the alphabet under test |
| P-5 | map with `byte % len` and no ceiling |
| P-6 | accept bytes at or above the ceiling |
| P-7 | `unbiased_ceiling` returns 256 |

Restore fully after each; confirm `git status --short` clean before the next.

### 4.4 `proptest`, conditionally

Dev-dependency only, version in `[workspace.dependencies]` per DEC-012.

**Hard condition: it must build on Rust 1.85.** Verify with
`RUSTUP_TOOLCHAIN=1.85.0 cargo check -p codlet --all-targets` before committing
to it. If it does not hold, use the hand-rolled deterministic generator fallback
(RFC-041 §3.4) and say so in your review request.

**Do not raise the MSRV to accommodate a test dependency.** If you find yourself
considering it, stop and escalate.

Confirm `core-deps` still passes — `proptest` must not reach any published
crate's normal dependency tree.

### 4.5 Threat model

Fill the INV-4 row's Guard and Negative-test columns. After this, no row in that
table is open.

## 5. Required tests

| Command | Must |
|---|---|
| `cargo test -p codlet --all-features` | pass, including all seven properties |
| `cargo test --workspace` | pass |
| `RUSTUP_TOOLCHAIN=1.85.0 cargo check -p codlet --all-targets` | pass |
| `cargo run -p xtask -- release-check` and `-- self-test` | pass, four gates |
| `cargo deny check bans licenses sources advisories` | pass — run it **bare**; `deny.toml` now sets `all-features = true` so this matches CI |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` | pass |

## 6. If P-3 fires — escalate, do not fix

P-3 failing means `Alphabet::new` accepts symbol sets that break INV-4. The
remedy is a public-API question — constrain `Alphabet::new`, or normalize on the
issue path — and RFC-041 §8.2 leaves it open deliberately until the test says
whether it fires.

**Report it with the failing symbol set and stop.** Do not constrain
`Alphabet::new`, do not change `issue_code`, and do not narrow the property to
make it pass. A property weakened until it passes is worth less than no property,
because it also carries a claim.

## 7. Acceptance criteria

1. P-1…P-7 implemented and passing (or P-3 reported as firing, per §6).
2. Generator coverage assertions present for P-1…P-4.
3. Every property observed failing against a deliberate breakage; both outputs
   recorded; tree clean after each.
4. `proptest` dev-only and 1.85-compatible, or the fallback used and explained.
5. `core-deps` green; `msrv` job green.
6. INV-4 row complete in the threat model; no open rows.
7. No change to `normalize`, `generate_code`, or `Alphabet`.
8. Full CI green.

## 8. Prohibited shortcuts

- Do not weaken a property to make it pass.
- Do not skip the coverage assertions because the properties already pass.
- Do not use a statistical test where RFC-041 §3.2 specifies an exact one.
- Do not raise the MSRV for a dev-dependency.
- Do not fix a P-3 failure.

## 9. Required evidence

Diff; all seven properties passing; the breakage output for each; `git status`
clean after each trial; the 1.85 check; `core-deps` output; CI run URL.

## 10. Review request

`.git-exclude/review-request/041-property-and-distribution-testing.md`; my
result returns at
`.git-exclude/reviewed/041-property-and-distribution-testing.md`.
