// RFC-040 INV-7: this file must NOT compile. `RedeemSuccess::_claim_proof`
// is `pub(crate)` inside codlet, so an external crate (this trybuild
// fixture) cannot name a value for it — there is no way to construct
// `RedeemSuccess` without going through `CodeAuth::redeem`, which only
// produces one after `claim_code` reports `Won`.
fn main() {
    let _ = codlet::RedeemSuccess {
        subject: codlet::secret::SubjectId::new("user".to_string()),
        grant: None,
    };
}
