-- codlet SQLite migration 0001
-- Creates the three core tables. All tables use a text `lookup_key` column
-- (lowercase hex HMAC) as the cryptographic lookup index, and a `key_version`
-- column so records can be identified for rotation (RFC-004 §12.2).
--
-- No foreign keys to host-domain tables: codlet tables are intentionally
-- isolated. Host applications add their own foreign key constraints in
-- separate migrations if desired (RFC-011 §10.4).

CREATE TABLE IF NOT EXISTS codlet_codes (
    id              TEXT    NOT NULL PRIMARY KEY,
    lookup_key      TEXT    NOT NULL UNIQUE,
    key_version     TEXT    NOT NULL,
    purpose         TEXT,
    scope           TEXT,
    grant_payload   TEXT,
    created_at      INTEGER NOT NULL,   -- Unix seconds UTC
    expires_at      INTEGER NOT NULL,   -- Unix seconds UTC
    used_at         INTEGER,            -- NULL = not yet claimed
    used_by_subject TEXT,               -- NULL = not yet claimed
    revoked_at      INTEGER             -- NULL = not revoked
);

-- Index for the common lookup path: find by HMAC where still redeemable.
CREATE INDEX IF NOT EXISTS idx_codlet_codes_lookup
    ON codlet_codes (lookup_key, used_at, revoked_at, expires_at);

-- Optional scope-filtered lookup.
CREATE INDEX IF NOT EXISTS idx_codlet_codes_scope
    ON codlet_codes (scope, used_at, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_sessions (
    id          TEXT    NOT NULL PRIMARY KEY,
    lookup_key  TEXT    NOT NULL UNIQUE,
    key_version TEXT    NOT NULL,
    subject     TEXT    NOT NULL,       -- host-owned subject identifier
    created_at  INTEGER NOT NULL,       -- Unix seconds UTC
    expires_at  INTEGER NOT NULL,       -- Unix seconds UTC
    revoked_at  INTEGER                 -- NULL = active
);

CREATE INDEX IF NOT EXISTS idx_codlet_sessions_lookup
    ON codlet_sessions (lookup_key, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_form_tokens (
    lookup_key      TEXT    NOT NULL PRIMARY KEY,
    key_version     TEXT    NOT NULL,
    subject_kind    TEXT    NOT NULL,   -- "anon", "auth:<id>", "flow:<id>"
    purpose         TEXT    NOT NULL,
    bound_resource  TEXT,               -- NULL = no binding
    issued_at       INTEGER NOT NULL,   -- Unix seconds UTC
    expires_at      INTEGER NOT NULL,   -- Unix seconds UTC
    consumed_at     INTEGER,            -- NULL = not yet consumed
    result_ref      TEXT                -- optional idempotency result
);

CREATE INDEX IF NOT EXISTS idx_codlet_form_tokens_lookup
    ON codlet_form_tokens (lookup_key, consumed_at, expires_at);
