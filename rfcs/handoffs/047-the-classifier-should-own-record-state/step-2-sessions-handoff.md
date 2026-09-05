# Implementation Handoff — RFC-047 Step 2 of 2: The Session Path

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-06
- **Milestone:** M6
- **Governing RFC:** [`../../accepted/047-the-classifier-should-own-record-state.md`](../../accepted/047-the-classifier-should-own-record-state.md)
- **Prerequisite:** step 1 merged and green — met (`1a994fa`, and the RFC-048 follow-up green at `450e423`).

Read RFC-047 before starting, **§3 and §6 in particular**. If execution
conflicts with the RFC, **stop and escalate**.

## 1. This is the dangerous half. Read this section twice.

Step 1 was safe because `claim_code`'s conditional UPDATE independently
re-enforces every condition: a classifier defect there produces a wrong
*reason*, never a claimable expired code.

**Sessions have no such second guard.** `find_active_session` **is** the
enforcement point for session expiry and revocation. Nothing downstream
re-checks. After this change, a defect in `classify_session` authenticates a
session that should have been rejected — an authentication bypass, the most
severe failure class this project has.

RFC-047 §6 names three conditions that must **all** hold. If any cannot be met,
**stop and report; do not ship a partial version of this.** Reverting to the
status quo plus RFC-046's honest documentation is an acceptable outcome. Shipping
a half-migrated session path is not.

1. Codes first, proven — **met**.
2. Conformance inverted to test rejection, on all four adapters in CI.
3. Classifier rejection logic observed failing against a broken classifier.

## 2. Change scope

- `crates/codlet/src/store/session.rs` — `find_active_session`'s contract;
  `ActiveSessionRecord` gains `revoked_at: Option<u64>`
- `crates/codlet/src/state/session.rs` — `classify_session` owns expiry and
  revocation; the RFC-046 C-1 rustdoc notes come off
- `crates/codlet/src/auth/session.rs` — if the call site needs adjusting
- `crates/codlet/src/mem/session.rs`, `codlet-sqlx` (sqlite + postgres),
  `codlet-worker/src/d1/session.rs` — drop the state predicate
- `crates/codlet-conformance/src/session.rs` — **inverted**, §4
- `crates/codlet-worker/tests/` — the D1 mirror, which `run_session_store_conformance` cannot reach
- `CHANGELOG.md`

## 3. Required implementation

### 3.1 Store returns state; classifier decides

`find_active_session` matches on lookup key alone and returns the record
including `expires_at` and `revoked_at`. No expiry or revocation predicate in
any adapter's query.

`classify_session` decides, in the order RFC-047 §8.1 fixed:
**revoked, then expired, then idle timeout, then authenticated.**

Idle timeout already works this way — you are bringing two more conditions to
where the third already lives.

### 3.2 Remove the C-1 notes in the same change that makes them false

`SessionFailure::Expired` and `::Revoked` carry rustdoc saying they are **not
currently produced**. This change makes them produced. Both notes must come off,
along with `NotFound`'s "currently also reported for Expired and Revoked" line.

If step 2 ships without this, the API documents the opposite of what the code
does — the exact defect RFC-046's C-1 existed to prevent, inverted.

### 3.3 The doc comment on `classify_session`

It currently explains that `None` collapses three causes because the store's
filter excludes them. After this change that explanation is wrong. Rewrite it;
do not leave it describing the old contract.

## 4. The conformance suite must invert — this is the dangerous part

`test_expired_session_not_active` and `test_revoked_session_not_active`
currently assert the store **excludes** those rows. They must assert the store
**returns** them and the classifier **rejects** them.

**A suite left asserting exclusion passes against an unmigrated adapter**,
silently leaving that backend enforcing in SQL while codlet believes it enforces
centrally. For sessions that means two possible enforcement points, one of which
you think is not there.

For each inverted test, ask: **does it still pass if an adapter kept its old
filter?** If yes, it is not doing its job. State that you checked this, per
test — as you did in step 1.

D1 cannot run `run_session_store_conformance` (`!Send`, wasm-only), so the JS
mirror needs the equivalent coverage. That was true in step 1 too; my change
list omitted it and you caught it. It is listed here.

## 5. Observed failing — mandatory, RFC-047 §6 condition 3

Break `classify_session` so it returns `Authenticated` for an expired record.
Confirm the conformance suite fails, on more than one backend. Revert; confirm
clean.

Then do it again for a revoked record. **Two conditions, two trials** — a single
trial proves one branch and this change moves two.

## 6. Acceptance criteria

1. No adapter's `find_active_session` carries an expiry or revocation predicate.
2. `classify_session` decides both, in the fixed order.
3. Conformance asserts return-and-reject; each inverted test verified to fail
   against a still-filtered adapter.
4. Two separate broken-classifier trials — expired and revoked — both observed
   failing, reverted.
5. `SessionFailure::Expired` and `::Revoked` reachable, with real-condition
   tests through `SessionManager::validate`.
6. C-1 rustdoc notes removed; `classify_session`'s doc comment rewritten.
7. Idle-timeout behaviour (RFC-044) unchanged, its tests untouched and passing.
8. Full CI green including `postgres-test` and Miniflare.

## 7. Prohibited shortcuts

- Do not keep a filter "in one adapter for now".
- Do not keep filters as defence in depth — owner-resolved, RFC-047 §8.2. Two
  enforcement points that can disagree is worse than one that is wrong.
- Do not leave the C-1 notes in place.
- Do not weaken an inverted test to make an adapter pass.
- Do not proceed if any of §1's three conditions fails. Report.

## 8. Known risks

| Risk | Mitigation |
|---|---|
| A classifier defect authenticates an expired or revoked session | §5's two trials; criterion 4. This is the risk that makes this the dangerous half |
| An adapter missed, still filtering in SQL | Criterion 3 — the only check that catches it |
| C-1 notes left behind, documentation inverted | Criterion 6 |
| Idle-timeout regression while touching the same function | Criterion 7 — RFC-044's tests must pass unmodified |

## 9. Required evidence

Diff; both broken-classifier trials with output; the "does this fail against a
filtered adapter" check per inverted test; real-condition tests for `Expired`
and `Revoked`; RFC-044's idle-timeout tests passing unmodified; CI run URL
including `postgres-test` and Miniflare.

## 10. Review request

`.git-exclude/review-request/047-classifier-owns-record-state-sessions.md`; my
result returns at
`.git-exclude/reviewed/047-classifier-owns-record-state-sessions.md`.

Expect an adversarial review. Step 1 moved a diagnostic; this moves an
authentication decision.
