# Implementation Handoff — RFC-044 Session Inactivity Timeout

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-04
- **Milestone:** M6, first item
- **Governing RFC:** [`../../accepted/044-session-inactivity-timeout.md`](../../accepted/044-session-inactivity-timeout.md)
- **Priority:** first in M6. RFC-046 may run in parallel; RFC-045 waits on this.

Read RFC-044 before starting. If execution conflicts with it, **stop and
escalate**.

## 1. Purpose

Add an opt-in idle timeout, and with it the first mutation the `SessionStore`
contract has ever had.

**This is the first M6 change that alters shipped behaviour.** Everything since
M4 has been machinery. Scope discipline matters correspondingly more.

## 2. Change scope

- `crates/codlet/src/store/session.rs` — `touch_session`; `last_seen_at` on the
  returned record
- `crates/codlet/src/state/session.rs` — idle-expiry classification
- `crates/codlet/src/auth/session.rs` — policy field, throttle, touch call
- `crates/codlet/src/mem/session.rs`, `codlet-sqlx` (sqlite + postgres),
  `codlet-worker/src/d1/session.rs` — the four adapters
- `crates/codlet-sqlx/migrations/*.sql`, `codlet-worker` migration — schema
- `crates/codlet-conformance/src/session.rs` — conformance coverage
- `docs/`, `CHANGELOG.md`

## 3. Non-change scope

- **The cookie**, its attributes, or its lifetime.
- **Absolute expiry semantics.** Idle timeout only ever shortens; nothing
  extends `expires_at`. If you find yourself writing code that moves it, stop.
- **The public failure surface.** Idle-expired collapses to `Unauthenticated`
  exactly as expired and revoked do. RFC-046 changes that type; this one does
  not.
- Session rotation — RFC-045.
- Any store method other than the one new `touch_session`.

## 4. Required implementation

### 4.1 Off by default, costing nothing

`idle_timeout: Option<Duration>`, defaulting to `None`. With it unset there must
be **no new read, no new write, and no behavioural difference** — a host that
does not opt in pays nothing.

Prove it: a test asserting that with `idle_timeout: None`, a validation performs
no store write. If your adapter fixtures cannot observe that, add a counting
fixture — "we believe it does not write" is not evidence.

### 4.2 Throttled touch

Touch only when `last_seen_at` is older than `idle_timeout / 20`, floored at
30 seconds.

Test it directly: N validations inside one granularity produce **exactly one**
write. This is the property that makes the feature affordable, and it is
invisible in ordinary use — so it needs its own test rather than being implied
by the others.

### 4.3 Idle expiry decided in codlet, not in SQL

`find_active_session` returns `last_seen_at`; `classify_session` decides
idle-expiry. **Do not add an idle predicate to any adapter's WHERE clause.**
Four implementations of one security rule is how this project got a PostgreSQL
adapter nobody had ever tested.

### 4.4 A failed touch must not log the user out

If `touch_session` fails on an otherwise-valid session, return
`Authenticated` and emit an audit event.

**Test this explicitly** with a fixture whose `touch_session` always fails,
asserting the request is still authenticated and the event fired. Getting this
backwards converts a transient storage error into a mass logout, and it is the
single most likely way this feature causes an incident.

### 4.5 Schema

`last_seen_at` NULL-able, added to all three schemas (D1 uses `REAL`, per
RFC-033's convention). **Additive, no backfill** — NULL reads as `created_at`.

Existing rows must validate unchanged. Test against a row inserted without the
column populated.

### 4.6 Conformance

`touch_session` joins the shared suite and must pass on all four adapters —
in-memory, SQLite, PostgreSQL, D1. The PostgreSQL suite needs Docker and runs in
CI; if you cannot run it locally, say so rather than inferring.

## 5. Required tests

| Test | Must |
|---|---|
| `idle_timeout: None` performs no write | pass, with a counting fixture |
| Throttling: N validations, one granularity | exactly one write |
| Idle-expired session | classifies `Unauthenticated` |
| Absolute expiry still enforced independently | pass |
| Failed `touch_session` | request still authenticated, audit event emitted |
| Pre-existing row with NULL `last_seen_at` | validates unchanged |
| Conformance, four adapters | pass |
| Full workspace, `release-check`, `self-test`, `cargo deny`, MSRV jobs | pass |

## 6. Acceptance criteria

1. Default behaviour byte-identical to today, proven by the no-write test.
2. Throttling proven by write count, not by inspection.
3. Idle expiry decided in `classify_session`; no adapter WHERE clause mentions
   it.
4. Failed touch leaves the session valid, with a test.
5. Migration additive; pre-existing rows validate.
6. Conformance green on all four adapters.
7. Full CI green.

## 7. Prohibited shortcuts

- Do not enable the timeout by default "because it is more secure".
- Do not fold `touch` into `find_active_session`. The separation is what keeps a
  bookkeeping failure from invalidating a session.
- Do not push idle expiry into adapter SQL.
- Do not extend `expires_at` anywhere.
- Do not claim the throttle works without a write-count test.

## 8. Known risks

| Risk | Mitigation |
|---|---|
| Touch-on-every-request slips in via a missed throttle branch | §4.2's write-count test |
| An adapter implements idle expiry in SQL "for efficiency" | §4.3; criterion 3 |
| Fail-closed on touch failure | §4.4; criterion 4 |
| D1's `REAL` timestamps drift from the integer convention elsewhere | Follow RFC-033's existing pattern exactly; do not invent a new one |

## 9. Required evidence

Diff; all §5 tests passing; the write-count output for §4.2; the failed-touch
test; conformance output for all four adapters; CI run URL.

## 10. Review request

`.git-exclude/review-request/044-session-inactivity-timeout.md`; my result
returns at `.git-exclude/reviewed/044-session-inactivity-timeout.md`.
