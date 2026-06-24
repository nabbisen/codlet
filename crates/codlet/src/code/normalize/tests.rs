//! Unit tests for the `normalize` module.
use super::*;

#[test]
fn strips_separators_and_uppercases() {
    assert_eq!(normalize("x7-y9 z2"), "X7Y9Z2");
    assert_eq!(normalize("X7Y9Z2"), "X7Y9Z2");
    assert_eq!(normalize("  a b - c "), "ABC");
}

#[test]
fn does_not_drop_ambiguous_characters() {
    // Compatibility guard: normalization must NOT remove 0/1/O/I/L
    // (contrast with the generation alphabet, which excludes them).
    assert_eq!(normalize("o1il0"), "O1IL0");
}

#[test]
fn idempotent() {
    for s in ["X7-Y9 Z2", "abc", "  ", "Ünïcödé", "a-b-c-1-2-3", ""] {
        assert_eq!(
            normalize(&normalize(s)),
            normalize(s),
            "not idempotent for {s:?}"
        );
    }
}

#[test]
fn empty_and_separator_only_become_empty() {
    assert_eq!(normalize(""), "");
    assert_eq!(normalize("  --  "), "");
}

#[test]
fn no_panic_on_arbitrary_unicode() {
    // Spot-check a range of scalar values; full coverage is in the
    // property test in the crate test suite.
    for cp in [
        0u32, 0x09, 0x20, 0x2d, 0x41, 0x7f, 0x80, 0xa0, 0x1f600, 0x10ffff,
    ] {
        if let Some(ch) = char::from_u32(cp) {
            let s: String = core::iter::once(ch).collect();
            let _ = normalize(&s);
        }
    }
}
