//! [`WorkerKeyProvider`] — loads HMAC key material from Cloudflare Worker
//! `Env` secrets (RFC-033 §9).
//!
//! Fails closed: if any configured secret binding is missing or empty,
//! construction returns `Err` (INV-2).

use codlet::error::KeyError;
use codlet::hashing::{HmacKeyRef, KeyProvider, KeyVersion, StaticKeyProvider};

/// Loads active and previous HMAC key material from Cloudflare Worker
/// `Env` secrets (RFC-033 §9).
#[derive(Clone)]
pub struct WorkerKeyProvider {
    inner: StaticKeyProvider,
}

impl WorkerKeyProvider {
    /// Load key material from `Env` secrets (RFC-033 §9).
    ///
    /// Fails closed if any binding is missing or empty (INV-2).
    pub fn from_env(
        env: &worker::Env,
        active_version: &str,
        active_binding: &str,
        previous: &[(&str, &str)],
    ) -> worker::Result<Self> {
        let active_bytes = load_secret(env, active_binding)?;
        let mut prev_keys = Vec::with_capacity(previous.len());
        for (version, binding) in previous {
            let bytes = load_secret(env, binding)?;
            prev_keys.push((KeyVersion::new(*version), bytes));
        }
        let provider = StaticKeyProvider::new(active_version, active_bytes, prev_keys)
            .map_err(|e| worker::Error::RustError(format!("codlet key error: {e}")))?;
        Ok(Self { inner: provider })
    }
}

fn load_secret(env: &worker::Env, binding: &str) -> worker::Result<Vec<u8>> {
    let secret = env
        .secret(binding)
        .map_err(|_| {
            worker::Error::RustError(format!(
                "missing Wrangler secret: {binding} (INV-2: key material required)"
            ))
        })?
        .to_string();
    if secret.is_empty() {
        return Err(worker::Error::RustError(format!(
            "empty Wrangler secret: {binding} (INV-2: key material must not be empty)"
        )));
    }
    Ok(secret.into_bytes())
}

impl KeyProvider for WorkerKeyProvider {
    fn active_hmac_key(&self) -> Result<HmacKeyRef<'_>, KeyError> {
        self.inner.active_hmac_key()
    }

    fn hmac_key_by_version(&self, version: &KeyVersion) -> Result<HmacKeyRef<'_>, KeyError> {
        self.inner.hmac_key_by_version(version)
    }

    fn all_hmac_keys(&self) -> Result<Vec<HmacKeyRef<'_>>, KeyError> {
        self.inner.all_hmac_keys()
    }
}
