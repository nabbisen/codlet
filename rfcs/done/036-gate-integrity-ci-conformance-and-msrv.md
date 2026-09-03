# RFC-036: Gate Integrity — CI Conformance, MSRV Enforcement, and Release-Discipline Accuracy

- **Status:** Implemented (Unreleased)
- **Target milestone:** M4
- **Primary crate(s):** none — CI and policy documentation
- **Source basis:** review findings 2026-08-31; RFC-002 §10.5; RFC-009; RFC-015; RFC-028

## 1. Summary

Restore the automated gates that enforce codlet's security invariants, add the
MSRV verification that SECURITY.md already claims exists, and correct the
release-discipline command list to describe gates that can actually be run.

## 2. Motivation

This began as a defect report and is recorded as an RFC because its outcome is a
durable rule, not a one-off patch: *a gate that has never been observed failing
is not a gate*, and *a policy document may not claim an enforcement that does not
exist*.

Three verified findings at commit `0962ca4`:

1. **Six CI jobs reference `codlet-core`; one references `codlet-examples`.**
   Neither package exists — `codlet-core` was renamed at v0.17.0 (DEC-002) and
   `codlet-examples` was split into five standalone packages at v0.16.2
   (DEC-003). `ci.yml` was last modified at `74a6bdf`, before the rename commit
   `fc5737c`.

   ```
   $ cargo tree -p codlet-core
   error: package ID specification `codlet-core` did not match any packages
   ```

   The `core-deps` job is the automated enforcement of RFC-002 §10.5 — the check
   that fails the build if `axum`, `tower`, `sqlx`, `tokio`, `worker`, `hyper`,
   or `reqwest` enters the `codlet` dependency tree. Runtime neutrality is listed
   as non-negotiable in CONTRIBUTING.md and has had **no machine enforcement for
   two releases**. The RFC-009 `Send`-compat proof and the core feature matrix
   are in the same state.

2. **MSRV is not enforced.** SECURITY.md states the MSRV "is enforced by CI".
   `rust-toolchain.toml` pins `channel = "stable"` and every job uses
   `dtolnay/rust-toolchain@stable`. Nothing verifies Rust 1.85. A dependency or
   language-feature bump could raise the effective MSRV silently and break
   downstream builds without any signal.

