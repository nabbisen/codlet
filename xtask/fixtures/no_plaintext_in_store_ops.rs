// Fixture for `xtask self-test`, gate `no-plaintext-in-store-ops`
// (RFC-040 §5.2). Not compiled into the workspace — see no_fallback_key.rs
// for why. Its path (xtask/fixtures/, not .../tests/) must not trip the
// gate's own "/tests/" exemption.
//
// Deliberately violates the gate: `.expose()` (the plaintext secret) passed
// directly into a `.bind(...)` call, exactly the shape that would persist
// the bearer value instead of its keyed lookup hash.
fn bad_store(secret: &Secret, query: Query) {
    query.bind(secret.expose());
}
