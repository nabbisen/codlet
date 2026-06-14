# codlet RFCs

Design proposals for codlet, governed by the RFC lifecycle policy in
`000-rfc-lifecycle-policy.md`. The folder a file lives in is the source of
truth for its state; the `Status` field inside each RFC mirrors its folder.

- `proposed/` — open for review; implementer should not yet rely on the design.
- `done/`     — accepted and Implemented.
- `archive/`  — withdrawn or superseded.

## Implemented

| ID | Title | Status |
|----|-------|--------|
| RFC-001 | [Project Scope, Product Shape, and Non-goals](./done/RFC-001-project-scope-product-shape-non-goals.md) | Implemented (v0.0.0) |
| RFC-002 | [Crate Architecture, Feature Flags, and Runtime Matrix](./done/RFC-002-crate-architecture-feature-flags-runtime-matrix.md) | Implemented (v0.0.0) |
| RFC-003 | [One-Time Code Policy, Generation, Normalization, and Validation](./done/RFC-003-one-time-code-policy-generation-normalization-validation.md) | Implemented (v0.1.0) |
| RFC-004 | [Secret Hashing, Key Providers, Domain Separation, and Key Versioning](./done/RFC-004-secret-hashing-key-providers-domain-separation-key-versioning.md) | Implemented (v0.1.0) |
| RFC-005 | [Code Lifecycle, Storage Contract, and Atomic Redemption](./done/RFC-005-code-lifecycle-storage-contract-atomic-redemption.md) | Implemented (v0.2.0) |
| RFC-006 | [Session Issuance, Validation, Revocation, and Cookie Policy](./done/RFC-006-session-issuance-validation-revocation-cookie-policy.md) | Implemented (v0.2.0) |
| RFC-007 | [Form Tokens, CSRF Protection, and Idempotency Replay](./done/RFC-007-form-tokens-csrf-protection-idempotency-replay.md) | Implemented (v0.2.0) |

## Proposed

| ID | Title |
|----|-------|
| RFC-008 | [Rate Limiting and Abuse Resistance](./proposed/RFC-008-rate-limiting-abuse-resistance.md) |
| RFC-009 | [Async Trait Strategy, Runtime Matrix, and Adapter Contracts](./proposed/RFC-009-async-trait-strategy-runtime-matrix-adapter-contracts.md) |
| RFC-010 | [Cloudflare Workers, D1, and KV Adapter](./proposed/RFC-010-cloudflare-workers-d1-kv-adapter.md) |
| RFC-011 | [SQLx and In-Memory Adapters](./proposed/RFC-011-sqlx-in-memory-adapters.md) |
| RFC-012 | [Error Model, Privacy, and Audit Events](./proposed/RFC-012-error-model-privacy-audit-events.md) |
| RFC-013 | [High-Level Orchestration API and Host Application Boundary](./proposed/RFC-013-high-level-orchestration-api-host-application-boundary.md) |
| RFC-014 | [zinnias-ciao Migration and Compatibility Plan](./proposed/RFC-014-zinnias-ciao-migration-compatibility-plan.md) |
| RFC-015 | [Test Strategy, Fuzzing, Release Gates, and Security Regression Tests](./proposed/RFC-015-test-strategy-fuzzing-release-gates-security-regression-tests.md) |
| RFC-016 | [Documentation, Examples, and Non-Technical UX Guidance](./proposed/RFC-016-documentation-examples-non-technical-ux-guidance.md) |
| RFC-017 | [Security Operations, Key Management, and Rotation](./proposed/RFC-017-security-operations-key-management-rotation.md) |
| RFC-018 | [Future Server / IdP Crate Strategy](./proposed/RFC-018-future-server---idp-crate-strategy.md) |
| RFC-019 | [Typestate API and Opaque Newtypes](./proposed/RFC-019-typestate-api-and-opaque-newtypes.md) |
| RFC-020 | [Randomness, Clock, and Deterministic Testing](./proposed/RFC-020-randomness-clock-and-deterministic-testing.md) |
| RFC-021 | [Error Taxonomy and User-Facing Copy Contract](./proposed/RFC-021-error-taxonomy-and-user-facing-copy-contract.md) |
| RFC-022 | [Database Atomicity, Isolation, and Race Semantics](./proposed/RFC-022-database-atomicity-isolation-and-race-semantics.md) |
| RFC-023 | [Adapter Conformance Test Suite](./proposed/RFC-023-adapter-conformance-test-suite.md) |
| RFC-024 | [Observability, Metrics, and Redaction](./proposed/RFC-024-observability-metrics-and-redaction.md) |
| RFC-025 | [Audit Sink and Security Event Vocabulary](./proposed/RFC-025-audit-sink-and-security-event-vocabulary.md) |
| RFC-026 | [Examples and Reference Applications](./proposed/RFC-026-examples-and-reference-applications.md) |
| RFC-027 | [Runtime Portability, WASM, and `?Send` Strategy](./proposed/RFC-027-runtime-portability-wasm-and-send-strategy.md) |
| RFC-028 | [Security Policy, Advisory Process, MSRV, and Release Discipline](./proposed/RFC-028-security-policy-advisory-process-msrv-and-release-discipline.md) |
| RFC-029 | [Idempotency Result Persistence](./proposed/RFC-029-idempotency-result-persistence.md) |
| RFC-030 | [Administrative Code Management API](./proposed/RFC-030-administrative-code-management-api.md) |
| RFC-031 | [Key Rotation Grace Period and Retirement](./proposed/RFC-031-key-rotation-grace-period-and-retirement.md) |
| RFC-032 | [Code Delivery Channel Boundary](./proposed/RFC-032-code-delivery-channel-boundary.md) |

## Archive

_None yet._
