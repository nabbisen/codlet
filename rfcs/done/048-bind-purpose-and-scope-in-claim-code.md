# RFC-048: Bind `purpose` and `scope` — SQL Injection in `claim_code`

- **Status:** Implemented (v0.19.1)
- **Target milestone:** M6, ahead of all remaining M6 work
- **Primary crate(s):** `codlet-sqlx`, `codlet-worker`, `xtask`
- **Severity:** Critical — confirmed exploitable in published releases
- **Source basis:** `.git-exclude/reviewed/SECURITY-FINDING-claim-code-sql-injection.md`

## 1. Summary

`claim_code` interpolates the host-supplied `purpose` and `scope` strings into
SQL with `format!("{p:?}")` instead of binding them. Bind them.

Then add a gate so the class cannot return, and prove that gate by running it
against the vulnerable code.

## 2. The defect

`{:?}` on a `&str` is Rust escaping, not SQL escaping: it emits a double-quoted
string with `"` → `\"`. SQLite does not process backslash escapes inside quoted
tokens, so the `"` closes the token and the remainder parses as SQL.

Confirmed against the shipped SQLite adapter, through the public
`CodeStore::claim_code` API:

```
scope = x" OR 1=1 --
→ 2 rows marked used, one of them in a different scope and never targeted
```

The `changed > 1` check fired **after** both writes — detection, not prevention.
And it is avoidable: a payload matching exactly one row (`x" OR id='<victim>' --`)
returns `changed == 1`, classifying as `ClaimOutcome::Won` with no error.

This breaks **DEC-009 / RFC-C** directly: scope isolation is enforced by the
clause that is injectable.

`sqlx::AssertSqlSafe` wraps the call and suppresses the compile-time check that
would have objected. The assertion is false.

## 3. Exact extent — bounded, and verified bounded

Six interpolation sites, all inside `claim_code`:

| File | Sites |
|---|---|
| `crates/codlet-sqlx/src/code.rs` | 3 literals, `{p:?}` / `{s:?}` |
| `crates/codlet-sqlx/src/postgres/code.rs` | 2 `push_str(&format!(…))` |
| `crates/codlet-worker/src/d1/code.rs` | 2 `push_str(&format!(…))` |

**Everything else is clean, and this was checked rather than assumed:**

- `find_redeemable` binds values in every adapter; the D1 variant interpolates
  only `{t}`, the configured table name.
- The PostgreSQL and D1 admin paths build `${param_idx}` placeholders and bind.
- `mem` has no SQL.
- The session, form-token, and rate-limit stores contain no value interpolation.

## 4. Decision

### 4.1 Bind the values

Each adapter selects one of a fixed set of **complete, constant** SQL strings
and binds every value. No fragment assembly carrying data.

The SQLite adapter's existing `match (purpose, scope)` over four literals is
already the right shape — the literals are simply wrong. PostgreSQL and D1 build
theirs by `push_str`; they should move to the same fixed-set form, because a
shape assembled from constant fragments is far easier to gate and to read than
one assembled from anything else.

Binding order differs per backend (`?` positional for SQLite/D1, `$N` for
PostgreSQL); the fixed-set form makes the argument order explicit at each site
rather than implied by control flow.

### 4.2 A gate, proven against the real defect

Add an `xtask` gate rejecting value interpolation into SQL in store operations.

**It must not false-positive on the legitimate cases**, which exist and are
correct: `{t}` for a configured table name, `${param_idx}` for a generated
placeholder, `LIMIT {n}` for a typed integer, and `WHERE {}` joining
placeholder-bearing fragments. A gate that fires on those gets weakened until it
fires on nothing.

**The acceptance test writes itself, and is unusually strong**: run the gate
against the pre-fix code. It must fire on all six sites across the three
adapters. A gate that cannot detect the defect that motivated it is not a gate —
and unlike every other gate in this project, this one can be tested against a
real historical vulnerability rather than a synthetic fixture.

This also joins `xtask self-test` with its own violation fixture (RFC-040 §3.2),
so it stays proven after the historical commit stops being convenient to reach.

### 4.3 Regression tests, per adapter, with the real payload

Each SQL adapter gets a test claiming with `scope = x" OR 1=1 --` and asserting
the claim is **not** won and no other row is touched. The literal payload, not a
sanitised approximation.

## 5. Non-goals

- No change to `claim_code`'s conditional-UPDATE semantics, its guard
  conditions, or INV-5. Only how values reach the query.
- No change to `find_redeemable`, including RFC-047's in-flight step-1 work.
- No input validation on `purpose`/`scope`. They are opaque host-owned strings
  (RFC-001) and must stay that way — the fix is correct parameterisation, not a
  character allowlist, which would be both a compatibility break and a weaker
  guarantee.
- No `AssertSqlSafe` removal where the assertion becomes true. Once no value is
  interpolated, the remaining dynamic shape is genuinely safe.

## 6. Security considerations

**Severity: critical.** Scope isolation is a documented guarantee (DEC-009) and
is bypassable. An attacker who influences `scope` can mark a targeted code used,
attributed to a subject of their choosing, without tripping the `changed > 1`
check.

**Reachability depends on the host.** `purpose` and `scope` are host-supplied,
and nothing in codlet's documentation warns they reach SQL unescaped. A host
sourcing a tenant from a URL segment is realistic and undocumented-as-dangerous.
For a library whose stated value is being a safe, auditable primitive,
host-supplied strings reaching SQL unparameterised is a defect independent of
whether a given host is exploitable today.

**PostgreSQL and D1 are unverified.** PostgreSQL treats `"…"` as an identifier
and likely errors rather than injects; D1 is SQLite-based and is presumed
exploitable. **Neither may be described as safe or unsafe in the CHANGELOG or
advisory until tested** — §8 requires both.

**Why it survived.** No gate looks at how store operations build SQL.
`no-plaintext-in-store-ops` guards what reaches the store, not how the query is
built. M5 proved every existing gate can fail; that exercise cannot reveal a
gate that was never written.

## 7. Release and disclosure

- Patch release. No API change, no MSRV change, no behavioural change for any
  correct usage.
- A GitHub Security Advisory, per SECURITY.md's commitment. The finding matches
  that document's own examples in spirit.
- **Yank: owner's decision, recorded either way.** SECURITY.md's carve-out is
  "a defect that cannot fail safely and cannot be remedied by upgrading". This
  one *is* remediable by upgrading, which argues against — but it does not fail
  safely, which is the other half. Deliberately not decided here.

## 8. Acceptance criteria

- No value interpolation in any SQL string in any adapter; `purpose` and `scope`
  bound in all three `claim_code` implementations.
- The `x" OR 1=1 --` regression test passes on SQLite, PostgreSQL, and D1 —
  **PostgreSQL under Docker in CI, D1 under Miniflare.** Not inferred from
  SQLite.
- The new gate fires on the pre-fix code at all six sites, with output recorded.
- The gate does not fire on `{t}`, `${param_idx}`, `LIMIT {n}`, or `WHERE {}`.
- The gate is in `release-check` and has a `self-test` fixture.
- `claim_code`'s guard conditions and INV-5 conformance unchanged.
- Full CI green including `postgres-test` and Miniflare.
