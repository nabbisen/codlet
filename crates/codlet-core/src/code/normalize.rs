//! Code normalization (RFC-003 FR-2, INV-4).
//!
//! Normalization must be **identical** on the issue path and the redeem path,
//! and **idempotent** (`normalize(normalize(x)) == normalize(x)`), or valid
//! codes fail to match their stored lookup key.
//!
//! ## Compatibility note
//!
//! The `zinnias-ciao` `normalize_invite_code` strips ASCII whitespace and
//! hyphens and uppercases ASCII letters — and **nothing else**. In particular
//! it does *not* drop the visually ambiguous characters `0 1 O I L` (those are
//! merely excluded from the generation *alphabet*, RFC-003 §4). codlet
//! reproduces that exact behavior so existing service codes keep matching. The
//! ambiguity handling lives in [`super::alphabet`], not here.

/// Normalize raw code input into its canonical form: strip ASCII whitespace and
/// `-`, uppercase ASCII letters, leave everything else untouched.
///
/// This never panics on arbitrary Unicode input. Non-ASCII characters are
/// preserved here; validation (RFC-003 FR-2) is responsible for rejecting them.
#[must_use]
pub fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
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
}
