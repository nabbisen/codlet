# RFC-054: The D1 Adapter Must Execute In a Test

- **Status:** Implemented (Unreleased) — reviewed and approved 2026-10-18
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet-conformance`, `codlet-worker`, CI
- **Source basis:** RFC-051 review request §5.1; verified and extended 2026-10-18

## 1. Summary

No test anywhere executes `codlet-worker`'s D1 stores. The Miniflare suite runs
a **JavaScript reimplementation** of their SQL; the Rust code is only
type-checked by a `wasm32` build. And the harness's schema does not match the
one codlet ships, so the one property it does claim to hold — SQL copied
verbatim — is false and undetectable.

This is RFC-038's finding in a new place: a conformance suite that certifies an
adapter it has never run.

## 2. Evidence

`crates/codlet-worker/tests/worker.js`, in its own header:

> Why JavaScript rather than compiled Rust: codlet-worker is a library crate
> (no fetch handler) … A JS harness that executes the same SQL is the correct
> architecture: it tests the D1 runtime binding behaviour … while the wasm32
> compile CI job verifies the Rust type constraints.
>
> All SQL is copied verbatim from the Rust implementation so any divergence is
> a test failure.

The last claim does not hold, because the harness creates its own tables:

| Source | Timestamp columns |
|---|---|
| shipped `crates/codlet-worker/migrations/0001_initial.sql` | **11 × `INTEGER`**, 0 × `REAL` |
| `crates/codlet-worker/tests/worker.js` | **11 × `REAL`**, 0 × `INTEGER` |

Zero overlap. The shipped migration is never executed by any test, so D1
conformance has never run against the schema codlet ships.

It also explains a documentation defect found separately: the adapter matrix
said D1 stores "timestamps as REAL (f64)" (corrected by RFC-053). That sentence
described the **test harness**, not the product. A mirror that drifts does not
merely fail to catch bugs; it becomes a second source of truth and feeds them
into the docs.

### 2.1 For RFC-051 the gap is total

RFC-051's fix is Rust control flow — a loop over lookup-key candidates. A
harness that mirrors SQL per route has nothing to mirror. The D1 half of that
change rests entirely on compile-checking and shape-similarity to two adapters
that are tested, which is what the implementer said and is the honest position.

## 3. Why the shared suite cannot be pointed at D1 today

One line:

```rust
S: CodeStore + Send + Sync + 'static,     // codlet-conformance/src/code.rs:18
```

`D1CodeStore` holds `Rc<worker::d1::D1Database>` (`d1/code.rs:19`) and is
therefore `!Send + !Sync`. Workers are single-threaded by design, so this is
correct for the adapter and wrong only as a *bound on the suite*.

Notably the suite was already written with `!Send` stores in mind:
`test_exactly_one_claim_winner` uses `spawn_local` on a `LocalSet`
specifically "so the test works with store types that are not `Send`"
(`code.rs:199`). The intent was there; the entry-point bound closes the door
the body left open.

## 4. Decision

### 4.1 Immediately: the harness must run the shipped migration

Delete `worker.js`'s inline `MIGRATION_SQL` and have the harness execute
`crates/codlet-worker/migrations/0001_initial.sql` itself. This is small,
independent of everything below, and removes the divergence that made the
harness a competing source of truth.

Do this even if §4.2 is deferred.

### 4.2 Relax the suite's bounds so a `!Send` store can be tested

Drop `Send + Sync + 'static` from the sequential tests and their entry points.
They `await` in sequence and never cross a thread, so the bounds buy nothing
for the adapters that satisfy them and exclude the one that does not.

`test_exactly_one_claim_winner` is different: it needs `tokio`'s `LocalSet`,
which does not exist in a Worker. Carve it into a separately-gated entry point
that keeps whatever bounds it genuinely requires, so relaxing the rest does not
drag tokio into a `wasm32` build.

The split is honest rather than cosmetic: after it, D1 runs every sequential
conformance test as real Rust, and its single-winner property stays covered by
the harness's concurrency route until a Worker-native equivalent exists. State
that limit in the adapter matrix rather than implying parity.

### 4.3 Execute the real stores in Miniflare

Add a **test-only** crate — not `codlet-worker` itself — with a
`#[event(fetch)]` handler that constructs the real `D1CodeStore`,
`D1SessionStore` and `D1FormTokenStore` against the Miniflare D1 binding and
runs §4.2's relaxed suite. Point the existing vitest job at it.

Keeping the fetch handler out of the published crate was the original and
correct instinct; the mistake was concluding that a JS reimplementation was
therefore "the correct architecture". A separate test crate gets both.

### 4.4 The harness's npm advisories

`npm install` in `crates/codlet-worker/tests` reports 19 advisories (3
critical). Test tooling only, and not reachable by any consumer. Run
`npm audit fix`, record what remains, and state plainly that this tree is
test-only — the same treatment RFC-052 gave the Rust side, so neither language's
test tooling is a standing unexplained warning.

## 5. Non-goals

- No fetch handler in `codlet-worker`. It stays a library.
- No change to the D1 adapters' behaviour, SQL, or the `Rc` handle. `!Send` is
  correct for Workers.
- No removal of the JS harness in this RFC. It still covers D1 binding
  behaviour and the concurrency route; it stops being the *only* evidence.
- No attempt to run `codlet-sqlx` under Miniflare.

## 6. Security considerations

INV-5 (single-winner claim) is enforced by D1's conditional UPDATE and
`meta().changes`. That path is currently proven only by a JavaScript copy of the
SQL — so for D1, an invariant the threat model treats as load-bearing has no
test touching the shipped code. The adapter matrix's ✓ for D1 atomic claim rests
on the mirror.

Nothing here says the D1 adapter is wrong. It says we do not know, and have
been presenting a mirror's result as the adapter's.

## 7. Acceptance criteria

1. The Miniflare harness executes the shipped migration; no schema SQL is
   duplicated in `worker.js`.
2. A check — gate or test — fails if the harness's schema and the shipped
   migration diverge again, or the duplication is gone such that it cannot.
3. The sequential conformance entry points compile for a `!Send + !Sync` store.
4. A test-only wasm crate runs those tests against the real `D1*Store` types in
   Miniflare, and they pass in CI.
5. `test_exactly_one_claim_winner` still runs for the in-memory, SQLite and
   PostgreSQL adapters, unchanged in strength.
6. The adapter matrix states exactly what is proven for D1 and by what.
7. `npm audit` findings resolved or recorded with a stated reason.
8. All blocking CI jobs green.
