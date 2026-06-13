# RFC-029: Idempotency Result Persistence

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

The source service has `form_tokens.result_ref` but does not yet fully use it. Basic replay detection prevents duplicate processing, but result persistence can improve user experience and robustness.

## Decision

codlet will support optional idempotency result persistence for form tokens. This is not required for v0.1 but should be designed before v1.0.

## Semantics

After a successful `Proceed`, the host application may attach a result reference to the consumed token. On replay, codlet can return `Replay { result_ref }` if present.

## Use cases

- double-clicked submit button;
- browser retry;
- mobile network retry;
- back/forward form resubmission;
- multi-step join flow where the first successful result should be reused.

## API sketch

```rust
pub enum TokenConsumeOutcome {
    Proceed(ConsumedToken),
    Replay { result_ref: Option<String> },
    Invalid,
}
```

`attach_result` must be allowed only for already-consumed tokens and should not change the one-time proceed invariant.

## Acceptance criteria

- Replay never returns `Proceed`.
- Result refs are opaque strings.
- Storing result refs is optional.
- Tests cover replay with and without result ref.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
