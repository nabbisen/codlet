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

codlet requires **Rust 1.85** or later (edition 2024). The MSRV is set in
`Cargo.toml` under `[workspace.package]` and is enforced by CI.

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

Every release must pass:
1. `cargo test --workspace --all-features`
2. `cargo clippy --workspace --all-features --all-targets -- -D warnings`
3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
4. `cargo run -p xtask -- release-check` (5 static security gates)

The gates and their rationale are documented in `xtask/src/main.rs`.
