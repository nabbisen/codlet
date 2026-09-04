//! Unit tests for the `alphabet` module.
use super::*;
use proptest::prelude::*;

#[test]
fn default_excludes_ambiguous_characters() {
    let a = Alphabet::unambiguous();
    for &c in b"01OIL" {
        assert!(
            !a.contains(c),
            "default alphabet contains ambiguous '{}'",
            c as char
        );
    }
    assert_eq!(a.len(), 31);
}

#[test]
fn ceiling_is_248_for_default() {
    assert_eq!(Alphabet::unambiguous().unbiased_ceiling(), 248);
}

#[test]
fn all_accepted_bytes_map_into_alphabet() {
    let a = Alphabet::unambiguous();
    for b in 0..a.unbiased_ceiling() {
        let sym = a.symbol_for_byte(b as u8);
        assert!(a.contains(sym));
    }
}

#[test]
fn rejects_small_duplicate_and_non_ascii() {
    assert_eq!(Alphabet::new(b"A"), Err(PolicyError::AlphabetTooSmall));
    assert_eq!(Alphabet::new(b"AAB"), Err(PolicyError::AlphabetNotUnique));
    assert_eq!(
        Alphabet::new(&[b'A', 0x80]),
        Err(PolicyError::AlphabetNotAscii)
    );
    assert!(Alphabet::new(b"AB").is_ok());
}

// ── RFC-041: INV-4 properties P-3 (alphabet safety), P-5 (exact uniformity),
//    P-7 (ceiling correctness) ───────────────────────────────────────────

/// P-7: `unbiased_ceiling()` is the largest multiple of `len` that is `<= 256`,
/// for every length the type permits -- not just the lengths reachable
/// through `Alphabet::new`'s public ASCII/uniqueness validation (which caps
/// out at 128 distinct ASCII bytes). Exhaustive over the arithmetic, so it
/// needs no property framework. `tests` is a child module of `alphabet`, so
/// the private `symbols` field is visible here, letting this bypass
/// `Alphabet::new` deliberately to reach lengths up to 256.
#[test]
fn p7_ceiling_is_the_largest_multiple_of_len_up_to_256() {
    for len in 2..=256usize {
        let a = Alphabet {
            symbols: vec![0u8; len],
        };
        let ceiling = a.unbiased_ceiling();
        assert!(
            ceiling <= 256,
            "ceiling {ceiling} exceeds 256 for len {len}"
        );
        assert_eq!(
            ceiling % len,
            0,
            "ceiling {ceiling} is not a multiple of len {len}"
        );
        assert!(
            ceiling + len > 256,
            "ceiling {ceiling} is not the largest multiple of {len} that is <= 256"
        );
    }
}

/// P-5: over all 256 byte values, every accepted byte (`< ceiling`) maps into
/// the alphabet, and each symbol is reached by exactly `ceiling / len` bytes
/// -- an exact count, not a tolerance. Exhaustive over the default alphabet's
/// 256-byte input domain.
#[test]
fn p5_exact_uniformity_for_default_alphabet() {
    let a = Alphabet::unambiguous();
    let ceiling = a.unbiased_ceiling();
    let len = a.len();
    let expected_per_symbol = ceiling / len;

    let mut counts = std::collections::HashMap::new();
    for b in 0u16..256 {
        let byte = b as u8;
        if (byte as usize) < ceiling {
            let sym = a.symbol_for_byte(byte);
            assert!(
                a.contains(sym),
                "accepted byte {byte} mapped to {sym:#x}, outside the alphabet"
            );
            *counts.entry(sym).or_insert(0usize) += 1;
        }
    }
    assert_eq!(
        counts.len(),
        len,
        "not every symbol in the alphabet was reached by an accepted byte"
    );
    for (&sym, &count) in &counts {
        assert_eq!(
            count, expected_per_symbol,
            "symbol {:?} (0x{sym:02x}) was reached by {count} bytes, expected exactly {expected_per_symbol}",
            sym as char
        );
    }
}

/// P-3: for any `Alphabet` -- not just the safe default -- every accepted
/// symbol must be a normalization fixed-point, or the issue and redeem paths
/// diverge (INV-4, RFC-041 §2.1). `Alphabet::new` now enforces this at
/// construction (RFC-043), so this generator can no longer assume every
/// unique-ASCII-byte set it draws is accepted -- it filters to the sets that
/// actually construct, via `Alphabet::new` itself, rather than assuming a
/// shape. Length 2..=20 unique ASCII bytes drawn, as before RFC-043.
fn arbitrary_ascii_alphabet() -> impl Strategy<Value = Alphabet> {
    proptest::collection::hash_set(0u8..128, 2..=20).prop_filter_map(
        "Alphabet::new rejects non-fixed-point symbols (RFC-043)",
        |set| {
            let symbols: Vec<u8> = set.into_iter().collect();
            Alphabet::new(&symbols).ok()
        },
    )
}

proptest! {
    #[test]
    fn p3_every_accepted_symbol_is_a_normalization_fixed_point(a in arbitrary_ascii_alphabet()) {
        for &sym in a.symbols() {
            let s = (sym as char).to_string();
            let normalized = crate::code::normalize::normalize(&s);
            prop_assert_eq!(
                normalized, s,
                "symbol {:?} (0x{:02x}) is not a normalization fixed-point; \
                 Alphabet::new accepts it but INV-4 would break if it were used",
                sym as char, sym
            );
        }
    }
}

// ── RFC-043: Alphabet::new rejects non-fixed-point symbols ─────────────────

#[test]
fn rejects_lowercase_symbol() {
    assert_eq!(
        Alphabet::new(b"Aa"),
        Err(PolicyError::AlphabetNotFixedPoint { byte: b'a' })
    );
}

#[test]
fn rejects_hyphen_symbol() {
    assert_eq!(
        Alphabet::new(b"A-"),
        Err(PolicyError::AlphabetNotFixedPoint { byte: b'-' })
    );
}

#[test]
fn rejects_ascii_whitespace_symbol() {
    // Tab: the byte RFC-041's P-3 found in an earlier run.
    assert_eq!(
        Alphabet::new(b"A\t"),
        Err(PolicyError::AlphabetNotFixedPoint { byte: b'\t' })
    );
}

#[test]
fn unambiguous_still_constructs() {
    // The regression that would matter most if the check were too strict.
    assert!(Alphabet::new(DEFAULT_ALPHABET).is_ok());
    assert_eq!(
        Alphabet::new(DEFAULT_ALPHABET).unwrap(),
        Alphabet::unambiguous()
    );
}
