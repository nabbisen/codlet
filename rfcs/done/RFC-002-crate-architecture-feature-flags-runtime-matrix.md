# RFC-002: Crate Architecture, Feature Flags, and Runtime Matrix

- **Status:** Implemented (v0.0.0)
- **Target milestone:** M0
- **Primary crate(s):** workspace-wide
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define the codlet workspace structure and prevent runtime-specific dependencies from leaking into core.

## 2. Motivation

The source service runs on Cloudflare Workers where D1 handles are `!Send`; common Rust web services run on Axum/Tower where middleware usually requires `Send`. The architecture must support both without compromising either.

## 3. Decision

Use separate crates for core and adapters. `codlet-core` remains runtime-independent. Worker and SQLx/Axum support live in separate crates or strictly optional features.

## 4. Detailed design


Workspace:

```text
crates/codlet-core
crates/codlet-worker
crates/codlet-sqlx
crates/codlet-axum
crates/codlet-test
examples/
rfcs/
```

Core dependency policy:

- no `worker`, `axum`, `tower`, `sqlx`, `tokio`, or Cloudflare-specific crates;
- minimal crypto and error dependencies;
- optional `serde` feature for record serialization;
- optional `std`/`alloc` split may be considered later but is not a v0.1 requirement.

Feature flags:

- `serde` for serializable public types;
- `test-utils` for deterministic fake RNG/clock;
- adapter crates should prefer separate packages rather than heavy features.

Runtime matrix:

| Crate | Target | Async Send? | Storage |
|---|---|---:|---|
| codlet-core | all | not tied | traits only |
| codlet-worker | wasm/Workers | `?Send` | D1/KV |
| codlet-sqlx | server | `Send` | SQLite/Postgres |
| codlet-axum | server | `Send` | user-supplied stores |
```


## 5. Security considerations

Keeping core small reduces supply-chain and runtime attack surface. Runtime adapters must not weaken core security defaults for convenience.

## 6. Host application responsibilities

The host application must integrate this RFC according to the documented boundary and must not treat codlet as an authorization system.

## 7. Tests and release gates


- CI matrix builds core without adapter features.
- CI matrix builds Worker adapter separately.
- CI matrix builds SQLx/Axum adapters separately.
- `cargo tree` review gate for core dependencies.


## 8. Migration notes

No existing application migration is required beyond adopting the new codlet API. 

## 9. Open questions

Whether the root `codlet` crate is a thin re-export from day one or introduced at v1.0. 


## 10. Expanded technical design

### 10.1 Crate responsibility matrix

| Crate | Stable responsibility | Forbidden responsibility | First useful milestone |
|---|---|---|---|
| `codlet-core` | Types, policies, state machines, cryptographic lookup derivation, store traits. | Runtime, framework, DB, HTTP response types. | M1 |
| `codlet-test` | Fake stores, fake clock/RNG, conformance macros/functions. | Production authentication behavior. | M1/M2 |
| `codlet-worker` | D1/KV stores, Worker request/cookie helpers. | zinnias-ciao business tables. | M3 |
| `codlet-sqlx` | SQLx-backed stores and migrations. | Axum-specific request extraction. | M3/M4 |
| `codlet-axum` | Extractors, middleware, cookie response helpers. | Storage policy or core semantics. | M4 |
| `codlet` facade | Re-export stable happy path. | Hide experimental APIs as stable. | v1 or late v0 |

### 10.2 Dependency direction rule

The dependency graph is one-way:

```text
codlet-axum ─┐
codlet-sqlx ─┼─> codlet-core
codlet-worker┘
codlet-test ───> codlet-core
examples ─────> adapters + core
```

No adapter may become a dependency of `codlet-core`. No example-only convenience may be moved into core simply because it is easy.

### 10.3 Feature flag safety classes

Feature flags should be documented by safety class:

| Class | Example | Review standard |
|---|---|---|
| Pure representation | `serde` | Verify no secret serialization. |
| Test-only | `test-utils` | Verify not enabled by default and visibly unsafe for production. |
| Runtime integration | `tracing`, `cookie` | Verify no semantic weakening. |
| Storage integration | SQLx features | Verify conformance tests. |

A feature flag that changes default security policy is not allowed. For example, `insecure-cookies` should not exist as a normal feature. If local development needs non-secure cookies, use an explicit runtime config with warnings and tests that production config rejects it.

### 10.4 MSRV and portability considerations

The architecture should choose an MSRV that supports stable async ergonomics needed by adapters. However, MSRV must not force core to adopt unstable patterns. Worker/WASM builds should be considered from the start: time, randomness, and HMAC dependencies must compile on intended targets or be abstracted behind traits.

### 10.5 Concrete acceptance checklist

- [ ] `cargo tree -p codlet-core` contains no framework, DB, or executor crates.
- [ ] Worker adapter compiles in a WASM-oriented target configuration.
- [ ] At least one server adapter can expose `Send` futures.
- [ ] Feature documentation states security impact.
- [ ] Adapter crates publish a conformance matrix.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
