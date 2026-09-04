//! Unit tests for the `normalize` module.
use super::*;
use proptest::prelude::*;

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
    // Spot-check a range of scalar values; exhaustive coverage is P-4 below.
    for cp in [
        0u32, 0x09, 0x20, 0x2d, 0x41, 0x7f, 0x80, 0xa0, 0x1f600, 0x10ffff,
    ] {
        if let Some(ch) = char::from_u32(cp) {
            let s: String = core::iter::once(ch).collect();
            let _ = normalize(&s);
        }
    }
}

// ── RFC-041: INV-4 properties P-1 (idempotence) and P-4 (totality) ─────────

/// Curated inputs guaranteed to cover the shapes RFC-041 §3.3 requires: a
/// hyphen, ASCII whitespace, a lowercase ASCII letter, a non-ASCII character,
/// and the empty string. Mixed into the property strategy below so every run
/// actually exercises these — a corpus that never produces them would pass
/// for the wrong reason, and no property-testing framework would say so.
const INTERESTING_INPUTS: &[&str] = &[
    "",
    "a-b-c",
    "  x y  ",
    "Ünïcödé",
    "abc",
    "X7-y9 z2",
    "\t\n\r",
    "😀-😀",
];

#[test]
fn interesting_inputs_cover_required_shapes() {
    // RFC-041 §3.3 requirement 2, made an explicit, checked fact rather than
    // an assumption about what the generator below happens to produce.
    assert!(
        INTERESTING_INPUTS.contains(&""),
        "corpus missing the empty string"
    );
    assert!(
        INTERESTING_INPUTS.iter().any(|s| s.contains('-')),
        "corpus missing a hyphen"
    );
    assert!(
        INTERESTING_INPUTS
            .iter()
            .any(|s| s.chars().any(|c| c.is_ascii_whitespace())),
        "corpus missing ASCII whitespace"
    );
    assert!(
        INTERESTING_INPUTS
            .iter()
            .any(|s| s.chars().any(|c| c.is_ascii_lowercase())),
        "corpus missing a lowercase ASCII letter"
    );
    assert!(
        INTERESTING_INPUTS.iter().any(|s| !s.is_ascii()),
        "corpus missing a non-ASCII character"
    );
}

/// Mixes the curated corpus above with a broader arbitrary-Unicode generator,
/// so proptest's shrinker still explores beyond the curated set while every
/// run is guaranteed (30% weight) to also sample the required shapes.
fn any_code_input() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => proptest::sample::select(INTERESTING_INPUTS).prop_map(str::to_owned),
        7 => ".{0,32}",
    ]
}

proptest! {
    #[test]
    fn p1_idempotent_for_arbitrary_str(s in any_code_input()) {
        // P-1: normalize(normalize(s)) == normalize(s).
        prop_assert_eq!(normalize(&normalize(&s)), normalize(&s));
    }

    #[test]
    fn p4_never_panics_on_arbitrary_unicode(s in any_code_input()) {
        // P-4: totality -- normalize must not panic on any input, including
        // the empty string and lone/adjacent non-ASCII scalar values.
        let _ = normalize(&s);
    }
}
