# Implementation Handoff — RFC-036 Gate Integrity

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-08-31
- **Milestone:** M4 (`ROADMAP.md`)
- **Governing RFC:** [`../../accepted/036-gate-integrity-ci-conformance-and-msrv.md`](../../accepted/036-gate-integrity-ci-conformance-and-msrv.md)
- **Priority:** highest. Do this before the RFC-035 handoff and before any feature work.

Read RFC-036 before starting. It is the authority; this document directs
execution only. If execution conflicts with the RFC, **stop and escalate** —
per RFC-000, a handoff may not override an RFC decision.

## 1. Purpose

Restore the automated gates that enforce codlet's security invariants. They have
not run since v0.17.0.

## 2. Background

At v0.17.0 the `codlet-core` crate was renamed to `codlet` (DEC-002), and at
v0.16.2 the single `codlet-examples` package was split into five standalone
example projects (DEC-003). `.github/workflows/ci.yml` was never updated for
either change; `git log` shows it was last touched at `74a6bdf`, before the
rename commit `fc5737c`.

Verified in the working tree at `0962ca4`:

```
$ cargo tree -p codlet-core
error: package ID specification `codlet-core` did not match any packages
```

Workspace packages are: `codlet`, `codlet-sqlx`, `codlet-worker`,
`codlet-conformance`, `xtask`, and the five example packages
(`axum_login_logout`, `sqlite_quickstart`, `sqlite_file`, `key_rotation`,
`form_token_csrf`). Neither `codlet-core` nor `codlet-examples` exists.

**Why this is a security matter, not cosmetics.** The `core-deps` job is the
automated enforcement of RFC-002 §10.5 — it fails the build if `axum`, `tower`,
`sqlx`, `tokio`, `worker`, `hyper`, or `reqwest` ever enters the `codlet`
dependency tree. Runtime neutrality is a stated non-negotiable invariant in
CONTRIBUTING.md and has had no machine enforcement for two releases. The
RFC-009 `Send`-compat proof and the core feature matrix are in the same state.

Separately, SECURITY.md asserts "The MSRV is set in `Cargo.toml` under
`[workspace.package]` and is enforced by CI." That is false. `rust-toolchain.toml`
pins `channel = "stable"`, and every CI job uses `dtolnay/rust-toolchain@stable`.
Nothing verifies Rust 1.85.

## 3. Change scope

- `.github/workflows/ci.yml`
- `SECURITY.md` — the MSRV claim and the release-discipline command list

## 4. Non-change scope — do not touch

- Any file under `crates/`, `examples/`, or `xtask/`. If a CI job fails for a
  reason other than a wrong package name, **stop and report** — do not fix
  library code under this handoff.
- `rust-toolchain.toml`. Leave `channel = "stable"`; MSRV is verified by a
  dedicated job, not by downgrading the default toolchain.
- `rfcs/` — **including this RFC's own status.** RFC-036 stays in `accepted/`
  when you finish; its transition to `done/` and the index rebuild are performed
  by the RFC-035 handoff, which runs next. This avoids rewriting the index
  twice.
- `CHANGELOG.md` history. Add an Unreleased entry only.
- Version numbers in `Cargo.toml`.

## 5. Required implementation

### 5.1 Package-name corrections in `ci.yml`

Replace `codlet-core` with `codlet` in all six jobs, updating the job `name:`
strings and the RFC-009 comment so they read `codlet`:

| Job | Line(s) | Correct command |
|-----|---------|-----------------|
| `wasm32-compile` | 48, 55 | `cargo build -p codlet --target wasm32-unknown-unknown` |
| `test-send-compat` | 79, 86 | `cargo test -p codlet --test rfc_009_compile --all-features` |
| `test-core` | 88, 93 | `cargo test -p codlet --all-features` |
| `core-feature-matrix` | 134, 141–143 | `cargo build -p codlet --no-default-features`; `cargo build -p codlet`; `cargo build -p codlet --features serde,test-utils` |
| `core-deps` | 151–158 | `cargo tree -p codlet -e normal --prefix none` |

Keep the forbidden-crate grep pattern in `core-deps` exactly as it is. Keep the
RFC references in the comments. For `core-deps` the crate name is **not** the
only change — see §5.2.

### 5.2 `core-deps` is failing open — harden it

