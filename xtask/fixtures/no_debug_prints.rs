// Fixture for `xtask self-test`, gate `no-debug-prints` (RFC-040 §5.2).
// Not compiled into the workspace — see no_fallback_key.rs for why. Its path
// (xtask/fixtures/, not .../tests/) must not trip the gate's own
// "/tests/" exemption, or this fixture would silently stop testing anything.
//
// Deliberately violates the gate: a debug print of secret-bearing material.
fn bad_leak(secret: &str) {
    println!("secret: {secret}");
}
