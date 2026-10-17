# RFC-051: Form Tokens Must Honour Every Lookup-Key Candidate

- **Status:** Implemented (Unreleased) — reviewed and approved 2026-10-18
- **Target milestone:** M7
- **Primary crate(s):** `codlet`, `codlet-sqlx`, `codlet-worker`, `codlet-conformance`
- **Source basis:** RFC-016 accuracy sweep review request §5 item 6; verified independently 2026-10-17

## 1. Summary

All four `FormTokenStore` adapters consume only the **first** lookup-key
candidate. Three separate places promise they try all of them. Because
candidates are ordered active-key-first, a form token issued under the previous
key is unreachable the moment a key is rotated, so the rotation grace period
documented for form tokens does not exist.

## 2. Evidence

Eight call sites — `consume_form_token` and `set_token_result` in each of
`mem/token.rs`, `codlet-sqlx/src/token.rs`, `postgres/token.rs`, `d1/token.rs`
— are all:

```rust
let lookup_key = candidates.first().expect("at least one candidate");
```

Against three written contracts:

| Location | Promise |
|---|---|
| `store/token.rs:129` | "Try each candidate lookup key in order; consume on the first match. Accepting multiple candidates supports rotation grace periods (RFC-A)." |
| `auth/token.rs:120` | "Derive one candidate per held key (RFC-A) so tokens written under previous keys remain consumable during the rotation grace period." |
| `hashing.rs:288` | "active first, then previous … so that records written under any held key are reachable during the rotation grace period." |

`docs/src/key-rotation.md` step 4 tells operators to wait out the maximum TTL
— "1 hour for form tokens" — before removing the old key. For form tokens that
wait accomplishes nothing: the tokens were already unusable.

**Codes and sessions are correct.** `mem/session.rs:46` uses
`candidates.iter().any(...)`; `codlet-sqlx/src/session.rs:22` loops over
candidates. The defect is confined to form tokens, which bounds it.

## 3. Impact

Fails **closed**: a form submission carrying a previous-key token is rejected,
not accepted. No authentication bypass, no data exposure.

The cost is availability and correctness during a rotation: every in-flight form
submission fails for up to the token TTL (1 hour by default), and the operator
following the documented procedure has no reason to expect it. Under a
compromise-driven emergency rotation — exactly when the procedure is run under
pressure — every outstanding form submission breaks silently.

## 4. Why this slipped

The same cause as RFC-047/050, with the polarity reversed: there, the behaviour
changed and the words stayed; here, the words were always right and the
behaviour never matched them.

**`codlet-conformance` has no test that passes more than one candidate.** The
suite proves single-candidate behaviour for every adapter and is silent on the
multi-candidate contract the trait advertises. A gate that cannot observe a
property does not guard it — the finding of RFC-036, in a new place.

## 5. Decision

### 5.1 Every candidate, in order

Both methods in all four adapters iterate `candidates` in order and act on the
first match, as `store/token.rs:129` already says. For the SQL adapters this
mirrors `session.rs`'s existing loop; no new query shape is required.

### 5.2 An empty candidate list is "no match", not a panic

Replace `.first().expect(...)` with iteration that naturally yields no match on
an empty slice: `consume_form_token` classifies as it would for an unknown
token, `set_token_result` updates nothing and returns `Ok(())`.

Two reasons. First, it is what the code and session lookups already do — zero
iterations, `Ok(None)` — so this makes the three traits consistent rather than
introducing a third behaviour. Second, RFC-013 §10.1 offers the store layer to
hosts, so this is a public surface that currently panics on a legal input, and
on `wasm32` a panic takes the Worker down.

**Rejected: a non-empty candidate type.** A `LookupKeys` newtype guaranteeing
non-emptiness would make the panic unrepresentable, and it was tempting. But
`all_hmac_keys` already guarantees at least the active key (`hashing.rs:158`),
so the supported flow path cannot produce an empty slice; the only caller who
can is a host reaching the store layer directly, and for that caller "no
candidates match nothing" is the obvious, unsurprising semantics. Adding a
wrapper type to three trait signatures — on a surface RFC-050 has just
simplified and M7 is about to freeze — would cost every host a constructor call
to prevent a case that cannot arise. Consistency across the three traits is the
cleaner answer, and it needs no new type.

### 5.3 Conformance must be able to see it

Add to `codlet-conformance`'s token suite:

1. A token inserted under key `v1`, then looked up with candidates ordered
   `[v2, v1]` — must consume. This is the test whose absence hid the defect.
2. The same for `set_token_result`.
3. The negative framing RFC-047 established: these must **fail** against a
   first-candidate-only adapter, not merely pass against a fixed one. Verify by
   temporarily reintroducing `.first()` and observing failure, and record what
   failed.

## 6. Non-goals

- No change to the rotation key order (active first, then previous).
- No change to `consume_form_token`'s atomicity, its conditional UPDATE, or
  `classify_token_consume`.
- No change to the code or session lookups — they are already correct.
- No change to token TTL defaults or `key-rotation.md`'s procedure, which
  becomes true once the code matches it.

## 7. Security considerations

No invariant changes; no guard is weakened. The change makes a documented
rotation guarantee real and removes a panic from a public surface.

Worth stating plainly: this was found by the dev team reading adjacent code
during a documentation sweep, not by any gate. The conformance addition in §5.3
is the part that stops it recurring.

## 8. Acceptance criteria

1. All eight implementations iterate every candidate in order.
2. No `.expect(...)` on `candidates` remains in any adapter; an empty slice
   yields no match and no panic.
3. The three conformance tests in §5.3 exist and pass on all four adapters.
4. §5.3's negative check performed and its failure output recorded.
5. No change to codes or sessions.
6. Full blocking CI green, PostgreSQL and D1 included.
