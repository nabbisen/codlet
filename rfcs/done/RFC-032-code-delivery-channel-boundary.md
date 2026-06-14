# RFC-032: Code Delivery Channel Boundary

## Status

Implemented (v0.8.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

One-time codes must reach users through some channel: verbal, paper, SMS, email, admin copy/paste, or internal message. codlet should not own delivery channels, but it should define safe integration boundaries.

## Decision

codlet core will not send email, SMS, push notifications, or messages. It will return a plaintext code exactly once to the caller after issue. Delivery is the host application's responsibility.

## Rationale

Delivery channels introduce major product and compliance obligations:

- email configuration and deliverability;
- SMS cost and regional rules;
- abuse handling;
- UI copy;
- privacy expectations;
- retries and support workflows.

These are host concerns, not core authentication primitives.

## API rule

`issue_code` returns `IssuedCode { id, plaintext_once, metadata }`. The plaintext should be intentionally hard to clone/log accidentally. After the host displays or sends it, codlet cannot recover it from storage.

## Optional future adapter

A future `codlet-delivery` crate may define traits for delivery, but it must remain separate from `codlet-core` and must not be required by embedded authentication.

## Acceptance criteria

- Core has no SMTP/SMS/push dependencies.
- README states that delivery is host-owned.
- Examples show safe display/copy behavior without persisting plaintext.
- Audit events record code issued without plaintext code.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
