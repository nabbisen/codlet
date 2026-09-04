# Implementation Handoff — RFC-047 Step 1 of 2: The Code Path

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-05
- **Milestone:** M6
- **Governing RFC:** [`../../accepted/047-the-classifier-should-own-record-state.md`](../../accepted/047-the-classifier-should-own-record-state.md)
- **Scope:** **the code path only.** The session path is step 2 and has its own handoff, written only after this one is green in CI.

Read RFC-047 before starting — §3 in particular, which explains why this step is
the code path and not the session path. If execution conflicts with the RFC,
**stop and escalate**.

## 1. Purpose

Move expiry and revocation decisions for one-time codes out of four adapters'
WHERE clauses and into one classifier.

## 2. Why this step is safe, and what makes it safe

`claim_code`'s conditional UPDATE independently enforces
`used_at IS NULL AND revoked_at IS NULL AND expires_at > ?`. If the new
classifier is wrong, an expired code still cannot be claimed — INV-5 rests on
the UPDATE, not on `find_redeemable`'s filter.

**That guard is the entire reason this step goes first. Do not touch
`claim_code`.** Not to "keep it consistent", not to remove a now-duplicated
predicate. Its redundancy is the safety property this step is built on.

## 3. Change scope

- `crates/codlet/src/store/code.rs` — `find_redeemable` contract;
  `RedeemableCode` gains `used_at: Option<u64>`, `revoked_at: Option<u64>`
- `crates/codlet/src/state/` — new classifier for code lookup
- `crates/codlet/src/auth/code.rs` — `find()` uses the classifier instead of
  `ok_or_else(NotFound)`
- `crates/codlet/src/mem/code.rs`, `codlet-sqlx` (sqlite + postgres),
  `codlet-worker/src/d1/code.rs` — drop the state predicate from the lookup
- `crates/codlet-conformance/src/code.rs` — **inverted** tests, §5
- `crates/codlet/src/state/session.rs` — the one-line rustdoc fix in §6
- `CHANGELOG.md`

## 4. Non-change scope

- **`claim_code`, in any adapter.** §2.
- **The session path.** `find_active_session`, `classify_session`,
  `SessionFailure`. That is step 2, and mixing them forfeits the sequencing.
- `PublicRedemptionError` and the public collapse. `Expired` and `Revoked`
  become reachable as *internal* reasons; every one still maps to
  `InvalidOrExpired`. INV-8 is untouched.
- Rate limiting, form tokens, `RedemptionFailReason::AlreadyUsed` (already
  produced from the claim path — verified; do not re-route it).

## 5. Required implementation

### 5.1 Store returns state; adapter stops deciding

`find_redeemable` selects by lookup key (and scope, as today) and returns the
record **including** `used_at` and `revoked_at`, with no expiry or revocation
predicate in the query.

Every adapter. If one keeps its filter, that backend silently continues
enforcing in SQL while codlet believes it enforces centrally — the exact
divergence this RFC exists to remove.

### 5.2 The classifier

A pure function taking the record and `now`, returning the outcome. **Decision
order is fixed** (RFC-047 §8.1, owner-resolved): **revoked, then expired, then
used, then redeemable.** A record can be several at once; revoked wins because
it is the only state an operator caused deliberately.

`None` from the store remains `NotFound`.

### 5.3 `auth/code.rs`

Replace the `ok_or_else(|| … NotFound)` with the classifier's outcome, mapping
each to the `RedemptionFailReason` it already has. The audit event now carries
the true reason instead of `NotFound` for every case.

**The public error must not change.** `PublicRedemptionError::from_reason`
already collapses all of these to `InvalidOrExpired`; verify by test that it
still does for the newly reachable reasons.

## 6. The dangerous part — inverting the conformance suite

RFC-047 §4.3, and read it before writing any test code.

The suite currently asserts `find_redeemable` **excludes** expired and revoked
rows. It must now assert the store **returns** them and the classifier
**rejects** them.

**A suite left asserting exclusion will pass against an unmigrated adapter.**
That is worse than an implementation bug, because it is what would hide one.
When you change these tests, check each one: does it still pass if an adapter
keeps its old filter? If yes, the test is not doing its job.

Also required, per RFC-047 §6 condition 3 and the M5 standard: **the
classifier's rejection logic must be observed failing** against a deliberately
broken classifier — one that returns `Redeemable` for an expired record. Record
the output, revert, confirm clean.

### 6.1 Incidental, fold in here

`SessionFailure`'s rustdoc says the no-conversion test lives "in this module's
`tests`". It lives at
`crates/codlet/tests/rfc_046_no_public_conversion_compile_fail.rs`. One line;
RFC-047 §10.

## 7. Required tests

| Test | Must |
|---|---|
| Expired code: store returns it, classifier rejects | pass, all four adapters |
| Revoked code: same | pass, all four adapters |
| Used code: same | pass |
| Decision order: revoked+expired record classifies `Revoked` | pass |
| Newly reachable reasons still collapse publicly | `InvalidOrExpired` for every one |
| Broken classifier | observed failing, reverted |
| `claim_code` still rejects an expired code | **unchanged and passing** — the guard this step relies on |
| Full workspace, gates, MSRV, `cargo deny` | pass |

## 8. Acceptance criteria

1. No adapter's `find_redeemable` carries an expiry/revocation predicate.
2. Classifier owns the decision, in the fixed order.
3. Conformance asserts return-and-reject, and each such test fails against an
   adapter that keeps its filter — state that you checked this.
4. Rejection logic observed failing against a broken classifier.
5. `Expired` and `Revoked` reachable, with real-condition tests.
6. Public error surface unchanged, proven by test.
7. `claim_code` untouched — `git diff` proves it.
8. Session path untouched — `git diff` proves it.
9. Full CI green, including PostgreSQL and D1.

## 9. Prohibited shortcuts

- Do not touch `claim_code` (§2).
- Do not start the session path.
- Do not leave a filter in "one adapter for now".
- Do not keep a filter as defence in depth — owner-resolved, RFC-047 §8.2.
- Do not report criterion 3 without having actually tried a still-filtered
  adapter.

## 10. Known risk

If an adapter is missed, that backend enforces in SQL while codlet believes it
enforces centrally — and everything passes. Criterion 3 is the only thing that
catches it. Treat it as the acceptance criterion, not paperwork.

## 11. Required evidence

Diff; all §7 tests; the broken-classifier output; the "does this test fail
against a filtered adapter" check for each inverted test; `git diff` proving
`claim_code` and the session path untouched; CI run URL.

## 12. Review request

`.git-exclude/review-request/047-classifier-owns-record-state-codes.md`; my
result returns at
`.git-exclude/reviewed/047-classifier-owns-record-state-codes.md`.
