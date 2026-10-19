# RFC-055: A Configured Rate Limit With No Key Is a Silent Opt-Out

- **Status:** Implemented (v0.21.0) — reviewed and approved 2026-10-18
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet`
- **Source basis:** RFC-053 review request §2.1; verified 2026-10-18

## 1. Summary

A host can supply a `RateLimitStore`, a `RateLimitPolicy`, and still get no rate
limiting — by passing `rate_key: None`. No error, no warning, no audit event.
RFC-053 deprecated `without_rate_limit` for being too easy a way to switch off
a required defence. This is the easier way, and it looks like the secure path.

## 2. Evidence

`crates/codlet/src/auth/code.rs`, three sites — `find` (`:158`), the
invalid-format failure path (`:196`), and the non-redeemable path (`:248`):

```rust
if let (Some(key), Some(rl_policy)) = (rate_key, &self.rate_limit_policy) {
```

Both halves must be present. A `None` key skips the check, skips
`record_failure`, and skips the `RateLimitHit` audit event — so nothing in the
logs distinguishes "limiter allowed this" from "limiter never ran".

This was found while switching the examples to a real limiter for RFC-053 §4.4:
all three already passed `None`, so wiring `MemRateLimitStore` into them would
have changed nothing. Three examples would have looked rate-limited and would
not have been.

## 3. Why it is worse than `without_rate_limit`

`without_rate_limit` is honest: the name says what you lose, and after RFC-053
the compiler argues with you. `CodeAuth::new(..., policy)` plus `None` reads as
the configured, careful path. The type system accepts it, the constructor
demanded a policy, and the failure is invisible at the call site and in the
audit trail.

Under the project's own standard — an API users cannot misread — a signature
that accepts a configuration meaning "the defence I just configured does not
run" is a defect in the signature, not in the caller.

## 4. Decision: fail closed (option b)

Accepted by the owner 2026-10-18. The options were weighed as below;
**(b) is the decision**, with (a) revisited at the freeze.

**(a) Make the key non-optional where a policy exists.** Strongest: the
combination becomes unrepresentable. Costs a signature change on `find`,
`claim` and `redeem_with_callback`, and requires every caller that genuinely
has no key — a CLI, a test — to synthesise one.

**(b) Fail closed when a policy is configured and no key is given.** Return a
`RedeemError` rather than proceeding unlimited. Cheap and loud; turns a silent
misconfiguration into an immediate error at the first request. Risks breaking a
host that relies on today's behaviour, which is acceptable pre-1.0.

**(c) Emit an audit event and proceed.** Cheapest, keeps every signature, and
makes the condition visible to anyone reading the audit trail. Weakest: a
defence that depends on someone noticing a log line.

**Decided: (b), with (a) considered at the freeze.** (b) is one branch,
needs no signature change, and converts an invisible failure into a loud one —
which is the property that matters, and the direction RFC-036's gate-integrity
work established. (a) is the cleaner end state and the freeze is the moment to
pay for it, but it should be decided with the whole `auth` surface in view
rather than alone.

(c) is rejected as a primary fix: adding a log line to a security control that
is not running is the fail-open shape this project has now corrected five times.

## 5. Non-goals

- No change to `RateLimitStore`, `RateLimitPolicy`, or `RateLimitOutcome`.
- No change to the unavailable-store policy, which is already explicit and
  correct.
- No change to `without_rate_limit`, which RFC-053 handled.
- Not a change to the session or form-token paths; this is the code-redemption
  surface.

## 6. Security considerations

Online code guessing is the attack rate limiting exists to stop, and RFC-053
established that it is required for every code policy. This is a way to satisfy
every visible requirement — store wired, policy set — and still not have it.
The examples demonstrate how natural the mistake is: all three made it, and the
project wrote them.

## 7. The wider audit: done, and the defect is bounded

§7 previously asked whether `auth`'s other managers share the pattern. I ran it
on 2026-10-18 rather than leaving it open.

**Five sites, all in the code-redemption path**, not three as my first review
said — `auth/code.rs` lines 158, 196, 248 and 313 use
`if let (Some(key), Some(rl_policy))`, and a fifth at ~297 uses a *different
shape* for the success path:

```rust
if let Some(key) = rate_key {
    if self.rate_limit_policy.is_some() {
        let _ = self.rate_limit_store.clear_failures(key).await;
    }
}
```

That nested form will not be found by grepping for the tuple pattern, and it is
the one that clears the counter after a success. A fix that misses it leaves
failure counts accumulating across successful redemptions.

They span `find` and `claim`.

**`auth/session.rs:204` is not an instance**, though it matches the shape:

```rust
if let (Some(idle_timeout), Some((session_id, last_seen))) = (self.idle_timeout, touch_target)
```

Here `None` *is* the configuration — no idle timeout — and `classify_session`
already takes `Option<Duration>` and treats `None` as "do not check". The guard
only decides whether to perform a bookkeeping touch, which is correctly skipped
when there is no idle timeout to maintain. Nothing configured is silently
disabled.

So the defect is confined to the code-redemption rate limiter. That is a
genuinely bounded answer rather than a reassuring one: the form-token and
session managers take no equivalent optional control.
