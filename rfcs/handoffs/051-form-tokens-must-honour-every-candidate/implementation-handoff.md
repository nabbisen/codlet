# Developer Handoff — RFC-051: Form Tokens Must Honour Every Lookup-Key Candidate

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-17
- **RFC:** [`../../accepted/051-form-tokens-must-honour-every-candidate.md`](../../accepted/051-form-tokens-must-honour-every-candidate.md)
- **Baseline:** `3767721`
- **Credit:** you found this reading adjacent code during the RFC-016 sweep. It was the most valuable thing in either review request.

## 1. Purpose

Make the four `FormTokenStore` adapters try every lookup-key candidate, as
their trait has always said they do, so form tokens survive a key rotation.

## 2. Change scope — indicative, not authoritative

Eight method bodies:

| File | Methods |
|---|---|
| `crates/codlet/src/mem/token.rs` | `consume_form_token` (~line 53), `set_token_result` (~line 130) |
| `crates/codlet-sqlx/src/token.rs` | both (~32, ~110) |
| `crates/codlet-sqlx/src/postgres/token.rs` | both (~39, ~116) |
| `crates/codlet-worker/src/d1/token.rs` | both (~80, ~159) |

Plus `crates/codlet-conformance/src/token.rs` for §5. §7 is binding.

## 3. The fix

Replace, in all eight:

```rust
let lookup_key = candidates.first().expect("at least one candidate");
```

**SQL and D1 adapters** — loop over candidates in order and return on the first
match, exactly as `codlet-sqlx/src/session.rs:22` already does for sessions.
Use that function as the shape to copy; it is the in-tree precedent and it is
correct.

**The in-memory adapter** — `mem/token.rs` scans rows in one pass, so match a
row when *any* candidate equals its `lookup_key`:
`candidates.iter().any(|c| row.lookup_key.ct_eq(c))`. Keep using `ct_eq`; do
not substitute `==`.

Equivalent to "in order" here because one token yields one row, so at most one
candidate can match. If you find a case where two rows could match different
candidates for the same token, stop and report — that would be a different bug.

**Preserve everything else in those bodies**: the conditional-consume logic, the
`bound_resource` exact-match semantics including the `None`/`Some` asymmetry,
the `changed > 1` → `StoreError::InvariantViolation` check, the follow-up
classification, and `classify_token_consume`. The only thing changing is which
keys are compared.

## 4. Empty candidates must not panic

An empty slice yields no match: `consume_form_token` classifies as it would for
an unknown token, `set_token_result` updates nothing and returns `Ok(())`. No
`expect`, no `unwrap`, no panic.

Iterating naturally gives this for free — zero iterations, no match — which is
why no explicit empty check should be needed. If you find yourself adding one,
the loop is in the wrong place.

This matches what the code and session lookups already do. Do **not** introduce
a non-empty candidate type; RFC-051 §5.2 records why that was considered and
rejected.

## 5. Conformance — this is the part that stops it recurring

The defect survived because no test passes more than one candidate. Add to
`crates/codlet-conformance/src/token.rs`:

1. Insert a token under a `v1` lookup key; call `consume_form_token` with
   candidates ordered `[v2_key, v1_key]` — must return `Proceed`. Order matters:
   put the non-matching key first, because that is the real post-rotation
   ordering and the only ordering that fails today.
2. The same arrangement for `set_token_result`.
3. Both must run for all four adapters through the shared suite, not just one.

Build the candidates the way the manager does — two distinct `LookupKey`s where
only the second matches the stored row. You do not need a real second key; a
syntactically valid non-matching key is enough and keeps the test independent of
key management.

**Then prove the tests detect the defect** (RFC-047's inverted-assertion
standard): put `.first()` back in one adapter, confirm the new tests fail, note
which assertion fired, revert. A test that passes against both the fixed and the
broken adapter is worthless, and you have no way to know which you wrote without
trying it. You did exactly this for RFC-050 criterion 6 — same thing here, and
this time keep the failure text.

## 6. Non-change scope

- No change to candidate ordering (active first, then previous).
- No change to codes or sessions — already correct, verified.
- No change to `classify_token_consume`, token TTLs, or `key-rotation.md`'s
  procedure, which becomes true once this lands.
- No change to the `FormTokenStore` trait signature or its rustdoc. The rustdoc
  is already right; that is the whole point.
- Do not "improve" the `bound_resource` semantics while you are in there.

## 7. Required verification

| Check | Expectation |
|---|---|
| `cargo test --workspace` | green |
| `cargo test -p codlet-sqlx --no-default-features --features postgres-test` | green (needs Docker; if unavailable say so and rely on CI) |
| `cargo check -p codlet-worker --target wasm32-unknown-unknown` | clean |
| `grep -rn 'candidates.first()' crates/` | no hits |
| `grep -rn "expect(\"at least one candidate\")" crates/` | no hits |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| the §5 revert trial | new tests fail against `.first()`, failure text quoted |

## 8. Acceptance criteria

Binding.

1. All eight implementations iterate every candidate in order.
2. No `.first()`/`.expect()` on `candidates` anywhere in the adapters; an empty
   slice yields no match and no panic.
3. The §5 tests exist in the shared suite and pass on all four adapters.
4. The §5 revert trial was performed and its failure output is quoted.
5. Codes and sessions unchanged.
6. No behavioural change to consume semantics other than which keys match —
   if a test assertion unrelated to candidate matching had to change, stop and
   report.
7. All blocking CI jobs green, PostgreSQL and D1 included.

## 9. Required evidence

`git diff --stat`; each §7 check with exit status; the two new conformance tests
quoted in full; the §5 failure output; CI run URL.

## 10. Review request

`.git-exclude/review-request/051-form-token-candidates.md`; result returns at
`.git-exclude/reviewed/051-form-token-candidates.md`.
