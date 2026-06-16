//! Key rotation example (RFC-016, RFC-017).
//!
//! Demonstrates:
//! - configuring an active key and a previous key during rotation;
//! - records written under the old key still validating during the grace period;
//! - a missing key version failing closed, not silently defaulting.
//!
//! Run with: `cargo run  # from examples/key_rotation/`

use codlet_core::error::KeyError;
use codlet_core::hashing::{KeyVersion, SecretDomain, SecretHasher, StaticKeyProvider};

fn main() {
    // ── Step 1: Initial deployment (only v1) ─────────────────────────────────
    let v1_key = b"version-one-key-32-bytes-example".to_vec();
    let hasher_v1_only =
        SecretHasher::new(StaticKeyProvider::single("v1", v1_key.clone()).expect("non-empty"));

    // Record written under v1.
    let (lk_v1, kv) = hasher_v1_only
        .lookup_key(SecretDomain::Session, "my-session-secret")
        .unwrap();
    assert_eq!(kv.as_str(), "v1");
    println!("[v1 only] issued session under key version: {kv}");

    // ── Step 2: Rotation — v2 becomes active, v1 is previous ─────────────────
    let v2_key = b"version-two-key-32-bytes-example".to_vec();
    let hasher_rotated = SecretHasher::new(
        StaticKeyProvider::new("v2", v2_key, vec![(KeyVersion::new("v1"), v1_key)])
            .expect("non-empty"),
    );

    // New records use v2.
    let (_lk_v2, kv2) = hasher_rotated
        .lookup_key(SecretDomain::Session, "new-session-secret")
        .unwrap();
    assert_eq!(kv2.as_str(), "v2");
    println!("[rotated] new session under key version: {kv2}");

    // Old record can still be re-derived for validation using v1.
    let lk_v1_check = hasher_rotated
        .lookup_key_with_version(
            SecretDomain::Session,
            "my-session-secret",
            &KeyVersion::new("v1"),
        )
        .unwrap();
    assert!(
        lk_v1.ct_eq(&lk_v1_check),
        "old record must still validate with previous key"
    );
    println!("[rotated] old v1 session still validates ✓");

    // ── Step 3: After grace period — v1 removed ───────────────────────────────
    let v2_only = SecretHasher::new(
        StaticKeyProvider::single("v2", b"version-two-key-32-bytes-example".to_vec())
            .expect("non-empty"),
    );

    // Attempting to derive a v1 lookup key now fails closed — no fallback.
    let result = v2_only.lookup_key_with_version(
        SecretDomain::Session,
        "my-session-secret",
        &KeyVersion::new("v1"),
    );
    assert!(
        matches!(result, Err(KeyError::MissingKeyVersion)),
        "missing key must fail closed, not produce a garbage lookup key"
    );
    println!("[post-rotation] v1 key removed — old records are now permanently invalid ✓");
    println!("  (ensure all v1 records have expired before removing the previous key)");
}
