# Changelog

All notable changes to codlet are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and codlet aims to follow
semantic versioning once it reaches a stable release.

## [Unreleased]

Nothing yet.

## [0.2.0] — 2026-06-14

Lifecycle state machines, storage traits, cookie policy, in-memory stores, and
a `Clock` abstraction. `codlet-core` now has all the primitives needed to
express a complete authentication flow at the type level; adapters and
orchestration come next.

### Added

- `clock` module: `Clock` trait + `SystemClock` (production) + `FixedClock`
  (deterministic, `test-utils`) (RFC-020).
- `state` module with three pure classifiers (no I/O, no `async`):
  - `classify_claim` / `ClaimOutcome` — atomic single-winner code claim
    (RFC-005);
  - `classify_token_consume` / `TokenConsumeOutcome` — form-token
    idempotency/CSRF classifier, ported verbatim from `zinnias-ciao`
    `contracts::auth` with its full regression suite (RFC-007);
  - `classify_session` / `SessionValidationOutcome` — session lookup result
    with `Authenticated`/`Unauthenticated` variants (RFC-006).
- `store` module with async traits and supporting types:
  - `CodeStore` (`find_redeemable`, `claim_code`, `insert_code`,
    `revoke_code`) plus `RedeemableCode`, `CodeRecord`, `ClaimRequest`, and
    helpers `expires_at_from_ttl`, `code_lookup_candidates` (RFC-005);
  - `SessionStore` (`find_active_session`, `insert_session`,
    `revoke_session`) plus `ActiveSessionRecord`, `SessionRecord` (RFC-006);
  - `FormTokenStore` (`insert_form_token`, `consume_form_token`,
    `set_token_result`) plus `FormTokenRecord`, `TokenSubject`
    (RFC-007). `TokenSubject::Anonymous/Authenticated/Flow` replaces the
    empty-string anti-pattern from the source service.
  - `store::error`: `StoreError` (internal) and `PublicAuthError`
    (single-variant public-safe collapse per INV-8).
- `cookie` module: `CookiePolicy` with named profiles (`ProductionStrict`,
  `ProductionLax`, `LocalDevelopment`), `build_set_cookie`,
  `build_clear_cookie` (RFC-006). `HttpOnly` is always set; `Secure` is
  mandatory in all production profiles; `Domain` omitted by default.
- `mem` module (`test-utils` feature only): `MemCodeStore`,
  `MemSessionStore`, `MemFormTokenStore` — in-memory implementations of
  the three store traits for tests and local dev. Non-production; documented
  clearly as such (RFC-011 in-memory portion).
- `tokio` as a dev-dependency for async integration tests.
- 21 new acceptance integration tests covering all RFC checklist items:
  find/claim/revoke/expiry for codes; session issuance/validation/revocation;
  form-token winner/replay/invalid/binding-mismatch/purpose-mismatch/expiry;
  `changed == 0` never-proceeds exhaustive check; anonymous vs authenticated
  subject separation; cookie attribute assertions.

### Changed

- RFC-005, RFC-006, RFC-007 moved `proposed/ → done/` (Implemented v0.2.0);
  RFC index regenerated. RFC-011 remains `proposed/` (SQLx adapter is M4;
  only the in-memory portion landed here).

### Security

- `changed == 0` never-proceeds invariant is both a classifier contract
  (`classify_token_consume`) and enforced in `MemFormTokenStore::consume_form_token`.
- `changed > 1` in code claim or form-token consume returns
  `StoreError::InvariantViolation`, never silently maps to `Lost`/`Invalid`.
- `TokenSubject` enum prevents the empty-string anonymous collision present in
  the source service (RFC-007 §13.3).
- Cookie `Domain` omitted by default; subdomain sharing requires explicit opt-in.

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nabbisen/codlet/compare/v0.1.0...v0.2.0

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

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nabbisen/codlet/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nabbisen/codlet/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/nabbisen/codlet/releases/tag/v0.0.0
