# RFC-003: One-Time Code Policy, Generation, Normalization, and Validation

- **Status:** Implemented (v0.1.0)
- **Target milestone:** M1
- **Primary crate(s):** codlet-core
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Specify human-friendly code policy, secure generation, normalization, and validation.

## 2. Motivation

The service handoff shows two important fixes already made: RNG failure must not fall back to deterministic bytes, and alphabet mapping must avoid modulo bias. These should become library-level guarantees.

## 3. Decision

codlet-core will expose `CodePolicy`, `CodeGenerator`, `normalize_code`, and validation APIs. Defaults use an unambiguous alphabet and at least 8 characters.

## 4. Detailed design


Default alphabet:

```text
23456789ABCDEFGHJKMNPQRSTUVWXYZ
```

This 31-character alphabet excludes commonly confused glyphs.

Generation algorithm:

1. compute `ceiling = 256 - (256 % alphabet_len)`;
2. read random bytes;
3. accept only bytes `< ceiling`;
4. map accepted byte as `alphabet[byte % alphabet_len]`;
5. continue until desired length;
6. return error on RNG failure.

Normalization:

- strip ASCII spaces and hyphens;
- uppercase ASCII letters;
- reject unsupported characters during validation;
- normalization is idempotent.

Validation:

```rust
pub fn validate_code_input(raw: &str, policy: &CodePolicy) -> Result<NormalizedCode, CodeInputError>;
```

The API returns a normalized value so callers cannot accidentally hash the unnormalized input.


## 5. Security considerations

Six-character codes are acceptable only with short expiry, single-use semantics, and rate limiting in small deployments. codlet default must be stronger. Generation must not use `unwrap_or_default`, modulo-only mapping, or predictable RNG.

## 6. Host application responsibilities

The host chooses code length and TTL according to its risk model. If it selects a short code, it must configure rate limiting and safe public errors.

## 7. Tests and release gates


- RNG failure returns error and produces no code.
- Rejection sampling ceiling is correct for the default alphabet.
- All accepted bytes map to valid alphabet characters.
- Ambiguous characters are absent from default alphabet.
- Normalization strips separators and uppercases.
- Normalization is idempotent.
- Invalid, empty, too-short, too-long, and special-character inputs are rejected.
- Distribution smoke test for generated characters.


## 8. Migration notes

zinnias-ciao may configure `length = 6` for compatibility, but the code should make this explicit and documented. 

## 9. Open questions

None at this stage. 


## 11. Expanded technical design

### 11.1 Policy object invariants

`CodePolicy` is not just configuration; it is a validated security object. Construction must reject impossible or risky shapes unless the host explicitly opts in.

| Invariant | Required behavior |
|---|---|
| alphabet length >= 2 | reject at construction |
| code length >= configured minimum | reject or require explicit `ShortCodePolicy` marker |
| alphabet has unique characters | reject duplicate alphabet entries |
| maximum input length >= normalized length | reject invalid config |
| separators not in alphabet | reject ambiguous config |
| TTL positive | reject zero or negative duration |

### 11.2 Code handling phases

codlet must clearly separate four representations:

| Phase | Type idea | Secret? | Persistable? |
|---|---|---:|---:|
| Generated plaintext | `IssuedCode` | Yes | No |
| User raw input | `RawCodeInput` | Yes | No |
| Canonical normalized | `NormalizedCode` | Yes | No |
| HMAC lookup | `LookupKey` | No plaintext, still sensitive | Yes |

Only `LookupKey` crosses the persistence boundary. Debug output for the first three should be redacted.

### 11.3 Entropy documentation

The docs should provide a small table so hosts understand policy tradeoffs:

| Alphabet | Length | Approx bits | Intended use |
|---:|---:|---:|---|
| 31 | 6 | 29.7 | Compatibility / small invite-only groups with strong throttling. |
| 31 | 8 | 39.6 | Default human-friendly invite/login codes. |
| 31 | 10 | 49.5 | Higher-risk public endpoints. |
| 32 | 10 | 50.0 | Power-of-two alphabet if ambiguity policy allows. |

The table must always be paired with a statement that online rate limiting is mandatory for short codes.

### 11.4 Input normalization edge cases

- ASCII hyphen and spaces are accepted separators.
- Tabs/newlines should be rejected or stripped consistently; the safer v1 default is reject after raw input trimming except ordinary spaces.
- Unicode whitespace should not silently normalize in v1 unless a separate internationalization policy is adopted.
- Lowercase ASCII is accepted and uppercased.
- Full-width letters/numbers should be rejected in v1 unless RFC revises canonicalization.

### 11.5 Concrete acceptance checklist

- [x] RNG failure test uses a fake RNG that always errors.
- [x] Rejection-sampling test covers the 31-character alphabet ceiling of 248.
- [x] `Debug`/serialization never prints plaintext code unless an explicit redaction wrapper is intentionally opened in tests.
- [x] Normalization property tests run on arbitrary Unicode input.
- [x] Public docs show grouped display as optional and normalization as canonical.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
