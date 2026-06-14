# Adapter Guarantee Matrix

This page states the atomicity and consistency guarantees of each codlet
storage adapter. Adapters that do not satisfy the atomic claim/consume
requirement are not production-safe for codlet's core use case.

## Guarantee definitions

| Property | Definition |
|----------|-----------|
| **Atomic claim** | Exactly one concurrent `claim_code` call observes `Won` for a given code. |
| **Atomic consume** | Exactly one concurrent `consume_form_token` call observes `Proceed` for a given token. |
| **Expiry filter** | `find_redeemable` and `find_active_session` exclude expired records server-side. |
| **Single-process safe** | Safe for all requests within one process. |
| **Multi-process safe** | Safe for multiple concurrent processes / instances. |

## Current adapters

| Adapter | Atomic claim | Atomic consume | Multi-process | Notes |
|---------|:-----------:|:--------------:|:-------------:|-------|
| `MemCodeStore` (test-utils) | ✓ (Mutex) | ✓ (Mutex) | ✗ | In-process only; not for production. |
| `SqliteStore` (codlet-sqlx) | ✓ (cond. UPDATE) | ✓ (cond. UPDATE) | ✓ (WAL mode) | SQLite serialises writes; WAL recommended for concurrent reads. Also implements `CodeAdminStore`. |
| `PostgresStore` (codlet-sqlx, `--features postgres`) | ✓ (cond. UPDATE, READ COMMITTED row-lock) | ✓ (cond. UPDATE) | ✓ | Multi-instance production. No `RETURNING`, no `FOR UPDATE`. Also implements `CodeAdminStore`. |
| `D1CodeStore` / `D1SessionStore` / `D1FormTokenStore` (codlet-worker) | ✓ (cond. UPDATE + `meta().changes`) | ✓ (cond. UPDATE + `meta().changes`) | ✓ (D1 global) | wasm32 target only; timestamps as REAL (f64). Also implements `CodeAdminStore`. |

## Rate-limit adapters

| Adapter | Consistency | Notes |
|---------|------------|-------|
| `MemRateLimitStore` (test-utils) | Exact (Mutex) | In-process only. |
| `KvRateLimitStore` (codlet-worker) | Eventual | ⚠ Eventually consistent — counters may under-count under distributed attack. See RFC-010 §12.3. |

---

# Secure Configuration Guide

## Code policy

`CodePolicy::default_human` is the recommended constructor. It produces 8-symbol
codes over the 31-symbol unambiguous alphabet (~39.6 bits of entropy) and does
not require rate limiting to be safe — the entropy is sufficient.

```rust
// Recommended: 8 symbols, ~39.6 bits, no rate-limit dependency.
let policy = CodePolicy::default_human(Duration::from_secs(24 * 3600))?;
```

For services migrating from an existing 6-symbol code system, `six_symbol` is
available as an explicit opt-in. It is marked `#[deprecated]` to emit a compiler
warning at every call site — the warning is the intended friction. Suppress it
with `#[allow(deprecated)]` only after confirming that rate limiting is in place.
6-symbol codes have only ~29.7 bits of entropy and **require** a `RateLimitStore`
to be safe against online guessing.

```rust
// Short-code compatibility — requires active rate limiting.
// Compiler will warn; suppress only after confirming rate limiting is wired.
#[allow(deprecated)]
let compat = CodePolicy::six_symbol(Duration::from_secs(4 * 3600))?;
// → 6-char, 31-symbol alphabet, ~29.7 bits entropy, 4 h TTL.
```

For lengths between 6 and 7, use `CodePolicy::short_compat` (also deprecated)
with an explicit length. The same rate-limiting requirement applies.

## Key provider

```rust
// Load from environment — never hard-code in source.
let key_bytes = std::env::var("CODLET_HMAC_KEY")
    .expect("CODLET_HMAC_KEY must be set")
    .into_bytes();
let provider = StaticKeyProvider::single("v1", key_bytes)
    .expect("non-empty key required");
```

Key requirements:
- at least 32 bytes (256 bits) of random material;
- generated with a CSPRNG (e.g. `openssl rand -hex 32`);
- stored in a secret manager or environment variable, never in source control.

## Cookie policy

```rust
// Production default — HttpOnly, Secure, SameSite=Strict.
let cookie = CookiePolicy::production_strict("app_sid", Duration::from_secs(30 * 86_400));

// Cross-site top-level nav (e.g. OAuth-like redirect after external flow).
let lax = CookiePolicy::production_lax("app_sid", Duration::from_secs(30 * 86_400));

// Local development only — Secure=false.
let dev = CookiePolicy::local_development("app_sid", Duration::from_secs(86_400));
```

Never use `local_development` in production.

## Rate limiting

Always check the rate limit **before** the code lookup:

```rust
let key = RateLimitKey::new(trusted_ip); // from a verified header
let ca = CodeAuth::new(store, rl_store, hasher, clock, audit, policy, rl_policy);
// CodeAuth::find() checks the rate limit automatically before any DB query.
let found = ca.find(&raw_input, Some(&key)).await?;
```

On success, clear the counter; on failure, record a failure. `CodeAuth` handles
this automatically when a `RateLimitKey` is provided.

## User-facing copy (RFC-016 §10.3)

Guidance for host applications — codlet does not own UI text:

| Situation | ✓ Say | ✗ Don't say |
|-----------|-------|-------------|
| Code wrong / expired / used | "Please check the code and try again." | "This code has already been used." |
| Form expired | "Please reload the page and try again." | "CSRF validation failed." |
| Session expired | "Please sign in again." | "Session expired." |
| Rate limited | "Too many attempts. Please wait." | "IP blocked." |
| Token for user | "code" | "token", "credential", "HMAC" |
