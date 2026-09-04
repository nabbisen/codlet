# Implementation Handoff — RFC-043 `Alphabet::new` Must Reject Non-Fixed-Point Symbols

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-04
- **Milestone:** M5-6, the closing item
- **Governing RFC:** [`../../accepted/043-alphabet-symbols-must-be-normalization-fixed-points.md`](../../accepted/043-alphabet-symbols-must-be-normalization-fixed-points.md)
- **Priority:** last in M5. RFC-041 merged at `a233a0c`; start from there.

Read RFC-043 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Fix the defect your own P-3 property found: `Alphabet::new` accepts symbols that
`normalize` would alter, which would make every code issued under such an
alphabet permanently unredeemable.

## 2. Change scope

- `crates/codlet/src/code/alphabet.rs` — validation in `Alphabet::new`
- `crates/codlet/src/error.rs` — one new `PolicyError` variant
- `crates/codlet/src/code/alphabet/tests.rs` — un-`#[ignore]` P-3; add
  per-class rejection tests
- `docs/src/threat-model.md` — INV-4 row: remove the open-gap note
- `CHANGELOG.md` — `[Unreleased]`

## 3. Non-change scope

- **`normalize`.** Do not widen it to accept more symbols. The constraint moves
  to the constructor, not the other way round.
- **`generate_code`, `issue_code`, or the redeem path.** RFC-043 §3.1 rejects
  normalizing on the issue path; do not implement it as a belt-and-braces
  addition.
- **`DEFAULT_ALPHABET` and `Alphabet::unambiguous()`** — both are already safe
  and must keep working unchanged.
- **P-3's assertion.** Un-`#[ignore]` it; do not otherwise edit it. It was
  written before the fix and fired against the real defect, which is what makes
  it worth having.

## 4. Required implementation

### 4.1 Validation

`Alphabet::new` gains a check, alongside the existing length / ASCII /
uniqueness rules: every symbol must satisfy `normalize(symbol) == symbol`.

Add a distinct `PolicyError` variant — the existing ones are
`AlphabetTooSmall`, `AlphabetNotAscii`, `AlphabetNotUnique`, so follow that
naming. The error must name the offending byte: a caller who passes a
lowercase-containing alphabet needs to know which symbol was rejected and why,
not merely that construction failed.

Order the check after the ASCII check — `normalize` is defined on `&str`, and
knowing the input is ASCII first keeps the conversion trivial.

### 4.2 Tests

- **Un-`#[ignore]` P-3.** It is the acceptance test. It must now pass.
- One rejection test per class: a lowercase letter, `-`, and an ASCII
  whitespace byte (tab is the one P-3 found in an earlier run).
- A test that `Alphabet::unambiguous()` still constructs successfully — the
  regression that would matter most if the check were written too strictly.
- Confirm the new variant is reachable and asserted, not merely defined.

### 4.3 Threat model

INV-4's row currently carries a **Known open gap** note describing this defect.
Remove that note; the row's Guard and Negative-test columns stay. After this, no
row in that table carries an open gap — state that accurately, and do not claim
more.

### 4.4 CHANGELOG

Record it as a fix with a behaviour change: `Alphabet::new` now rejects symbols
that normalization would alter. Note that this is breaking in the sense that
previously-accepted input is now refused, and that **every such input was
already broken** — no working deployment can be affected. Mention that
`Alphabet::unambiguous()` and `DEFAULT_ALPHABET` are unaffected.

## 5. Required tests

| Command | Must |
|---|---|
| `cargo test -p codlet --all-features` | pass, **including P-3, no longer ignored** |
| `cargo test --workspace` | pass |
| `RUSTUP_TOOLCHAIN=1.85.0 cargo check -p codlet -p codlet-conformance -p codlet-worker -p xtask --all-targets` | pass — note the multi-package form; `-p codlet` alone misses `test-utils` feature unification |
| `cargo run -p xtask -- release-check` and `-- self-test` | pass, four gates |
| `cargo deny check bans licenses sources advisories` | pass (bare — `all-features = true` is set) |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` | pass |

## 6. Acceptance criteria

1. `Alphabet::new` rejects non-fixed-point symbols with a distinct, named error
   identifying the byte.
2. P-3 un-`#[ignore]`d and passing.
3. Per-class rejection tests for lowercase, `-`, and ASCII whitespace.
4. `Alphabet::unambiguous()` still constructs; full suite green.
5. INV-4's threat-model row carries no open gap.
6. No change to `normalize`, `generate_code`, `issue_code`, or
   `DEFAULT_ALPHABET`.
7. Full CI green.

## 7. Prohibited shortcuts

- Do not widen `normalize` instead of narrowing the constructor.
- Do not also normalize on the issue path "for safety" — RFC-043 §3.1 explains
  why that is the worse option, and doing both muddies which mechanism holds
  the invariant.
- Do not weaken P-3 to make it pass.
- Do not claim in the threat model that INV-4 is proven beyond what the tests
  show.

## 8. Known risk

The check could be written too strictly and reject `DEFAULT_ALPHABET` or some
legitimate symbol set. Acceptance criterion 4 is the guard; run it early rather
than at the end.

## 9. Required evidence

Diff; P-3 passing un-ignored; the three rejection tests; the `unambiguous()`
test; the 1.85 check; `release-check`, `self-test`, `cargo deny`; CI run URL.

## 10. Review request

`.git-exclude/review-request/043-alphabet-symbols-must-be-normalization-fixed-points.md`;
my result returns at
`.git-exclude/reviewed/043-alphabet-symbols-must-be-normalization-fixed-points.md`.
