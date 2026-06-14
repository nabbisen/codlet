# RFC-019: Typestate API and Opaque Newtypes

## Status

Implemented (v0.6.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

The source service is safe because its code path is narrow and familiar to its maintainers. A public crate will be used by developers who may accidentally pass raw strings in the wrong place, persist plaintext values, or confuse code lookup with final claim. A security crate must make these mistakes difficult.

## Decision

codlet will represent security-sensitive and state-sensitive values with opaque newtypes. Constructors validate and normalize. Secret-bearing types redact `Debug` and do not expose inner strings except through narrowly scoped methods.

## Required types

- `PlainCode` — received from user or generated for display once.
- `NormalizedCode` — canonical code used for lookup-key derivation.
- `CodeLookupKey` — HMAC output used by storage.
- `RedeemableCode` — record found by lookup but not yet claimed.
- `ClaimedCode` — proof that `claim_code` won.
- `SessionSecret` — plaintext session cookie value.
- `SessionLookupKey` — HMAC output for storage.
- `FormTokenSecret` — plaintext form token.
- `FormTokenLookupKey` — HMAC output for storage.
- `SubjectId` — host-owned identity anchor.
- `Purpose`, `ScopeKey`, `BoundResource`, `OpaqueGrant` — host-owned strings with validation.

## Typestate boundaries

`SessionManager::issue_session` should accept `SubjectId`, not raw user/membership records. Host applications can create `SubjectId` only after their own business transaction is ready.

`CodeAuth::claim_code` returns `ClaimOutcome::Won(ClaimedCode)` only for the conditional-update winner. APIs that create sessions after code redemption should require `ClaimedCode` or force explicit handling of `ClaimOutcome`.

## API constraints

- Do not expose public tuple struct fields for secrets.
- Do not implement `Display` for plaintext secrets unless it is an explicit one-time reveal wrapper.
- Do not implement `Serialize` for plaintext secrets by default.
- Do not accept raw `String` for purpose/scope in high-level APIs.

## Alternatives considered

Using raw strings everywhere would reduce boilerplate but would make it easy to confuse plaintext, normalized, and hashed values. This is rejected.

## Acceptance criteria

- `cargo test` proves secret-bearing `Debug` output is redacted.
- It is impossible to call claim/session APIs with raw `String` without explicit conversion.
- All conversion failures return typed errors.
- Documentation explains why `SubjectId` is host-owned.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
