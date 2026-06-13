# Contributing to codlet

codlet is a security library. Contributions are evaluated first for whether
they preserve the security invariants, then for everything else.

## Design before code

Follow the workflow: requirement → external design → internal design →
implementation → testing. Substantial changes start as an RFC under `rfcs/`
(see `rfcs/000-rfc-lifecycle-policy.md`). The folder an RFC lives in is the
source of truth for its state; keep each RFC's `Status` field consistent with
its folder, and update `rfcs/README.md` in the same change that moves an RFC.

## Non-negotiable invariants

Do not submit changes that weaken any of these (see RFC-001/002 and the threat
model):

- secrets are stored only as keyed HMAC lookup values — never plaintext;
- missing key material and RNG failure fail closed, with no fallback;
- one-time claim and single-use form-token consume are atomic and
  single-winner (`changed == 0` never proceeds);
- public redemption failures are generic and do not reveal code state;
- session cookies are `HttpOnly; Secure; SameSite=Strict` by default;
- `codlet-core` gains no web-framework, database, or async-executor dependency;
- no authorization, user, role, permission, or community concept enters core.

## Before opening a PR

Run, from the workspace root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- release-check
```

Add tests for the design specification, not merely the code as written. New
security behavior needs the corresponding release gate or conformance test.

## Style

- `#![forbid(unsafe_code)]` stays in every crate unless a future adapter RFC
  explicitly justifies an exception.
- Use `thiserror` or a small handwritten error type in public APIs; not
  `anyhow`.
- Secret-bearing types implement a redacted `Debug` by hand.
- Keep files within the workspace line-count guidance; split by logical
  boundary when they grow.
- English for all code and documentation.

## License

By contributing you agree your contributions are licensed under Apache-2.0.
