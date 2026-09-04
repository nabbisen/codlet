# RFC-045: Session Rotation on Privilege Change

- **Status:** Proposed
- **Target milestone:** M6
- **Primary crate(s):** `codlet`, adapters, `codlet-conformance`
- **Source basis:** `ROADMAP.md` M6; OWASP Session Management Cheat Sheet
- **Depends on:** RFC-044 (introduces the first `SessionStore` mutation)

## 1. Summary

Let a host replace a live session's secret without ending the session:
`SessionManager::rotate` issues a new secret, invalidates the old one, and
returns a fresh `Set-Cookie`. The subject stays signed in; the bearer token
changes.

Rotation is **host-triggered on privilege change**. Rotation on every request is
explicitly rejected — §4.

## 2. Motivation

A session secret that never changes is valid for its whole lifetime to anyone
who obtains it. The standard mitigation is to rotate at the moments where a
stolen token becomes materially more valuable — most importantly when the
subject's authorization changes.

codlet cannot detect those moments. It authenticates; the host authorizes
(DEC-001), so only the host knows that a subject just became an administrator.
Rotation is therefore an operation codlet *offers* and the host *invokes* — the
same division as `revoke`.

## 3. Decision

```rust
pub async fn rotate<R: RandomSource>(
    &self,
    current: &SessionValidationOutcome,
    new_session_id: SessionId,
    rng: &mut R,
) -> Result<IssuedSession, SessionError>;
```

Taking an `Authenticated` outcome — not a raw cookie value — means rotation can
only follow a successful validation in the same request. This mirrors
`issue`'s `RedeemSuccess` requirement (INV-7): the type makes the precondition
unforgeable rather than documented.

### 3.1 Semantics

1. Generate a new 256-bit secret and derive its lookup key.
2. Insert a new record: same `subject`, **same absolute `expires_at`**, fresh
   `created_at`, fresh `last_seen_at`.
3. Revoke the old record.
4. Return the new `Set-Cookie`.

**The absolute expiry does not move.** Rotation changes the credential, not the
session's lifetime — otherwise a host rotating on a schedule would grant
unbounded sessions, and rotation would become a privilege-escalation vector
against the project's own expiry policy.

### 3.2 Ordering: insert then revoke, never the reverse

Between the two writes, both records are briefly valid. That is deliberate.

The alternative ordering — revoke first — leaves a window where *neither* is
valid, so a concurrent in-flight request from the same subject is logged out by
a security improvement. Overlap risks a stolen old token surviving milliseconds
longer; the reverse risks spurious logouts on every rotation.

**Neither ordering is atomic**, and this RFC does not pretend otherwise. D1 has
no multi-statement transaction (RFC-033), so a cross-adapter atomic swap is not
available. If the revoke fails after the insert succeeds, the outcome is two
live sessions for one subject — the old one still expiring on its original
schedule. That is a real residual risk, recorded in §6, and it is why §3.3
exists.

### 3.3 A failed revoke is reported, not swallowed

If step 3 fails, `rotate` returns the new `IssuedSession` **and** emits a
distinct audit event naming the un-revoked session id. The host is signed in
with a fresh credential and the operator can see that an old one outlived it.

Returning an error instead would leave the host holding a cookie it does not
know is valid, which is worse in every direction.

## 4. Rejected: rotation on every request

Rotating per request is the strongest form and is unimplementable safely here.

Browsers issue concurrent requests. Two requests carrying the same cookie both
rotate; one wins; the other holds a token that was revoked mid-flight — and the
user is logged out by their own page load. Mitigations exist (grace windows
accepting the previous secret for N seconds) and they amount to *not* rotating
per request while claiming to.

It also multiplies §3.2's non-atomic swap by request volume, on a storage
contract that cannot make it atomic.

**Not proposed. If it is ever wanted, it needs a grace-window design and its own
RFC** — the grace window, not the rotation, is where the difficulty lives.

## 5. Non-goals

- No automatic or scheduled rotation. Host-triggered only.
- No change to `issue`, `validate`, or `revoke`.
- No change to the absolute expiry model — §3.1.
- No re-authentication requirement. Rotation is not step-up auth.

## 6. Security considerations

**What it buys.** A token stolen before a privilege change is useless after it,
provided the host rotates at that moment. codlet cannot enforce that the host
does; documentation must say so plainly rather than implying rotation is
automatic.

**Residual risk — the non-atomic swap.** §3.2. Two sessions can coexist if the
revoke fails. Bounded by the original absolute expiry, surfaced by an audit
event, not eliminated. A host requiring strict single-session semantics must
enforce it above codlet.

**What it does not buy.** Nothing against an attacker who steals the *new*
cookie. Rotation narrows a window; it does not close one.

## 7. Alternatives considered

1. **Rotate on every request.** Rejected — §4.
2. **Revoke-then-insert.** Rejected — §3.2. Trades a rare overlap for a routine
   logout.
3. **Reuse the record, replacing the lookup key in place.** Rejected: the update
   would have to be conditional on the old key to be safe, and a failed
   conditional update leaves the caller unable to distinguish "someone else
   rotated first" from "the store failed" — the exact ambiguity INV-5 exists to
   avoid.
4. **Return an error on failed revoke.** Rejected — §3.3.

## 8. Open question

Should `rotate` require the host to state a reason, recorded in the audit event?
Recommend yes as a `&str` the host supplies — an audit trail of rotations
without causes is hard to act on during an incident, and the host is the only
party that knows why.

## 9. Acceptance criteria

- `rotate` accepts only an `Authenticated` outcome; a compile-fail test proves
  it cannot be called otherwise.
- Absolute `expires_at` is carried forward unchanged — tested.
- Insert precedes revoke — tested by an adapter fixture failing the revoke and
  asserting the new session works and the audit event fires.
- The old secret does not validate after a successful rotation.
- Conformance coverage on all four adapters.
