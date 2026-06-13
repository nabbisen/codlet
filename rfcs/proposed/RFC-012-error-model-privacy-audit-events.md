# RFC-012: Error Model, Privacy, and Audit Events

- **Status:** Proposed
- **Target milestone:** M3
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Separate public-safe authentication outcomes from internal diagnostics and define redacted security events.

## 2. Motivation

The source service deliberately uses generic messages so users cannot learn whether a code exists, expired, was used, or was revoked. codlet must preserve this pattern without hiding useful internal diagnostics.

## 3. Decision

codlet will expose typed internal errors, public-safe error mapping, and optional `AuditSink` events with redaction guarantees.

## 4. Detailed design


Public outcomes:

```rust
pub enum PublicRedemptionError {
    InvalidOrExpired,
    RateLimited,
    TemporarilyUnavailable,
}
```

Internal reasons:

```rust
pub enum RedemptionFailReason {
    InvalidFormat,
    NotFound,
    Expired,
    Revoked,
    AlreadyUsed,
    RateLimited,
    StoreUnavailable,
}
```

Mapping:

- by default, `InvalidFormat`, `NotFound`, `Expired`, `Revoked`, and `AlreadyUsed` map to the same public result;
- `RateLimited` may map to a safe throttle result;
- `StoreUnavailable` maps according to host policy.

Audit events:

```rust
pub enum CodeAuthEvent {
    CodeIssued { code_id: CodeId },
    CodeRedeemed { code_id: CodeId, subject_id: SubjectId },
    RedemptionFailed { reason: RedemptionFailReason },
    SessionIssued { session_id: SessionId, subject_id: SubjectId },
    SessionRevoked { session_id: SessionId },
    FormTokenReplay { purpose: String },
    RateLimitHit { key_fingerprint: String },
}
```

Events must not include plaintext secrets or raw IP addresses by default. A hashed/fingerprinted rate key is safer.


## 5. Security considerations

Enumeration resistance requires generic public messages. Privacy requires that audit logs do not become a side channel for secrets or personal free text.

## 6. Host application responsibilities

The host maps codlet events into its own audit schema and chooses user-facing messages. It must not display internal reasons directly.

## 7. Tests and release gates


- All invalid code states map to the same public error.
- Audit event serialization contains no forbidden keys: code, token, secret, hmac, pepper, cookie.
- Secret values generated in tests do not appear in logs/events.
- Debug for secret types is redacted.


## 8. Migration notes

zinnias-ciao can keep its own `audit_log`; codlet only emits events to be mapped. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Two-layer error model

codlet needs separate internal and public error layers:

```text
InternalCause: detailed, structured, logged redacted, useful for tests/ops
PublicError: small, generic, safe for UI/API responses
```

Examples:

| Internal cause | Public error |
|---|---|
| code not found | invalid_or_expired_code |
| code expired | invalid_or_expired_code |
| code already used | invalid_or_expired_code |
| code revoked | invalid_or_expired_code |
| form token binding mismatch | form_not_ready |
| missing key provider | temporary_problem / internal config |
| store unavailable | temporary_problem |

### 10.2 Audit event vocabulary

Security events should be generic:

- `code.issue.succeeded`
- `code.redeem.succeeded`
- `code.redeem.failed`
- `code.revoke.succeeded`
- `session.issue.succeeded`
- `session.validate.failed`
- `session.revoke.succeeded`
- `form_token.consume.replay`
- `rate_limit.blocked`
- `key_provider.missing_version`

The host may map these into business audit events but codlet should not define business semantics.

### 10.3 Redaction policy

Events may include IDs and redacted classifications. Events must not include plaintext code, plaintext token, session secret, full lookup key, HMAC key, display name, or raw IP unless the host deliberately adds its own metadata outside codlet.

### 10.4 Concrete acceptance checklist

- [ ] Every public error has a documented internal cause mapping.
- [ ] Authentication failures collapse enumeration-sensitive details.
- [ ] Audit events are redacted by construction.
- [ ] `Debug` and `Display` implementations do not leak secrets.
- [ ] REST/API examples avoid stack traces and detailed failure reason in responses.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
