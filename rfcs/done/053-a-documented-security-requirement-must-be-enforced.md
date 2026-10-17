# RFC-053: A Documented Security Requirement Must Be Enforced, Not Described

- **Status:** Implemented (Unreleased) — reviewed and approved 2026-10-18
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet`, `docs/`
- **Source basis:** RFC-016 accuracy sweep review request §5 items 2 and 3; verified independently 2026-10-17

## 1. Summary

Two security requirements exist only as prose. One of them is also **wrong**,
and it tells readers they may switch off the defence the threat model calls
mandatory.

## 2. Finding 1: the entropy claim is wrong, and it is wrong in the dangerous direction

`docs/src/adapter-matrix-and-config.md:78-80`, under the heading **Secure
Configuration Guide**:

> `CodePolicy::default_human` … produces 8-symbol codes over the 31-symbol
> unambiguous alphabet (~39.6 bits of entropy) and **does not require rate
> limiting to be safe — the entropy is sufficient.**

The arithmetic is right: 31⁸ ≈ 8.53 × 10¹¹ ≈ 2³⁹·⁶. The security conclusion
does not follow from it.

39.6 bits is the cost of guessing **one particular** code. An attacker does not
have to. The endpoint accepts any valid code, so the relevant probability scales
with the number of codes outstanding:

P(at least one hit) ≈ N × G / 31⁸, for N codes outstanding and G guesses.

| Outstanding codes | Guesses | Rate needed | λ = N·G/31⁸ | P(≥1 hit) = 1 − e^−λ |
|---|---|---|---|---|
| 10,000 | 10⁶ | ~31/s for 9 h | 0.0117 | **1.2 %** |
| 100,000 | 10⁷ | ~309/s for 9 h | 1.1725 | **69 %** |

**Corrected 2026-10-18.** This table first read "effectively certain" for the
second row, from treating λ > 1 as a probability. λ is an expected count; the
probability is 1 − e^−λ = 69%. The linear form N·G/31⁸ is usable only while it
is small. 69% is not a mitigation — a two-in-three chance of consuming a
stranger's code is a successful attack — so the conclusion is unchanged.

A 24-hour TTL — `default_human`'s own documented example — is what makes N
large. And a hit is not nothing: `claim_code` binds the code to the *claiming*
subject, so guessing a stranger's code means consuming their grant. For a
reset-style or invite-style flow that is account takeover or unauthorised
access to whatever the grant confers.

`docs/src/threat-model.md` lists mandatory rate limiting as the defence against
online guessing. The Secure Configuration Guide tells the reader they can skip
it. **The threat model is right.**

This is not a tension between two documents to be split down the middle. One of
them is incorrect.

### 2.1 It is not merely theoretical: the API makes skipping easy

`CodeAuth::without_rate_limit` (`auth/code.rs:451`) sets
`rate_limit_policy: None`, and the generic parameter documents `()` as a way to
opt out. A reader who takes the guide at its word has a one-call, no-warning
path to the configuration the threat model forbids — while the 6-symbol policy,
which is *more* obviously dangerous, is `#[deprecated]` so that the compiler
argues with you.

The friction is attached to the wrong thing. 6-symbol codes get a compiler
warning; switching off rate limiting entirely gets an encouraging sentence.

## 3. Finding 2: two different key-length minimums, neither enforced

| Source | Says |
|---|---|
| `docs/src/threat-model.md:97` | key material "must be ≥ 16 bytes" |
| `docs/src/adapter-matrix-and-config.md:117` | "at least 32 bytes (256 bits) of random material" |
| `StaticKeyProvider::new` (`hashing.rs:184`) | rejects only an **empty** key |

A one-byte key is accepted today. Both documents state a minimum; neither is
true, and they disagree about what the untrue minimum is. A reader who checks
the code finds no minimum at all.

## 4. Decision

### 4.1 Rate limiting is required; say so once, in both places

Delete the claim. Replace it with the actual reasoning — that per-code entropy
is not the relevant quantity when the attacker may guess against the whole
outstanding population — and state that a `RateLimitStore` is required for
every code policy, not only short ones.

Keep the 6-symbol paragraph's stronger framing; it was never the problem.

