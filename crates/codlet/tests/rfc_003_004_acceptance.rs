//! Cross-cutting acceptance tests for RFC-003 and RFC-004.
//!
//! These complement the unit tests inside each module:
//! - a deterministic pseudo-random sweep over Unicode for normalization
//!   idempotence and no-panic (RFC-003 §11.5);
//! - frozen HMAC lookup-key vectors per domain (RFC-004 §12.3) so any future
//!   refactor or adapter must reproduce the exact same bytes.

use codlet::code::normalize;
use codlet::hashing::{SecretDomain, SecretHasher, StaticKeyProvider};

/// Small xorshift so the sweep is reproducible without adding a dev-dependency.
struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[test]
fn normalization_is_idempotent_and_total_over_unicode() {
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        // Build a short random string from arbitrary scalar values, including
        // whitespace, hyphens, ASCII letters, and assorted Unicode.
        let len = (rng.next() % 6) as usize;
        let mut s = String::new();
        for _ in 0..len {
            let pick = rng.next();
            let b = (pick >> 8) as u8;
            let ch = match pick % 8 {
                0 => ' ',
                1 => '-',
                2 => '\t',
                3 => char::from(b'a' + b % 26),
                4 => char::from(b'A' + b % 26),
                5 => char::from(b'0' + b % 10),
                _ => char::from_u32((pick % 0x11_0000) as u32).unwrap_or('x'),
            };
            s.push(ch);
        }
        let once = normalize(&s);
        let twice = normalize(&once);
        assert_eq!(once, twice, "normalize not idempotent for {s:?}");
        // No separators or lowercase ASCII survive.
        assert!(!once.contains(' ') && !once.contains('-') && !once.contains('\t'));
        assert!(!once.bytes().any(|b| b.is_ascii_lowercase()));
    }
}

// ── RFC-004 §12.3 frozen test vectors ────────────────────────────────────────
//
// Fixed (non-production) key and inputs. If any of these change, a stored
// lookup key written by an older codlet would stop matching — a breaking change
// requiring a key-version migration. The expected hex is computed by the
// reference implementation and frozen here.

const TEST_KEY: &[u8] = b"codlet-test-vector-key-v1";
const TEST_SECRET: &str = "ABCD2345";

fn vector_hasher() -> SecretHasher<StaticKeyProvider> {
    SecretHasher::new(StaticKeyProvider::single("test-v1", TEST_KEY.to_vec()).unwrap())
}

#[test]
fn print_reference_vectors() {
    // Not an assertion: emits the vectors so they can be cross-checked against
    // other adapters/languages. Run with `--nocapture` to view.
    let h = vector_hasher();
    for d in [
        SecretDomain::Code,
        SecretDomain::Session,
        SecretDomain::FormToken,
        SecretDomain::FlowTicket,
    ] {
        let (lk, ver) = h.lookup_key(d, TEST_SECRET).unwrap();
        println!(
            "domain={:<11} version={} lookup={}",
            d.label(),
            ver,
            lk.as_str()
        );
    }
}

#[test]
fn vectors_are_stable_and_domain_separated() {
    let h = vector_hasher();
    let code = h.lookup_key(SecretDomain::Code, TEST_SECRET).unwrap().0;
    let session = h.lookup_key(SecretDomain::Session, TEST_SECRET).unwrap().0;
    let form = h
        .lookup_key(SecretDomain::FormToken, TEST_SECRET)
        .unwrap()
        .0;
    let flow = h
        .lookup_key(SecretDomain::FlowTicket, TEST_SECRET)
        .unwrap()
        .0;

    // Frozen expected outputs (HMAC-SHA256, prefixing scheme, lowercase hex).
    assert_eq!(code.as_str(), EXPECT_CODE);
    assert_eq!(session.as_str(), EXPECT_SESSION);
    assert_eq!(form.as_str(), EXPECT_FORM);
    assert_eq!(flow.as_str(), EXPECT_FLOW);

    // Each is a 64-char lowercase hex string.
    for v in [&code, &session, &form, &flow] {
        assert_eq!(v.as_str().len(), 64);
        assert!(
            v.as_str()
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }
}

// These constants are filled in from the reference run below; see the
// build step that captures `print_reference_vectors` output.
const EXPECT_CODE: &str = "e6d6e47e5d9bb72c61d20a7e90a42e6176994a9ab06d9c656ca5f349c631a922";
const EXPECT_SESSION: &str = "9d8e94501624dd08a784b87f687e0286142cc4d99a1a7d5b987e75a859f3add4";
const EXPECT_FORM: &str = "553e2afa84e4b3bfacd25ba15b9c006683fb6d24d7bfa889c7a61bc9496e1c34";
const EXPECT_FLOW: &str = "4bdb2ede4a6c1472aeffcd69f92c68d4dcf6feff31a5497a2ec5e06d78c90705";
