# Changelog

All notable changes to codlet are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and codlet aims to follow
semantic versioning once it reaches a stable release.

## [Unreleased]

Nothing yet.

## [0.1.0] — 2026-06-13

First functional primitives in `codlet-core`. Implements RFC-003 (one-time code
policy, generation, normalization, validation) and RFC-004 (secret hashing, key
providers, domain separation, key versioning). Still pre-1.0 and incomplete: no
storage traits, session/form-token lifecycle, or adapters yet.

### Added

- `secret` module: redacted `SecretString` plus `PlainCode`, `SessionSecret`,
  `FormTokenSecret` (redacted `Debug`/`Display`/serde) and opaque `CodeId`,
  `SubjectId`, `SessionId` newtypes (RFC-019 foundation).
- `rng` module: `RandomSource` trait, `SystemRandom`, and (under `test-utils`)
  deterministic `FixedBytesRandom` / `AlwaysFailRandom`. RNG failure is fatal;
  no fallback value is produced (RFC-020, INV-3).
- `code` module: validated `Alphabet` (unambiguous 31-symbol default),
  idempotent `normalize`, `CodePolicy` with `default_human` (≥8 chars),
  `short_compat`, and `legacy_ciao_6`, rejection-sampling `generate_code`, and
  `validate_code_input` (RFC-003).
- `hashing` module: `SecretDomain`, `KeyVersion`, `LookupKey` (with constant-
  time `ct_eq`), `KeyProvider`/`HmacKeyRef`, `StaticKeyProvider` (active +
  previous keys, empty key rejected), and `SecretHasher` deriving
  domain-separated HMAC-SHA256 lookup keys via the prefixing scheme (RFC-004).
- `error` module: internal error layer (`RandomError`, `KeyError`,
  `PolicyError`, `CodeInputError`).
- Frozen HMAC test vectors per domain and a reproducible Unicode normalization
  property test (RFC-003 §11.5, RFC-004 §12.3).
- `xtask release-check` now enforces real static gates: no fallback-key literal,
  no silently-defaulted/swallowed RNG result, no debug prints in library code
  (RFC-015 §9). Verified to fail on injected violations.

### Changed

- `codlet-core` is `std` for v0.1; the `std`/`no_std` split is deferred
  (RFC-002 §4).
- RFC-003 and RFC-004 moved from `rfcs/proposed/` to `rfcs/done/`
  (Implemented); RFC index regenerated.

### Security

- Domain-separated lookup keys: the same plaintext yields distinct keys across
  the code/session/form-token/flow-ticket domains (test-enforced).
- Secret-bearing types and key material are redacted in `Debug`/`Display`/serde
  (INV-1, SR-38); tests assert no plaintext leakage.
- codlet lookup keys are intentionally not bit-identical to zinnias-ciao's
  (domain prefix added); the future migration adapter will provide a
  `legacy_no_domain` mode (RFC-004 §9.1, RFC-014).

## [0.0.0] — 2026-06-13

Phase 0 bootstrap. No authentication functionality yet; this release
establishes the repository, process, and an empty `codlet-core` skeleton.

### Added

- Cargo workspace with `codlet-core` (skeleton) and `xtask` (release-gate
  runner skeleton).
- `#![forbid(unsafe_code)]` and a shared workspace lint policy.
- RFC process and directory structure under `rfcs/`, governed by the
  4-folder lifecycle policy (`proposed/`, `done/`, `archive/`), with an index
  at `rfcs/README.md`.
- Design RFC-001 (project scope, product shape, non-goals) and RFC-002 (crate
  architecture, feature flags, runtime matrix) accepted and moved to
  `rfcs/done/`. RFC-003 through RFC-032 placed under `rfcs/proposed/`.
- Recommendation added to RFC-004 favouring HMAC message prefixing for domain
  separation, and noting that codlet lookup keys are deliberately not
  bit-identical to zinnias-ciao's, so the migration adapter needs a
  `legacy_no_domain` mode.
- Project hygiene: `README.md`, `SECURITY.md`, `LICENSE` (Apache-2.0),
  `NOTICE`, `CONTRIBUTING.md`, CI workflow, `rust-toolchain.toml`,
  `.gitignore`.

### Security

- Verified `codlet-core`'s dependency tree contains no web-framework, database,
  or async-executor crates (RFC-002 acceptance gate): only `hmac`, `sha2`,
  `subtle`, `getrandom`, `thiserror`.

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nabbisen/codlet/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/nabbisen/codlet/releases/tag/v0.0.0
