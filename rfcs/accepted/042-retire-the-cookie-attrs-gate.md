# RFC-042: Retire `cookie-attrs-present` — a Text Grep Cannot Assert Emitted Behaviour

- **Status:** Accepted
- **Target milestone:** M5
- **Primary crate(s):** `xtask`, `docs`
- **Source basis:** RFC-040 review finding C-1, 2026-09-03

## 1. Summary

The `cookie-attrs-present` release gate cannot detect the regression it exists
to prevent. Retire it, and point the invariant at the behavioural tests in
`crates/codlet/src/cookie/tests.rs`, which already prove it correctly.

## 2. The defect

`check_cookie_attrs` matches with `src.contains(attr)` over the **whole file
text, comments included**, for `HttpOnly`, `Secure`, and `SameSite`.

The real `crates/codlet/src/cookie.rs` names all three in its module
documentation and declares `enum SameSitePolicy` with variants rendering
`"SameSite=Strict"`, `"SameSite=Lax"`, `"SameSite=None"`. **Those literals alone
satisfy the gate.** If `build_set_cookie` stopped appending the attribute
tomorrow, the file would still contain every string the gate looks for, and the
gate would still report `gate ok`.

Confirmed empirically under RFC-040's self-test: given a fixture that models a
realistic regression — documentation naming all three attributes, a builder
emitting two — the gate **passes**, and `xtask self-test` correctly reports
`cookie-attrs-present did not fail against its own violation fixture`.

The other four gates are unaffected. They search for *banned* patterns and skip
comments via `is_comment()`. This is the only *presence* gate, and for a
presence check that comment-skipping protection inverts: a comment mentioning
the attribute makes the gate pass rather than fail. Their fixtures were checked
and all four model genuine violations in code.

## 3. What is actually guarding this invariant

**The invariant is not unguarded, and was never unguarded.**
`crates/codlet/src/cookie/tests.rs` asserts on the *emitted string*:

```rust
fn set_cookie_contains_required_attributes() {
    let c = p().build_set_cookie("mysecret");
    assert!(c.contains("HttpOnly"), "missing HttpOnly");
    assert!(c.contains("Secure"), "missing Secure");
    assert!(c.contains("SameSite=Strict"), "missing SameSite=Strict");
    …
}
```

with `local_development_omits_secure`, `lax_profile_uses_lax_samesite`, and
`clear_cookie_uses_max_age_zero` covering the other profiles. These test what
the builder *produces*, which is the property that matters and the one a grep
over source text can never reach.

So the security posture is sound. What is defective is a **claim**: the gate has
been reporting assurance it does not provide, for as long as it has existed.

## 4. Decision

**Retire `cookie-attrs-present`.** Remove the gate, its fixture, and its entry
from the gate table.

A text grep cannot assert emitted behaviour. Repairing it in place is not
possible without reimplementing enough of a Rust parser to know which string
literals reach the output — and the correct tool for "does this function emit
X" already exists, is already written, and already runs: the unit test.

Retiring a check is not weakening. The gate contributed nothing but a green tick,
and a green tick that cannot go red is worse than no check, because it
suppresses the question.

### 4.1 The threat model must stop citing it

`docs/src/threat-model.md` currently states:

> **Cookie leakage via JS.** Session cookies are `HttpOnly` by default. The
> `cookie-attrs-present` gate ensures this cannot be accidentally removed.

The second sentence is false. Replace it with a citation of the behavioural
tests, which are what actually ensure it.

### 4.2 Four gates, not five

Sixteen places in the repository say "five gates". **Only two of them may be
edited**, and the distinction matters more than the count:

**Live documents — correct these:**

- `SECURITY.md` — the release-discipline list, item 16.
- `xtask/src/main.rs` — the doc comment on `library_sources` describing what an
  empty corpus would do to "all five gates".

**Historical records — do not touch:**

- `CHANGELOG.md` (3 occurrences) — an accurate account of what existed at those
  releases. Rewriting it would be falsification, the same rule that has governed
  every handoff in this milestone.
- `rfcs/done/014-…` — an Implemented RFC, and a historical record.
- `rfcs/accepted/040-…` and its handoff (7 occurrences) — they describe the
  state at drafting and the work as executed. RFC-040's motivation *is* that
  five gates shared a defect; that was true when written.
- This RFC.

*(§4.2 corrected 2026-09-03 before the handoff was written. The original text
said the count "must be corrected wherever it appears", which would have
directed the implementer to rewrite CHANGELOG history and an Implemented RFC —
prohibited by every other handoff in this milestone. A count is only wrong in a
document that asserts the present.)*

## 5. Non-goals

- No change to `cookie.rs`, `CookiePolicy`, or any emitted cookie. Nothing about
  codlet's runtime behaviour changes.
- No new gate replacing this one. The tests are the replacement and they predate
  the gate.
- No change to the other four gates.
- No weakening of RFC-006 §13.2's cookie requirements.

## 6. Security considerations

The invariant — production cookies carry `HttpOnly`, `Secure`, `SameSite` — is
unchanged and remains verified, by tests that assert on emitted output and run
in CI on every push.

What changes is that the project stops claiming a second, independent guard that
does not work. Removing a false assurance improves the accuracy of the security
documentation, which for a security library is the thing being shipped.

Residual risk: the behavioural tests become the sole guard. That is acceptable —
they are the *better* guard, and they were already the effective one.

## 7. Alternatives considered

1. **Strip comments before matching.** Insufficient: `enum SameSitePolicy` and
   its variant literals are code, not comments, and would satisfy the gate on
   their own.
2. **Match only inside `build_set_cookie`.** Requires parsing Rust to find the
   function body and determine which literals reach the return value. That is a
   compiler, and we have one — it runs the tests.
3. **Keep the gate as defence in depth.** Rejected: it provides no defence. A
   check that cannot fail is not a weak layer, it is a decorative one, and this
   milestone exists because decorative checks concealed real defects for two
   releases.

## 8. Open questions

None. The behavioural tests already exist and pass; this RFC removes something
and corrects documentation.

## 9. Acceptance criteria

- `cookie-attrs-present` removed from `xtask`, along with its fixture.
- `cargo run -p xtask -- self-test` green with four gates.
- `cargo run -p xtask -- release-check` green with four gates.
- `docs/src/threat-model.md` cites the behavioural tests, not the gate.
- `SECURITY.md` and the `library_sources` doc comment say four; no historical
  record edited.
- No change to any emitted cookie, and `cookie/tests.rs` still passes unmodified.
