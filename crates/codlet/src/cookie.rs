//! Secure cookie construction (RFC-006 §13.2).
//!
//! [`CookiePolicy`] encodes named profiles. `HttpOnly` and `Secure` are
//! mandatory in all production profiles. `SameSite=Strict` is the default.
//!
//! The resulting header values are plain strings so they can be passed to any
//! HTTP framework without coupling to a specific `Cookie` crate.

use std::time::Duration;

/// SameSite cookie attribute values (RFC-6265bis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameSitePolicy {
    /// `SameSite=Strict` — cookies sent only in same-site requests.
    /// Default and recommended for session cookies (RFC-006 §4).
    #[default]
    Strict,
    /// `SameSite=Lax` — cookies sent on top-level cross-site navigation.
    /// Appropriate when the host needs to receive a cookie after a redirect
    /// from an external flow (RFC-006 §13.2 `ProductionLax` profile).
    Lax,
    /// `SameSite=None` — must always be accompanied by `Secure`. Not emitted
    /// by any built-in profile; available for framework adapters.
    None,
}

impl SameSitePolicy {
    /// The attribute string fragment, without the leading `; `.
    #[must_use]
    pub const fn attr(self) -> &'static str {
        match self {
            SameSitePolicy::Strict => "SameSite=Strict",
            SameSitePolicy::Lax => "SameSite=Lax",
            SameSitePolicy::None => "SameSite=None",
        }
    }
}

/// Named cookie profile (RFC-006 §13.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CookieProfile {
    /// `Secure=true; HttpOnly=true; SameSite=Strict`. Default.
    #[default]
    ProductionStrict,
    /// `Secure=true; HttpOnly=true; SameSite=Lax`.
    ProductionLax,
    /// `Secure=false; HttpOnly=true; SameSite=Lax`. Must be explicitly chosen;
    /// not for production. `Secure=false` is rejected if the active profile is
    /// a production one.
    LocalDevelopment,
}

/// Policy governing cookie construction (RFC-006 §4).
///
/// Build with [`CookiePolicy::production_strict`] for the standard profile,
/// or use the builder methods to customise. `HttpOnly=true` cannot be disabled
/// (RFC-006 §13.2: "A production profile should reject `Secure=false`").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookiePolicy {
    name: String,
    path: String,
    max_age: Duration,
    same_site: SameSitePolicy,
    secure: bool,
    domain: Option<String>,
}

impl CookiePolicy {
    /// Standard production policy: `HttpOnly; Secure; SameSite=Strict; Path=/`.
    /// `Domain` is omitted to produce a host-only cookie (RFC-006 §5,
    /// implementation note: omitting `Domain` avoids subdomain leakage).
    #[must_use]
    pub fn production_strict(name: impl Into<String>, max_age: Duration) -> Self {
        Self {
            name: name.into(),
            path: "/".to_string(),
            max_age,
            same_site: SameSitePolicy::Strict,
            secure: true,
            domain: None,
        }
    }

    /// Production policy with `SameSite=Lax` for cross-site top-level flows.
    #[must_use]
    pub fn production_lax(name: impl Into<String>, max_age: Duration) -> Self {
        Self {
            name: name.into(),
            path: "/".to_string(),
            max_age,
            same_site: SameSitePolicy::Lax,
            secure: true,
            domain: None,
        }
    }

    /// Development-only policy: `Secure=false; SameSite=Lax`. The caller must
    /// document why this is acceptable; it must not be used in production.
    #[must_use]
    pub fn local_development(name: impl Into<String>, max_age: Duration) -> Self {
        Self {
            name: name.into(),
            path: "/".to_string(),
            max_age,
            same_site: SameSitePolicy::Lax,
            secure: false,
            domain: None,
        }
    }

    /// Override `Path`. Defaults to `/`.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set an explicit `Domain` attribute. Pass `None` to produce a host-only
    /// cookie (the default and recommended choice).
    #[must_use]
    pub fn with_domain(mut self, domain: Option<impl Into<String>>) -> Self {
        self.domain = domain.map(Into::into);
        self
    }

    /// The configured cookie name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The configured max-age as a [`Duration`].
    #[must_use]
    pub fn max_age_duration(&self) -> std::time::Duration {
        self.max_age
    }

    /// Whether this policy requires the `Secure` attribute.
    #[must_use]
    pub fn is_secure(&self) -> bool {
        self.secure
    }

    /// Build a `Set-Cookie` header value that delivers `secret` to the client.
    ///
    /// `secret` must be the **plaintext** session or token secret — the only
    /// moment it crosses the wire. The caller must not log the returned string.
    #[must_use]
    pub fn build_set_cookie(&self, secret: &str) -> String {
        let mut parts = format!(
            "{}={}; Max-Age={}; Path={}; HttpOnly; {}",
            self.name,
            secret,
            self.max_age.as_secs(),
            self.path,
            self.same_site.attr(),
        );
        if self.secure {
            parts.push_str("; Secure");
        }
        if let Some(d) = &self.domain {
            parts.push_str("; Domain=");
            parts.push_str(d);
        }
        parts
    }

    /// Build a `Set-Cookie` header value that clears this cookie (e.g. logout).
    ///
    /// Uses `Max-Age=0` with the same path/domain/name so browsers delete the
    /// existing cookie (RFC-006 §4 "clear cookie helper mirrors path/domain/name").
    #[must_use]
    pub fn build_clear_cookie(&self) -> String {
        let mut parts = format!(
            "{}=; Max-Age=0; Path={}; HttpOnly; {}",
            self.name,
            self.path,
            self.same_site.attr(),
        );
        if self.secure {
            parts.push_str("; Secure");
        }
        if let Some(d) = &self.domain {
            parts.push_str("; Domain=");
            parts.push_str(d);
        }
        parts
    }
}

#[cfg(test)]
mod tests;
