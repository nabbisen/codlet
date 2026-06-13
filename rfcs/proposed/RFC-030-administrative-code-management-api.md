# RFC-030: Administrative Code Management API

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

Host applications need to issue, list, revoke, and inspect code metadata. codlet should provide safe primitives without becoming an admin UI or authorization system.

## Decision

codlet will expose administrative code-management APIs for metadata only. Authorization for calling these APIs remains entirely host-owned.

## Operations

- issue code;
- revoke code by ID and optional scope;
- list code metadata by scope;
- get code metadata by ID;
- count active/expired/used/revoked codes.

## Returned metadata

Metadata may include:

- code ID;
- creation time;
- expiry time;
- used/revoked status;
- opaque grant;
- key version;
- scope key if configured as safe to return.

Metadata must not include plaintext codes or lookup HMACs.

## Host responsibilities

The host must decide:

- who may issue codes;
- who may revoke codes;
- whether scope key is visible;
- how code delivery happens;
- how admin UI text is written.

## Acceptance criteria

- Listing APIs cannot return plaintext secrets.
- Revoke is scoped where a scope is supplied.
- Wrong-scope revoke has no effect.
- API docs explicitly say authorization is host-owned.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
