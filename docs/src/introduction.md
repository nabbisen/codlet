# codlet

Embedded one-time-code authentication primitives for Rust web services.

codlet authenticates a subject; the host application authorizes that subject.
It provides auditable security primitives — code generation, normalization,
keyed lookup derivation, atomic one-time redemption, session and form-token
lifecycle, rate-limit contracts — plus optional runtime adapters.

This documentation is organized for three audiences:

- **New users:** the README quick start, the runnable programs in the
  workspace `examples/` directory, and the guides in this book — secure
  configuration, key rotation, and migrating from an existing service.
  Step-by-step tutorials are not written yet.
- **Intermediate users:** the API reference is each crate's rustdoc (docs.rs),
  and the [threat model](threat-model.md) states the security invariants.
- **Maintainers/contributors:** project goals, design philosophy, the RFC
  process, and local development. See the workspace `rfcs/` directory and
  `CONTRIBUTING.md`.

> codlet is pre-v1. The full authentication stack is implemented.
> See `CHANGELOG.md` for the current version and release history.
