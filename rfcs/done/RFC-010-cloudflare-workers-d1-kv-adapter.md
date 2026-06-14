# RFC-010: Cloudflare Workers, D1, and KV Adapter

- **Status:** Implemented (v0.7.0)
- **Target milestone:** M4
- **Primary crate(s):** codlet-worker
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Provide the first production adapter for Cloudflare Workers using D1 for persistent records and KV for rate limiting.

## 2. Motivation

The source service already runs on Workers/D1/KV, making this adapter the lowest-friction extraction path and the best reference for codlet adoption.

## 3. Decision

Implement `codlet-worker` with D1-backed code/session/form-token stores, KV rate limiting, Worker secret loading, and cookie helpers.

## 4. Detailed design


Components:

- `WorkerKeyProvider` loads active/previous HMAC keys from `Env` secrets.
- `D1CodeStore` implements `CodeStore`.
- `D1SessionStore` implements `SessionStore`.
- `D1FormTokenStore` implements `FormTokenStore`.
- `KvRateLimitStore` implements `RateLimitStore`.
- request helpers extract cookies and host-provided rate-limit keys.

Configuration:

```rust
pub struct D1TableConfig {
    pub codes: String,
    pub sessions: String,
    pub form_tokens: String,
}
```

Compatibility mode supports existing service table names and maps columns where possible.

D1 limitations:

- codlet can make individual claim/consume updates atomic;
- it cannot make host-owned user creation plus session creation atomic unless the host runtime supports a transaction covering all writes;
- docs must describe orphaned claim recovery.


## 5. Security considerations

Worker adapter must preserve fail-closed key loading and atomic conditional updates. KV rate limiting may be fail-open only when explicitly configured.

## 6. Host application responsibilities

The host must bind D1/KV namespaces correctly, configure secrets, and understand transaction boundaries around its own user/membership writes.

## 7. Tests and release gates


- D1 claim exactly-one-winner test.
- D1 token consume replay test.
- D1 expired/revoked filters.
- KV threshold and failure-policy tests.
- Worker key missing fails closed.
- Compatibility schema smoke test against zinnias-ciao-like tables.


## 8. Migration notes

zinnias-ciao should adopt this adapter after pure-core extraction. Table-name configuration avoids immediate schema rename. 

## 9. Open questions

None at this stage. 


## 12. Expanded technical design

### 12.1 Worker adapter layering

`codlet-worker` should have submodules by responsibility:

```text
worker::d1::code_store
worker::d1::session_store
worker::d1::form_token_store
worker::kv::rate_limit_store
worker::http::cookies
worker::http::client_identity
worker::migrations
```

This prevents Cloudflare HTTP convenience code from mixing with D1 atomicity code.

### 12.2 D1 timestamp policy

Use one timestamp convention throughout the adapter: Unix seconds, Unix milliseconds, or RFC3339 text. The service handoff uses integer-ish time comparisons conceptually. The adapter must document exact storage type and compare using the same `now` value passed into operations to avoid boundary inconsistencies.

### 12.3 KV caveat language

The docs should explicitly say: KV-backed rate limiting is suitable for small services and friction reduction, but not a strong global anti-abuse mechanism under distributed attack. Hosts needing stronger guarantees should combine D1 counters, Cloudflare WAF/rules, Turnstile-like friction, or upstream protection.

### 12.4 Corrected fallback identity

The adapter may fall back to the literal identity string `unknown` only as an explicit degraded mode. Docs must warn that many users sharing `unknown` can cause false positives or ineffective limits. Prefer configuration that declares trusted proxy headers.

### 12.5 Concrete acceptance checklist

- [ ] D1 code claim uses conditional update and checks affected rows.
- [ ] D1 form-token consume uses conditional update and checks affected rows.
- [ ] KV rate limit docs mention eventual consistency.
- [ ] Worker request identity extraction is configurable.
- [ ] Local test strategy does not require production Cloudflare credentials.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
