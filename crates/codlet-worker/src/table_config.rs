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
    /// Config matching the zinnias-ciao v0.36.1 table names.
    ///
    /// Use this when migrating the service without immediately renaming
    /// tables. See `docs/src/migration-from-zinnias-ciao.md` for the full
    /// migration plan.
    ///
    /// Note: column names are still expected to match the codlet schema.
    /// Apply `ALTER TABLE … ADD COLUMN key_version …` before using this.
    pub fn zinnias_ciao_compat() -> Self {
        Self {
            codes: "invite_codes",
            sessions: "sessions",
            form_tokens: "form_tokens",
        }
    }
}
