# Implementation Handoff — RFC-048 SQL Injection in `claim_code` (Critical)

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-05
- **Milestone:** M6 — **ahead of all other M6 work.** RFC-047 step 2 and RFC-045 wait.
- **Governing RFC:** [`../../accepted/048-bind-purpose-and-scope-in-claim-code.md`](../../accepted/048-bind-purpose-and-scope-in-claim-code.md)
- **Finding:** [`../../../.git-exclude/reviewed/SECURITY-FINDING-claim-code-sql-injection.md`](../../../.git-exclude/reviewed/SECURITY-FINDING-claim-code-sql-injection.md)

Read RFC-048 and the finding before starting. If execution conflicts with the
RFC, **stop and escalate**.

You found this. It is a real, confirmed, critical vulnerability in published
crates.

## 1. Purpose

Bind `purpose` and `scope` in `claim_code`, and add a gate so the class cannot
return.

## 2. Do the gate first — the order is the point

**Write and run the gate before applying the fix.** The vulnerable code is in
your working tree right now, which makes RFC-048 §4.2's acceptance criterion
free to satisfy honestly: the gate must fire on all six real sites.

Do it the other way round and you are left demonstrating the gate against code
you have already fixed, or against a synthetic fixture — which is what every
other gate in this project had to settle for. This one can be proven against an
actual historical vulnerability. Take that.

Record the output of the gate firing on the unfixed tree. That output is the
single most valuable piece of evidence in this handoff.

## 3. Change scope

- `xtask/src/main.rs` — the new gate; its `self-test` fixture
- `xtask/fixtures/` — one violation fixture
- `crates/codlet-sqlx/src/code.rs` — `claim_code`, 3 literals
- `crates/codlet-sqlx/src/postgres/code.rs` — `claim_code`, 2 sites
- `crates/codlet-worker/src/d1/code.rs` — `claim_code`, 2 sites
- Regression tests, per adapter
- `crates/codlet-worker/tests/` — the D1 regression test via Miniflare
- `SECURITY.md` — the gate joins the release-discipline list
- `CHANGELOG.md`

## 4. Non-change scope

- **`claim_code`'s guard conditions and semantics.** `used_at IS NULL AND
  revoked_at IS NULL AND expires_at > ?` stays exactly as it is. You are
  changing how two values reach the query, nothing else. INV-5 is untouched.
- **No input validation on `purpose`/`scope`.** They are opaque host-owned
  strings (RFC-001). A character allowlist is a compatibility break and a weaker
  guarantee than binding. If you find yourself writing one, stop.
- **`find_redeemable`**, including RFC-047 step 1's in-flight changes.
- The session, form-token, and rate-limit stores. Verified clean; leave them.
- The four legitimate interpolations — `{t}`, `${param_idx}`, `LIMIT {n}`,
  `WHERE {}` — which are correct and must keep working.

## 5. Required implementation

### 5.1 The gate

Reject value interpolation into SQL in store operations.

**The hard requirement is not catching the bug — it is not firing on the
legitimate four.** A gate that false-positives on `{t}` or `${param_idx}` gets
weakened until it catches nothing, which is precisely how `cookie-attrs-present`
became decorative (RFC-042). Design for the false-positive case first, then
check it still catches the real one.

Joins `release-check`, and gets a `self-test` violation fixture (RFC-040 §3.2)
so it stays proven once the pre-fix commit is no longer convenient to reach.

### 5.2 The fix

Each adapter selects one of a fixed set of **complete, constant** SQL strings
and binds every value.

SQLite's `match (purpose, scope)` over four literals is already the right shape
— only the literals are wrong. Move PostgreSQL and D1 to the same form rather
than patching their `push_str` assembly: a shape built from constant alternatives
is easier to gate, and easier for the next reader to confirm at a glance.

Binding order differs per backend (`?` positional, `$N` numbered). The fixed-set
form makes the argument order explicit at each site instead of implied by
control flow — get this right; a mis-ordered bind is a new bug in a security fix.

### 5.3 Regression tests — the literal payload

Per SQL adapter: claim with `scope = x" OR 1=1 --` and assert the claim is
**not won** and **no other row is touched**. Assert both; a test that only checks
the return value would pass against a mass update that happened to report `Lost`.

Use the literal payload, not a sanitised approximation.

## 6. Verification that cannot be inferred

**SQLite:** confirmed exploitable; your regression test must show it fixed.

**D1:** presumed exploitable, unverified. Run the regression test under
Miniflare, for real. If D1 turns out **not** to be exploitable, say so — that is
a finding, not a disappointment, and it changes what the advisory says.

**PostgreSQL:** unverified. Likely errors rather than injects, because `"…"` is
an identifier there. **CI's `postgres-test` job is the only evidence.** Do not
state a PostgreSQL conclusion you have not run.

**The CHANGELOG must not characterise D1 or PostgreSQL until tested.** Report
what the tests show, and if one could not run, say that instead.

## 7. Acceptance criteria

1. Gate written and run against the **unfixed** tree, firing on all six sites,
   output recorded.
2. Gate does not fire on `{t}`, `${param_idx}`, `LIMIT {n}`, `WHERE {}`.
3. Gate in `release-check` with a `self-test` fixture.
4. No value interpolation in any adapter's SQL; `purpose`/`scope` bound in all
   three `claim_code`s.
5. Regression test passes on SQLite, PostgreSQL (CI), and D1 (Miniflare) —
   asserting both not-won and no-other-row-touched.
6. `claim_code`'s guard conditions unchanged; INV-5 conformance still green.
7. No input validation added.
8. CHANGELOG states only what was tested.
9. Full CI green including `postgres-test` and Miniflare.

## 8. Prohibited shortcuts

- Do not fix before writing the gate (§2).
- Do not add a character allowlist.
- Do not change `claim_code`'s guard conditions.
- Do not weaken the gate to clear a false positive on the legitimate four —
  redesign it.
- Do not describe D1 or PostgreSQL as fixed, safe, or unaffected without a run.
- Do not sanitise the payload in the regression test.

## 9. Known risks

| Risk | Mitigation |
|---|---|
| Mis-ordered bind parameters in the rewrite | §5.2; the INV-5 conformance suite must still pass on every adapter |
| Gate false-positives, gets weakened | §5.1 — design against the legitimate four first |
| D1 or PostgreSQL untested and assumed | §6; criterion 5 requires runs, not inference |
| The fix is correct but the gate does not actually catch the original | §2 makes this impossible to get wrong if done in order |

## 10. Required evidence

The gate firing on the unfixed tree (all six sites); the gate not firing on the
legitimate four; `self-test` output; each adapter's regression test including
the D1 Miniflare run and the PostgreSQL CI job; INV-5 conformance green on all
four adapters; CI run URL.

## 11. Review request

`.git-exclude/review-request/048-bind-purpose-and-scope-in-claim-code.md`; my
result returns at
`.git-exclude/reviewed/048-bind-purpose-and-scope-in-claim-code.md`.

Given the severity, expect this review to be adversarial about the *fix* as well
as the tests: a security patch is the worst possible place for a new defect.
