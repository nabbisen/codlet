# Changelog

All notable changes to codlet are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and codlet aims to follow
semantic versioning once it reaches a stable release.

## [Unreleased]

Nothing yet.

## [0.8.0] — 2026-06-14

Final planned RFC sprint: observability hooks, admin API, security policy, and
closing out all remaining proposed RFCs. All 31 planned RFCs are now
implemented; RFC-018 (future server/IdP) is archived as post-v1. 142 tests.

### Added

- `metrics` module (RFC-024): `MetricsObserver` trait (fire-and-forget
  counter/outcome hook), `NoopMetrics` (default zero-cost implementation),
  `CapturingMetrics` (test-utils), `Outcome` enum with stable `label()`
  strings, and `counter` module with 8 recommended counter-name constants.
  Gate: counter names verified to contain no sensitive vocabulary.

- `admin` module (RFC-030): `CodeAdminStore` optional extension trait with
  `list_codes` (with `CodeListFilter`) and `get_code_meta`; `CodeMeta`
  metadata record (no plaintext code, no HMAC lookup key — enforced by type
  design and test); `CodeStats` aggregate; `CodeListFilter` with scope/active
  helpers; in-memory stub for `MemCodeStore` under `test-utils`.

- `SECURITY.md` (RFC-028): complete security policy covering supported
  versions, MSRV policy (1.85, never raised in patch), reporting address and
  response targets, disclosure policy, advisory format, and explicit list of
  what constitutes a security bug. Release checklist matches the 5 `xtask`
  gates.

### Changed

- RFC-024, RFC-025, RFC-026, RFC-027, RFC-028, RFC-029, RFC-030, RFC-031,
  RFC-032 moved `proposed/ → done/` (Implemented v0.8.0). All 31 planned
  RFCs are now implemented.
- RFC-018 (future server/IdP strategy) moved to `archive/` as post-v1 deferred.
- `proposed/` directory is now empty.

### Security

- `CodeMeta` contains no plaintext code value and no HMAC lookup key; a
  `Debug`-output test asserts no sensitive vocabulary appears in the type.
- `MetricsObserver::increment` must not block; `counter` names are tested to
  contain no sensitive vocabulary (no `key`, `secret`, `hmac`, etc. in the
  label strings that would be exported to metric backends).
- SECURITY.md is now complete and linked from README (RFC-028 acceptance).

## [0.7.0] — 2026-06-14

Documentation layer and compilable examples (RFC-016), plus the RFC-010
groundwork. 22 RFCs implemented, 10 remaining, 133 tests, 5 gates.

### Added

- **`codlet-examples`** (new crate, RFC-016): three compilable binaries that
  each run end-to-end and produce correct output:
  - `sqlite_quickstart` — complete issue→validate→claim→session→validate flow
    using `codlet-sqlx`; shows host authorization note after authentication.
  - `key_rotation` — configures active + previous keys, re-derives a v1 record
    under the rotated config, then proves missing-version fails closed.
  - `form_token_csrf` — issue, first-submit Proceed, duplicate-submit Replay,
    wrong-subject Invalid, wrong-purpose Invalid; includes UX copy guidance.
  All examples follow RFC-016 §10.2 rules (no hard-coded production secrets,
  no plaintext code logging, safe defaults throughout).

- `docs/src/threat-model.md` — what codlet protects against (online guessing,
  double-claim, session forgery, code enumeration, plaintext persistence, JS
  cookie access, weak HMAC), what it does NOT protect against (authorization,
  user management, offline attack after key+DB leak, KV eventual consistency),
  and the 8 security invariants with their INV-N labels.

- `docs/src/adapter-matrix-and-config.md` — adapter guarantee matrix (atomic
  claim/consume, multi-process safety for each adapter), secure configuration
  guide (code policy, key provider, cookie policy, rate limiting), and
  user-facing copy guidance table (say "code" not "token", generic failure
  messages, no jargon).

- `docs/src/SUMMARY.md` updated with threat model and adapter matrix pages.

- `xtask release-check`: `no-debug-prints` gate now exempts `codlet-examples`
  binaries (intentional terminal output for demonstrations).

### Changed

- RFC-010 (Cloudflare Workers/D1 adapter) and RFC-016 (documentation and
  examples) moved `proposed/ → done/` (Implemented v0.7.0).
  22 RFCs implemented total; 10 remaining (post-v1 / future).

### Security

- All three examples verified to compile and run correctly in CI (`cargo run`),
  satisfying RFC-016 §10.4 "all example code compiles."
- Example binaries use `production_strict` cookie policy, 8-char codes, and
  `StaticKeyProvider` with a clearly-labelled placeholder key.

## [0.6.0] — 2026-06-14

Typestate completions, two new release gates, key rotation and migration docs,
and `.gitignore` updated to the standard Cargo template. 133 tests, 5 static
release gates, 20 RFCs implemented.

### Added

