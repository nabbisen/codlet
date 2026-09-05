// Fixture for `xtask self-test`, gate `no-interpolated-sql-values` (RFC-048).
// Not compiled into the workspace — see no_fallback_key.rs for why. Its path
// (xtask/fixtures/, not .../tests/) must not trip the gate's own "/tests/"
// exemption.
//
// Deliberately violates the gate: interpolates a host-supplied value directly
// into a SQL comparison via `format!("{:?}")`, exactly the shape that let a
// SQL injection reach `claim_code`'s `scope` handling. `{:?}` on a `&str` is
// Rust escaping, not SQL escaping.
fn bad_query(scope: &str) -> String {
    format!("SELECT * FROM codlet_codes WHERE scope = {scope:?}")
}
