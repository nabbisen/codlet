# Implementation Handoff — RFC-042 Retire `cookie-attrs-present`

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M5
- **Governing RFC:** [`../../accepted/042-retire-the-cookie-attrs-gate.md`](../../accepted/042-retire-the-cookie-attrs-gate.md)
- **Priority:** immediately, ahead of committing RFC-040. Apply on top of your uncommitted RFC-040 working tree.

Read RFC-042 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Remove a release gate that cannot detect the regression it exists to prevent,
and correct the two live documents that claim otherwise.

## 2. Background

Your C-1 fixture proved it: given documentation naming all three attributes and
a builder emitting two, `cookie-attrs-present` passes. `check_cookie_attrs`
greps the whole file text, and `enum SameSitePolicy`'s own variant literals
(`"SameSite=Strict"` and friends) satisfy it regardless of what the builder
emits.

The invariant itself was never unguarded.
`crates/codlet/src/cookie/tests.rs::set_cookie_contains_required_attributes`
asserts on the **emitted string**, which is the property that matters. The gate
was a redundant second claim that could not deliver.

## 3. Change scope

- `xtask/src/main.rs` — remove the gate and its `check_` function; correct the
  `library_sources` doc comment
- `xtask/fixtures/cookie.rs` — delete
- `SECURITY.md` — release-discipline item 16
- `docs/src/threat-model.md` — the "Cookie leakage via JS" paragraph
- `CHANGELOG.md` — `[Unreleased]`

## 4. Non-change scope — read this carefully

- **`crates/codlet/src/cookie.rs` and `cookie/tests.rs`.** Not one byte. The
  tests are now the sole guard and they already pass; touching them here would
  confuse "we removed a broken check" with "we changed how cookies work".
- **No emitted cookie changes.** Nothing about codlet's runtime behaviour is in
  scope.
- **The other four gates**, their fixtures, and `self-test`'s structure.
- **Historical records — do not correct their "five gates" wording.** Sixteen
  places say five; only the two in §5.3 may change. `CHANGELOG.md`,
  `rfcs/done/014-…`, `rfcs/accepted/040-…` and its handoff all describe the past
  accurately. RFC-040's motivation *is* that five gates shared a defect — that
  was true when written. Rewriting them would be the falsification every
  handoff in this milestone has prohibited.

## 5. Required implementation

### 5.1 Remove the gate

Delete `gate_cookie_attrs`, `check_cookie_attrs`, its entry in the `GATES`
table, its `self-test` fixture pairing, and `xtask/fixtures/cookie.rs`.

Leave no commented-out remnant. If someone later wants this idea back, RFC-042
records why it did not work.

### 5.2 Correct the `library_sources` doc comment

It currently says an empty vector "let all five gates in `release_check` report
`Ok(())`". Make it four. This comment is describing present behaviour, so it is
in scope where the historical documents are not.

### 5.3 The two live documents

**`SECURITY.md`**, release-discipline item 16: "5 static security gates" → four.

**`docs/src/threat-model.md`**, the "Cookie leakage via JS" paragraph, currently:

> Session cookies are `HttpOnly` by default. The `cookie-attrs-present` gate
> ensures this cannot be accidentally removed.

The second sentence is false. Replace it with a citation of what does ensure it —
the behavioural tests in `crates/codlet/src/cookie/tests.rs`, which assert on the
emitted `Set-Cookie` string across the production, lax, and development
profiles. Name the test file so a reviewer can follow it.

Do **not** simply delete the sentence. A threat model that claims nothing
guards an invariant is as inaccurate as one that names the wrong guard.

### 5.4 CHANGELOG

Record what was removed and why, in one entry: the gate could not detect a
builder that stopped emitting an attribute, because it matched file text
including documentation and enum literals; the invariant remains guarded by the
behavioural tests, which is where it always was.

State plainly that this removes a check. A reader scanning for security-relevant
changes should not have to infer that from the phrasing.

## 6. Required tests

| Command | Must |
|---|---|
| `cargo run -p xtask -- self-test` | **green**, four gates |
| `cargo run -p xtask -- release-check` | green, four gates |
| `cargo test -p codlet --lib cookie` | pass, unmodified |
| `cargo test --workspace` | pass |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo deny check bans licenses sources` | pass (RFC-039 is merged; keep it green) |

## 7. Acceptance criteria

1. `cookie-attrs-present` gone from `xtask`; no fixture, no remnant.
2. `self-test` green with four gates — this is what unblocks RFC-040's commit.
3. `release-check` green with four gates.
4. `SECURITY.md` and the `library_sources` comment say four.
5. `docs/src/threat-model.md` cites the behavioural tests.
6. `git diff` on `crates/codlet/src/cookie.rs` and `cookie/tests.rs` is **empty**.
7. No historical record edited — `git diff CHANGELOG.md` shows only a new
   `[Unreleased]` entry, and `git diff rfcs/` shows nothing outside this
   handoff's own directory.

## 8. Prohibited shortcuts

- Do not repair the gate instead of removing it. RFC-042 §7 records why every
  repair considered was rejected.
- Do not touch `cookie.rs` or its tests.
- Do not edit CHANGELOG history or any RFC body to fix a "five gates" count.
- Do not delete the threat-model sentence without replacing it.

## 9. Commit sequence

Two commits, in this order, both pushed together:

1. **RFC-042** — this work.
2. **RFC-040** — the invariant-verification work you have been holding, which
   is already approved and now has a green `self-test` to land on.

Reversing them would put a knowingly-red blocking CI job on `main`.

## 10. Required evidence

Diff; `self-test` and `release-check` output showing four gates green; the
cookie tests passing unmodified; `git diff --stat` proving criteria 6 and 7;
CI run URL after push.

## 11. Review request

`.git-exclude/review-request/042-retire-the-cookie-attrs-gate.md`; my result
returns at `.git-exclude/reviewed/042-retire-the-cookie-attrs-gate.md`.
