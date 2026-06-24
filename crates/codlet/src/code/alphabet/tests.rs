//! Unit tests for the `alphabet` module.
use super::*;

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
