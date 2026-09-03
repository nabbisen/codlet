# RFC-041: Property and Distribution Testing — and Guarding INV-4

- **Status:** Accepted
- **Target milestone:** M5
- **Primary crate(s):** `codlet`
- **Source basis:** `ROADMAP.md` M5-2/M5-3/M5-4; RFC-015 (partial); RFC-040 §3.3 (INV-4 left open)

## 1. Summary

Close the last open row of the invariant map — **INV-4, normalization is
identical on the issue and redeem paths and idempotent** — with property tests,
and replace the example-based coverage of code generation with exact
combinatorial assertions.

Recommends **not** adding a coverage-guided fuzzing harness, with reasons. That
is a scope reduction against the approved M5-4 and needs the owner's decision.

## 2. Motivation

RFC-040 mapped every invariant to a guard and a negative test except INV-4,
whose row reads *"(none yet) — deferred to RFC-041"*. It is the only invariant
in the threat model with no guard at all.

`normalize` has five example-based tests. They are good examples, and examples
cannot establish idempotence — a property quantified over all inputs.

### 2.1 The half of INV-4 nobody is testing

INV-4 has two halves. Idempotence is the visible one. The other is *"identical
on the issue path and the redeem path"*, and it currently rests on an
unverified assumption.

`CodeAuth::issue_code` does:

```rust
let normalized = plain.expose().to_string(); // already in canonical form
```

That comment is load-bearing and unchecked. It is true today because
`DEFAULT_ALPHABET` is `b"ABCDEFGHJKMNPQRSTUVWXYZ23456789"` — uppercase, no
hyphens, no whitespace — so a generated code is already a normalization
fixed-point.

`Alphabet::new` accepts any symbol set of length ≥ 2. **A custom alphabet
containing a lowercase letter or a hyphen would silently break INV-4**: the
issue path would store the un-normalized form, the redeem path would normalize
the user's input, and the lookup keys would never match. Every code issued under
that policy would be unredeemable, and no existing test would notice.

That is the property worth testing, and it is not hypothetical — `Alphabet::new`
is public API.

## 3. Decision

### 3.1 INV-4 properties

| Property | Statement |
|---|---|
| P-1 idempotence | `normalize(normalize(s)) == normalize(s)` for arbitrary `&str` |
| P-2 issue/redeem agreement | for any policy, a generated code is a normalization fixed-point: `normalize(generate_code(p)) == generate_code(p)` |
| P-3 alphabet safety | for any `Alphabet`, every symbol is a normalization fixed-point — i.e. `Alphabet::new` accepts no symbol that normalization would alter |
| P-4 totality | `normalize` never panics on arbitrary Unicode |

P-3 is the general form of P-2 and the one that would have caught the latent
defect in §2.1. If it fails for a symbol set `Alphabet::new` currently accepts,
that is a finding about `Alphabet::new`, to be reported — not fixed under this
RFC.

### 3.2 Generation: exact assertions, not statistics

M5-3 asked for "distribution tests". A chi-square test over sampled output is
the obvious reading and the wrong one: it is flaky, its failures get
re-run rather than investigated, and a flaky security test is one that gets
disabled.

The rejection sampler admits an **exact** treatment instead. The default
alphabet has 31 symbols; `256 % 31 == 8`, so `unbiased_ceiling()` is 248 and
each of the 31 symbols is reachable from exactly `248 / 31 == 8` byte values.

| Property | Statement |
|---|---|
| P-5 exact uniformity | over all 256 byte values, each alphabet symbol is produced by exactly `ceiling / len` bytes; no accepted byte maps outside the alphabet |
| P-6 rejection | every byte `>= ceiling` is rejected, never mapped — driven by a deterministic RNG feeding known bytes |
| P-7 ceiling correctness | for every alphabet length 2..=256, `unbiased_ceiling()` is the largest multiple of `len` that is `<= 256` |

These are deterministic, exhaustive over the input domain, and cannot flake.
P-7 in particular is exhaustive over every alphabet size the type permits.

### 3.3 A property test must be shown to fail

The standing risk, stated plainly: **a property test that passes because its
generator never produces an interesting input is indistinguishable from one that
passes because the property holds.** This milestone has twice found a check that
could not fail; that failure mode is available here in a subtler form.

Two requirements, both mandatory:

1. **Negative demonstration.** Each property must be observed failing against a
   deliberately broken implementation — e.g. a `normalize` that lowercases, an
   `unbiased_ceiling` returning 256. Recorded, in the manner RFC-036 §3.4
   established and RFC-040 §3.2 automated.
2. **Generator coverage assertions.** Each property test asserts its input
   corpus actually contains the interesting shapes: at minimum inputs with
   hyphens, ASCII whitespace, lowercase letters, non-ASCII characters, and the
   empty string. A property over a corpus of `"ABC"`-alikes proves nothing, and
   nothing in a property-testing framework will tell you that.

