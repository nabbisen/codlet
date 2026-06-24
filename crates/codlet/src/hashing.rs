//! Secret hashing, key providers, domain separation, and key versioning
//! (RFC-004).
//!
//! Every persisted secret (code, session, form token) is stored only as a
//! keyed HMAC [`LookupKey`], never in plaintext (INV-1). Lookup keys are
//! domain-separated so the same plaintext used in two roles derives two
//! different keys, and every derivation is tagged with the [`KeyVersion`] of
//! the key that produced it so keys can be rotated without an all-or-nothing
//! migration (RFC-004 §12.2).
//!
//! ## Derivation scheme (prefixing — RFC-004 §9.1 recommendation)
//!
//! ```text
//! message = "codlet/v1/lookup" || 0x00 || domain_label || 0x00 || secret_bytes
//! LookupKey = lowercase_hex( HMAC-SHA256(key_bytes, message) )
//! ```
//!
//! The fixed context string and `0x00` separators make the label and secret
//! unambiguous, so distinct domains cannot collide. This is intentionally
//! **not** a simple `HMAC(pepper, value)` with no domain
//! or prefix); the migration adapter (RFC-014) supplies a legacy mode for
//! existing rows.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::FORMAT_VERSION;
use crate::error::KeyError;

type HmacSha256 = Hmac<Sha256>;

/// The lookup context label, combined with [`FORMAT_VERSION`] into the HMAC
/// message prefix. Bumping the format version changes every derived key.
const LOOKUP_CONTEXT: &str = "lookup";

/// Identifier of the key version that produced a [`LookupKey`]. Stored beside
/// every lookup key (RFC-004 §12.2). Not secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyVersion(String);

impl KeyVersion {
    /// Wrap a version label.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the version label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for KeyVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A keyed lookup value: the lowercase-hex HMAC of a domain-separated message.
///
/// Contains no plaintext and is safe to persist, but is still sensitive (it is
/// the database lookup index). Compare lookup keys with
/// [`LookupKey::ct_eq`], not `==`, when the comparison could be timing-attacked.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LookupKey(String);

impl LookupKey {
    /// Borrow the hex digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constant-time equality over the hex bytes (RFC-004; service parity with
    /// `hmac_hex_eq`). Length is allowed to leak: lookup keys are fixed-width.
    #[must_use]
    pub fn ct_eq(&self, other: &LookupKey) -> bool {
        let a = self.0.as_bytes();
        let b = other.0.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        a.ct_eq(b).into()
    }
}

/// The role a secret plays. Part of the HMAC message, so it cross-namespaces
/// lookup keys (RFC-004 §12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretDomain {
    /// One-time code lookup.
    Code,
    /// Session secret lookup.
    Session,
    /// Form-token lookup.
    FormToken,
    /// Pre-auth flow / join-ticket lookup.
    FlowTicket,
}

impl SecretDomain {
    /// The stable wire label embedded in the HMAC message. Changing these
    /// strings is a breaking change to stored lookup keys.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            SecretDomain::Code => "code",
            SecretDomain::Session => "session",
            SecretDomain::FormToken => "form_token",
            SecretDomain::FlowTicket => "flow_ticket",
        }
    }
}

/// A borrowed HMAC key plus the version that identifies it.
pub struct HmacKeyRef<'a> {
    /// The version label of this key.
    pub version: KeyVersion,
    /// The raw key bytes. Never logged or formatted.
    pub bytes: &'a [u8],
}

impl core::fmt::Debug for HmacKeyRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HmacKeyRef")
            .field("version", &self.version)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

/// Supplies HMAC key material. Synchronous, so key lookup does not couple to a
/// web/runtime async model (RFC-004 §3.3). **No fallback key exists**: missing
/// material is an error (INV-2, SR-29).
pub trait KeyProvider {
    /// The active key used for new derivations.
    ///
    /// # Errors
    /// [`KeyError::MissingActiveKey`] if none is configured.
    fn active_hmac_key(&self) -> Result<HmacKeyRef<'_>, KeyError>;

    /// A specific historical key, for validating records written under an older
    /// version during rotation.
    ///
    /// # Errors
    /// [`KeyError::MissingKeyVersion`] if that version is unknown. Callers must
    /// fail closed for that candidate rather than falling back.
    fn hmac_key_by_version(&self, version: &KeyVersion) -> Result<HmacKeyRef<'_>, KeyError>;

    /// All held keys (active first, then previous) for generating verification
    /// candidates during validation (RFC-A).
    ///
    /// The returned vec always contains at least the active key.
    fn all_hmac_keys(&self) -> Result<Vec<HmacKeyRef<'_>>, KeyError>;
}

/// A key provider holding an active key and zero or more previous keys, in
/// memory. Suitable for production when constructed from real secret material
/// loaded at startup, and for tests/examples.
///
/// There is deliberately no `Default` or empty constructor that would yield a
/// usable-but-keyless provider: you must supply real bytes (INV-2).
#[derive(Clone)]
pub struct StaticKeyProvider {
    active_version: KeyVersion,
    keys: Vec<(KeyVersion, Vec<u8>)>,
}

impl StaticKeyProvider {
    /// Construct from an active version+key and optional previous versions.
    ///
    /// # Errors
    /// [`KeyError::InvalidKeyMaterial`] if the active key is empty.
    pub fn new(
        active_version: impl Into<String>,
        active_key: Vec<u8>,
        previous: Vec<(KeyVersion, Vec<u8>)>,
    ) -> Result<Self, KeyError> {
        if active_key.is_empty() {
            return Err(KeyError::InvalidKeyMaterial);
        }
        let active_version = KeyVersion::new(active_version);
        let mut keys = Vec::with_capacity(previous.len() + 1);
        keys.push((active_version.clone(), active_key));
        keys.extend(previous);
        Ok(Self {
            active_version,
            keys,
        })
    }

