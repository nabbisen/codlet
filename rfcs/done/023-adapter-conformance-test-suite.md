# RFC-023: Adapter Conformance Test Suite

## Status

Implemented (v0.5.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

codlet's safety depends heavily on adapters. Each adapter may target a different runtime and storage engine, so ad hoc tests are insufficient.

## Decision

codlet will provide a reusable adapter conformance test suite. Any adapter crate must pass it before being described as production-ready.

## Suite structure

```rust
pub async fn run_code_store_conformance<F, S>(factory: F)
where F: StoreFactory<S>, S: CodeStore;
```

Equivalent runners should exist for sessions, form tokens, and rate limits.

## Required code-store tests

- insert and find redeemable;
- nonexistent returns none/generic;
- expired not redeemable;
- used not redeemable;
- revoked not redeemable;
- exactly one claim winner;
- scope-based revoke works;
- wrong scope does not revoke;
- key version round trips.

## Required session-store tests

- active session found;
- expired session not active;
- revoked session not active;
- wrong HMAC not active;
- key version round trips.

## Required form-token tests

- valid consume proceeds;
- replay returns replay;
- expired invalid;
- purpose mismatch invalid;
- subject mismatch invalid;
- bound-resource mismatch invalid;
- `changed == 0` never proceeds.

## Acceptance criteria

- In-memory store passes conformance suite.
- D1 adapter must pass before release.
- SQLx adapter must pass before release.
- Adapter README lists conformance status.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
