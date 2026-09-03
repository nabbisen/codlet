# Security Policy

codlet is a security-sensitive authentication library. This policy covers
vulnerability reporting, supported versions, disclosure process, and what
constitutes a security defect.

## Supported versions

codlet has not yet reached a stable (v1.0) release. No version currently
receives long-term security support. Security fixes are delivered in the latest
release only.

| Version | Supported |
|---------|-----------|
| 0.x (latest) | Fixes in next release |
| < latest | Not supported |

After v1.0, this table will follow semantic versioning: the current major
version receives security backports; prior majors do not.

## Minimum supported Rust version (MSRV)

`rust-version` is declared per crate, not once for the whole workspace — the
floor differs between the runtime-neutral core and the SQLx adapter:

| Crate | MSRV | Why |
|---|---|---|
| `codlet` | **Rust 1.85** (edition 2024) | `[workspace.package].rust-version`, inherited by `rust-version.workspace = true` |
| `codlet-sqlx` | **Rust 1.94** | Its own `sqlx` 0.9.0 dependency declares `rust-version = "1.94.0"`; `codlet-sqlx` declares this explicitly rather than inheriting the workspace default |

`codlet-conformance`, `codlet-worker`, and `xtask` also hold to 1.85
(dev/internal tooling, not published).

Enforced by two CI jobs in `.github/workflows/ci.yml`: `msrv` builds `codlet`,
`codlet-conformance`, `codlet-worker`, and `xtask` with
`RUSTUP_TOOLCHAIN=1.85.0`; `msrv-sqlx` builds `codlet-sqlx` with
`RUSTUP_TOOLCHAIN=1.94.0`. Both assert the active `rustc` version before
building, so neither can silently pass on the `stable` toolchain that
`rust-toolchain.toml` pins for local development.

MSRV policy:
- MSRV is never raised in a patch release.
- Before v1.0: MSRV may be raised in a minor release with a CHANGELOG note.
- After v1.0: MSRV is raised only in a major release.

## Reporting a vulnerability

Report suspected vulnerabilities privately via **GitHub's private
vulnerability reporting** on the repository, rather than opening a public issue.

Please include:
- affected crate and version or commit;
- a description of the vulnerability;
- a minimal reproduction if possible.

Do not include live secrets (HMAC keys, session cookies, plaintext codes) in
a report.

**Response targets (best effort, pre-v1):**
- Acknowledgement within 5 business days.
- Status update within 15 business days.
- Fix or mitigation within 30 business days for critical issues.

## Disclosure policy

Coordinated disclosure. We ask reporters to allow us to prepare a fix before
public disclosure. We will credit reporters unless they request anonymity.

Advisories will be published via the GitHub Security Advisory tab after a fix
is available.

## What constitutes a security bug

Non-exhaustive examples treated as security bugs (see also `docs/src/threat-model.md`):

- Plaintext persistence of a code, session secret, or form-token secret.
- Any fallback HMAC key, or HMAC operation succeeding with missing key material.
- RNG failure producing a deterministic or partial secret instead of an error.
- A code claim or form-token consume that can succeed more than once under
  concurrency.
- A redemption failure path that reveals whether a code exists, is expired,
  revoked, or already used (enumeration).
- A session cookie built without `HttpOnly`, `Secure`, or `SameSite` in a
  production policy.
- A secret value appearing in `Debug`/`Display` output, logs, or audit events.
- An adapter claiming conformance while failing the `codlet-conformance` suite.

## Release discipline

A release requires every job in `.github/workflows/ci.yml` to be green on the
release commit. Publishing while CI is red on that commit is a release-process
violation, regardless of what was run locally.

The gate set CI actually runs:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace` (Linux and macOS)
4. `cargo build -p codlet --target wasm32-unknown-unknown`
5. `cargo build -p codlet-worker --target wasm32-unknown-unknown`
6. Miniflare integration tests: `cd crates/codlet-worker/tests && npm ci && npx vitest run`
7. `cargo test -p codlet --test rfc_009_compile --all-features`
8. `cargo test -p codlet --all-features`
9. `cargo test -p codlet-conformance --all-features`
10. `cargo test -p codlet-sqlx --no-default-features --features sqlite`
11. `cargo test -p codlet-sqlx --no-default-features --features postgres-test`
    — **requires a Docker daemon** (testcontainers); this is the only gate
    that cannot run in a Docker-less environment.
12. Build and run the example packages (`test-examples` job)
13. Core feature matrix: `cargo build -p codlet` with `--no-default-features`,
    with defaults, and with `--features serde,test-utils`
14. Core dependency gate: asserts no framework, database, or async-executor
    crate has entered `codlet`'s dependency tree (RFC-002 §10.5)
15. MSRV: `cargo check -p codlet -p codlet-conformance -p codlet-worker -p xtask
    --all-targets` built on Rust 1.85 (`msrv` job); `cargo check -p codlet-sqlx
    --all-targets` built on Rust 1.94 (`msrv-sqlx` job) — see the MSRV table
    above for why the floor differs
16. `cargo run -p xtask -- release-check` (5 static security gates, documented
    in `xtask/src/main.rs`)
17. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`

There is no locally-runnable command that reproduces the full set without
Docker; gate 11 is the one exception and must be verified in an environment
that has it (CI does).