- `secret` module additions (RFC-019):
  - `NormalizedCode` — distinct type for the post-normalization canonical form,
    preventing confusion between raw user input and the value passed to HMAC
    derivation.
  - `Purpose` — validated non-empty purpose label; `Purpose::new("")` returns
    `None`.
  - `ScopeKey` — host-owned scope/boundary label.
  All three are exported from `codlet_core` root.

- Two new `xtask release-check` gates (RFC-015):
  - `cookie-attrs-present` — verifies `HttpOnly`, `Secure`, and `SameSite`
    appear in `cookie.rs`; fails if any is removed.
  - `no-plaintext-in-store-ops` — bans `.expose()` on the same line as
    `.bind(` or `INSERT` in library source, preventing accidental plaintext
    persistence.
  Both gates verified to fire on injected violations. Total: 5 static gates.

- `docs/src/key-rotation.md` — operational key management and rotation guide
  (RFC-017): key states, planned rotation procedure, emergency compromise
  procedure, what codlet does vs. does not do.

- `docs/src/migration-from-zinnias-ciao.md` — migration plan (RFC-014):
  HMAC incompatibility explanation, parallel lookup strategy, schema migration
  SQL, column mapping table, cookie name compatibility, checklist.

- `docs/src/SUMMARY.md` updated with new pages.

- `.gitignore` replaced with the standard Cargo template (covers `debug/`,
  `target`, `*.rs.bk`, `*.pdb`, `mutants.out*/`, RustRover hints).

### Changed

- RFC-014, RFC-015, RFC-017, RFC-019 moved `proposed/ → done/`
  (Implemented v0.6.0). 20 RFCs total implemented, 12 remaining proposed.

## [0.5.0] — 2026-06-14

First production adapter: SQLite via SQLx. A new shared conformance test suite
verifies that all adapters — in-memory and SQLite — satisfy the single-winner
claim/consume contracts under real concurrency. 130 tests total across the
workspace.

### Added

- **`codlet-conformance`** (new crate, RFC-023): parameterised async conformance
  test suite. Contains `run_code_store_conformance`, `run_session_store_conformance`,
  and `run_form_token_store_conformance` — each takes a factory function and
  runs the full RFC-023 required test list against any store implementation.
  The code-store suite includes the concurrent-claim race test (RFC-022):
  8 tasks hit a `tokio::Barrier` simultaneously and exactly one must win.
  Split into four modules: `fixtures`, `code`, `session`, `token`.

- **`codlet-sqlx`** (new crate, RFC-011): SQLite adapter implementing all three
  store traits using a single embedded migration (`0001_initial.sql`). Code claim
  and form-token consume use a conditional `UPDATE … WHERE … AND <guard>` with
  affected-row count check (RFC-022). Uses WAL mode for concurrent access.
  Passes all five conformance tests including the concurrent race test.

- `codlet-sqlx`: migration runner (`run_migrations`) embedded via `include_str!`,
  idempotent (`IF NOT EXISTS`), safe to call on startup.

- `codlet-sqlx`: `SqliteStore` — a cheaply-clonable pool wrapper implementing
  `CodeStore + SessionStore + FormTokenStore`.

- `.gitignore` updated to the standard Cargo template (covers `debug/`, `target`,
  `*.rs.bk`, `*.pdb`, `mutants.out*/`, RustRover hints).

### Changed

- RFC-011, RFC-022, RFC-023 moved `proposed/ → done/` (Implemented v0.5.0).
  16 RFCs total implemented.

### Security

- The SQLite conditional UPDATE pattern is documented in `codlet-sqlx/src/code.rs`
  and `token.rs` to make the atomicity requirement visible at the implementation
  site, not just in the trait docs.
- `changed > 1` in code claim and form-token consume returns
  `StoreError::InvariantViolation` — same as the in-memory adapter.
- The conformance suite's race test is run under `tokio::task::LocalSet` with
  a `Barrier`, so it works with both `Send` and `!Send` store implementations.

## [0.4.0] — 2026-06-14

High-level orchestration layer (`auth` module). A host can now implement a
complete authentication flow end-to-end — issue, find, claim, session — without
writing glue code against every primitive individually. 122 tests total.

### Added

- `auth` module with five sub-modules (RFC-013, RFC-009):
  - `auth::code`: `CodeAuth<CS, RL, K, C, A>` manager with `issue_code`,
    `find` (rate-limit check → input validation → store lookup), `claim`
    (atomic won/lost with rate-limit record/clear), `redeem_with_callback`
    (full RFC-013 §10.3 8-step flow order), and `revoke_code`. Session
    issuance is only possible after `claim` or `redeem_with_callback` returns
    `RedeemSuccess` — enforced at the type level via `ClaimProof`.
  - `auth::session`: `SessionManager<SS, K, C, A>` with `issue` (requires
    `RedeemSuccess` proof), `validate`, and `revoke` (returns clear-cookie
    header string). Generates 32-byte session secrets; stores only the HMAC
    lookup key; plaintext leaves only in the `Set-Cookie` value.
  - `auth::token`: `FormTokenManager<TS, K, C, A>` with `issue`, `consume`
    (idempotency replay with `result_ref`), and `set_result`.
  - `auth::norate`: `NoRateLimit` — zero-cost opt-out `RateLimitStore` for
    hosts that handle rate limiting at the network layer.
  - `auth::error`: `RedeemError` (5 variants, each carrying internal reason +
    public mapping), `SessionError`, `FormTokenError`, `IssuedSession`,
    `RedeemSuccess`, `ClaimProof` (zero-size proof token).
