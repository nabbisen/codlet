// RFC-045: this file must NOT compile. `SessionValidationOutcome::Authenticated`
// is `#[non_exhaustive]`, so an external crate (this trybuild fixture) cannot
// construct one with struct-literal syntax — the only way to obtain a real
// `Authenticated` value is a genuine call to `SessionManager::validate`,
// which routes through `classify_session`. This is what makes
// `SessionManager::rotate`'s precondition (an outcome that actually came
// from a validation) unforgeable rather than merely documented.
fn main() {
    let _ = codlet::SessionValidationOutcome::Authenticated {
        subject: codlet::secret::SubjectId::new("user".to_string()),
        session_id: codlet::secret::SessionId::new("sess".to_string()),
        expires_at: 0,
    };
}