Then reconcile: `threat-model.md`'s mandatory framing becomes the single
statement, and the configuration guide cites it instead of contradicting it.

### 4.2 `without_rate_limit` carries the friction it deserves

Mark `CodeAuth::without_rate_limit` `#[deprecated]` with a note pointing at the
threat model, exactly as `CodePolicy::six_symbol` is marked. The compiler
warning *is* the mechanism — RFC-003 already established that pattern in this
codebase, and this is a better place for it than the one it currently guards.

Deliberately **not** removed. Tests and single-process examples legitimately opt
out, and removing the constructor would push hosts toward hand-rolling a
no-op `RateLimitStore`, which is the same risk with none of the visibility. A
warning at every call site is the right amount of friction: passable, but not
silently.

### 4.3 Enforce 32 bytes where keys enter

`StaticKeyProvider::new` rejects any key — active or previous — shorter than
32 bytes, with an error that states the requirement rather than only the
rejection.

32, not 16: it matches HMAC-SHA256's output length, it is what the
configuration guide already told readers to do, and picking the stricter of two
documented numbers cannot invalidate a key someone chose by following the docs.
`threat-model.md:97` is corrected from 16 to 32.

**Scope of the guarantee, stated honestly.** This covers `StaticKeyProvider`,
the provider the documentation documents. A host implementing `KeyProvider`
itself can still supply a short key, because `SecretHasher::new`
(`hashing.rs:263`) is infallible and validating there would make every host's
setup fallible for a case the built-in provider already prevents. So the
minimum goes into the `KeyProvider` trait docs as a contract for custom
implementations, and the residual gap is recorded here rather than papered over.
Revisit if a custom provider ever ships in-tree.

### 4.4 The examples must stop teaching the opt-out

Three of the five example packages call `CodeAuth::without_rate_limit`:
`examples/axum_login_logout`, `examples/sqlite_file`,
`examples/sqlite_quickstart`. Examples are the most-copied code a library
publishes, so the guide currently says "rate limiting is optional", the threat
model says it is mandatory, and the examples demonstrate going without.

`MemRateLimitStore` already exists (`crates/codlet/src/mem.rs:15`), so switching
them costs a constructor argument and a `RateLimitPolicy` — no new dependency,
no architectural change.

Switch all three to a real rate limiter. Where an example keeps the opt-out for
brevity it must say in a comment that it is not a production configuration and
point at the threat model — but the default for an example is the secure path.

This also resolves the `#[deprecated]` fallout from §4.2: after this change the
only in-tree `#[allow(deprecated)]` sites are the three tests that deliberately
exercise the opt-out, which is where that annotation belongs.

## 5. Non-goals

- No change to `DEFAULT_ALPHABET`, to the 8-symbol default length, or to any
  TTL default. The defaults are fine; the claim about them was not.
- No change to `RateLimitStore`, `RateLimitPolicy`, or the eventual-consistency
  caveat for `KvRateLimitStore` (RFC-010 §12.3).
- `without_rate_limit` is not removed — §4.2.
- No new validation in `SecretHasher::new` — §4.3.

## 6. Security considerations

§4.1 is the substantive change: the project currently ships a "Secure
Configuration Guide" that recommends an insecure configuration. Of everything
found in this sweep, it is the item most likely to have caused real harm,
because it reads as considered advice and the arithmetic in it is correct.

§4.3 converts two false statements into one enforced one. §4.2 moves existing
friction onto the decision that actually warrants it.

## 7. Acceptance criteria

1. No document claims any code policy is safe without rate limiting.
2. `threat-model.md` and `adapter-matrix-and-config.md` agree, with the threat
   model as the single statement of the requirement.
3. `CodeAuth::without_rate_limit` is `#[deprecated]` and the workspace still
   builds warning-free (`-D warnings`). The only `#[allow(deprecated)]` sites
   are tests that deliberately exercise the opt-out.
4. All three examples named in §4.4 use a real `RateLimitStore`.
5. `StaticKeyProvider::new` rejects any key under 32 bytes, active or previous,
   with a test for each position.
6. `threat-model.md` says 32 bytes; the `KeyProvider` trait docs state the
   minimum as a contract for custom implementations.
7. Full blocking CI green.
