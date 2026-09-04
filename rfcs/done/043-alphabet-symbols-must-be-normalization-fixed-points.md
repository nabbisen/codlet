# RFC-043: `Alphabet::new` Must Reject Symbols That Normalization Would Alter

- **Status:** Implemented (v0.19.0)
- **Target milestone:** M5 (scope addition — owner decision required)
- **Primary crate(s):** `codlet`
- **Source basis:** RFC-041 property P-3, which fired against real `Alphabet::new`

## 1. Summary

`Alphabet::new` accepts symbols that `normalize` would change. Any code
generated from such an alphabet would fail to match itself on the redeem path,
violating INV-4. Reject those symbols at construction.

## 2. The finding

RFC-041's P-3 fires against unmodified production code:

```
symbol 'a' (0x61) is not a normalization fixed-point;
Alphabet::new accepts it but INV-4 would break if it were used
minimal failing input: Alphabet { symbols: [0, 97] }
```

`Alphabet::new` validates length ≥ 2, all-ASCII, and byte uniqueness. It does
**not** require `normalize(symbol) == symbol`. `normalize` strips ASCII
whitespace and `-` and uppercases ASCII letters, so every lowercase letter, `-`,
and ASCII whitespace byte passes validation while being unable to survive a
round trip.

**Latent, not live.** `Alphabet::unambiguous()` — the only constructor shipped
code calls — uses `DEFAULT_ALPHABET`, which is entirely safe. This is about what
the public API permits a caller to build, not about anything codlet does today.

**Consequence if used.** `issue_code` stores the lookup key for the
un-normalized generated code; the redeem path normalizes user input before
hashing. The keys never match. Every code issued under such a policy would be
permanently unredeemable — an availability failure, loud and total.

## 3. Decision

`Alphabet::new` rejects any symbol that is not a normalization fixed-point,
with a distinct error variant naming the offending byte.

### 3.1 Why not normalize on the issue path

The alternative — normalizing the generated code before deriving its lookup key
— also makes INV-4 hold, and is worse.

`Alphabet::new`'s uniqueness check is over **raw bytes**. An alphabet containing
both `'a'` and `'A'` passes it as two distinct symbols. Normalizing on issue
would then map both to `'A'`: the alphabet's declared length would overstate its
effective symbol count, and every entropy calculation built on it would be
wrong. Code entropy is `len^length`; `docs/src/threat-model.md` claims ~39.6 bits
for the default policy on exactly that arithmetic.

That option **converts a loud failure into a silent one** — from codes that
visibly cannot be redeemed, to codes that work with less entropy than the
documentation promises. For a security library that is the strictly worse trade,
and it is the whole argument.

Constraining the constructor fails closed at the point of misuse and keeps the
entropy accounting sound. It also makes normalized uniqueness follow for free:
fixed-point symbols that are unique as raw bytes are unique after normalization.

### 3.2 Compatibility

A breaking change in the sense that `Alphabet::new` rejects input it previously
accepted. Acceptable pre-v1, and **every rejected input was already broken** —
no caller can have a working deployment that this breaks. `Alphabet::unambiguous()`
and `DEFAULT_ALPHABET` are unaffected.

## 4. Non-goals

- No change to `normalize`, `generate_code`, `issue_code`, or the redeem path.
- No change to `DEFAULT_ALPHABET` or `Alphabet::unambiguous()`.
- No change to INV-4's wording. This makes the invariant structurally
  guaranteed rather than assumed.
- No widening of `normalize`'s behaviour to accommodate more symbols.

## 5. Testing

- **P-3 is un-`#[ignore]`d and must pass.** It is the acceptance test; it was
  written before the fix and fired against the real defect, which is the
  strongest evidence a test can carry.
- A unit test per rejected class: lowercase letter, `-`, ASCII whitespace.
- A test that `Alphabet::unambiguous()` still constructs successfully.
- After this, `docs/src/threat-model.md`'s INV-4 row carries no open gap.

## 6. Security considerations

Strengthens INV-4 from an invariant that holds by convention — because the
default alphabet happens to be safe — to one enforced at construction.

No shipped behaviour changes; no consumer on `Alphabet::unambiguous()` is
affected.

## 7. Alternatives considered

1. **Normalize on the issue path.** Rejected — §3.1. Trades a loud failure for a
   quiet entropy loss.
2. **Document the constraint without enforcing it.** Rejected: the constraint is
   machine-checkable at construction, and this milestone's evidence is that
   documented-but-unenforced invariants are the ones that break.
3. **Leave it — no shipped code is affected.** Rejected: it is public API. A
   constructor that accepts a configuration guaranteeing total failure is a
   defect regardless of whether anyone has hit it yet.

## 8. Scope — resolved

~~Open question: fold into M5, or defer?~~ **Resolved: accepted into M5 by the
owner, 2026-09-04.** M5 does not close carrying an open INV-4 gap. Recorded in
`ROADMAP.md` as M5-6.

## 9. Acceptance criteria

- `Alphabet::new` rejects non-fixed-point symbols with a distinct, named error.
- P-3 un-`#[ignore]`d and passing.
- Per-class rejection tests present.
- `Alphabet::unambiguous()` unaffected; full suite green.
- INV-4's threat-model row carries no open gap.
