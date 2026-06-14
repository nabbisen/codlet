# RFC-015: Test Strategy, Fuzzing, Release Gates, and Security Regression Tests

- **Status:** Implemented (v0.6.0)
- **Target milestone:** M6
- **Primary crate(s):** workspace-wide
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Turn the handoff invariants into automated tests and release gates.

## 2. Motivation

Authentication crates need regression resistance. The service already has useful tests; codlet should promote them into a shared conformance suite.

## 3. Decision

codlet releases require unit, property, integration, adapter conformance, fuzz, and static release gates for critical invariants.

## 4. Detailed design


Test layers:

1. pure unit tests for normalization, generation, hashing, classification;
2. property tests for normalization and rejection sampling;
3. store conformance tests for code/session/form-token stores;
4. adapter integration tests;
5. web-framework example tests;
6. release-gate static scans.

Critical release gates:

- no fallback key string;
- no `unwrap_or_default` or `ok()` suppression around RNG;
- cookie attributes enforced;
- `changed == 0` never proceeds;
- exactly one claim winner;
- generic public error mapping;
- no plaintext secret in serialized store records or audit events.

Fuzz targets:

- code normalization;
- code validation;
- cookie parsing if codlet parses cookies;
- form-token classification input model.


## 5. Security considerations

Tests are part of the security design. Every adapter must pass the same behavioral suite or clearly document why it is not production-safe.

## 6. Host application responsibilities

Host applications should copy selected integration tests into their own CI, especially full join and double-submit flows.

## 7. Tests and release gates


This RFC defines the required tests. Implementation is complete when CI enforces them across all supported crates.


## 8. Migration notes

Existing zinnias-ciao tests should be copied or reimplemented in codlet, then the service should depend on codlet tests for shared behavior. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Test taxonomy

codlet's test suite should be organized by invariant, not only by module:

| Test family | Examples | Blocks release? |
|---|---|---:|
| Pure unit | normalization, HMAC vectors, error mapping | Yes |
| Property/fuzz | arbitrary input normalization, token parsing | Yes for core targets |
| Store conformance | claim/consume exactly-one-winner | Yes for production adapters |
| Framework integration | cookie/extractor behavior | Yes for adapter release |
| Static release gates | no fallback keys, no plaintext logs | Yes |
| Documentation examples | quickstarts compile and use safe defaults | Yes before v1 |

### 10.2 Security regression gates

The following regressions should be named and blocked:

- RNG failure fallback returns deterministic code.
- Rejection sampling replaced by modulo-only mapping.
- `changed == 0` proceeds in claim/consume.
- Public error reveals code exists/expired/used.
- Cookie helper omits `HttpOnly`, `Secure`, or SameSite default.
- Plaintext bearer secret appears in persisted record/debug/audit.
- Key provider silently falls back to a static default.

### 10.3 Fuzz/property targets

- code input normalization and validation;
- token/cookie parsing if implemented;
- redaction wrappers under debug/serde;
- state-machine classification for changed/found/expired/consumed combinations.

### 10.4 Concrete acceptance checklist

- [x] `codlet-test` can run the same store conformance suite against in-memory, D1, and SQLx stores.
- [~] CI separates core, adapters, examples, and docs. *(Note: done: CI now has per-crate jobs (core, conformance, sqlx, examples) since v0.8.0+audit)*
- [x] Security gates are documented as release blockers.
- [x] Regression tests include the exact two service-side bugs already fixed: RNG fail-open and modulo bias.
- [~] v1 release notes include security policy and supported adapter matrix. *(Note: deferred: pre-v1; v1 release notes will include security policy and adapter matrix)*


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
