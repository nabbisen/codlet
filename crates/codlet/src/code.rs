//! One-time code policy, generation, normalization, and validation (RFC-003).
//!
//! The four representations of a code are kept distinct (RFC-003 §11.2):
//! generated plaintext and user input are [`crate::secret::PlainCode`] /
//! normalized `String` (secret, never persisted); the persisted value is a
//! [`crate::hashing::LookupKey`] derived in the [`crate::hashing`] module.

pub mod alphabet;
pub mod generate;
pub mod normalize;
pub mod policy;

pub use alphabet::{Alphabet, DEFAULT_ALPHABET};
pub use generate::{generate_code, validate_code_input};
pub use normalize::normalize;
pub use policy::{CodePolicy, DEFAULT_MAX_RAW_LEN, SECURE_MIN_HUMAN_LENGTH, SHORT_COMPAT_LENGTH};
