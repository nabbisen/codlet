# Migrating from zinnias-ciao to codlet

This guide covers adopting codlet in a service that currently uses the
authentication primitives embedded in `zinnias-ciao` v0.36.1.

## What is different

| Concern | zinnias-ciao | codlet |
|---------|-------------|--------|
| HMAC derivation | `HMAC(pepper, value)` — no domain prefix | `HMAC(key, "codlet/v1/lookup\0domain\0value")` |
| Code length | 6 chars (29.7 bits) | ≥ 8 chars default; 6 via `legacy_ciao_6` |
| Key versioning | None | `key_version` column on every record |
| Normalization | Strip whitespace + hyphens, uppercase | Same (compatible) |
| Cookie name | `ciao_sid` | Configurable via `CookiePolicy` |

### HMAC incompatibility

codlet's domain-separated HMAC produces **different bytes** from the
service's `hmac_hex(pepper, value)`. Existing rows in `invite_codes`,
`sessions`, and `form_tokens` will not match codlet lookups.

**Migration options:**

**Option A — Hard cutover with re-issuance (recommended for codes)**  
Accept that existing invite codes, sessions, and form tokens are
invalidated. Issue new codes through codlet and require users to log in
again. Simplest operationally.

**Option B — Parallel lookup during grace period (recommended for sessions)**  
During the transition window, accept both the old HMAC (computed inline)
and the new codlet HMAC. Once all old sessions have expired (≤ 30 days),
remove the parallel path.

```rust
// Pseudocode: parallel lookup
let new_lk = hasher.lookup_key(SecretDomain::Session, cookie_value)?;
let session = session_store.find_active_session(&[new_lk], now).await?;
if session.is_none() {
    // Fall back to old-style lookup for the transition window.
    let old_hmac = legacy_hmac(pepper, cookie_value);
    session = legacy_session_db::find_active(&db, &old_hmac).await?;
}
```

## Schema migration

Add the `key_version` column to existing tables:

```sql
ALTER TABLE invite_codes   ADD COLUMN key_version TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE sessions       ADD COLUMN key_version TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE form_tokens    ADD COLUMN key_version TEXT NOT NULL DEFAULT 'legacy';
```

Create new codlet tables alongside (from `codlet-sqlx` migration `0001`):

```sql
-- codlet_codes, codlet_sessions, codlet_form_tokens
-- (see codlet-sqlx/migrations/0001_initial.sql)
```

Then migrate data or issue fresh records as appropriate.

## Column mapping

| zinnias-ciao | codlet |
|-------------|--------|
| `invite_codes.code_hmac` | `codlet_codes.lookup_key` |
| `invite_codes.grants_role` | `codlet_codes.grant_payload` |
| `invite_codes.community_id` | `codlet_codes.scope` |
| `sessions.session_hmac` | `codlet_sessions.lookup_key` |
| `sessions.user_id` | `codlet_sessions.subject` |
| `form_tokens.token_hmac` | `codlet_form_tokens.lookup_key` |
| `form_tokens.user_id` | `codlet_form_tokens.subject_kind` |

## Code length

Existing 6-character invite codes can continue to be validated using
`CodePolicy::legacy_ciao_6`. New codes should use `CodePolicy::default_human`
(8 characters minimum).

## Cookie name

Configure `CookiePolicy` with the existing cookie name:

```rust
let policy = CookiePolicy::production_strict(
    "ciao_sid",
    Duration::from_secs(30 * 86_400),
);
```

## Checklist

- [ ] Add `key_version` columns to existing tables.
- [ ] Choose hard cutover vs parallel lookup for each record type.
- [ ] Configure `StaticKeyProvider` with the existing pepper as a previous
      key and a new key as active.
- [ ] Set cookie name to `ciao_sid` in `CookiePolicy`.
- [ ] Use `CodePolicy::legacy_ciao_6` for 6-char code compatibility or
      re-issue 8-char codes to all pending invites.
- [ ] Remove parallel lookup path once old records have expired.
