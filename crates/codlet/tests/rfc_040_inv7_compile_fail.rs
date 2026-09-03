//! RFC-040 INV-7 negative test: constructing [`codlet::RedeemSuccess`]
//! outside a won claim must not compile. Uses `trybuild` (dev-dependency
//! only, owner-approved 2026-09-03) to assert this automatically rather
//! than relying on a one-off manual check.
//!
//! INV-7 is the only invariant enforced by the type system rather than a
//! runtime test, so this harness is what keeps it a standing property: if
//! the harness itself stopped running, nothing else would notice the guard
//! vanish (RFC-040 §5.4).

#[test]
fn redeem_success_is_unconstructible_outside_a_won_claim() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/redeem_success_unconstructible.rs");
}
