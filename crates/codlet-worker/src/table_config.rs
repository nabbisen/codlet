//! Table name configuration for D1 adapters (RFC-033 §10).
//!
//! Column names are fixed by the codlet migration schema. If the host
//! service uses different column names, a SQL view or `ALTER TABLE RENAME COLUMN`
//! migration is the host's responsibility — codlet does not parameterise column names.

/// Table names used by the D1 adapters.
///
/// The default uses codlet's own table names (`codlet_codes`, etc.), which
/// match `migrations/0001_initial.sql`. Use a custom config to connect to
/// an existing service schema without immediately renaming tables.
#[derive(Debug, Clone)]
pub struct D1TableConfig {
    /// Table name for one-time codes. Default: `"codlet_codes"`.
    pub codes: &'static str,
    /// Table name for sessions. Default: `"codlet_sessions"`.
    pub sessions: &'static str,
    /// Table name for form tokens. Default: `"codlet_form_tokens"`.
    pub form_tokens: &'static str,
}

impl Default for D1TableConfig {
    fn default() -> Self {
        Self {
            codes: "codlet_codes",
            sessions: "codlet_sessions",
            form_tokens: "codlet_form_tokens",
        }
    }
}

impl D1TableConfig {
    /// Table-name preset for services that use `invite_codes`, `sessions`,
    /// and `form_tokens` as their existing table names.
    ///
    /// **This is a table-name override only.** Column names are not remapped.
    /// codlet's SQL always uses its own column names (`lookup_key`,
    /// `key_version`, `grant_payload`, `scope`, `used_by_subject`, `subject`,
    /// `subject_kind`, etc.). Before using this preset, the existing tables
    /// must have all of those columns present — either by renaming columns with
    /// `ALTER TABLE … RENAME COLUMN` or by running a fresh codlet migration.
    ///
    /// See `docs/src/migration-from-an-existing-service.md` for the full
    /// column rename SQL and migration sequence.
    pub fn with_existing_table_names() -> Self {
        Self {
            codes: "invite_codes",
            sessions: "sessions",
            form_tokens: "form_tokens",
        }
    }
}
