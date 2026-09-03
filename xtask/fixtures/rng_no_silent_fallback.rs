// Fixture for `xtask self-test`, gate `rng-no-silent-fallback` (RFC-040 §5.2).
// Not compiled into the workspace — see no_fallback_key.rs for why.
//
// Deliberately violates the gate: `.ok()` swallows the RNG result on the
// same line as the `getrandom` call.
fn bad_fill(buf: &mut [u8]) {
    let _ = getrandom::fill(buf).ok();
}
