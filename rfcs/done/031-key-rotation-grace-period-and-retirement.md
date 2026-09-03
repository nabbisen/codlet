# RFC-031: Key Rotation Grace Period and Retirement

## Status

Implemented (v0.8.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

HMAC keys may need to rotate. Immediate invalidation of all active codes, form tokens, and sessions may be operationally unacceptable. But indefinite old-key support is risky.

## Decision

codlet will support key versions and a controlled rotation lifecycle.

## Key states

- `active` — used for new lookup keys.
- `verify_only` — accepted for existing records but not used for new records.
- `retired` — not accepted.

## Record requirements

Every HMAC-backed record must store the key version used to create its lookup key:

- code HMAC key version;
- session HMAC key version;
- form-token HMAC key version.

## Rotation workflow

1. Add new key as `active`.
2. Move previous key to `verify_only`.
3. New records use the new key.
4. Existing records continue validating until expiry or migration.
5. Retire old key after maximum TTL window passes.

## Acceptance criteria

- Missing key version fails validation, not fallback.
- Active key is used for new records.
- Verify-only key validates old records.
- Retired key does not validate.
- Documentation includes operational rotation checklist.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
