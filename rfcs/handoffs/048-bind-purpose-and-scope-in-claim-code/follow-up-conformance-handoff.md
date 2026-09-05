# Follow-Up Handoff — RFC-048: Move the Metacharacter Regression into the Shared Conformance Suite

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-05
- **Governing RFC:** [`../../done/048-bind-purpose-and-scope-in-claim-code.md`](../../done/048-bind-purpose-and-scope-in-claim-code.md) — this completes RFC-048's §4.2 intent, so it lives beside that RFC's implementation handoff rather than acquiring an RFC of its own (RFC-000 permits multiple companions per RFC; RFC-035 §3.5 requires a governing RFC, and this has one).
- **Issue only after 0.19.1 is published.** Owner-approved sequencing: the security release ships minimal first.

## 1. Purpose

RFC-048's regression test proves the fix on the adapters that exist. It does not
protect adapters that do not exist yet.

## 2. Why this is not optional tidying

The literal-payload test lives in `crates/codlet-sqlx/tests/conformance.rs` and
the D1 JS harness. **`codlet-conformance` has zero coverage of it.**

RFC-023 makes passing the shared suite the precondition for calling an adapter
production-ready. As things stand, that suite would certify a newly written
adapter carrying this exact vulnerability, and every gate would stay green:

- `no-interpolated-sql-values` scans source for a *pattern*. An adapter that
  builds SQL a different way, or lives outside the scanned tree, passes it.
- The adapter-local tests are not inherited by anything.

A source-pattern gate and a behavioural conformance test are not substitutes.
The gate says "nobody wrote it the known-bad way"; the suite says "this adapter
is not exploitable". Only the second is a claim about adapters in general.

## 3. Change scope

- `crates/codlet-conformance/src/code.rs` — the new shared test
- `crates/codlet/src/mem/code.rs` — **only if** the in-memory store fails it
  (see §5)
- `CHANGELOG.md` — `[Unreleased]`

## 4. Non-change scope

- No adapter's `claim_code` logic, unless §5 applies.
- The existing adapter-local tests. Leave them; a shared test and a local one
  are not duplicates when the local one also covers backend-specific shape.
- The gate, the fix, anything in `find_redeemable`.

## 5. Required implementation

Add a shared conformance test asserting that a `claim_code` whose `scope`
contains SQL metacharacters cannot affect any row other than a legitimately
matching one.

Cover, at minimum, the literal `x" OR 1=1 --` and the single-row variant
`x" OR id='<other>' --` — the second is the one that returned a silent `Won`
before the fix, and it is the more important of the two.

Assert **both**: that the claim is not won, and that no other row was touched. A
test checking only the return value passes against a mass update that happens to
report `Lost`.

Also assert the legitimate case still claims — the same reasoning as in the
review of RFC-048: a suite that only proves rejection would pass against an
adapter that rejects everything.

**If the in-memory store fails this**, that is a finding, not a chore. Report it
before fixing: `mem` has no SQL, so a failure would mean the collapse happens
somewhere other than SQL construction, which changes what we believe about the
defect's shape.

## 6. Acceptance criteria

1. The test is in `codlet-conformance`, not adapter-local.
2. It passes for in-memory, SQLite, PostgreSQL (CI), and D1 (Miniflare).
3. It asserts not-won, no-other-row-touched, and legitimate-scope-still-works.
4. **It is observed failing** against an adapter with the interpolation
   reintroduced — the same trial as RFC-048, now proving the *shared* suite
   catches it. Record the output.
5. Adapter-local tests still pass, unmodified.
6. Full CI green.

Criterion 4 is the point of the exercise. A shared test that has not been seen
to fail is a shared test that might certify the next adapter as safe for the
same reason the last one was.

## 7. Prohibited shortcuts

- Do not delete the adapter-local tests as "now redundant".
- Do not weaken the payload.
- Do not fix an in-memory failure before reporting it (§5).

## 8. Required evidence

The new test passing on all four adapters; the criterion-4 trial output;
CI run URL.

## 9. Review request

`.git-exclude/review-request/048-followup-shared-conformance.md`; my result
returns at `.git-exclude/reviewed/048-followup-shared-conformance.md`.
