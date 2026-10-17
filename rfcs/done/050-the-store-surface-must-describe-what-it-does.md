# RFC-050: The Store Surface Must Describe What It Does

- **Status:** Implemented (Unreleased) — `c3664cc`, `d2f84ee`; reviewed and approved 2026-10-17
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet`, all four adapters, `codlet-conformance`
- **Source basis:** documentation audit 2026-10-17; RFC-047; RFC-013 §10.1

## 1. Summary

RFC-047 moved expiry, revocation and use decisions out of the store and into the
classifiers. It did not change the store's **names**, its **types**, or its
**signatures**, all of which still describe the filtering behaviour that was
removed.

The rustdoc was updated everywhere and is correct. That is why nothing broke —
and why this went unnoticed. A surface that is documented-correct and
self-contradicting is the failure mode this RFC exists to close.

## 2. The complete finding

**Four names that are now false:**

| Name | What it says | What it does |
|---|---|---|
| `CodeStore::find_redeemable` | finds redeemable codes | returns records "regardless of use, expiry, or revocation state" |
| `RedeemableCode` | a redeemable code | carries `used_at` and `revoked_at`; may be neither |
| `SessionStore::find_active_session` | finds active sessions | returns records "regardless of expiry or revocation state" |
| `ActiveSessionRecord` | an active session | carries `revoked_at`; may be expired or revoked |

**Two dead parameters.** Both lookups still take `now: u64`. It is referenced by
**zero** adapter implementations — verified across in-memory, SQLite, PostgreSQL
and D1. Its stated justification is "symmetry with other store methods and
possible future use".

There is no such future use. RFC-047 §8.2 settled that adapters do **not**
filter, and both classifiers already take `now`. The parameter cannot acquire a
purpose without reversing an accepted decision.

**One positional footgun.** `classify_code_lookup` takes four loose scalars:

```rust
pub fn classify_code_lookup(
    revoked_at: Option<u64>,
    used_at: Option<u64>,
    expires_at: u64,
    now: u64,
) -> CodeLookupOutcome
```

`revoked_at` and `used_at` are both `Option<u64>` and adjacent. **Transposing
them compiles silently and misclassifies**, and after RFC-047 that reason feeds
the audit log and the rate limiter. Its sibling `classify_session` already takes
the record instead, so the two classifiers do not teach the same lesson.

## 3. Why this is a safety matter, not a naming preference

RFC-013 §10.1 offers four entry points, and the second is:

> Store service layer (`store::*` traits): custom routing and special flows.

So the store traits are a **documented, supported** surface for hosts writing
their own flows. A host on that layer reads the names, the types and the
signature — not the rustdoc paragraph — and writes:

```rust
if store.find_active_session(&candidates, now).await?.is_some() {
    // authenticated
}
```

That authenticates expired and revoked sessions. The dead `now` parameter makes
it worse than a bare misnomer: passing a timestamp to a lookup is a promise that
time was considered.

**This is not hypothetical. We made the mistake ourselves.** The audit found
exactly that code in `docs/src/migration-from-an-existing-service.md`, in the
path it calls "recommended for sessions" — written by someone reading a surface
that was, at the time, telling the truth.

A name whose own documentation has to warn you against it has already failed.

## 4. Decision

### 4.1 Rename to what the operations actually are

| From | To |
|---|---|
| `find_redeemable` | `lookup_code` |
| `RedeemableCode` | `StoredCode` |
| `find_active_session` | `lookup_session` |
| `ActiveSessionRecord` | `StoredSession` |

`lookup_*` rather than `find_*`: the parameter is already
`candidates: &[LookupKey]`, and "lookup" is purely mechanical — it carries no
implication of validity, which is precisely the implication that must go. `find`
invites the question "find *what*?", and the honest answer is a record, not a
verdict.

`Stored*` rather than `*State` or `*Row`: it pairs with the existing write-side
`CodeRecord` / `SessionRecord` — *Record in, Stored out* — and says plainly that
this is what storage holds, with no claim about whether it is usable.

### 4.2 Drop `now` from both lookups

The store becomes time-independent. All time-dependence lives in
`classify_code_lookup` and `classify_session`, which already take `now`.

This is the part worth more than the renames: **the architectural boundary
RFC-047 established becomes visible in the types.** A trait that takes no clock
cannot make a time-based decision, and a reader can see that without reading a
word of prose. The separation stops being a convention and becomes a property.

### 4.3 `classify_code_lookup` takes the record

```rust
pub fn classify_code_lookup(record: &StoredCode, now: u64) -> CodeLookupOutcome
```

Removes the transposition footgun and makes the two classifiers symmetric, so
learning one transfers to the other.

## 5. Non-goals

- **No behavioural change whatsoever.** Every decision is made in the same place
  by the same logic. This RFC changes only what the surface says about itself.
- No change to `classify_session`'s semantics or to the decision order
  (revoked → expired → used/idle), fixed by RFC-047 §8.1.
- No change to `claim_code`'s conditional UPDATE — still the INV-5 enforcement
  point, and `lookup_code`'s state fields remain a pre-filter, not a substitute.
- No change to `insert_*`, `revoke_*`, or `touch_session`.
- No change to the public error types or the `SessionFailure` surface.

## 6. Compatibility

Breaking: four public names and three signatures. Mechanical and
compiler-caught — there is no silent-failure path.

Cheap now and expensive later: codlet is not in production, breaking changes are
accepted (owner, 2026-09-06), and **M7 freezes this surface**. After the freeze a
name that contradicts its own documentation is permanent.

## 7. Security considerations

No invariant changes. INV-5's guard is `claim_code`'s conditional UPDATE, which
this does not touch; INV-7 and INV-9's type-level proofs are untouched.

The security value is entirely in §3: a supported API layer stops teaching
callers a false model. The project has one documented instance of that model
causing a wrong authentication decision, authored in-house, which is the
strongest evidence available that the surface — not the reader — was at fault.

## 8. Alternatives considered

1. **Do nothing; the rustdoc is correct.** Rejected. It requires every reader to
   read a paragraph that contradicts the identifier they just typed, and the
   project has already demonstrated that readers do not.
2. **Rename only the session side.** Rejected — that was the audit's initial
   scope and it was incomplete. The code path has the identical defect, and
   fixing half would leave the two paths inconsistent, which is its own
   confusion.
3. **Keep `now` for a future filtering adapter.** Rejected: RFC-047 §8.2
   decided adapters must not filter. The parameter cannot be used without
   reversing that.
4. **Defer past M7.** Rejected: the freeze is the deadline, not a reason to
   wait.

## 9. Open question

Should `CodeLookupOutcome` and `SessionValidationOutcome` converge on a shared
shape? They answer the same kind of question for different entities and
currently differ in structure. **Recommendation: not now** — it is a larger
design question, and this RFC's value is that it changes no behaviour. Worth
revisiting during M7's API audit, with this RFC's symmetry work as the
starting point.

## 10. Acceptance criteria

- The four renames applied across `codlet`, all four adapters,
  `codlet-conformance`, tests, examples and docs.
- `now` removed from both lookup signatures; no adapter takes a clock.
- `classify_code_lookup` takes `&StoredCode`.
- **No behavioural change**, demonstrated: the full suite passes with no test
  assertion altered except for renamed identifiers.
- Conformance green on all four adapters, PostgreSQL and D1 included.
- `docs/src/migration-from-an-existing-service.md`'s example uses the new names
  and classifies — coordinated with the RFC-016 accuracy sweep so the fix is
  not written twice.
- No remaining occurrence of the four old names outside `rfcs/done/` and
  `CHANGELOG.md` history.
