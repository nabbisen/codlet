//! The per-test registries (`*_TESTS` / `run_*_test`) must stay in step with the
//! tests actually defined, or a Workers harness that runs them one by one would
//! silently skip a test the full suite runs.

use codlet::mem::{MemCodeStore, MemFormTokenStore, MemSessionStore};
use codlet_conformance as c;

/// Count the `async fn test_*` definitions in a suite's source file.
fn defined(src: &str) -> usize {
    src.matches("\nasync fn test_").count()
}

#[tokio::test]
async fn every_listed_test_is_runnable_and_unknown_names_are_not() {
    for name in c::CODE_STORE_TESTS {
        assert!(c::run_code_store_test(name, &|| async { MemCodeStore::new() }).await);
    }
    for name in c::SESSION_STORE_TESTS {
        assert!(c::run_session_store_test(name, &|| async { MemSessionStore::new() }).await);
    }
    for name in c::FORM_TOKEN_STORE_TESTS {
        assert!(c::run_form_token_store_test(name, &|| async { MemFormTokenStore::new() }).await);
    }
    assert!(!c::run_code_store_test("no_such_test", &|| async { MemCodeStore::new() }).await);
}

#[test]
fn registries_list_every_defined_sequential_test() {
    // code.rs also defines the concurrent race test, which is deliberately not
    // in `CODE_STORE_TESTS` (it has its own entry point).
    let code = defined(include_str!("../src/code.rs"));
    assert_eq!(
        code,
        c::CODE_STORE_TESTS.len() + 1,
        "code.rs vs CODE_STORE_TESTS"
    );
    let session = defined(include_str!("../src/session.rs"));
    assert_eq!(
        session,
        c::SESSION_STORE_TESTS.len(),
        "session.rs vs SESSION_STORE_TESTS"
    );
    let token = defined(include_str!("../src/token.rs"));
    assert_eq!(
        token,
        c::FORM_TOKEN_STORE_TESTS.len(),
        "token.rs vs FORM_TOKEN_STORE_TESTS"
    );
}
