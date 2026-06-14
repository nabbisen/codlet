//! Secret-bearing and opaque-identifier newtypes.
//!
//! Secret types wrap a [`SecretString`] whose `Debug` (and `Display`, where
//! present) implementations are redacted, so a plaintext code, session secret,
//! or form-token secret cannot leak through logs, panic messages, or
//! `{:?}`-formatting (threat model INV-1, SR-38). The plaintext is reachable
//! only through an explicit [`SecretString::expose`] call, which is easy to
//! grep for in review.
//!
//! These are the v0.1 foundations of the typestate model in RFC-019. They are
//! deliberately minimal: enough to make misuse visible, without committing to
//! the full typestate surface before the store traits exist.

/// A string holding a sensitive value whose contents are never shown by
/// `Debug` or `Display`.
///
/// The inner value is accessible only via [`SecretString::expose`]. Equality is
/// provided for tests and lookup bookkeeping; it is **not** constant-time and
/// must not be used to compare secrets that an attacker can influence by timing
/// — compare derived [`crate::hashing::LookupKey`] values instead.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a value as a secret. The value is moved in and never copied to any
    /// formatting buffer.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Borrow the plaintext. Named `expose` so its use is visible in review and
    /// easy to grep for; callers must not log or persist the returned value.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Number of bytes in the underlying value. Length is not considered
    /// sensitive for the fixed-width secrets codlet generates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the underlying value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl core::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretString(<redacted>)")
    }
}

impl core::fmt::Display for SecretString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Serialize as the redaction marker, never the plaintext (SR-3, SR-39).
#[cfg(feature = "serde")]
impl serde::Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("<redacted>")
    }
}

/// Define a secret-bearing newtype over [`SecretString`] with redacted
/// `Debug`/`Display` inherited from the inner type.
macro_rules! secret_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $name(SecretString);

        impl $name {
            /// Wrap an already-generated or received plaintext value.
            #[must_use]
            pub fn new(value: String) -> Self {
                Self(SecretString::new(value))
            }

            /// Borrow the plaintext. See [`SecretString::expose`].
            #[must_use]
            pub fn expose(&self) -> &str {
                self.0.expose()
            }

            /// Borrow the inner [`SecretString`].
            #[must_use]
            pub fn as_secret(&self) -> &SecretString {
                &self.0
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(s)
            }
        }
    };
}

secret_newtype! {
    /// A one-time code in plaintext — either freshly generated for one-time
    /// display, or received as user input. Never persisted (INV-1).
    PlainCode
}

secret_newtype! {
    /// A session secret in plaintext. Lives only in the cookie; only its
    /// derived lookup key is stored (RFC-006).
    SessionSecret
}

secret_newtype! {
    /// A form-token secret in plaintext. Lives only in the rendered form or a
    /// short-lived cookie; only its derived lookup key is stored (RFC-007).
    FormTokenSecret
}

/// Define an opaque, non-secret string identifier newtype.
macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(String);

        impl $name {
            /// Wrap a host- or store-provided identifier.
            #[must_use]
            pub fn new(value: String) -> Self {
                Self(value)
            }

            /// Borrow the identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

id_newtype! {
    /// Identifier of a code record. Not a secret; safe to log and display.
    CodeId
}

id_newtype! {
    /// Host-owned identity anchor returned after authentication. codlet does
    /// not interpret its meaning (RFC-001).
    SubjectId
}

id_newtype! {
    /// Identifier of a session record. Not a bearer credential on its own
    /// (RFC-006 §13.1).
    SessionId
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_string_redacts_debug_and_display() {
        let s = SecretString::new("hunter2".to_string());
        assert_eq!(format!("{s:?}"), "SecretString(<redacted>)");
        assert_eq!(format!("{s}"), "<redacted>");
        // The plaintext must not appear in either rendering.
        assert!(!format!("{s:?}").contains("hunter2"));
        assert!(!format!("{s}").contains("hunter2"));
        // But is reachable explicitly.
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn secret_newtypes_redact_debug() {
        let c = PlainCode::new("ABCD2345".to_string());
        let dbg = format!("{c:?}");
        assert!(
            !dbg.contains("ABCD2345"),
            "PlainCode Debug leaked plaintext: {dbg}"
        );
        assert!(dbg.contains("<redacted>"));
        assert_eq!(c.expose(), "ABCD2345");
    }

    #[test]
    fn id_newtype_displays_and_roundtrips() {
        let id = CodeId::new("abc123".to_string());
        assert_eq!(id.as_str(), "abc123");
        assert_eq!(format!("{id}"), "abc123");
        assert_eq!(CodeId::from("x".to_string()).as_str(), "x");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn secret_serializes_redacted() {
        let c = SessionSecret::new("supersecret".to_string());
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"<redacted>\"");
        assert!(!json.contains("supersecret"));
    }
}