Requirement 2 is the one that is easy to skip and the reason this section
exists.

### 3.4 Library

`proptest`, as a **dev-dependency only**, version in `[workspace.dependencies]`
per DEC-012, subject to one hard condition: **it must build on the 1.85 MSRV**.
The `msrv` CI job covers `codlet` with `--all-targets`, so a proptest that
requires a newer toolchain will turn that job red rather than fail silently.

If it does not hold 1.85, the fallback is a hand-rolled deterministic generator
seeded from the existing test RNG. Every property above is expressible that way
— P-5 through P-7 are exhaustive and need no framework at all — at the cost of
shrinking. Do not raise the MSRV to accommodate a test dependency.

## 4. Recommendation against a fuzzing harness — owner decision required

M5-4 approved "fuzz targets, CI smoke mode". I recommend **not** building them,
and record the reasoning rather than quietly dropping the item.

Coverage-guided fuzzing earns its cost on code that parses untrusted bytes into
structure: state machines, decoders, deserializers. codlet's untrusted-input
surface is `validate_code_input(&str)` and `normalize(&str)`, which are:

- **total** — no indexing, no slicing, no recursion, no arithmetic on
  attacker-controlled lengths;
- **`#![forbid(unsafe_code)]`**, so no memory-safety class exists to find;
- **already UTF-8-validated** by the type system before entry, so the input
  space is "arbitrary valid Unicode" — exactly what P-1 and P-4 quantify over.

Coverage guidance buys little against ~15 lines of straight-line code with no
branches on input structure. Against that: `cargo-fuzz` requires a **nightly
toolchain**, which this project does not otherwise use and whose MSRV discipline
it would sit awkwardly beside, plus a corpus to store, maintain, and keep
meaningful.

**Recommended:** defer fuzzing until codlet acquires a component that parses
untrusted structured input — a cookie or header parser, a token decoder, a wire
format. None exists today; RFC-032 keeps delivery channels out of scope, and
codlet builds cookies rather than parsing them. Revisit if that changes.

**If the owner prefers to keep M5-4**, it should be its own RFC: a nightly CI
job, corpus policy, and a decision about whether fuzz findings block a release
are not a footnote to this one.

## 5. Non-goals

- No change to `normalize`, `generate_code`, `Alphabet`, or any behaviour. This
  RFC only observes.
- No repair of anything P-3 uncovers — report and escalate.
- No new invariants; INV-4's wording is unchanged.
- No statistical/chi-square testing (§3.2).

## 6. Security considerations

INV-4 currently has no guard. A normalization divergence between the issue and
redeem paths does not leak secrets — it makes codes unredeemable, a
availability failure, not a confidentiality one. That is why this ranks below
M5's other work and is last in the sequence.

The generation properties are more security-relevant: P-5 through P-7 are what
stand between the project and a modulo-biased code generator, which would reduce
effective entropy below the ~39.6 bits the threat model claims for the default
policy. That claim is currently supported by example tests and a code comment.

## 7. Alternatives considered

1. **Chi-square distribution testing.** Rejected — §3.2. Flaky, and the exact
   assertion is strictly stronger.
2. **`quickcheck` instead of `proptest`.** Weaker shrinking, less maintained.
   Either satisfies the RFC; the implementer may substitute with a note if
   proptest's MSRV blocks it.
3. **Property tests without generator-coverage assertions.** Rejected — §3.3.
   This is the difference between a test and the appearance of one.
4. **Raising the MSRV to accommodate `proptest`.** Rejected outright. A test
   dependency does not get to move a published crate's floor.

## 8. Open questions

1. ~~**M5-4 fuzzing: defer or build?**~~ **Resolved: deferred, by owner
   approval 2026-09-03**, accepting this RFC including its §4 recommendation.
   M5-4 is formally withdrawn from M5 scope — recorded in `ROADMAP.md` rather
   than dropped silently, per the anti-pattern RFC-037 exists to close.

   **Trigger to revisit:** codlet acquiring a component that parses untrusted
   structured input — a cookie or header parser, a token decoder, a wire
   format. None exists today. If that changes, fuzzing gets its own RFC
   covering the nightly toolchain, corpus policy, and whether findings block a
   release.
2. If P-3 fails against alphabets `Alphabet::new` currently accepts — likely,
   since it validates only length ≥ 2 — is the remedy to constrain
   `Alphabet::new` or to normalize on the issue path? A public-API question,
   deferred until the test says whether it fires.

## 9. Acceptance criteria

- P-1 through P-7 implemented and passing.
- Every property observed failing against a deliberately broken implementation,
  recorded.
- Generator coverage assertions present for P-1 through P-4.
- `docs/src/threat-model.md`'s INV-4 row names its guard and negative test; no
  row left open.
- `proptest` (or the fallback) is dev-only; `core-deps` still green; the `msrv`
  job still green on 1.85.
- No change to `normalize`, `generate_code`, or `Alphabet`.
