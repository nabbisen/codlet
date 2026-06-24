//! Unit tests for the `cookie` module.
use super::*;

const HOUR: Duration = Duration::from_secs(3_600);

fn p() -> CookiePolicy {
    CookiePolicy::production_strict("test_sid", HOUR)
}

#[test]
fn set_cookie_contains_required_attributes() {
    let c = p().build_set_cookie("mysecret");
    assert!(c.contains("HttpOnly"), "missing HttpOnly");
    assert!(c.contains("Secure"), "missing Secure");
    assert!(c.contains("SameSite=Strict"), "missing SameSite=Strict");
    assert!(c.contains("Path=/"), "missing Path=/");
    assert!(c.contains("Max-Age=3600"), "missing Max-Age");
    assert!(c.starts_with("test_sid=mysecret"), "wrong name=value");
}

#[test]
fn clear_cookie_uses_max_age_zero() {
    let c = p().build_clear_cookie();
    assert!(c.contains("Max-Age=0"), "clear must use Max-Age=0");
    assert!(c.contains("HttpOnly"), "missing HttpOnly");
    assert!(c.contains("Secure"), "missing Secure");
    assert!(c.contains("SameSite=Strict"));
    assert!(c.starts_with("test_sid=;"), "wrong name on clear");
}

#[test]
fn domain_omitted_by_default() {
    let c = p().build_set_cookie("s");
    assert!(!c.contains("Domain="), "default must omit Domain");
}

#[test]
fn explicit_domain_is_emitted() {
    let c = p().with_domain(Some("example.com")).build_set_cookie("s");
    assert!(c.contains("Domain=example.com"), "explicit domain missing");
}

#[test]
fn clear_cookie_mirrors_path_and_domain() {
    let policy = CookiePolicy::production_strict("sid", HOUR)
        .with_path("/app")
        .with_domain(Some("example.com"));
    let set = policy.build_set_cookie("s");
    let clear = policy.build_clear_cookie();
    assert!(set.contains("Path=/app"));
    assert!(clear.contains("Path=/app"));
    assert!(set.contains("Domain=example.com"));
    assert!(clear.contains("Domain=example.com"));
}

#[test]
fn local_development_omits_secure() {
    let c = CookiePolicy::local_development("dev_sid", HOUR).build_set_cookie("s");
    assert!(!c.contains("; Secure"), "dev profile must not set Secure");
    assert!(c.contains("HttpOnly"), "HttpOnly always required");
}

#[test]
fn lax_profile_uses_lax_samesite() {
    let c = CookiePolicy::production_lax("sid", HOUR).build_set_cookie("s");
    assert!(c.contains("SameSite=Lax"));
    assert!(c.contains("; Secure"));
}

#[test]
fn secret_not_duplicated_elsewhere_in_value() {
    // Sanity: the secret appears exactly once (as the value), not in any
    // attribute name.
    let c = p().build_set_cookie("hunter2");
    let count = c.matches("hunter2").count();
    assert_eq!(count, 1, "secret appeared {count} times in {c:?}");
}
