//! Code normalization (RFC-003 FR-2, INV-4).
//!
//! Normalization must be **identical** on the issue path and the redeem path,
//! and **idempotent** (`normalize(normalize(x)) == normalize(x)`), or valid
//! codes fail to match their stored lookup key.
//!
//! ## Compatibility note
//!
//! Normalization strips ASCII whitespace and
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
mod tests;
