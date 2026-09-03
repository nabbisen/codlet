# Implementation Handoff — Release 0.18.0

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M4 close-out (`ROADMAP.md`)
- **Governing RFCs:** RFC-035, RFC-036, RFC-037, RFC-038 (all `done/`); release discipline per `SECURITY.md`
- **Owner approval:** version 0.18.0 confirmed 2026-09-03

Note: this handoff governs a release, not an RFC, so it sits under
`rfcs/handoffs/release-0.18.0/` rather than an `NNN-slug` directory. It is the
one exception to the RFC-numbered convention; do not generalise it.

## 1. Purpose

Publish the M4 work as 0.18.0.

## 2. Background

M4 is complete: CI run 33736488609 on `a543fde`, 19/19 green. Four RFCs are in
`done/` carrying `Status: Implemented (Unreleased)` because the version was
undecided at the time. The owner has now confirmed **0.18.0**.

**Why a minor and not 0.17.2:** SECURITY.md states "MSRV is never raised in a
patch release", and `codlet-sqlx`'s declared MSRV rose from 1.85 to 1.94 under
decision D-1. A patch release is not permissible.

## 3. Change scope

- `Cargo.toml` — `[workspace.package].version`, and the two internal dependency
  pins in `[workspace.dependencies]` (`codlet`, `codlet-sqlx`)
- `CHANGELOG.md` — convert `[Unreleased]` to the released heading
- `rfcs/done/03{5,6,7,8}-*.md` — four Status lines
- `rfcs/README.md` — Version column for those four rows

## 4. Non-change scope

- No code, no CI, no docs prose. If anything under `crates/` needs to change,
  this is not a release — stop and escalate.
- No RFC body content beyond the four Status lines.
- No new CHANGELOG content. The entries are written; you are dating them.
- **Do not yank any prior version.** Standing policy, SECURITY.md — 0.18.0
  supersedes the defective releases and that is the whole remedy.

## 5. Required implementation

### 5.1 Version

`Cargo.toml`: `[workspace.package].version` → `0.18.0`, and the `codlet` and
`codlet-sqlx` entries in `[workspace.dependencies]` → `0.18.0`. Both must move
together or the workspace will not resolve for publication.

### 5.2 CHANGELOG

Change `## [Unreleased]` to `## [0.18.0] — <release date>`, and add a fresh
empty `## [Unreleased]` above it. Do not edit the existing entry text.

### 5.3 RFC statuses (the recorded release-time task)

In each of `rfcs/done/035-`, `036-`, `037-`, `038-`:

```
- **Status:** Implemented (Unreleased)   →   - **Status:** Implemented (v0.18.0)
```

Update the Version column for those four rows in `rfcs/README.md` to `v0.18.0`.
Then re-run acceptance criterion 3 from the RFC-035 handoff — the link check —
to confirm the index is still intact.

### 5.4 Release procedure

Follow SECURITY.md's release discipline, which now requires **CI green on the
release commit** — that clause was added in M4 precisely because v0.17.1 shipped
red. Then the clean-room verification and the publish sequence from the project's
established procedure:

```sh
cargo publish -p codlet
# wait until 0.18.0 is visible in the crates.io index, then:
cargo publish -p codlet-sqlx
```

`codlet-worker` and `codlet-conformance` are `publish = false`; a bare root
`cargo publish` is unsupported by design (DEC-004).

**Stop before `cargo publish` and report.** Prepare and verify everything, push
the release commit, confirm CI green, and hand back for the release-readiness
check. Publication is irreversible under the no-yank policy — that policy makes
the pre-publish gate the only gate there is.

## 6. Required tests

The full CI suite on the release commit, green. Plus the clean-room tarball
verification the project requires before every release.

## 7. Acceptance criteria

1. `Cargo.toml` version and both internal pins read `0.18.0`.
2. `CHANGELOG.md` has a dated `[0.18.0]` section and a fresh empty
   `[Unreleased]`.
3. All four M4 RFCs read `Implemented (v0.18.0)`; `rfcs/README.md` agrees.
4. RFC-035 acceptance criterion 3 (index links) still passes — 39 links, none
   broken.
5. CI green on the release commit, run URL recorded.
6. Clean-room verification passed.
7. Nothing published yet.

## 8. Prohibited shortcuts

- Do not publish. Hand back first.
- Do not yank anything.
- Do not edit CHANGELOG entry text while dating it.
- Do not bump the version in only one of the two places in `Cargo.toml`.

## 9. Required evidence

Diff; CI run URL for the release commit; clean-room verification output;
`cargo publish --dry-run -p codlet` and `-p codlet-sqlx` output.

## 10. Required review-request format

Per §9.2 of the workflow document. File at
`.git-exclude/review-request/release-0.18.0.md`; my result returns at
`.git-exclude/reviewed/release-0.18.0.md`.
