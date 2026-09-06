//! RFC-045 negative test: constructing a
//! [`codlet::SessionValidationOutcome::Authenticated`] value from outside the
//! crate must not compile. Uses `trybuild` (dev-dependency only, already
//! owner-approved for RFC-040's INV-7 harness), mirroring that harness.
//!
//! `rotate` takes `&SessionValidationOutcome`, the same enum `validate`
//! returns for both success and failure — the type itself cannot reject an
//! `Unauthenticated` value passed to `rotate` (both are valid instances of
//! the same sum type; Rust has no way to express "only this variant" as a
//! distinct static type without extracting it into a private-proof-carrying
//! value the way [`codlet::RedeemSuccess`] does for INV-7). What *can* be
//! made unforgeable, and is here, is the `Authenticated` variant itself: a
//! caller cannot fabricate one out of thin air and hand it to `rotate`
//! claiming a validation happened when it did not. This harness is what
//! keeps that a standing property, the same reason RFC-040's INV-7 harness
//! exists.
#[test]
fn session_validation_outcome_authenticated_is_unconstructible_outside_the_crate() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/session_authenticated_unconstructible.rs");
}
