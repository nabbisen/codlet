//! Randomness abstraction (RFC-020).
//!
//! All secret generation goes through [`RandomSource`] so it can be made
//! deterministic in tests and so that production failure propagates instead of
//! silently degrading. The cardinal rule: **RNG failure is fatal to the
//! operation; no fallback value is ever produced** (INV-3).

use crate::error::RandomError;

/// A source of cryptographically secure random bytes.
///
/// Implementations must fill the entire buffer with unpredictable bytes or
/// return [`RandomError`]. Returning `Ok(())` after a partial or zeroed fill is
/// a security defect.
pub trait RandomSource {
    /// Fill `dest` entirely with secure random bytes, or fail.
    ///
    /// # Errors
    /// Returns [`RandomError`] if secure randomness cannot be obtained. Callers
    /// must propagate the error and must not substitute any default value.
    fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RandomError>;
}

/// Production randomness backed by the platform CSPRNG via `getrandom`.
///
/// On WASM/Workers this delegates to `crypto.getRandomValues` (matching the
/// source service's RNG path).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRandom;

impl SystemRandom {
    /// Construct the system randomness source.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl RandomSource for SystemRandom {
    fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RandomError> {
        // Propagate any failure as RandomError. Never `unwrap`/`expect` here:
        // the source service's `random_token` used `.expect("getrandom failed")`
        // which would panic rather than fail closed gracefully; codlet returns
        // a typed error so callers can map it to a generic public failure.
        getrandom::fill(dest).map_err(|_| RandomError)
    }
}

/// Deterministic test randomness. Available under the `test-utils` feature and
/// in this crate's own tests. **Never** use in production: output is
/// predictable by construction.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone)]
pub struct FixedBytesRandom {
    bytes: Vec<u8>,
    pos: usize,
}

#[cfg(any(test, feature = "test-utils"))]
impl FixedBytesRandom {
    /// Create a source that yields the given bytes in order, cycling when
    /// exhausted. Useful for steering rejection sampling in tests.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        assert!(
            !bytes.is_empty(),
            "FixedBytesRandom needs at least one byte"
        );
        Self { bytes, pos: 0 }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl RandomSource for FixedBytesRandom {
    fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RandomError> {
        for slot in dest.iter_mut() {
            *slot = self.bytes[self.pos % self.bytes.len()];
            self.pos += 1;
        }
        Ok(())
    }
}

/// A randomness source that always fails. Used to prove fail-closed behavior
/// (RFC-003 §11.5 acceptance: "RNG failure test uses a fake RNG that always
/// errors").
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysFailRandom;

#[cfg(any(test, feature = "test-utils"))]
impl RandomSource for AlwaysFailRandom {
    fn fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), RandomError> {
        Err(RandomError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_random_fills_distinct_buffers() {
        let mut r = SystemRandom::new();
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        r.fill_bytes(&mut a).unwrap();
        r.fill_bytes(&mut b).unwrap();
        // Astronomically unlikely to be equal if real entropy is used.
        assert_ne!(a, b);
        assert!(a.iter().any(|&x| x != 0));
    }

    #[test]
    fn always_fail_random_errors() {
        let mut r = AlwaysFailRandom;
        let mut buf = [0u8; 4];
        assert_eq!(r.fill_bytes(&mut buf), Err(RandomError));
    }

    #[test]
    fn fixed_bytes_random_is_deterministic() {
        let mut r = FixedBytesRandom::new(vec![1, 2, 3]);
        let mut buf = [0u8; 5];
        r.fill_bytes(&mut buf).unwrap();
        assert_eq!(buf, [1, 2, 3, 1, 2]);
    }
}
