# RFC-025: Audit Sink and Security Event Vocabulary

## Status

Implemented (v0.8.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

Host services often need audit logs, but codlet must not own host audit schemas or business event vocabulary.

## Decision

codlet will expose an optional `AuditSink` trait with a small security event vocabulary. Host applications map these events to their own audit tables.

## Event vocabulary

```rust
pub enum CodeAuthEventKind {
    CodeIssued,
    CodeRedeemAttempted,
    CodeRedeemRejected,
    CodeClaimWon,
    CodeClaimLost,
    CodeRevoked,
    SessionIssued,
    SessionValidated,
    SessionRevoked,
    FormTokenIssued,
    FormTokenConsumed,
    RateLimitBlocked,
}
```

## Event payload

Payload may include:

- timestamp;
- operation correlation ID;
- opaque scope classification;
- public outcome category;
- adapter name;
- key version;
- host-provided safe metadata.

Payload must not include plaintext secrets, lookup keys, or business roles unless host application explicitly adds them after redaction review.

## zinnias-ciao mapping

`CodeClaimWon` can be mapped by the service to its existing `audit_log` row with `target_kind="invite_code"` and `action="redeemed"`. codlet should not write this row directly.

## Acceptance criteria

- `AuditSink` is optional.
- Default sink is no-op.
- Redaction tests cover all event kinds.
- Host application can attach correlation IDs without leaking secret values.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
