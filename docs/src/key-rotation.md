# Key Management and Rotation

codlet stores every code, session secret, and form-token secret as a
**keyed HMAC lookup value** — never in plaintext. The key material
(the HMAC pepper) is the most sensitive operational secret in a codlet
deployment. This guide covers how to manage it safely.

## Key versioning

Every record stores a `key_version` string alongside its `lookup_key`.
This makes rotation possible without an all-or-nothing cutover: old records
keep working while new ones use the new key.

```
codlet_codes:    lookup_key, key_version
codlet_sessions: lookup_key, key_version
codlet_form_tokens: lookup_key, key_version
```

## Configuring keys

Supply an active key and any previous keys through `StaticKeyProvider`:

```rust
use codlet_core::hashing::{StaticKeyProvider, KeyVersion};

let provider = StaticKeyProvider::new(
    "v2",                            // active version label
    load_secret_bytes("CODLET_KEY_V2"),  // from env / secret manager
    vec![
        (KeyVersion::new("v1"), load_secret_bytes("CODLET_KEY_V1")),
    ],
).expect("key material required");
```

- **No fallback key.** `StaticKeyProvider::new` fails if the active key
  bytes are empty. There is no built-in default; a missing key is a
  configuration error, not a degraded-mode.
- **Missing previous key version** → `KeyError::MissingKeyVersion`. The
  record that required it cannot be validated until the key is added back
  (or the record expires).

During validation, codlet derives one HMAC lookup candidate per held key
(active + all previous) via `SecretHasher::lookup_key_candidates()` and
passes the full slice to the store. A session or code issued under a previous
key remains reachable as long as that key is listed in `previous`.

## Planned rotation procedure

1. **Generate** a new key outside application logs (e.g. `openssl rand -hex 32`).
2. **Deploy** the new key as active and the old key as `previous`:

   ```
   CODLET_KEY_VERSION=v2
   CODLET_KEY_V2=<new-secret>
   CODLET_KEY_V1=<old-secret>   # still accepted for old records
   ```

3. **Verify** that new records are written with the new `key_version`.
4. **Wait** at least as long as the maximum TTL of records written with the
   old key (e.g. 30 days for sessions, 1 hour for form tokens).
   Alternatively, force-expire old records by revoking sessions issued
   under `v1` via admin tooling.
5. **Remove** the old key from `previous`. Verify no active records still
   reference the old version (a SELECT on `key_version = 'v1'` across all
   tables should return zero rows).

## Emergency compromise procedure

If both the database *and* the HMAC key material are suspected compromised:

1. **Revoke sessions** for all users or the affected cohort (set
   `revoked_at` on all session rows, or drop them).
2. **Revoke outstanding codes** and form tokens for the affected scope.
3. **Rotate the key immediately** — deploy a fresh key as active with no
   previous.
4. **Inspect audit events** for `code.redeem.succeeded` and
   `session.issue.succeeded` events around the suspected window.
5. Communicate recovery steps to affected users through a channel that is
   not itself dependent on the compromised credentials.

If only the database is compromised (key material is safe), HMAC lookup
values are computationally hard to reverse; regenerating keys is still
recommended out of caution but is less urgent.

## What codlet does not do

codlet does not store, rotate, or generate key material. That is the host
application's responsibility. codlet accepts key material through
`KeyProvider` and fails closed if it is absent.
