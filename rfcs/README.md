# codlet RFCs

Governed by `000-rfc-lifecycle-policy.md` (listed below at its `done/` path).
The folder is the source of truth for an RFC's state; the `Status` field
inside each file mirrors it.

- `proposed/` — open for review
- `accepted/` — review complete; implementer may start
- `done/` — Implemented (fully or partially)
- `archive/` — Withdrawn or superseded

Counts below are derived from the filesystem, not hand-maintained.

## Proposed (0)

No open RFCs.

## Accepted (3)

| ID | Title | Milestone |
|----|-------|-----------|
| [039](./accepted/039-supply-chain-scanning.md) | Supply-Chain Scanning | M5 |
| [040](./accepted/040-invariant-verification.md) | Invariant Verification — Every Guard Observed Failing | M5 |
| [042](./accepted/042-retire-the-cookie-attrs-gate.md) | Retire `cookie-attrs-present` — a Text Grep Cannot Assert Emitted Behaviour | M5 |

## Implemented (38)

| ID | Title | Version |
|----|-------|---------|
| [000](./done/000-rfc-lifecycle-policy.md) | RFC Lifecycle Policy | v0.0.0 |
| [001](./done/001-project-scope-product-shape-non-goals.md) | Project Scope, Product Shape, and Non-goals | v0.0.0 |
| [002](./done/002-crate-architecture-feature-flags-runtime-matrix.md) | Crate Architecture, Feature Flags, and Runtime Matrix | v0.0.0 |
| [003](./done/003-one-time-code-policy-generation-normalization-validation.md) | One-Time Code Policy, Generation, Normalization, and Validation | v0.1.0 |
| [004](./done/004-secret-hashing-key-providers-domain-separation-key-versioning.md) | Secret Hashing, Key Providers, Domain Separation, and Key Versioning | v0.1.0 |
| [005](./done/005-code-lifecycle-storage-contract-atomic-redemption.md) | Code Lifecycle, Storage Contract, and Atomic Redemption | v0.2.0 |
| [006](./done/006-session-issuance-validation-revocation-cookie-policy.md) | Session Issuance, Validation, Revocation, and Cookie Policy | v0.2.0 |
| [007](./done/007-form-tokens-csrf-protection-idempotency-replay.md) | Form Tokens, CSRF Protection, and Idempotency Replay | v0.2.0 |
| [008](./done/008-rate-limiting-abuse-resistance.md) | Rate Limiting and Abuse Resistance | v0.3.0 |
| [009](./done/009-async-trait-strategy-runtime-matrix-adapter-contracts.md) | Async Trait Strategy, Runtime Matrix, and Adapter Contracts | v0.4.0 |
| [010](./done/010-cloudflare-workers-d1-kv-adapter.md) | Cloudflare Workers, D1, and KV Adapter | v0.7.0 |
| [011](./done/011-sqlx-in-memory-adapters.md) | SQLx and In-Memory Adapters | v0.5.0 |
| [012](./done/012-error-model-privacy-audit-events.md) | Error Model, Privacy, and Audit Events | v0.3.0 |
| [013](./done/013-high-level-orchestration-api-host-application-boundary.md) | High-Level Orchestration API and Host Application Boundary | v0.4.0 |
| [014](./done/014-zinnias-ciao-migration-compatibility-plan.md) | Existing Service Migration and Compatibility Plan | v0.6.0 *(partial)* |
| [015](./done/015-test-strategy-fuzzing-release-gates-security-regression-tests.md) | Test Strategy, Fuzzing, Release Gates, and Security Regression Tests | v0.6.0 *(partial)* |
| [016](./done/016-documentation-examples-non-technical-ux-guidance.md) | Documentation, Examples, and Non-Technical UX Guidance | v0.7.0 |
| [017](./done/017-security-operations-key-management-rotation.md) | Security Operations, Key Management, and Rotation | v0.6.0 |
| [019](./done/019-typestate-api-and-opaque-newtypes.md) | Typestate API and Opaque Newtypes | v0.6.0 |
| [020](./done/020-randomness-clock-and-deterministic-testing.md) | Randomness, Clock, and Deterministic Testing | v0.3.0 |
| [021](./done/021-error-taxonomy-and-user-facing-copy-contract.md) | Error Taxonomy and User-Facing Copy Contract | v0.3.0 |
| [022](./done/022-database-atomicity-isolation-and-race-semantics.md) | Database Atomicity, Isolation, and Race Semantics | v0.5.0 |
| [023](./done/023-adapter-conformance-test-suite.md) | Adapter Conformance Test Suite | v0.5.0 |
| [024](./done/024-observability-metrics-and-redaction.md) | Observability, Metrics, and Redaction | v0.8.0 |
| [025](./done/025-audit-sink-and-security-event-vocabulary.md) | Audit Sink and Security Event Vocabulary | v0.8.0 |
| [026](./done/026-examples-and-reference-applications.md) | Examples and Reference Applications | v0.8.0 |
| [027](./done/027-runtime-portability-wasm-and-send-strategy.md) | Runtime Portability, WASM, and `?Send` Strategy | v0.8.0 |
| [028](./done/028-security-policy-advisory-process-msrv-and-release-discipline.md) | Security Policy, Advisory Process, MSRV, and Release Discipline | v0.8.0 |
| [029](./done/029-idempotency-result-persistence.md) | Idempotency Result Persistence | v0.8.0 |
| [030](./done/030-administrative-code-management-api.md) | Administrative Code Management API | v0.8.0 |
| [031](./done/031-key-rotation-grace-period-and-retirement.md) | Key Rotation Grace Period and Retirement | v0.8.0 |
| [032](./done/032-code-delivery-channel-boundary.md) | Code Delivery Channel Boundary | v0.8.0 |
| [033](./done/033-cloudflare-workers-d1-kv-adapter-implementation.md) | Cloudflare Workers / D1 / KV Adapter (`codlet-worker`) | v0.11.0 |
| [034](./done/034-postgresql-adapter-implementation.md) | PostgreSQL Adapter (`codlet-sqlx` postgres feature) | v0.12.0 |
| [035](./done/035-rfc-directory-conformance-and-naming.md) | RFC Directory Conformance, Naming, and Lifecycle-Policy Placement | v0.18.0 |
| [036](./done/036-gate-integrity-ci-conformance-and-msrv.md) | Gate Integrity — CI Conformance, MSRV Enforcement, and Release-Discipline Accuracy | v0.18.0 |
| [037](./done/037-withdraw-codlet-axum-framework-adapter.md) | Withdraw the `codlet-axum` Framework Adapter from Planned Scope | v0.18.0 |
| [038](./done/038-migration-runner-must-not-parse-sql.md) | The Migration Runner Must Not Parse SQL | v0.18.0 |

RFC-002 describes `crates/codlet-axum` and `crates/codlet-test` in its
workspace layout; neither was built. `codlet-test`'s role was filled by
`codlet-conformance`, and `codlet-axum` was formally withdrawn from planned
scope by RFC-037. RFC-002 itself is not edited — it is an Implemented RFC and
a historical record; the divergence is tracked here and in `ROADMAP.md`.

RFCs with a companion implementation handoff under
[`rfcs/handoffs/`](./handoffs/): [035](./handoffs/035-rfc-directory-conformance-and-naming/),
[036](./handoffs/036-gate-integrity-ci-conformance-and-msrv/),
[038](./handoffs/038-migration-runner-must-not-parse-sql/). A handoff has no
lifecycle state of its own — it inherits the state of its governing RFC.

## Archive (1)

| ID | Title | Reason |
|----|-------|--------|
| [018](./archive/018-future-server-idp-crate-strategy.md) | Future Server / IdP Crate Strategy | Withdrawn — deferred post-v1 |
