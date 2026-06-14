//! Table name configuration for D1 adapters (RFC-033 §10).
//!
//! Column names are fixed by the codlet migration schema. If the host
//! service uses different column names, a SQL view or ALTER TABLE migration
//! is the host's responsibility — codlet does not parameterise column names.

/// Table names used by the D1 adapters.
///
/// The default uses codlet's own table names (`codlet_codes`, etc.), which
/// match `migrations/0001_initial.sql`. Override to connect to an existing
/// service schema without a rename migration (e.g. zinnias-ciao uses
/// `invite_codes`, `sessions`, `form_tokens`).
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
    /// Table-name preset for zinnias-ciao v0.36.1 (`invite_codes`, `sessions`,
    /// `form_tokens`).
    ///
    /// **This is a table-name override only.** Column names are not remapped.
    /// codlet's SQL always uses its own column names (`lookup_key`,
    /// `key_version`, `grant_payload`, `scope`, `used_by_subject`, `subject`,
    /// `subject_kind`, etc.). Before using this preset, the zinnias-ciao tables
    /// must have all of those columns present — either by renaming the existing
    /// columns or by running codlet's migration against the existing tables after
    /// dropping or renaming conflicting columns.
    ///
    /// See `docs/src/migration-from-zinnias-ciao.md` for the full column rename
    /// SQL and migration sequence.
    pub fn zinnias_ciao_tables() -> Self {
        Self {
            codes: "invite_codes",
            sessions: "sessions",
            form_tokens: "form_tokens",
        }
    }
}
