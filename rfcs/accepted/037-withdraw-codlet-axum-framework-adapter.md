# RFC-037: Withdraw the `codlet-axum` Framework Adapter from Planned Scope

- **Status:** Accepted
- **Target milestone:** M4
- **Primary crate(s):** none — planning scope only
- **Source basis:** RFC-002 §4; original design-package roadmap Phase 5; owner direction 2026-08-31

## 1. Summary

Formally withdraw `codlet-axum` from planned scope. It was specified in
RFC-002 §4 and planned as roadmap Phase 5, never built, and never withdrawn.
This RFC converts a silent scope drop into a recorded decision.

## 2. Motivation

RFC-002 §4 lists `crates/codlet-axum` in the workspace layout, and the original
roadmap Phase 5 planned an extractor/middleware crate with session-cookie
extraction, response-cookie helpers, and CSRF helpers. Neither was ever built.
`crates/codlet-test` in the same list was likewise never built; the role was
filled by `codlet-conformance`.

RFC-000 names silent withdrawal as an anti-pattern: an abandoned item that is
never formally closed leaves contributors reviewing work nobody intends to do,
and leaves the design record asserting a structure that does not exist.

## 3. Decision

`codlet-axum` is withdrawn from planned scope. Axum integration remains
demonstrated by `examples/axum_login_logout`, a working end-to-end login/logout
service that covers cookie extraction, `Set-Cookie` emission, and per-request
session validation without a dedicated crate.

RFC-002 is **not edited**. It is Implemented and is a historical record; its
divergence from as-built is recorded in `rfcs/README.md` and in `ROADMAP.md`.

## 4. Risk assessment for future work

The owner asked specifically whether withdrawal creates risk for future
updates. Four dimensions were examined.

### 4.1 Reversibility — low risk

No code exists to delete, and a future `codlet-axum` would be a **new crate**,
not a modification of `codlet`. Adding it is purely additive under semantic
versioning and remains available at any time, including after v1.0. Deferring
costs nothing in compatibility terms; this is not a door that closes.

### 4.2 Does the core stay adapter-ready? — low risk, but conditionally

The relevant constraint is that codlet's storage traits must remain satisfiable
under the `Send + Sync` bounds an Axum/Tower middleware imposes. That constraint
is guarded by a test in codlet's own suite —
`rfc_009_compile::send_sync_store_satisfies_axum_style_bounds` — not by the
existence of an adapter crate. Withdrawing the crate does not weaken it.

**Condition:** that test is executed by the `test-send-compat` CI job, which is
one of the jobs broken since v0.17.0 and is currently not running. Withdrawal is
safe *because* the guard exists, so the guard must actually execute. RFC-036
restores it. These two RFCs are coupled: **037 is safe only once 036 has
landed.**

### 4.3 Early warning on upstream Axum churn — low risk, mitigated

A maintained adapter would act as a canary for breaking changes in Axum major
versions. Without one, that signal comes instead from
`examples/axum_login_logout`, which is compiled in CI. RFC-036 restores that job
too, so the canary survives withdrawal.

### 4.4 The `codlet-axum` name on crates.io — residual risk, routed onward

`codlet-axum` is unreserved. A third party could publish under it, and for a
security library a plausible-looking `codlet-axum` that adopters mistake for
official is a user-facing confusion and supply-chain risk, not merely an
inconvenience.

This risk is **not specific to this withdrawal**, and is not an argument for
keeping `codlet-axum` on the roadmap. `codlet-worker` and `codlet-conformance`
are both `publish = false` and equally unreserved today. There is one namespace
question covering all of them, and withdrawal merely makes it visible.

Interim mitigation, cheap and immediate: state the official crate list in
`README.md`, so an unofficial crate can be contradicted by the project's own
documentation. Recorded as §7 below.

## 5. Non-goals

- No edit to RFC-002 or any other Implemented RFC.
- No removal or reduction of `examples/axum_login_logout`. It is now the sole
  demonstration of framework integration and its value increases under this
  decision.
- No decision about `codlet-worker`'s publication status — that remains DEC-013,
  due at M7.

## 6. Alternatives considered

1. **Resurrect Phase 5 and build `codlet-axum`.** Rejected: it adds a public
   surface that must stay stable across v1.0, in exchange for ergonomics the
   example already demonstrates. A framework adapter is also the component most
   exposed to upstream churn, and codlet's release discipline is calibrated for
   a small, auditable surface.
2. **Leave it silently unbuilt.** Rejected: it is the anti-pattern this RFC
   exists to close, and it leaves RFC-002 asserting a workspace that does not
   exist with nothing anywhere recording why.
3. **Withdraw and immediately reserve the crates.io name.** Deferred rather than
   rejected — see §7.

## 7. Open question routed to M7

Should the project reserve `codlet-axum`, `codlet-worker`, and
`codlet-conformance` on crates.io by publishing minimal placeholder crates?

**Recommendation:** decide this at M7 alongside DEC-013, as a single namespace
policy rather than three separate calls. In the meantime, add the official crate
list to `README.md` (M4, folded into the RFC-035 migration handoff).

**Owner decision required at M7.** The trade-off is that placeholder crates
occupy names the project may never use, against the risk of an unofficial crate
claiming an official-looking name in a security-sensitive namespace.

## 8. Acceptance criteria

- `ROADMAP.md` records `codlet-axum` as withdrawn, citing this RFC.
- `rfcs/README.md` lists this RFC and notes RFC-002's as-built divergence.
- `README.md` states the official crate list.
- No change to any Implemented RFC body.
- This RFC does not ship before RFC-036 (see §4.2).