- 11 new acceptance integration tests covering every RFC-013 checklist item:
  complete two-step issue→find→claim→session round trip; callback-based flow;
  lost claim has no proof (`Err`, not a session); host callback error leaves
  claim consumed but no session issued; public errors are generic regardless of
  internal cause; expired session returns `Unauthenticated`; logout clears
  session and returns `Max-Age=0` cookie; wrong-subject form-token rejected.

### Changed

- RFC-013 and RFC-009 moved `proposed/ → done/` (Implemented v0.4.0).
- `auth/code.rs` split: `NoRateLimit` extracted to `auth/norate.rs` to keep
  all source files within the 300-ELOC guideline.

### Security

- Session issuance is structurally gated: `SessionManager::issue` accepts only
  a `RedeemSuccess`, which wraps a `ClaimProof` that is only constructible when
  `claim_code` returns `ClaimOutcome::Won`. The compiler prevents issuing a
  session without a confirmed won claim (RFC-013 §5).
- `redeem_with_callback` enforces RFC-013 §10.3 step order: rate-limit check
  and input validation happen before the claim; the host callback runs only
  after a confirmed won claim; if the callback fails, no session is issued and
  the code is consumed (host must compensate — documented).
- Public errors from all orchestration paths return generic messages regardless
  of the internal cause, verified by test.

## [0.3.0] — 2026-06-14

M3 complete: rate limiting, two-layer error model, and audit events.
`codlet-core` now contains the full primitive layer — every security-critical
concept has a type, a classifier, a store trait, and a test. 111 tests total.

### Added

- `audit` module: `CodeAuthEvent` enum (10 variants, stable `noun.verb.outcome`
  keys), `AuditSink` trait, `NoopAuditSink`, and (under `test-utils`)
  `CollectingAuditSink`. All event fields are **redacted by construction**: no
  plaintext code, token, session secret, lookup key, or raw IP address appears
  in any variant (RFC-012).
- `store::ratelimit`: `RateLimitKey`, `RateLimitPolicy` (with
  `default_invite()`: 10 failures / 5 min / key), `RateLimitUnavailable`
  (`FailOpen` / `FailClosed` / `SoftDenyAfterThreshold`), `RateLimitOutcome`,
  and `RateLimitStore` trait with `check` / `record_failure` / `clear_failures`
  (RFC-008).
- `mem::MemRateLimitStore` — in-memory rate-limit store (`test-utils`, RFC-008
  in-memory portion). Documents its best-effort counter atomicity.
- Error model extensions in `error` module (RFC-012/021):
  - `RedemptionFailReason` — 7 internal variants for logging/metrics.
  - `PublicRedemptionError` — `InvalidOrExpired` / `RateLimited` /
    `TemporarilyUnavailable`, with `from_reason()` mapping.
  - `PublicFormError` — `ExpiredOrInvalid` / `TemporarilyUnavailable`.
  - `PublicSessionError` — `MissingOrExpired` / `TemporarilyUnavailable`.
- 18 new acceptance integration tests covering all RFC-008 and RFC-012
  checklist items: enumeration collapse, rate-limit threshold/clear/isolation,
  fail-open default, fingerprint privacy, audit-event key stability, no-secret
  in event debug output.

### Changed

- RFC-008, RFC-012, RFC-020, RFC-021 moved `proposed/ → done/`
  (Implemented v0.3.0). RFC index regenerated.

### Security

- All enumeration-sensitive redemption states (`NotFound`, `Expired`,
  `Revoked`, `AlreadyUsed`, `InvalidFormat`) map to `InvalidOrExpired` — a
  single public error — via `PublicRedemptionError::from_reason()`. Test-
  enforced exhaustively.
- `RateLimitKey::fingerprint()` returns a prefix safe for audit events and
  metrics labels; the full key is never emitted in `CodeAuthEvent::RateLimitHit`.
- `CodeAuthEvent` is `#[non_exhaustive]` so adding variants is non-breaking.

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

[Unreleased]: https://github.com/nabbisen/codlet/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/nabbisen/codlet/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/nabbisen/codlet/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/nabbisen/codlet/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/nabbisen/codlet/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/nabbisen/codlet/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/nabbisen/codlet/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/nabbisen/codlet/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nabbisen/codlet/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/nabbisen/codlet/releases/tag/v0.0.0
