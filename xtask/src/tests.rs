//! Unit tests for the `xtask` gate infrastructure (RFC-040 §3.1, §5.1).
use super::*;

#[test]
fn empty_corpus_is_an_error_not_an_empty_pass() {
    // RFC-040 §3.1: absence of evidence is never evidence of absence. Before
    // this fix, an empty corpus silently produced `Ok(vec![])`, which every
    // gate's `hits.is_empty()` check then read as "no violations found" —
    // the exact shape of the `core-deps` fail-open defect (RFC-036 §3.5),
    // reproduced here for `xtask`.
    let result = require_nonempty(Vec::new(), "some/nonexistent/path");
    assert!(
        result.is_err(),
        "an empty corpus must be an error, not an empty Ok(vec![])"
    );
}

#[test]
fn nonempty_corpus_passes_through_unchanged() {
    let sources = vec![("a.rs".to_string(), "fn main() {}".to_string())];
    let result = require_nonempty(sources.clone(), "irrelevant");
    assert_eq!(result, Ok(sources));
}

#[test]
fn assert_covers_fails_when_the_claimed_crate_is_absent() {
    let sources = vec![("crates/codlet-sqlx/src/lib.rs".to_string(), String::new())];
    assert!(
        assert_covers(&sources, "crates/codlet/src/").is_err(),
        "a corpus that only contains codlet-sqlx must not be reported as \
         covering codlet — `codlet-sqlx/src/` must not satisfy a \
         `codlet/src/` substring match"
    );
}

#[test]
fn assert_covers_passes_when_the_claimed_crate_is_present() {
    let sources = vec![("crates/codlet/src/lib.rs".to_string(), String::new())];
    assert!(assert_covers(&sources, "crates/codlet/src/").is_ok());
}

#[test]
fn real_library_sources_is_nonempty_and_covers_codlet() {
    // Sanity check against the real corpus, not a fixture: proves the
    // production path (not just the unit-testable helpers) actually holds
    // in this checkout.
    let sources = library_sources().expect("real crates/ corpus must not be empty");
    assert_covers(&sources, "crates/codlet/src/").expect("real crates/ corpus must include codlet");
}