3. **The `core-deps` gate fails open — it reports success while checking
   nothing.** This is the most serious finding, and it was found by reading the
   CI history rather than the workflow file. On commit `0962ca4`, where
   `cargo tree -p codlet-core` provably errors, the `core dependency gate` job
   **passed**
   ([run 28082000417](https://github.com/nabbisen/codlet/actions/runs/28082000417)).

   Root cause is the shell, not the crate name:

   ```sh
   tree="$(cargo tree -p codlet-core -e normal --prefix none | sort -u)"
   ```

   `cargo tree` exits 101, but the exit status of the assignment is the status of
   the *last command in the pipeline* — `sort` — which is 0. `$tree` is set to
   the empty string, the subsequent `grep` matches nothing, and the job reports
   success. The RFC-002 §10.5 runtime-neutrality check has therefore not merely
   been absent; it has been **actively reporting a false pass**.

   A red job is a visible defect. A green job that checks nothing is a defect
   that also destroys the evidence that would reveal it.

4. **v0.17.1 was published to crates.io with CI red.** Seven jobs failed on the
   released commit `0962ca4`, and the preceding release commit `fc5737c` failed
   too. The v0.17.1 handoff bundle states "Ready. All required gates passed at
   v0.17.1", citing hand-written summary logs. Whatever was run locally, the
   project's own CI disagreed and the release proceeded anyway.

5. **Two `codlet-sqlx` job failures are real and pre-existing**, unrelated to the
   rename. Both stem from finding 6 below: `test codlet-sqlx (SQLite adapter
   conformance)` runs `--all-features`, which activates `postgres-test`,
   which requires Docker — the job dies in
   `postgres_tests::postgres_admin_list_and_get`. A job named "SQLite adapter
   conformance" is failing on PostgreSQL tests it should never have compiled.

6. **The documented release discipline cannot be executed.** SECURITY.md
   mandates `cargo test --workspace --all-features` and the equivalent clippy
   invocation. `--all-features` activates `codlet-sqlx`'s `postgres-test`
   feature, which requires Docker via testcontainers. In any environment without
   Docker the documented gate cannot run at all.

## 3. Decision

### 3.1 CI conformance

Every job references a package that exists in `cargo metadata`. The example jobs
address the five standalone packages; only the three non-interactive examples
are executed (`axum_login_logout` binds a port and never exits; `sqlite_file` is
a deliberate two-invocation flow).

### 3.2 MSRV verification

MSRV is verified **per crate, against each crate's own declared floor**.
`rust-version` is a per-package field; `[workspace.package].rust-version` only
supplies a default that a package opts into with `rust-version.workspace = true`.
A single workspace-wide floor would be a claim the dependency tree does not
support.

| Crate | Floor | Determined by |
|---|---|---|
| `codlet`, `codlet-conformance`, `codlet-worker`, `xtask` | 1.85 | workspace default; verified to build clean |
| `codlet-sqlx` | 1.94 | its own `sqlx` 0.9.0 dependency declares `rust-version = "1.94.0"` |

Two jobs enforce this: `msrv` checks the 1.85 crates on 1.85, `msrv-sqlx` checks
`codlet-sqlx` on 1.94. The `publish = false` examples that depend on
`codlet-sqlx` keep inheriting 1.85 and are excluded from enforcement — nothing
consumes their declared `rust-version` as a promise.

Raising the whole workspace to 1.94 was rejected: it would impose a bleeding-edge
toolchain on every consumer of the runtime-neutral core to satisfy a constraint
only the SQLx adapter has. Downgrading `sqlx` to preserve a version number was
also rejected — it inverts the priority between a dependency choice made under
RFC-034 and a number in a manifest. An MSRV is a factual claim about a crate,
not an aspiration for a workspace.

*(§3.2 revised 2026-09-03 by the RFC-036 review, finding R-1. The original text
specified a single `cargo check --workspace --all-targets` on 1.85, which the
dependency tree cannot satisfy.)*

`rust-toolchain.toml` stays pinned to `stable`. It is not downgraded: the
default developer toolchain and the MSRV floor are different things, and
conflating them would silently hold the whole project at 1.85 tooling.

The job must set `RUSTUP_TOOLCHAIN`, which overrides `rust-toolchain.toml`, and
must assert the active version before building. Merely installing 1.85 is not
sufficient — `rust-toolchain.toml` would override it and the job would pass on
stable, for the wrong reason. **A verification that can pass without verifying
anything is worse than no verification**, because it converts an unknown into a
false assurance.

### 3.3 Release discipline describes reality

SECURITY.md's command list is replaced with the gate set CI actually runs, with
the PostgreSQL job marked Docker-dependent. The document is corrected to match
the executable gates; the gates are not bent to match the document.

The same misconception is corrected in CI itself: the SQLite conformance job
selects the SQLite feature explicitly rather than `--all-features`, so it stops
dragging in the Docker-dependent PostgreSQL path.

### 3.5 Shell safety in every gate script

Every multi-command `run:` block that a gate depends on must begin with:

```sh
set -euo pipefail
```

and must assert that the data it inspects is non-empty before drawing a
conclusion from it. Finding 3 is what happens without this: a check whose input
silently became empty concluded "no forbidden crates found."

The general rule: **a gate must fail when it cannot perform its check.** Absence
of evidence may never be reported as evidence of absence.

### 3.4 Standing rule: gates are proven by observed failure

A gate is accepted as working only when it has been seen to fail on a
deliberately introduced violation, with that failure output recorded as
evidence. This applies to the `core-deps` gate under this RFC, and to every
future gate added under M5.

Rationale: all five `xtask` gates and the `core-deps` check are grep-based. A
grep whose pattern no longer matches the code it guards fails open, silently and
permanently. Finding 3 is that failure mode, caught in production: the gate
returned green for two releases while inspecting an empty string.

This rule applies to the five `xtask` release gates as well. They have never been
observed failing either, and they are the same shape of check. Verifying them is
scoped to M5 rather than here, but it is now a known gap, not an assumption.

## 4. Non-goals

- No library code changes. A CI job that fails for a genuine, previously masked
  reason is a finding to report, not a licence to modify `crates/`.
- No change to the five `xtask` release gates or the eight threat-model
  invariants.
- No new advisory or supply-chain scanning — that is M5-1, deliberately
  separate.
- No adversarial verification of the five `xtask` release gates — deferred to
  M5, recorded in §3.4.

## 5. Security considerations

This RFC restores, and does not alter, the enforcement of existing invariants.
The security-relevant outcome is that RFC-002 §10.5 (runtime neutrality) and
RFC-009 (`Send` compatibility) become machine-checked again.

The MSRV job additionally protects downstream consumers: codlet declares
`rust-version = "1.85"`, and a consumer pinned to that toolchain currently has
no protection against an accidental bump.

## 6. Compatibility effects

None for published crates. `rust-toolchain.toml` is unchanged, so no developer's
local toolchain is affected.

## 7. Tests and release gates

- All existing CI jobs execute against real packages.
- New `msrv` job, with an in-job assertion that 1.85 is the active toolchain.
- `core-deps` proven by observed failure per §3.4.

## 8. Alternatives considered

1. **Fix the workflow silently as a bug, with no RFC.** Rejected once §3.4
   emerged as a durable rule — a rule that lives only in a handoff is lost when
   the handoff is archived.
2. **Pin `rust-toolchain.toml` to 1.85 instead of adding a job.** Rejected: it
   would hold all local development at MSRV tooling and would still not fail
   when the effective MSRV rises, because everyone would simply be building on
   1.85 and never notice.
3. **Change CI to `--all-features` so SECURITY.md becomes true.** Rejected:
   it makes the whole suite Docker-dependent, so CI would break in any
   environment without a Docker daemon.

## 9. Open questions

None.

## 10. Acceptance criteria

Enumerated in the Developer Handoff at
`rfcs/handoffs/036-gate-integrity-ci-conformance-and-msrv/implementation-handoff.md`.
