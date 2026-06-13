# RFC-021: Error Taxonomy and User-Facing Copy Contract

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

Authentication APIs often leak information through detailed errors. codlet must give host applications enough detail for logs and metrics while preserving generic public failure categories.

## Decision

codlet will separate internal errors from public authentication failures.

## Error layers

### Internal errors

Internal errors are for developers/operators:

- `ConfigurationError`;
- `RandomError`;
- `KeyError`;
- `StoreError`;
- `InvalidInputError`;
- `SecurityInvariantError`.

### Public failures

Public failures are safe to map to UI:

- `RedemptionFailed`;
- `RateLimited`;
- `FormExpiredOrInvalid`;
- `SessionMissingOrExpired`;
- `OperationCouldNotBeCompleted`.

`RedemptionFailed` must cover nonexistent, expired, revoked, and already-used codes.

## HTTP guidance

Adapters may provide suggested status codes, but public behavior should avoid distinguishable differences that enable enumeration. For a join form, returning the same page with a generic hint is acceptable.

## Logging

Logs may include stable event categories and correlation IDs. Logs must not include plaintext codes, token values, session values, HMAC keys, or detailed public failure reasons.

## Acceptance criteria

- Unit tests verify all code-not-redeemable states map to the same public failure.
- Documentation gives host applications UI copy examples without exposing state.
- Internal errors remain available for trusted server logs.
- No `Display` output for public failures leaks the detailed reason.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
