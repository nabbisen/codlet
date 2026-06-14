# RFC-009: Async Trait Strategy, Runtime Matrix, and Adapter Contracts

- **Status:** Implemented (v0.4.0)
- **Target milestone:** M4
- **Primary crate(s):** codlet-core + all adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Resolve the `Send` versus `?Send` split required by Workers and conventional Rust web frameworks.

## 2. Motivation

Cloudflare Workers/D1 handles may not be `Send`, while Axum/Tower integrations generally require `Send`. A single careless async trait design could block one target.

## 3. Decision

Define core store traits with `?Send` compatibility and provide Send-safe wrappers or parallel adapter traits for Send runtimes if necessary. Adapter conformance tests, not trait inheritance alone, define security behavior.

## 4. Detailed design


Options considered:

1. `async_trait(?Send)` traits everywhere;
2. GAT/future associated types with optional Send bounds;
3. separate `LocalCodeStore` and `CodeStore` traits;
4. synchronous command objects with adapter-specific executors.

Initial decision:

- use `async_trait(?Send)` for core local traits in v0.1/v0.2;
- provide Send adapter wrappers for Axum/Tower if practical;
- revisit with native async traits/GAT design before v1 API freeze.

Adapter contracts:

- every adapter must declare whether operations are atomic;
- every adapter must run the shared conformance suite;
- framework middleware must not require Worker-only types.


## 5. Security considerations

The trait strategy must not create two subtly different security semantics. `LocalCodeStore` and `SendCodeStore`, if split, must share a conformance suite and identical outcomes.

## 6. Host application responsibilities

The host chooses the adapter matching its runtime. It should not wrap non-Send stores in unsafe Send abstractions.

## 7. Tests and release gates


- Core compiles for Worker target.
- Axum adapter compiles with Send requirements.
- Same conformance tests run against all stores.
- No `unsafe impl Send` in adapter code unless separately reviewed by RFC.


## 8. Migration notes

No existing application migration is required beyond adopting the new codlet API. 

## 9. Open questions

Before v1, re-evaluate native async trait support and whether the public trait model should avoid `async_trait`. 


## 11. Expanded technical design

### 11.1 Semantic trait versus transport helper

The store traits should be transport-agnostic. A store method should never receive an HTTP request. Instead, adapters extract request data and create semantic requests:

```text
HTTP Request -> Adapter extraction -> RedeemCodeRequest -> codlet-core service -> Store trait
```

This is essential for testability and for non-HTTP uses such as CLI/admin tools.

### 11.2 Trait stability strategy

Before v1, mark store traits as "adapter API" and reserve breaking changes. Stabilize request/response value types first because those encode the security contract. Trait ergonomics can be revised after Worker and SQLx adapters both exist.

### 11.3 Transaction-capable design

Do not hard-code `&self` as the only way to access persistence if transaction handles will be needed. Possible future pattern:

```text
Store             creates or borrows transaction-capable context
StoreTransaction  implements the same semantic operations inside a DB transaction
```

The initial design can document this as reserved, but public APIs should not prevent it.

### 11.4 Concrete acceptance checklist

- [x] Store method inputs are semantic structs, not framework request objects.
- [~] Worker compile test proves no accidental `Send` requirement. *(Note: deferred: Workers CI requires wasm32 target and Cloudflare credentials not available in this environment)*
- [~] Axum compile test proves server-side `Send` integration remains ergonomic. *(Note: deferred: Axum adapter crate not yet in scope)*
- [x] Store trait documentation states which operations are atomic and fail-closed.
- [x] Adapter conformance tests are parameterized over store implementations.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
