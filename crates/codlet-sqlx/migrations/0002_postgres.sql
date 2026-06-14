-- codlet PostgreSQL migration 0001
-- Apply this instead of 0001_initial.sql for PostgreSQL deployments.
--
-- Differences from the SQLite migration (0001_initial.sql):
--   - BIGINT instead of INTEGER for timestamps (same i64 Rust binding,
--     no 2038 issue, explicit 8-byte storage).
--   - No PRAGMA statements (SQLite-only).
--   - IF NOT EXISTS on all CREATE TABLE and CREATE INDEX (PostgreSQL 9.1+).
--
-- RFC-034 §6: BIGINT stores Unix seconds as i64. f64 is NOT used here
-- (unlike the D1/wasm32 adapter in RFC-033); SQLx maps i64 to BIGINT natively.

CREATE TABLE IF NOT EXISTS codlet_codes (
    id              TEXT    NOT NULL PRIMARY KEY,
    lookup_key      TEXT    NOT NULL UNIQUE,
    key_version     TEXT    NOT NULL,
    purpose         TEXT,
    scope           TEXT,
    grant_payload   TEXT,
    created_at      BIGINT  NOT NULL,   -- Unix seconds UTC
    expires_at      BIGINT  NOT NULL,   -- Unix seconds UTC
    used_at         BIGINT,             -- NULL = not yet claimed
    used_by_subject TEXT,               -- NULL = not yet claimed
    revoked_at      BIGINT              -- NULL = not revoked
);

CREATE INDEX IF NOT EXISTS idx_codlet_codes_lookup
    ON codlet_codes (lookup_key, used_at, revoked_at, expires_at);

CREATE INDEX IF NOT EXISTS idx_codlet_codes_scope
    ON codlet_codes (scope, used_at, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_sessions (
    id          TEXT    NOT NULL PRIMARY KEY,
    lookup_key  TEXT    NOT NULL UNIQUE,
    key_version TEXT    NOT NULL,
    subject     TEXT    NOT NULL,
    created_at  BIGINT  NOT NULL,
    expires_at  BIGINT  NOT NULL,
    revoked_at  BIGINT
);

CREATE INDEX IF NOT EXISTS idx_codlet_sessions_lookup
    ON codlet_sessions (lookup_key, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_form_tokens (
    lookup_key      TEXT    NOT NULL PRIMARY KEY,
    key_version     TEXT    NOT NULL,
    subject_kind    TEXT    NOT NULL,
    purpose         TEXT    NOT NULL,
    bound_resource  TEXT,
    issued_at       BIGINT  NOT NULL,
    expires_at      BIGINT  NOT NULL,
    consumed_at     BIGINT,
    result_ref      TEXT
);

CREATE INDEX IF NOT EXISTS idx_codlet_form_tokens_lookup
    ON codlet_form_tokens (lookup_key, consumed_at, expires_at);
