//! RFC-046 §4.2 negative test: there must be no way to convert
//! [`codlet::SessionFailure`] into [`codlet::PublicSessionError`]. Uses
//! `trybuild` (dev-dependency only) the same way RFC-040's INV-7 test does --
//! a compile-time boundary needs a compile-time test, not a runtime one.
//!
//! This is RFC-046's most important acceptance criterion (§4.2, "everything
//! else here is mechanical; this is the one that keeps the RFC's argument
//! true a year from now"): if a future contributor adds
//! `impl From<SessionFailure> for PublicSessionError` "for convenience," this
//! is what catches it.

#[test]
fn session_failure_has_no_conversion_to_public_session_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/session_failure_not_convertible_to_public_session_error.rs");
}
