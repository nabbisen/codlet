# Threat Model

codlet is a one-time-code authentication library, not a general-purpose
identity platform. This document states what codlet protects against, what it
does not protect against, and the invariants that must hold for it to be
secure.

## What codlet protects against

**Online code guessing.** Short human-friendly codes are guessable in a small
number of attempts without controls. codlet defends with:
- mandatory rate limiting (`RateLimitStore`) checked before any lookup;
- failure counters incremented for invalid-format *and* not-found results,
  not only for lost concurrent claims — all guesses count toward the limit;
- codes long enough for the configured window (8+ symbols over 31-symbol
  alphabet = ~39.6 bits entropy by default);
- single-use enforcement via atomic conditional UPDATE.

**Double-claim under concurrency.** Two concurrent requests submitting the same
code will both attempt `claim_code`. The conditional UPDATE (`WHERE used_at IS
NULL AND expires_at > :now`) ensures exactly one winner. The loser receives
`ClaimOutcome::Lost` and must not proceed to session issuance. This is
verified by the conformance suite's concurrent race test.

**Session replay and forgery.** Session secrets are generated with 256 bits of
cryptographic randomness and stored only as HMAC lookup values. A lookup key
stolen from the database cannot be reversed to the bearer secret.

**Code enumeration.** All redemption failure states — not found, expired,
revoked, already used, format error — map to the same public error
(`PublicRedemptionError::InvalidOrExpired`). An attacker cannot distinguish
whether a code exists.

**Form-token replay and CSRF.** Form tokens are single-use. A duplicate submit
returns `Replay`, not a second execution. Token binding (subject, purpose,
bound resource) prevents token reuse across forms or users.

**Plaintext secret persistence.** Codes, session secrets, and form-token
secrets are never stored in plaintext. Only keyed HMAC lookup values are
persisted. The `no-plaintext-in-store-ops` release gate catches violations.

**Cookie leakage via JS.** Session cookies are `HttpOnly` by default. The
`cookie-attrs-present` gate ensures this cannot be accidentally removed.

**Key exhaustion / weak HMAC.** codlet uses HMAC-SHA-256 which provides
128-bit collision resistance. Key material must be ≥ 16 bytes; the
`StaticKeyProvider` rejects empty key bytes.

## What codlet does NOT protect against

**Authorization.** codlet authenticates (proves who a subject is); the host
application must authorize (decide what that subject may do). codlet never
checks membership, roles, or permissions.

**User management.** codlet stores no display name, email address, or
profile data. `SubjectId` is an opaque host-owned identifier.

**Offline code guessing after key+database compromise.** If both the HMAC key
and the database are leaked, an attacker can attempt to brute-force codes
offline. Mitigations: short TTLs (codes expire), high-entropy codes (8+
chars), and key rotation after compromise.

**Multi-process KV counter races.** KV-backed rate limiting (Workers KV) uses
eventual consistency. Under a high-concurrency distributed attack, counters may
be under-counted. Use D1/SQL-backed counters or Cloudflare WAF rules for
stronger guarantees.

**Network-level attacks.** codlet does not parse HTTP headers, implement TLS,
or make decisions about trusted proxies. The host application must provide
HTTPS and extract trustworthy rate-limit keys (e.g. from a verified client IP
or platform-provided header).

## Security invariants

These must hold for codlet to be secure:

| # | Invariant |
|---|-----------|
| INV-1 | Secrets are stored only as HMAC lookup values — never plaintext. |
| INV-2 | Missing key material fails the operation — no fallback key exists. |
| INV-3 | RNG failure fails the operation — no deterministic fallback value. |
| INV-4 | Normalization is identical on issue and redeem paths, and idempotent. |
| INV-5 | `claim_code` uses a conditional UPDATE; `changed == 0` never proceeds. |
| INV-6 | `consume_form_token` uses a conditional UPDATE; `changed == 0` never proceeds. |
| INV-7 | Session issuance requires a `RedeemSuccess` proof from a won claim. |
| INV-8 | All non-success redemption states map to one generic public error. |

The `xtask release-check` command enforces a subset of these statically on
every release.
