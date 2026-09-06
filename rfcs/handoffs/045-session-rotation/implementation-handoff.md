# Implementation Handoff — RFC-045 Session Rotation on Privilege Change

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-06
- **Milestone:** M6 — **the last item**
- **Governing RFC:** [`../../accepted/045-session-rotation.md`](../../accepted/045-session-rotation.md)
- **Prerequisites:** RFC-044 (the first `SessionStore` mutation) and RFC-047 step 2 — both merged and green (`67ab9ed`).

Read RFC-045 before starting, **§3.2 and §3.3 in particular**. If execution
conflicts with the RFC, **stop and escalate**.

## 1. Purpose

Let a host replace a live session's secret without ending the session, at the
moment the subject's authorization changes.

## 2. The signature, stated once so it is not ambiguous

RFC-045 §3's code block predates §8's resolution. The final shape includes the
reason:

```rust
pub async fn rotate<R: RandomSource>(
    &self,
    current: &SessionValidationOutcome,
    new_session_id: SessionId,
    reason: &str,
    rng: &mut R,
) -> Result<IssuedSession, SessionError>;
```

`reason` is host-supplied and recorded in the audit event (RFC-045 §8, resolved
by owner acceptance). Parameter order is yours; the set is not.

Taking a `SessionValidationOutcome` rather than a raw cookie is the point:
rotation can only follow a validation in the same request. **The type carries
the precondition, the same way `RedeemSuccess` carries INV-7's.**

## 3. Change scope

- `crates/codlet/src/auth/session.rs` — `rotate`
- `crates/codlet/src/audit.rs` — a rotation event, and a distinct
  failed-revoke event
- `crates/codlet-conformance/src/session.rs` — rotation coverage
- `crates/codlet-worker/tests/` — the D1 mirror, which
  `run_session_store_conformance` cannot reach
- `crates/codlet/tests/` — acceptance tests; the compile-fail test
- `docs/`, `CHANGELOG.md`

## 4. Non-change scope

- **`issue`, `validate`, `revoke`.** Rotation composes existing store
  operations; it does not modify them.
- **No new store trait method.** `insert_session` and `revoke_session` are
  sufficient. If you find yourself wanting one, stop and escalate — that is a
  contract change and this handoff does not authorise it.
- **The absolute expiry model.** §5.2.
- **`classify_session`**, the RFC-047 work, and the code path.
- No automatic or scheduled rotation. Host-triggered only.

## 5. Required implementation

### 5.1 Order: insert, then revoke. Never the reverse.

Between the two writes both records are briefly valid. That is deliberate and
RFC-045 §3.2 explains why: revoke-first leaves a window where *neither* is
valid, so a concurrent in-flight request from the same subject gets logged out
by a security improvement.

If you find yourself reasoning that revoke-first is "safer", re-read §3.2 before
acting on it.

### 5.2 The absolute expiry does not move

The new record carries the **same `expires_at`** as the old one. Fresh
`created_at`, fresh `last_seen_at`.

A host rotating on a schedule must not thereby grant an unbounded session. This
needs its own test asserting the carried-forward value, not merely that the new
session validates.

### 5.3 A failed revoke returns success, with an audit event

If the insert succeeds and the revoke fails, `rotate` returns the new
`IssuedSession` **and** emits a distinct audit event naming the un-revoked
session id.

Returning `Err` would leave the host holding a fresh cookie it does not know is
valid — worse in every direction. This is the same shape as RFC-044's failed
`touch`: fail closed on the authentication question, fail open on bookkeeping.

**Test it** with a store fixture whose `revoke_session` always fails: assert the
new session works, the old one is still valid (it was not revoked), and the
event fired.

### 5.4 A consequence of RFC-047 that sharpens one test

Before RFC-047 step 2, a revoked session classified as `NotFound`. It now
classifies as **`Revoked`**.

So the acceptance criterion "the old secret does not validate after rotation"
becomes testable precisely: assert `Unauthenticated { reason: Revoked }`, not
merely "not authenticated". Do that — it distinguishes a revoked old session
from one that was never found, which is exactly the confusion RFC-046 existed
to end.

### 5.5 The compile-fail test

`rotate` must be uncallable without an `Authenticated` outcome. Prove it with a
`trybuild` compile-fail case, mirroring RFC-040's INV-7 harness.

A runtime check that returns an error for `Unauthenticated` is **not** the same
guarantee and is not what RFC-045 §3 specifies.

## 6. Required tests

| Test | Must |
|---|---|
| Rotation succeeds; new secret validates | pass |
| Old secret after rotation | `Unauthenticated { reason: Revoked }` — §5.4 |
| `expires_at` carried forward unchanged | asserted directly, not inferred |
| Failed revoke | returns `Ok`, old session still valid, audit event fired |
| `rotate` on an `Unauthenticated` outcome | **does not compile** |
| Reason recorded in the audit event | asserted |
| Conformance, four adapters | pass, including D1 via Miniflare and PostgreSQL in CI |
| RFC-044 idle-timeout and RFC-047 tests | unchanged and passing |

## 7. Acceptance criteria

1. Insert precedes revoke, proven by the §5.3 failure fixture.
2. `expires_at` carried forward, tested.
3. Failed revoke returns `Ok` with an audit event, tested.
4. Compile-fail test proves the type-level precondition.
5. Old secret reports `Revoked`, not merely unauthenticated.
6. Reason string reaches the audit event.
7. No new store trait method; `issue`/`validate`/`revoke` unchanged.
8. Full CI green including `postgres-test` and Miniflare.

## 8. Prohibited shortcuts

- Do not revoke first.
- Do not extend `expires_at`.
- Do not return `Err` on a failed revoke.
- Do not substitute a runtime check for the compile-fail test.
- Do not add a store trait method — escalate instead.
- Do not implement rotation-on-every-request, or a grace window. RFC-045 §4
  rejects both; a grace window needs its own RFC.

## 9. Known risks

| Risk | Mitigation |
|---|---|
| Revoke-first "for safety" | §5.1; criterion 1's fixture would not detect it — read §3.2 |
| Absolute expiry silently extended | Criterion 2, asserted on the value |
| Failed revoke turned into an error | Criterion 3 |
| Two live sessions after a failed revoke | Accepted residual risk, RFC-045 §6. Bounded by the original expiry, surfaced by the audit event, not eliminated. Do not attempt to fix it here — an atomic swap is unavailable on D1 |

## 10. Required evidence

Diff; all §6 tests; the failed-revoke fixture output; the compile-fail test;
conformance on four adapters; CI run URL.

## 11. Review request

`.git-exclude/review-request/045-session-rotation.md`; my result returns at
`.git-exclude/reviewed/045-session-rotation.md`.

This completes M6. Expect the review to check the ordering and the expiry
carry-forward first — they are the two places where a plausible-looking
implementation is wrong.
