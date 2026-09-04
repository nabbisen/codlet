# RFC-046: Host-Visible Session Failure Reasons

- **Status:** Proposed
- **Target milestone:** M6
- **Primary crate(s):** `codlet`
- **Source basis:** `ROADMAP.md` M6; RFC-006 §13.5; DEC-006

## 1. Summary

Let the host distinguish *why* a session failed to validate — expired, revoked,
idle-timed-out, not found, malformed — while the response the end user sees
stays exactly as it is today.

## 2. Motivation

`SessionValidationOutcome` is binary: `Authenticated` or `Unauthenticated`. That
collapse is deliberate (RFC-006 §13.5) and correct for what the *user* sees.

It is unhelpful for the host, which cannot currently tell:

- **"Your session timed out — sign in again"** from **"Sign in"**, so every
  expiry looks like a first visit;
- an operational spike in *revoked* sessions (incident response working, or a
  bug revoking wrongly) from a spike in *expired* ones (normal);
- a **malformed cookie** — a truncated value, a mangled proxy — from a legitimate
  logged-out visitor, which is the difference between an infrastructure alarm
  and background noise.

Hosts currently work around this by inferring from cookie presence, which is
wrong for exactly the malformed case.

## 3. Decision

`Unauthenticated` gains a reason:

```rust
Unauthenticated { reason: SessionFailure }

pub enum SessionFailure {
    NoCookie,        // no session cookie presented
    Malformed,       // present but not a well-formed secret
    NotFound,        // well-formed, no matching record
    Expired,         // absolute expiry passed
    IdleTimeout,     // idle timeout passed (RFC-044)
    Revoked,         // explicitly revoked
}
```

### 3.1 Why this does not weaken DEC-006

DEC-006 and INV-8 prevent an **attacker** from distinguishing failure states —
the enumeration oracle that makes guessing tractable.

The distinction that matters is *who receives the information*.
`SessionValidationOutcome` is returned to the host application, in-process,
about **a credential the caller already presented**. An attacker submitting a
random cookie learns nothing from a value they never see; whatever the host
renders is the host's choice, exactly as it is today.

This is also not the redemption path. INV-8 governs `PublicRedemptionError`,
where an attacker is guessing *codes* and the oracle is real. Session validation
answers "is this cookie you already hold still good", and the guessing game has
already been lost or won by the time the reason is computed.

**The line this RFC must not cross:** these variants must never become part of
`PublicSessionError` or reach a rendered response by default. The type must make
the boundary visible.

### 3.2 Naming and placement

`SessionFailure` lives in `state`, beside the outcome it belongs to — not in
`error`, which is where `PublicSessionError` lives. Physical separation, so a
future contributor reaching for a public error does not find these by accident.

Documentation carries an explicit warning: **do not render these to end users
verbatim.** The host is free to map `Expired` and `IdleTimeout` to "your session
ended, sign in again" — both are already inferable from the fact that the user
was signed in — but `NotFound` and `Revoked` distinguish states an unauthenticated
visitor should not learn.

### 3.3 Compatibility

Breaking: `SessionValidationOutcome::Unauthenticated` becomes a struct variant,
so exhaustive matches must change. Pre-v1, mechanical, and the change is the
point — a silent field addition would let hosts keep ignoring information they
asked for.

`is_authenticated()` and `subject()` are unchanged.

## 4. Non-goals

- **No change to `PublicSessionError`** or to any rendered response.
- **No change to the redemption path.** INV-8 is untouched; RFC-021's
  user-facing copy contract stands.
- No metrics or audit changes beyond `SessionFailure` becoming available as a
  label — subject to §5.
- No new store capability. Every reason is already derivable from what
  `find_active_session` returns plus RFC-044's `last_seen_at`.

## 5. Security considerations

**Metrics cardinality and inference.** `SessionFailure` as a metrics label is
the obvious use and the one that needs a bound: labelled counters are fine;
anything that ties a reason to a subject or a session id turns operational
telemetry into a behavioural record. RFC-024's redaction rules apply unchanged,
and this RFC adds no exemption.

**The realistic failure mode is a host echoing the reason.** codlet cannot
prevent it. The mitigations are the type-level separation in §3.2 and blunt
documentation. Worth stating plainly: this RFC hands the host a sharper tool and
relies on the boundary being legible.

## 6. Alternatives considered

1. **Leave it binary.** The status quo, and defensible — but it makes correct
   UX ("your session timed out") impossible to build, and pushes hosts into
   cookie-presence inference that is wrong for `Malformed`.
2. **A separate `last_failure_reason()` accessor.** Rejected: an out-of-band
   channel is easy to consult by accident and easy to forget; the enum on the
   variant makes the information's provenance obvious at the match site.
3. **Expose the reason only under a Cargo feature.** Rejected: a security-
   relevant API that appears and disappears by feature flag is harder to reason
   about than one that is always present with a documented boundary.

## 7. Open question

Should `Malformed` distinguish "wrong length" from "invalid encoding"? Recommend
no. Both mean the same thing operationally — something mangled the cookie — and
the finer split is the kind of detail that ends up in a user-facing message.

## 8. Acceptance criteria

- `Unauthenticated { reason }` with the six variants; `is_authenticated()` and
  `subject()` unchanged.
- No variant reachable from `PublicSessionError`; a test asserts the two types
  share no conversion.
- Each reason produced by a test exercising the real condition, not a
  constructed value.
- Documentation carries the do-not-render warning at the type, not only in the
  RFC.
- CHANGELOG records the breaking match change with a migration line.
