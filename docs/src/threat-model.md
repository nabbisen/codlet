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

**Cookie leakage via JS.** Session cookies are `HttpOnly` by default. This is
ensured by the behavioural tests in `crates/codlet/src/cookie/tests.rs`, which
assert on the emitted `Set-Cookie` string across the production, lax, and
development profiles — not by a static gate. A prior release-time text-scan
gate (`cookie-attrs-present`) claimed to guard this too, but matched whole
file text including documentation and enum literals, so it could not detect a
builder that stopped emitting an attribute; it was retired (RFC-042) once
`xtask self-test` (RFC-040) proved it against a realistic fixture.

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

Per RFC-040: each invariant names its guard **and** the negative test proving
that guard can fail. A guard with no recorded negative test is not treated as
verified — see RFC-036 §3.4, generalised in RFC-040 §3.1 to "a gate must fail
when it cannot perform its check". No row in this table is open.

| # | Invariant | Guard | Negative test |
|---|-----------|-------|----------------|
| INV-1 | Secrets are stored only as HMAC lookup values — never plaintext. | `xtask` gate `no-plaintext-in-store-ops` | `xtask self-test` fixture `xtask/fixtures/no_plaintext_in_store_ops.rs` (RFC-040 §3.2) |
| INV-2 | Missing key material fails the operation — no fallback key exists. | `xtask` gate `no-fallback-key` | `xtask self-test` fixture `xtask/fixtures/no_fallback_key.rs` |
| INV-3 | RNG failure fails the operation — no deterministic fallback value. | `xtask` gate `rng-no-silent-fallback` | `xtask self-test` fixture `xtask/fixtures/rng_no_silent_fallback.rs` |
| INV-4 | Normalization is identical on issue and redeem paths, and idempotent. | `crates/codlet/src/code/normalize/tests.rs` properties `p1_idempotent_for_arbitrary_str`, `p4_never_panics_on_arbitrary_unicode`; `crates/codlet/src/code/alphabet/tests.rs` properties `p3_every_accepted_symbol_is_a_normalization_fixed_point`, `p5_exact_uniformity_for_default_alphabet`, `p7_ceiling_is_the_largest_multiple_of_len_up_to_256`; `crates/codlet/src/code/generate/tests.rs` properties `p2_generated_code_is_a_normalization_fixed_point_under_safe_policies`, `p6_every_byte_at_or_above_ceiling_is_rejected_never_mapped` (RFC-041); `Alphabet::new` itself rejects any symbol that is not a normalization fixed-point, naming the offending byte via `PolicyError::AlphabetNotFixedPoint` (RFC-043) | Each property's own `#[test]`/`proptest!` block breaking against a deliberately reverted production-code trial (RFC-041 §3.3; see the RFC-041 review request for per-property breakage output). `p3_every_accepted_symbol_is_a_normalization_fixed_point` fired against real `Alphabet::new` before RFC-043 (confirming the gap) and now passes unconditionally, since construction enforces the property structurally rather than by convention. `alphabet/tests.rs`'s `rejects_lowercase_symbol`, `rejects_hyphen_symbol`, and `rejects_ascii_whitespace_symbol` cover one rejected class each; `unambiguous_still_constructs` guards against the check being too strict. |
| INV-5 | `claim_code` uses a conditional UPDATE; `changed == 0` never proceeds. | `codlet-conformance` concurrent-claim test, run against every adapter (in-memory, SQLite, PostgreSQL, D1) | `crates/codlet/tests/rfc_040_invariant_verification.rs`: `inv5_claim_with_changed_zero_reports_lost_not_won`, `inv5_second_claim_of_an_already_won_code_also_reports_lost`, `inv5_changed_greater_than_one_surfaces_as_invariant_violation_not_lost` |
| INV-6 | `consume_form_token` uses a conditional UPDATE; `changed == 0` never proceeds. | `codlet-conformance` form-token consume test, run against every adapter | `crates/codlet/tests/rfc_040_invariant_verification.rs`: `inv6_consume_with_changed_zero_reports_invalid_not_proceed`, `inv6_second_consume_of_an_already_consumed_token_replays_not_proceeds`, `inv6_changed_greater_than_one_surfaces_as_invariant_violation_not_replay` |
| INV-7 | Session issuance requires a `RedeemSuccess` proof from a won claim. | Type system: `RedeemSuccess::_claim_proof` is `pub(crate)`, constructible only via a won `claim_code` | `crates/codlet/tests/rfc_040_inv7_compile_fail.rs` — `trybuild` `compile_fail`; the only invariant proven by a compile-failure test rather than a runtime assertion |
| INV-8 | All non-success redemption states map to one generic public error. | `PublicRedemptionError::from_reason` | `crates/codlet/tests/rfc_040_invariant_verification.rs`: `inv8_every_redemption_fail_reason_is_classified_by_an_exhaustive_match` — an exhaustive `match` with no wildcard arm, so an unhandled new `RedemptionFailReason` variant fails to compile rather than silently escaping the check |

`cargo run -p xtask -- release-check` runs four static gates, three of which
enforce INV-1, INV-2, and INV-3 above; the fourth, `no-debug-prints`, guards
against secret-bearing debug output and is not tied to a single numbered
invariant in this table. `xtask self-test` (also run in CI) proves each of the
four can fail against a deliberate violation, not merely that it exists
(RFC-040 §3.2). A fifth gate, `cookie-attrs-present`, guarded the
`HttpOnly`/`Secure`/`SameSite` cookie invariant described under "Cookie
leakage via JS" above — also outside this numbered list — and was retired
under this same standard once self-test proved it could not actually detect
a violation; see RFC-042.
