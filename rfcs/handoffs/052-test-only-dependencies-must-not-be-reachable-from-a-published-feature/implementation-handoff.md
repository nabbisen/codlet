# Developer Handoff — RFC-052: Test-Only Dependencies Must Not Be Reachable From a Published Feature

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-17
- **RFC:** [`../../accepted/052-test-only-dependencies-must-not-be-reachable-from-a-published-feature.md`](../../accepted/052-test-only-dependencies-must-not-be-reachable-from-a-published-feature.md)
- **Baseline:** `3767721`
- **Priority: do this one first.** It unblocks the next release tag.

## 1. Purpose

Clear four `rustls` advisories, and stop `codlet-sqlx` shipping Docker test
tooling to consumers who enable `postgres-test`.

Your earlier diagnosis of this CI job was wrong and the RFC records why — read
§2 of the RFC before starting, because the instinct it corrects ("the gate is
broken, change the workflow") would do real damage here.

## 2. Change scope — indicative, not authoritative

`crates/codlet-sqlx/Cargo.toml`, `Cargo.lock`, `deny.toml`,
`.github/workflows/ci.yml` (the `core-deps` job), and possibly
`crates/xtask/` if you put the gate there. §7 is binding.

## 3. Step 1: the lockfile

```sh
cargo update -p rustls
```

I ran this: it moves `rustls` 0.23.41 → **0.23.46** and `rustls-webpki`
0.103.13 → 0.103.15, two packages, lockfile only. 0.23.46 is the exact fix
version for all four advisories.

Commit this alone. It is the release blocker and it should be bisectable.

## 4. Step 2: the dependency class

In `crates/codlet-sqlx/Cargo.toml`:

- move `testcontainers-modules` from `[dependencies]` (where it is
  `optional = true`) to `[dev-dependencies]`, dropping `optional`;
- change `postgres-test = ["postgres", "dep:testcontainers-modules"]` to
  `postgres-test = ["postgres"]`.

`crates/codlet-sqlx/tests/conformance.rs:358`'s `#[cfg(feature = "postgres-test")]`
gate keeps working unchanged — the feature still exists, it just carries no
dependency. CI's `cargo test -p codlet-sqlx --no-default-features --features postgres-test`
(`ci.yml:123`) must still run those tests.

**If Cargo rejects any part of this, stop and report.** I believe optional
dev-dependencies are not permitted, which is why the feature keeps its name and
loses only the `dep:` — but verify rather than assume, and tell me if the shape
has to change.

## 5. Step 3: `deny.toml` — one trap here

The `ISC` entry's comment currently justifies itself by saying the subtree
ships to consumers. After Step 2 that is false, so rewrite the comment: the
subtree is reached only through dev-dependencies.

**Keep the `ISC` entry itself.** `include-dev = true` is set deliberately, so
`cargo deny check licenses` still walks dev-dependencies and still needs `ISC`
allowed. Deleting it turns the *blocking* licenses gate red. Change the words,
not the list.

## 6. Step 4: the gate

Extend `core-deps` with the `codlet-sqlx` assertion in RFC-052 §4.3. Copy its
shape exactly — capture to a variable, `test -n`, assert a known marker line is
present, **then** match. That order is what stops a `cargo tree` failure from
passing as an empty grep, and `--color never` is what stopped RFC-049's version
of this from breaking on ANSI codes in CI.

Then **prove the gate fires**: reintroduce `testcontainers-modules` as an
optional normal dependency on a scratch branch, confirm the job fails, revert.
Quote the failure output. A gate not demonstrated against a violation is not
yet a gate — RFC-036.

If you put it in `xtask` instead of inline, add the `self-test` fixture too.

## 7. Required verification

| Check | Expectation |
|---|---|
| `grep -A2 '^name = "rustls"$' Cargo.lock` | `0.23.46` or later |
| `cargo deny check advisories` | clean, exit 0 |
| `cargo deny check bans licenses sources` | clean, exit 0 — confirms §5 did not break the licenses gate |
| `cargo tree -p codlet-sqlx --features postgres --no-default-features -e normal --prefix none --color never` | none of `testcontainers`, `bollard`, `ureq`, `rustls`, `ring`, `webpki` |
| `cargo test -p codlet-sqlx --no-default-features --features postgres-test` | Docker tests run and pass |
| `cargo test --workspace` | green |
| the §6 violation trial | gate fails, output quoted |

Run `cargo deny` the way CI does. **`cargo-deny-action` defaults to
`--all-features`**; a bare `cargo deny check` does not reproduce it, which cost
a false pass on RFC-039. State which invocation you used.

## 8. Acceptance criteria

Binding; §2 yields to these.

1. `rustls >= 0.23.46` in `Cargo.lock`; `cargo deny check advisories` clean.
2. `testcontainers-modules` is a dev-dependency; `postgres-test` declares no `dep:`.
3. The Docker-backed PostgreSQL tests still run and pass.
4. The `--features postgres` normal graph contains no crate from §7's list.
5. The gate exists, is wired into CI, and was **demonstrated** to fail against a
   reintroduced violation.
6. `cargo deny check bans licenses sources` still clean, with `ISC` still
   allowed and its comment corrected.
7. All blocking CI jobs green, and the advisories job green.

## 9. Prohibited shortcuts

- **Do not add an `ignore` entry to `deny.toml`.** RFC-039 §3.1: reported,
  never silenced.
- **Do not touch `rust-toolchain.toml` or the workflow's toolchain setup** to
  chase the "override toolchain is not installed" message. It appears in the
  blocking sibling job, which passes. It is noise.
- Do not set `continue-on-error` anywhere new, or relax the release gate.
- Do not remove the `ISC` licence entry — §5.
- Do not pin `rustls` in a manifest. It is not a direct dependency; the lockfile
  is the right place.

## 10. Required evidence

The `cargo update` diff; each §7 check with its exit status and the invocation
used; the §6 violation trial's failure output; the corrected `deny.toml`
comment quoted; CI run URL.

## 11. Review request

`.git-exclude/review-request/052-test-only-dependencies.md`; result returns at
`.git-exclude/reviewed/052-test-only-dependencies.md`.