    /// Convenience constructor with a single key and no previous versions.
    ///
    /// # Errors
    /// [`KeyError::InvalidKeyMaterial`] if `key` is empty.
    pub fn single(version: impl Into<String>, key: Vec<u8>) -> Result<Self, KeyError> {
        Self::new(version, key, Vec::new())
    }
}

impl core::fmt::Debug for StaticKeyProvider {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StaticKeyProvider")
            .field("active_version", &self.active_version)
            .field("key_versions", &self.keys.len())
            .field("keys", &"<redacted>")
            .finish()
    }
}

impl KeyProvider for StaticKeyProvider {
    fn active_hmac_key(&self) -> Result<HmacKeyRef<'_>, KeyError> {
        self.keys
            .iter()
            .find(|(v, _)| *v == self.active_version)
            .map(|(v, k)| HmacKeyRef {
                version: v.clone(),
                bytes: k,
            })
            .ok_or(KeyError::MissingActiveKey)
    }

    fn hmac_key_by_version(&self, version: &KeyVersion) -> Result<HmacKeyRef<'_>, KeyError> {
        self.keys
            .iter()
            .find(|(v, _)| v == version)
            .map(|(v, k)| HmacKeyRef {
                version: v.clone(),
                bytes: k,
            })
            .ok_or(KeyError::MissingKeyVersion)
    }

    fn all_hmac_keys(&self) -> Result<Vec<HmacKeyRef<'_>>, KeyError> {
        if self.keys.is_empty() {
            return Err(KeyError::MissingActiveKey);
        }
        Ok(self
            .keys
            .iter()
            .map(|(v, k)| HmacKeyRef {
                version: v.clone(),
                bytes: k,
            })
            .collect())
    }
}

/// Derives [`LookupKey`]s from secrets using a [`KeyProvider`].
#[derive(Debug, Clone)]
pub struct SecretHasher<K> {
    key_provider: K,
}

impl<K: KeyProvider> SecretHasher<K> {
    /// Wrap a key provider.
    #[must_use]
    pub fn new(key_provider: K) -> Self {
        Self { key_provider }
    }

    /// Borrow the underlying key provider.
    #[must_use]
    pub fn key_provider(&self) -> &K {
        &self.key_provider
    }

    /// Derive a lookup key for `value` in `domain` using the **active** key.
    /// Returns the key plus the active [`KeyVersion`] to store alongside it.
    ///
    /// # Errors
    /// Propagates [`KeyError`] from the provider (e.g. missing active key).
    pub fn lookup_key(
        &self,
        domain: SecretDomain,
        value: &str,
    ) -> Result<(LookupKey, KeyVersion), KeyError> {
        let key = self.key_provider.active_hmac_key()?;
        let lk = derive(key.bytes, domain, value);
        Ok((lk, key.version))
    }

    /// Derive one lookup-key candidate per held key (active first, then
    /// previous). Managers pass the full slice to store finders so that
    /// records written under any held key are reachable during the rotation
    /// grace period (RFC-A).
    ///
    /// # Errors
    /// Propagates [`KeyError::MissingActiveKey`] if no keys are configured.
    pub fn lookup_key_candidates(
        &self,
        domain: SecretDomain,
        value: &str,
    ) -> Result<Vec<(LookupKey, KeyVersion)>, KeyError> {
        let keys = self.key_provider.all_hmac_keys()?;
        Ok(keys
            .into_iter()
            .map(|k| {
                let lk = derive(k.bytes, domain, value);
                (lk, k.version)
            })
            .collect())
    }

    /// Derive a lookup key for `value` in `domain` using a specific key
    /// `version`. Used during validation to re-derive candidates for records
    /// written under older keys.
    ///
    /// # Errors
    /// Propagates [`KeyError::MissingKeyVersion`] if the version is unknown.
    pub fn lookup_key_with_version(
        &self,
        domain: SecretDomain,
        value: &str,
        version: &KeyVersion,
    ) -> Result<LookupKey, KeyError> {
        let key = self.key_provider.hmac_key_by_version(version)?;
        Ok(derive(key.bytes, domain, value))
    }
}

/// Pure derivation: `HMAC-SHA256(key, ctx || 0x00 || domain || 0x00 || value)`,
/// returned as lowercase hex. Kept private; the public surface goes through
/// [`SecretHasher`].
fn derive(key_bytes: &[u8], domain: SecretDomain, value: &str) -> LookupKey {
    // HMAC accepts any key length; new_from_slice only errors for impossible
    // key sizes which Hmac<Sha256> does not have, so this cannot fail.
    let mut mac =
        HmacSha256::new_from_slice(key_bytes).expect("HMAC-SHA256 accepts any key length");
    mac.update(FORMAT_VERSION.as_bytes());
    mac.update(b"/");
    mac.update(LOOKUP_CONTEXT.as_bytes());
    mac.update(&[0u8]);
    mac.update(domain.label().as_bytes());
    mac.update(&[0u8]);
    mac.update(value.as_bytes());
    let digest = mac.finalize().into_bytes();
    LookupKey(hex_lower(&digest))
}

/// Lowercase hex encoding without pulling in the `hex` crate, keeping the core
/// dependency set minimal (NFR-3).
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests;
