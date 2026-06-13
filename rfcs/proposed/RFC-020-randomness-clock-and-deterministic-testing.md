# RFC-020: Randomness, Clock, and Deterministic Testing

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

The service already fixed a dangerous RNG fallback pattern. codlet must preserve and generalize that fix. It must also make expiry and race tests deterministic without weakening production behavior.

## Decision

codlet will define `RandomSource` and `Clock` traits. Production uses the platform CSPRNG and system clock. Tests and examples can use deterministic implementations.

## Randomness requirements

- All secret generation returns `Result`.
- RNG failure is fatal to that operation.
- No fallback value may be produced.
- Rejection sampling must be used for non-power-of-two alphabets.
- The generator must avoid modulo bias by rejecting bytes above `floor(256 / alphabet_len) * alphabet_len - 1`.

## Clock requirements

- Expiry comparisons use an injected clock in testable services.
- Stored timestamps should use a consistent UTC format.
- Adapters may translate to native database timestamp types but must preserve ordering.

## Test requirements

- Mock RNG returning error.
- Mock RNG returning boundary bytes for rejection sampling.
- Deterministic clock at issue time and consume time.
- Expired code/session/token tests using controlled time.

## Acceptance criteria

- `CodeGenerator` cannot be constructed without a random source or explicit default.
- No `unwrap_or_default` appears near random generation.
- Deterministic tests do not require sleeping.
- Rejection sampling tests cover every accepted byte mapping.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
