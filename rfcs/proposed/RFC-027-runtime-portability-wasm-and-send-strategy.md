# RFC-027: Runtime Portability, WASM, and `?Send` Strategy

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

The source service runs in Cloudflare Workers, where runtime constraints differ from native Tokio services. codlet should support Workers without forcing all users into Workers-specific dependencies.

## Decision

`codlet-core` will be runtime-neutral. Async traits may use `?Send` where needed, and adapter crates will document their runtime requirements.

## Runtime targets

- Native async Rust services, especially Axum/Tower/Tokio.
- Cloudflare Workers/WASM with D1/KV adapter.
- In-memory tests without a web runtime.

## Feature flag policy

- `std` default for v0.1.
- `wasm` for Workers-compatible behavior.
- `sqlx` only in adapter crate.
- `axum` only in adapter crate.
- no framework dependency in `codlet-core`.

## Trait sendness

Two options:

1. Use `async_trait(?Send)` for core store traits, maximizing Workers compatibility.
2. Define separate `Send` bounds in native adapter layers.

The RFC chooses option 1 for core, with adapters free to add stricter bounds.

## Acceptance criteria

- `codlet-core` compiles without Tokio.
- Worker adapter does not require `Send` futures if the platform cannot provide them.
- Native examples document runtime feature requirements.
- Public API avoids Cloudflare-specific types in core.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
