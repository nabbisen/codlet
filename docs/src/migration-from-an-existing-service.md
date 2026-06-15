# Migrating from an existing service to codlet

This guide covers adopting codlet in a service that already has its own
authentication primitives — invite codes, sessions, and form tokens — and needs
to migrate them to codlet.

## What is different

| Concern | Typical existing service | codlet |
|---------|-------------|--------|
| HMAC derivation | `HMAC(pepper, value)` — typically no domain prefix | `HMAC(key, "codlet/v1/lookup\0domain\0value")` |
| Code length | Often 6 chars (29.7 bits) | ≥ 8 chars default; 6 via `six_symbol` |
| Key versioning | None | `key_version` column on every record |
| Normalization | Strip whitespace + hyphens, uppercase | Same (compatible) |
| Cookie name | Host-defined (e.g. `my_sid`) | Configurable via `CookiePolicy` |

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

### Option A — Use codlet tables alongside existing tables (recommended)

Run `codlet-sqlx`'s migration to create fresh codlet tables. Leave the existing
service tables untouched during the transition. Use `D1TableConfig::default()`
(which targets `codlet_codes`, `codlet_sessions`, `codlet_form_tokens`).

```sql
-- Run codlet-sqlx/migrations/0001_initial.sql (or the D1 equivalent).
-- New codlet_codes, codlet_sessions, codlet_form_tokens tables are created.
-- Existing invite_codes, sessions, form_tokens tables are not touched.
```

All new codes, sessions, and tokens are issued into the codlet tables. Run the
parallel lookup path for sessions during the grace period (see HMAC section).
Drop the old tables once they are empty.

### Option B — Rename existing columns in-place (in-place migration)

D1 (SQLite) supports `ALTER TABLE … RENAME COLUMN` since SQLite 3.25. Use this
to rename the existing columns to match codlet's schema, then use
`D1TableConfig::with_existing_table_names()` to keep the existing table names.

**`invite_codes` → codlet column names:**

```sql
ALTER TABLE invite_codes RENAME COLUMN code_hmac            TO lookup_key;
ALTER TABLE invite_codes RENAME COLUMN grants_role          TO grant_payload;
ALTER TABLE invite_codes RENAME COLUMN community_id         TO scope;
ALTER TABLE invite_codes RENAME COLUMN used_by_membership_id TO used_by_subject;
ALTER TABLE invite_codes ADD COLUMN key_version TEXT NOT NULL DEFAULT 'legacy';
-- Verify remaining columns: id, created_at, expires_at, used_at, revoked_at
-- Add purpose, grant_payload (if absent) as needed.
```

**`sessions` → codlet column names:**

```sql
ALTER TABLE sessions RENAME COLUMN session_hmac TO lookup_key;
ALTER TABLE sessions RENAME COLUMN user_id      TO subject;
ALTER TABLE sessions ADD COLUMN key_version TEXT NOT NULL DEFAULT 'legacy';
-- Verify: id, expires_at, revoked_at
```

> **`created_at` type conflict.** codlet stores `created_at` as `INTEGER`
> (Unix seconds). If your existing `sessions` table already has a `created_at`
> column (e.g. as `TEXT` in ISO-8601 format), the `ADD COLUMN` above is
> omitted — adding it would fail with `duplicate column name`. codlet reads
> `created_at` only for the `CodeAdminStore` listing API, not for session
> validation, so the conflict does not break runtime auth. However, admin
> listings will show incorrect timestamps for migrated rows.
>
> To resolve correctly: drop the old column and add it as `INTEGER`, or accept
> stale `created_at` values for pre-migration sessions. This type conflict is
> another reason **Option A (fresh codlet tables) is strongly preferred** for
> services with an existing `sessions.created_at` column.

**`form_tokens` → codlet column names:**

```sql
ALTER TABLE form_tokens RENAME COLUMN token_hmac TO lookup_key;
ALTER TABLE form_tokens RENAME COLUMN user_id    TO subject_kind;
ALTER TABLE form_tokens ADD COLUMN key_version  TEXT    NOT NULL DEFAULT 'legacy';
ALTER TABLE form_tokens ADD COLUMN purpose      TEXT    NOT NULL DEFAULT 'legacy';
ALTER TABLE form_tokens ADD COLUMN bound_resource TEXT;
ALTER TABLE form_tokens ADD COLUMN issued_at    INTEGER NOT NULL DEFAULT 0;
-- Verify: expires_at, consumed_at, result_ref
```

After these renames, `D1TableConfig::with_existing_table_names()` can be used. The
existing data will be readable by codlet, but HMAC lookups will still fail for
old rows because the HMAC format has changed (see above). Use the parallel
lookup path for sessions during the grace period.

> **Important:** `D1TableConfig::with_existing_table_names()` remaps **table names
> only**. It does not remap column names. The column renames above are
> prerequisites for in-place migration.

## Column mapping

| Existing service | codlet |
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
`CodePolicy::six_symbol`. New codes should use `CodePolicy::default_human`
(8 characters minimum).

## Cookie name

Configure `CookiePolicy` with the existing cookie name:

```rust
// Use whatever cookie name your service currently sets.
let policy = CookiePolicy::production_strict(
    "my_sid",  // replace with your existing cookie name
    Duration::from_secs(30 * 86_400),
);
```

## Checklist

- [ ] Add `key_version` columns to existing tables.
- [ ] Choose hard cutover vs parallel lookup for each record type.
- [ ] Configure `StaticKeyProvider` with the existing pepper as a previous
      key and a new key as active.
- [ ] Set cookie name to your existing session cookie name in `CookiePolicy`.
- [ ] Use `CodePolicy::six_symbol` for 6-char code compatibility or
      re-issue 8-char codes to all pending invites.
- [ ] Remove parallel lookup path once old records have expired.
