# Implementation Handoff — RFC-046 Host-Visible Session Failure Reasons

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-04
- **Milestone:** M6
- **Governing RFC:** [`../../accepted/046-host-visible-session-failure-reasons.md`](../../accepted/046-host-visible-session-failure-reasons.md)
- **Priority:** may run in parallel with RFC-044. No dependency between them, with one coordination point — §4.3.

Read RFC-046 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Let the host learn *why* a session failed, without changing anything the end
user sees.

## 2. Change scope

- `crates/codlet/src/state/session.rs` — `SessionFailure`; `Unauthenticated`
  becomes a struct variant
- `crates/codlet/src/auth/session.rs` — populate the reason
- `crates/codlet/src/lib.rs` — re-export
- Call sites and tests updated for the new variant shape
- `docs/src/threat-model.md` or the session docs — the do-not-render warning
- `CHANGELOG.md`

## 3. Non-change scope

- **`PublicSessionError`.** Not one line. RFC-046 §3.1's whole argument rests on
  these two types staying apart.
- **The redemption path.** `PublicRedemptionError`, `RedemptionFailReason`, and
  INV-8 are untouched. If you find yourself editing anything under
  `auth/code.rs`, stop.
- `is_authenticated()` and `subject()` — behaviour unchanged.
- No store changes. Every reason is derivable from what the store already
  returns.

## 4. Required implementation

### 4.1 The enum, and where it lives

Six variants: `NoCookie`, `Malformed`, `NotFound`, `Expired`, `IdleTimeout`,
`Revoked`.

**In `state`, beside the outcome — not in `error`.** That is deliberate physical
separation (RFC-046 §3.2): a contributor reaching into `error` for a public type
must not find these next to `PublicSessionError`.

### 4.2 The boundary must be enforced, not just documented

Add a test asserting **no conversion exists** between `SessionFailure` and
`PublicSessionError` — no `From`, no `Into`, no helper that maps one to the
other. A compile-fail test is the right shape if a runtime one cannot express
it.

This is the acceptance criterion that matters most. Everything else here is
mechanical; this is the one that keeps the RFC's argument true a year from now.

### 4.3 `IdleTimeout` and RFC-044

`IdleTimeout` is only reachable once RFC-044 lands. Define the variant now — the
enum should not change shape twice — and if RFC-044 is not yet merged when you
implement, leave it unreachable with a comment naming RFC-044, and say so in
your review request.

**Do not implement idle-expiry detection here** to make the variant reachable.
That is RFC-044's work and duplicating it would give two implementations of one
rule.

### 4.4 Each reason produced by a real condition

Every variant needs a test that reaches it through the actual condition — revoke
a session and validate; let one expire; present a truncated cookie — **not** by
constructing the enum value directly. A test that builds the value proves the
enum compiles, not that the code path produces it.

### 4.5 Documentation

The do-not-render warning goes **on the type**, in rustdoc, not only in the RFC.
State that `Expired` and `IdleTimeout` are safe to surface as "your session
ended", while `NotFound` and `Revoked` distinguish states an unauthenticated
visitor should not learn.

### 4.6 CHANGELOG

Breaking: exhaustive matches on `SessionValidationOutcome` must change. Include
a one-line migration example. Say plainly that it is breaking rather than
leaving it to inference.

## 5. Acceptance criteria

1. Six variants, in `state`, on `Unauthenticated`.
2. **No conversion to `PublicSessionError`, enforced by a test.**
3. Each variant produced by a test exercising the real condition.
4. `is_authenticated()` and `subject()` unchanged, with their existing tests
   passing untouched.
5. Rustdoc carries the do-not-render warning.
6. `PublicSessionError` and the whole redemption path unmodified — `git diff`
   proves it.
7. CHANGELOG records the break with a migration line.
8. Full CI green.

## 6. Prohibited shortcuts

- Do not add a `From<SessionFailure> for PublicSessionError`, however convenient.
- Do not put `SessionFailure` in `error`.
- Do not implement idle-expiry logic here (§4.3).
- Do not produce a variant in a test by constructing it directly.
- Do not weaken the redemption path's error collapse to make the two paths
  "consistent" — they are deliberately different, for the reason RFC-046 §3.1
  gives.

## 7. Known risk

The realistic failure mode is a host echoing the reason to an end user. codlet
cannot prevent that; §4.2's separation and §4.5's warning are the mitigation.
Write the warning as if for someone who will paste the value into a template,
because eventually someone will.

## 8. Required evidence

Diff; the no-conversion test; a test per variant with the condition that
produced it; `git diff` on `error.rs` and `auth/code.rs` showing no change; CI
run URL.

## 9. Review request

`.git-exclude/review-request/046-host-visible-session-failure-reasons.md`; my
result returns at
`.git-exclude/reviewed/046-host-visible-session-failure-reasons.md`.
