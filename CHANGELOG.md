# Changelog

All notable changes to codlet are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and codlet aims to follow
semantic versioning once it reaches a stable release.

## [Unreleased]

Nothing yet.

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

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.0.0...HEAD
[0.0.0]: https://github.com/nabbisen/codlet/releases/tag/v0.0.0
