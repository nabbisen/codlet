# RFC-026: Examples and Reference Applications

## Status

Implemented (v0.8.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

Authentication libraries are often misused because examples optimize for brevity over safety. codlet examples must teach correct host boundaries.

## Decision

codlet will ship examples that demonstrate safe, realistic integration while keeping application authorization outside codlet.

## Required examples

### `minimal-issue-redeem`

Pure in-memory example showing issue → lookup → claim → session issue.

### `minimal-axum`

Axum app with:

- join form;
- code redemption;
- app-owned subject creation;
- session cookie;
- generic errors.

### `worker-d1-join`

Cloudflare Workers/D1 style example aligned with `zinnias-ciao` constraints.

### `migration-ciao-sketch`

Non-production sketch showing how existing service tables map to codlet traits.

## Example rules

- No example may store plaintext codes in a database.
- No example may create a user/session unless claim wins.
- No example may expose detailed public failure states.
- Examples must include comments pointing out what the host application owns.

## Acceptance criteria

- Examples compile in CI.
- Examples run with deterministic test mode.
- README references examples by use case.
- Security warnings are close to code, not only in prose.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
