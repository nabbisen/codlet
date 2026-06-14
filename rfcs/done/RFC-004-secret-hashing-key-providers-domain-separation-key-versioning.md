# RFC-004: Secret Hashing, Key Providers, Domain Separation, and Key Versioning

- **Status:** Implemented (v0.1.0)
- **Target milestone:** M1
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define HMAC lookup-key derivation, key provider behavior, key versions, and domain separation for all bearer secrets.

## 2. Motivation

The source service stores codes, sessions, and form tokens as HMAC values, which is the right core pattern. Its development pepper fallback and missing key-version columns must not be carried into codlet.

## 3. Decision

codlet will require explicit key material, derive lookup keys with domain separation, and store key versions with all lookup keys.

## 4. Detailed design


Public concepts:

```rust
pub enum SecretDomain { Code, Session, FormToken, JoinTicket }
pub struct KeyVersion(String);
pub struct LookupKey(String);

pub trait KeyProvider {
    fn active_hmac_key(&self) -> Result<HmacKeyRef<'_>, KeyError>;
    fn hmac_key_by_version(&self, version: &KeyVersion) -> Result<HmacKeyRef<'_>, KeyError>;
}
```

Lookup-key derivation:

```text
HMAC-SHA256(key, "codlet/v1/" || domain_label || 0x00 || secret_bytes)
```

The exact bytes and test vectors must be specified before implementation.

All persistent records include:

- lookup key;
- key version;
- creation timestamp.

No fallback key:

- adapter constructors validate required secrets;
- test providers are explicit and under test modules/features.


## 5. Security considerations

A database leak without key material should not reveal bearer secrets. Key versioning avoids a future all-or-nothing migration. Domain separation prevents cross-use of lookup keys across code/session/form-token namespaces.

## 6. Host application responsibilities

The host must store HMAC keys in a secret manager or platform secret store, rotate them according to operational policy, and protect logs from key exposure.

## 7. Tests and release gates


- Known HMAC test vector for every domain.
- Same key/input/domain gives same lookup key.
- Different domain gives different lookup key.
- Different key gives different lookup key.
- Missing key fails closed.
- Secret newtypes redact debug output.
- Key-version round trip test.


## 8. Migration notes

zinnias-ciao must add key-version columns or use adapter compatibility defaults before key rotation is supported. 

## 9. Open questions

Whether to use HMAC message prefixing or HKDF-derived subkeys for domain separation. Prefixing is simpler; subkeys are more explicit.

### 9.1 Implementer recommendation (2026-06, bootstrap review)

Both approaches give correct cross-domain separation. Weighing them:

**Prefixing** — `HMAC-SHA256(key, "codlet/v1/" || domain_label || 0x00 || secret)`:

- one HMAC operation per lookup; no extra key-derivation step;
- trivially portable across adapters (D1/Workers WASM, SQLx) — every backend
  computes the identical byte sequence, which keeps test vectors simple;
- the `0x00` separator after a fixed-width/variable-but-unambiguous label
  prevents canonicalization ambiguity between label and secret;
- one dependency (`hmac`); no HKDF crate, keeping the core footprint minimal
  (NFR-3).

**HKDF subkeys** — derive `k_code`, `k_session`, `k_formtoken` from the master
key, then `HMAC(k_domain, secret)`:

- cleaner key-hygiene story (each domain has a distinct key);
- but adds a derivation step and a dependency, and the separation benefit over
  prefixing is marginal because HMAC is already a PRF keyed by the master key —
  distinct prefixes already yield computationally independent outputs.

**Recommendation: adopt prefixing** for v0.1. It satisfies the security goal
(cross-domain lookup keys differ), is the smallest dependency surface, and
makes the published test vectors (RFC-004 §12.3) backend-independent. The exact
byte layout to be frozen with vectors at implementation time is:

```text
message = "codlet/v1/lookup" || 0x00 || domain_label || 0x00 || secret_bytes
domain_label ∈ { "code", "session", "form_token", "flow_ticket" }
```

Note this is intentionally distinct from `zinnias-ciao`'s `HMAC(pepper, value)`
(no domain, no prefix), so codlet lookup keys are **not** bit-identical to the
service's existing rows. The migration adapter (RFC-014) must therefore offer a
`legacy_no_domain` derivation mode for existing `invite_codes`/`sessions`/
`form_tokens` data, rather than assuming byte-identical HMACs.

This recommendation does not by itself accept the RFC; RFC-004 remains
`proposed/` until the primitive is implemented and vectors are published.


## 12. Expanded technical design

### 12.1 Domain-separated lookup format

Every lookup key is derived from the same conceptual tuple:

```text
algorithm_id = "hmac-sha256"
context      = "codlet/v1/lookup"
domain       = "code" | "session" | "form_token" | "flow_ticket"
secret       = canonical secret bytes
```

The domain label is part of the MAC input, not just metadata. This means the same plaintext used as a form token and as a code will produce different lookup keys.

### 12.2 Key version storage

Key versions must be stored with records because future validation cannot infer the original key from the lookup key alone. Record lookup may follow one of two designs:

1. **Lookup by active/previous derived keys:** derive candidate lookup keys for active and previous keys, then query by lookup key. This avoids exposing key version before lookup but requires multiple derivations.
2. **Lookup by derived key plus version:** include version in query when the host knows it. This is less useful for bearer inputs because the presented secret does not include version.

For code/session/form-token validation, design (1) is usually required. Once a record is found, its stored key version must match the key used to derive the candidate, or validation fails closed.

### 12.3 Test vectors

Before v1, publish test vectors:

```text
key_version: test-v1
key_hex: ... fixed test bytes ...
domain: code
secret: ABCD2345
lookup_hex: ...
```

Vectors are not production secrets; they guarantee cross-adapter compatibility.

### 12.4 Key provider failure classification

| Failure | Operation behavior | Public mapping |
|---|---|---|
| no active key on issue | fail config/operation | service unavailable/generic |
| missing previous key on validation | fail closed for that candidate | generic auth failure |
| malformed key bytes | startup/config error | not user-visible detail |
| duplicate active versions | startup/config error | not user-visible detail |

### 12.5 Concrete acceptance checklist

- [x] All lookup functions accept a domain label/type.
- [x] All stores include `key_version` fields.
- [x] At least one test vector per domain exists.
- [x] No dev fallback key exists in normal constructors.
- [x] Redaction tests cover debug, display, serde, and audit paths.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
