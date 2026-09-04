# RFC-044: Session Inactivity Timeout

- **Status:** Accepted
- **Target milestone:** M6
- **Primary crate(s):** `codlet`, `codlet-sqlx`, `codlet-worker`, `codlet-conformance`
- **Source basis:** `ROADMAP.md` M6; the deferred "RFC-F" scope in the v0.17.1 handoff bundle

## 1. Summary

Add an optional **idle timeout** alongside the existing absolute expiry: a
session becomes invalid after a configured period without use, independently of
its absolute lifetime.

This requires the first mutation the `SessionStore` contract has ever had, and
that — not the timeout itself — is the substance of this RFC.

## 2. Motivation

`codlet_sessions` records `created_at` and `expires_at`. Validation is a lookup
filtered on `expires_at > now AND revoked_at IS NULL`. A session issued with a
30-day absolute TTL remains valid for 30 days whether the subject used it once
or continuously.

For codlet's stated audience — invite-only access for non-technical users on
shared or borrowed devices — an abandoned session staying live for weeks is the
realistic exposure, more than any cryptographic concern. An idle timeout bounds
the window between "user walks away" and "session is worthless".

## 3. The cost, stated first

**Validation currently performs no writes.** `find_active_session` is a single
indexed read on every authenticated request. An idle timeout requires the store
to know when the session was last used, which means writing on the read path.

That is a material change:

- **PostgreSQL / SQLite** — a write per request contends with the same rows it
  reads, and turns a read-only hot path into a read-write one.
- **D1** — writes are markedly more expensive than reads, and Workers
  deployments are the original target (RFC-033).
- **Every adapter** — the conformance suite gains a mutation to prove.

An idle timeout that costs a database write per page view is not obviously worth
having. The design below exists to make it cheap enough to be worth having.

## 4. Decision

### 4.1 Opt-in, off by default

`SessionPolicy` gains an optional `idle_timeout: Option<Duration>`. When `None`
— the default — behaviour is exactly as today: no new column read, no write, no
cost. Existing hosts are unaffected and pay nothing.

A security feature that imposes a per-request write on hosts that did not ask
for it would be a poor trade, and defaults that cost something get switched off
wholesale rather than tuned.

### 4.2 Throttled touch, not touch-per-request

When enabled, `last_seen_at` is updated only if it is older than a **touch
granularity** — a fraction of the idle timeout, defaulting to one twentieth,
floored at 30 seconds.

For a 30-minute idle timeout that is one write per 90 seconds of continuous
activity, not one per request. The cost of the feature becomes proportional to
session *duration*, not to request *volume*, which is the difference between
viable and not.

The granularity is the precision loss: a session may survive up to one
granularity beyond its nominal idle timeout. That is stated in the docs as the
contract, not hidden — an idle timeout is a coarse control and pretending to
second-precision would be false advertising.

### 4.3 Store contract

```rust
fn touch_session(
    &self,
    session_id: &SessionId,
    now: u64,
) -> impl Future<Output = Result<(), StoreError>>;
```

Deliberately **not** part of `find_active_session`. Keeping the read and the
write separate means an adapter cannot accidentally make the read path
conditional on write success, and a `touch` failure must never invalidate an
otherwise-valid session (§4.5).

`find_active_session` gains `last_seen_at` in its returned record so the
manager can decide both whether the session is idle-expired and whether a touch
is due.

**Idle expiry is enforced in codlet, not in the adapter's WHERE clause.** The
store returns the record; `classify_session` decides. Pushing the rule into
per-adapter SQL would give four implementations of one security decision, and
the project already knows what that produces.

### 4.4 Classifier

`classify_session` gains the idle-timeout inputs and returns
`Unauthenticated` for an idle-expired session — the same collapse as expired
and revoked (RFC-006 §13.5). The public surface does not change.

### 4.5 A touch failure must not invalidate a session

If `touch_session` fails while the session is otherwise valid, the request is
**authenticated anyway** and the failure is recorded as an audit event.

The alternative — failing closed on a bookkeeping write — converts a transient
storage error into a mass logout. Fail-closed is right where the question is
"is this subject authentic"; it is wrong where the question is "did we manage to
record a timestamp". Getting this backwards is the most likely way this feature
causes an incident.

## 5. Schema

`last_seen_at INTEGER` (NULL-able), added to `codlet_sessions` in all three
adapters, plus the D1 `REAL` timestamp convention (RFC-033).

NULL means "never touched"; the manager treats NULL as `created_at`. That makes
the migration additive with no backfill — existing rows behave as though last
seen at issuance, which is the only honest reading of a value that was never
recorded.

## 6. Non-goals

- **Not sliding absolute expiry.** The absolute lifetime remains fixed at
  issuance. Idle timeout shortens it; nothing extends it.
- **No session rotation** — RFC-045.
- **No change to the cookie**, its attributes, or its lifetime.
- **No change to the public failure surface** — INV-8 and RFC-006 §13.5 stand.

## 7. Security considerations

Strictly narrowing: a session that would previously have been valid may now be
rejected. Nothing becomes valid that was not before.

`last_seen_at` is a new piece of per-session metadata. It is not a secret, but
it is behavioural data about a subject, and it must not appear in audit events
or metrics at a resolution finer than the touch granularity — otherwise the
audit log becomes an activity trace, which is a privacy property codlet has not
promised and should not acquire by accident.

## 8. Alternatives considered

1. **Touch on every request.** Rejected — §3. The cost is unjustifiable on D1.
2. **Idle expiry in the adapter's WHERE clause.** Rejected — §4.3. Four
   implementations of one security rule.
3. **Encode last-seen in the cookie.** Rejected: it makes the client
   authoritative over its own timeout, and the cookie currently carries only a
   secret, which is a property worth keeping.
4. **Do nothing; let hosts set shorter absolute TTLs.** Genuinely viable, and
   the reason §4.1 makes this opt-in. A host that would rather re-authenticate
   daily than carry a write path is making a reasonable choice.

## 9. Open questions

1. Should the touch granularity be configurable, or fixed at
   `idle_timeout / 20`, floor 30s? Recommend fixed initially — one fewer knob
   to misconfigure, and the failure mode of a badly chosen granularity is either
   cost or imprecision, neither obvious to the person setting it.
2. Should `touch_session` be a separate trait (`SessionTouchStore`) so adapters
   can omit it? Recommend no: an optional-capability trait means the manager
   must handle "configured but unsupported", and a security control that
   silently does not apply is worse than one that is absent.

## 10. Acceptance criteria

- `idle_timeout` defaults to `None`; with it unset, no behaviour, no reads of
  the new column, and no writes.
- `touch_session` implemented and conformance-tested on all four adapters.
- Idle-expired sessions classify as `Unauthenticated`.
- A `touch_session` failure leaves the request authenticated and emits an audit
  event — with a test proving it.
- Throttling proven: N requests inside one granularity produce exactly one
  write.
- Migration is additive; existing rows validate unchanged.
