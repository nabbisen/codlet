# codlet

Embedded one-time-code authentication primitives for Rust web services.

codlet authenticates a subject; the host application authorizes that subject.
It provides auditable security primitives — code generation, normalization,
keyed lookup derivation, atomic one-time redemption, session and form-token
lifecycle, rate-limit contracts — plus optional runtime adapters.

This documentation is organized for three audiences:

- **New users:** features, tutorials, and integration guides (forthcoming).
- **Intermediate users:** API references and functional specifications
  (forthcoming).
- **Maintainers/contributors:** project goals, design philosophy, the RFC
  process, and local development. See the workspace `rfcs/` directory and
  `CONTRIBUTING.md`.

> codlet is at the pre-release bootstrap stage. See `CHANGELOG.md`.
