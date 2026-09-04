// RFC-046 §4.2: this file must NOT compile. `SessionFailure` must have no
// `From`/`Into` conversion to `PublicSessionError` -- that boundary is the
// whole argument of RFC-046 §3.1 (see the module docs on `SessionFailure`).
// If this file ever starts compiling, someone added the conversion this test
// exists to forbid.
fn main() {
    let failure = codlet::SessionFailure::NotFound;
    let _public: codlet::PublicSessionError = failure.into();
}