Do not treat this job as a rename fix. On the released commit `0962ca4`, where
`cargo tree -p codlet-core` errors, this job **passed**
([run 28082000417](https://github.com/nabbisen/codlet/actions/runs/28082000417)).

Cause: in `tree="$(cargo tree … | sort -u)"` the assignment takes the exit status
of `sort`, not of `cargo tree`. The command errored, `$tree` became empty, the
grep found nothing, and the job concluded "no forbidden crates". It has been
reporting a false pass for two releases.

Replace the step body with a version that cannot pass without checking:

```yaml
      - name: assert no framework/db/executor crates in codlet
        run: |
          set -euo pipefail
          tree="$(cargo tree -p codlet -e normal --prefix none | sort -u)"
          echo "$tree"
          # a gate must fail when it cannot perform its check
          test -n "$tree"
          echo "$tree" | grep -q '^codlet ' || { echo "::error::unexpected cargo tree output"; exit 1; }
          if echo "$tree" | grep -Eiq '^(axum|tower|tower-http|sqlx|tokio|async-std|worker|hyper|reqwest)( |$)'; then
            echo "::error::forbidden runtime/db/framework crate found in codlet"
            exit 1
          fi
```

Apply `set -euo pipefail` to every other multi-command `run:` block in the
workflow as well (RFC-036 §3.5).

### 5.3 `test-examples` job

The examples are five standalone packages, not bins of one package. Replace the
job body with:

```yaml
      - run: cargo build -p axum_login_logout -p sqlite_quickstart -p sqlite_file -p key_rotation -p form_token_csrf
      - run: cargo run -p sqlite_quickstart
      - run: cargo run -p key_rotation
      - run: cargo run -p form_token_csrf
```

Build all five; run only these three. `axum_login_logout` binds a port and never
exits, and `sqlite_file` is a deliberate two-invocation flow — neither belongs in
a non-interactive job. This matches which examples the original job ran.

### 5.4 New job: MSRV verification

Add a job that proves the workspace builds on Rust 1.85.

**Critical pitfall:** `rust-toolchain.toml` pins `channel = "stable"` and takes
precedence over an installed default toolchain. A job that merely installs 1.85
will silently build with stable and pass for the wrong reason. Set
`RUSTUP_TOOLCHAIN`, which overrides `rust-toolchain.toml`, and assert the version
before building:

```yaml
  msrv:
    name: MSRV 1.85 build
    runs-on: ubuntu-latest
    env:
      RUSTUP_TOOLCHAIN: "1.85.0"
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@1.85.0
      - name: assert the MSRV toolchain is actually in use
        run: |
          rustc --version
          rustc --version | grep -q '1\.85\.' || { echo "::error::not building with 1.85"; exit 1; }
      - run: cargo check --workspace --all-targets
```

Use `cargo check`, not `cargo test`: the goal is to prove the code compiles on
the declared MSRV, not to re-run the suite on an old toolchain.

### 5.5 Pre-existing `codlet-sqlx` job failures — fix one, investigate one

Two jobs fail for reasons unrelated to the rename. They are in scope because
both are CI configuration, not library code.

**`test codlet-sqlx (SQLite adapter conformance)`** runs
`cargo test -p codlet-sqlx --all-features`. `--all-features` activates
`postgres-test`, which requires Docker via testcontainers — so a job named
"SQLite adapter conformance" compiles and runs PostgreSQL tests and dies in
`postgres_tests::postgres_admin_list_and_get`. Select the backend explicitly:

```yaml
      - run: cargo test -p codlet-sqlx --no-default-features --features sqlite
```

This mirrors the existing postgres job, which already selects its backend
explicitly, and it is the same misconception as finding 6 in RFC-036 §2.

**`test codlet-sqlx postgres adapter (RFC-034)`** also fails. Its command already
selects features correctly, so the cause is elsewhere — most likely the Docker
daemon or testcontainers setup on the runner. **Investigate and report; do not
guess and do not disable it.** If it needs a service container or a Docker
setup step, propose the change in your review request rather than applying an
unreviewed fix. If it turns out to indicate a real defect in the PostgreSQL
adapter, stop and escalate immediately — that adapter's conformance suite
carries the INV-5 single-winner claim test.

### 5.6 `SECURITY.md` corrections

1. The MSRV section currently claims enforcement that did not exist. Once §5.3
   lands the claim becomes true — keep the sentence, and name the job so a reader
   can find it (e.g. "enforced by the `msrv` job in `.github/workflows/ci.yml`").
2. Add a sentence recording that a release requires CI green on the release
   commit. v0.17.1 was published while seven CI jobs were failing; the release
   discipline currently has no clause that forbids this.
3. The "Release discipline" list names four commands, two of which
   (`cargo test --workspace --all-features`, `cargo clippy --workspace
   --all-features --all-targets`) cannot run in an environment without Docker:
   `--all-features` activates `codlet-sqlx`'s `postgres-test` feature, which
   requires testcontainers. Replace the list with the gate set CI actually runs,
   and mark the PostgreSQL job as Docker-dependent. Do not change CI to match
   the old text — the old text is the thing that is wrong.

## 6. Required tests

No new Rust tests. The deliverable is that the existing suite actually executes.

## 7. Required documentation updates

`SECURITY.md` per §5.4, and a `CHANGELOG.md` `[Unreleased]` entry describing the
CI repair and the new MSRV gate.

## 8. Acceptance criteria

1. Every job in `ci.yml` names a package that exists in `cargo metadata`.
2. All jobs pass on `main`.
3. The `core-deps` gate is proven to work, in **two** trials, both recorded:
   - **Forbidden crate.** Add `tokio` as a normal dependency of `crates/codlet`,
     run the job's command, confirm it reports the forbidden crate, then revert
     completely (`git checkout -- crates/codlet/Cargo.toml`, restore
     `Cargo.lock`).
   - **Broken check.** Temporarily change `-p codlet` to `-p codlet-nonexistent`
     and confirm the job now **fails** rather than passing green. This is the
     exact regression that went undetected for two releases; without this trial
     the fix is unverified.

   A gate nobody has seen fail is not a verified gate.
4. The `msrv` job prints a `rustc 1.85.x` version line in its log.
5. `SECURITY.md` describes only commands that can actually be run.

## 9. Prohibited shortcuts

- Do not delete, rename away, or `continue-on-error` a job to make CI green. If
  a job cannot pass, report it — a removed gate is worse than a red one.
- Do not add `--all-features` at workspace level to make the docs match; it pulls
  in the Docker-dependent postgres path.
- Do not touch library code. A compile error in `crates/` under this handoff is
  an escalation, not a fix.
- Do not rewrite CHANGELOG history.
- Do not report acceptance criterion 3 as met without pasting the real failure
  output.

## 10. Security constraints

Nothing in this handoff may weaken the five `xtask` release gates, the eight
threat-model invariants (INV-1…8), or the core dependency policy. The point is
to restore enforcement, not to renegotiate it.

## 11. Known risks

| Risk | Mitigation |
|------|------------|
| A repaired job fails for a real, pre-existing reason | Already materialised in the two `codlet-sqlx` jobs (§5.5). Report; do not fix library code here. |
| The postgres job failure turns out to be an adapter defect, not a runner issue | Escalate immediately — that suite carries the INV-5 single-winner test |
| The MSRV job passes while secretly using stable | Guarded by the explicit `rustc --version` assertion in §5.3 |
| Reverting the criterion-3 trial leaves `Cargo.lock` dirty | Check `git status` is clean before opening the review |

## 12. Required evidence

- The full diff.
- CI run URL, or the complete local output of every job command.
- The criterion-3 trial: the command run, the failure output, and `git status`
  showing a clean tree afterwards.
- `rustc --version` line from the MSRV job.

## 13. Required review-request format

Follow §9.2 of `ai-multi-agent-software-development-organization-and-workflow.md`:
implementation summary, addressed requirements, changed files, important
decisions, differences from this handoff, executed tests, results, build and
static-analysis results, unresolved issues, known limitations, requested review
focus.

Report to the owner only the path of this file and your review-request file.

File the review request at
`.git-exclude/review-request/036-gate-integrity-ci-conformance-and-msrv.md`.
The architect's review result is returned at
`.git-exclude/reviewed/036-gate-integrity-ci-conformance-and-msrv.md`.
Review artefacts stay out of `rfcs/handoffs/`, which holds design companions
only (RFC-035 §3.5).
