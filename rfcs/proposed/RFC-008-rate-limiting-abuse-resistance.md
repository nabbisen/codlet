# RFC-008: Rate Limiting and Abuse Resistance

- **Status:** Proposed
- **Target milestone:** M3
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define a runtime-neutral rate-limit model for code redemption and other auth-sensitive actions.

## 2. Motivation

Short human-friendly codes require online guessing controls. The source service uses 10 failures per 5 minutes per IP in Cloudflare KV, but that policy and backend should be configurable.

## 3. Decision

codlet will expose `RateLimitStore` and `RateLimitPolicy`. Core accepts host-provided keys and does not parse network headers.

## 4. Detailed design


Policy:

```rust
pub struct RateLimitPolicy {
    pub max_failures: u32,
    pub window: Duration,
    pub unavailable: RateLimitUnavailable,
}

pub enum RateLimitUnavailable {
    FailOpen,
    FailClosed,
    SoftDenyAfterThreshold(u32),
}
```

Flow:

1. host computes `RateLimitKey` from a trusted source;
2. codlet checks rate limit before expensive lookup;
3. on invalid code lookup, codlet records failure;
4. on successful lookup/redeem, codlet clears failures.

Adapters:

- Worker KV adapter for existing service path;
- in-memory adapter for tests;
- SQL/Redis adapters may be future.


## 5. Security considerations

Rate limiting is necessary but not sufficient. Header spoofing can defeat IP keys outside a trusted proxy chain. KV read-modify-write counters can under-count under concurrency and must be documented.

## 6. Host application responsibilities

The host must select a trustworthy key, often based on connection IP or trusted platform header. It must choose fail-open or fail-closed according to service risk.

## 7. Tests and release gates


- Threshold blocks attempts.
- Successful redemption clears failures.
- FailOpen allows when backend errors.
- FailClosed blocks when backend errors.
- KV adapter documents non-atomic behavior.
- Generic error still used when rate-limited unless host intentionally shows a safe throttle message.


## 8. Migration notes

zinnias-ciao can map its `invite_fail:{ip}` KV keys to the Worker adapter or start a new namespaced prefix. 

## 9. Open questions

None at this stage. 


## 13. Expanded technical design

### 13.1 Rate-limit dimensions and composition

A single dimension is often insufficient. The design should allow composing dimensions into one or more counters:

```text
IP + purpose
IP + scope + purpose
scope + purpose
code_fingerprint + purpose
```

The first protects the service broadly. The second protects tenant/community-specific code pools. The third detects distributed attacks against one scope. The fourth slows repeated guesses of the same code but must be privacy-preserving.

### 13.2 Failure recording policy

Failures should be classified before recording:

| Failure class | Count by default? | Reason |
|---|---:|---|
| malformed oversized input | maybe, abuse path | Could be bot/probe. |
| ordinary typo/too short | no or low weight | Avoid punishing non-technical users. |
| normalized plausible but not redeemable | yes | Online guessing signal. |
| storage unavailable | no | Avoid corrupt counters. |
| rate-limited attempt | optional | Can extend lockout; risk of griefing. |

### 13.3 Lockout and griefing

Rate limiting can be abused to block legitimate users. Therefore codlet should prefer short rolling windows and identity dimensions that do not let an attacker easily lock out a known victim. For invite codes, per-IP plus per-scope counters are safer than per-code lockout alone.

### 13.4 Observability counters

Recommended redacted metrics:

- `codlet_rate_limit_checked_total{purpose,outcome}`
- `codlet_rate_limit_blocked_total{purpose}`
- `codlet_code_redeem_failed_total{purpose,public_class}`
- `codlet_code_redeem_succeeded_total{purpose}`

No metric label should include raw code, raw IP, subject display name, or lookup key.

### 13.5 Concrete acceptance checklist

- [ ] Rate-limit policy documents fail-open/fail-closed choice.
- [ ] Check-before-lookup pattern exists in examples.
- [ ] Success clears failure counter if configured.
- [ ] Counters do not store plaintext code.
- [ ] Public limited response does not reveal code existence.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
