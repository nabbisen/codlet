# Implementation Handoff — Release 0.19.0

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-04
- **Milestone:** M5 close-out
- **Governing RFCs:** RFC-039, 040, 041, 042, 043 (all `done/`); release discipline per `SECURITY.md`
- **Owner approval:** release-before-M6 confirmed 2026-09-04

Release handoffs sit under `rfcs/handoffs/release-<version>/` rather than an
`NNN-slug` directory — the one exception to the RFC-numbered convention, as with
0.18.0.

## 1. Purpose

Publish M5 as 0.19.0.

## 2. Background

M5 is complete: CI run 33835821327 on `03d0924`, 22/22 green. Five RFCs are in
`done/` at `Status: Implemented (Unreleased)`.

**Unlike 0.18.0, nothing forces this version number.** There is no MSRV change,
and RFC-043's tightened `Alphabet::new` validation rejects only configurations
that were already broken. 0.19.0 is the owner's deliberate choice, taken so that
M6 — the first milestone to change shipped behaviour substantially — gets its
own release boundary.

**This is the first release with `release-gates.yml` in place.** The
tag-triggered advisory check did not exist at 0.18.0. See §5.5.

## 3. Change scope

- `Cargo.toml` — `[workspace.package].version` and both `[workspace.dependencies]` pins
- `CHANGELOG.md` — convert `[Unreleased]` to the released heading
- `rfcs/done/{039,040,041,042,043}-*.md` — five Status lines
- `rfcs/README.md` — Version column for those five rows
- `SECURITY.md` — one gate-list correction, §5.4

## 4. Non-change scope

- No code, no CI workflow changes. If anything under `crates/` needs to change,
  this is not a release — stop and escalate.
- No RFC body content beyond the five Status lines.
- No new CHANGELOG content; you are dating entries that are already written.
- **Do not yank any prior version.** Standing policy, SECURITY.md.

## 5. Required implementation

### 5.1 Version

`Cargo.toml`: `[workspace.package].version` → `0.19.0`, and the `codlet` and
`codlet-sqlx` entries in `[workspace.dependencies]` → `0.19.0`. Both must move
together or the workspace will not resolve for publication.

### 5.2 CHANGELOG

`## [Unreleased]` → `## [0.19.0] — <release date>`, with a fresh empty
`## [Unreleased]` above it. Do not edit the entry text.

Check before dating: **RFC-043's entry must be legible as a breaking change** to
`Alphabet::new`'s accepted input. A reader scanning for API-behaviour changes
should find it without inference. If the existing wording buries it, say so in
your review request rather than rewriting it unilaterally — it was reviewed as
written.

### 5.3 RFC statuses

In each of `rfcs/done/039-`, `040-`, `041-`, `042-`, `043-`:

```
- **Status:** Implemented (Unreleased)   →   - **Status:** Implemented (v0.19.0)
```

Update the Version column for those five rows in `rfcs/README.md` from
`*(Unreleased)*` to `v0.19.0`. Then re-run the index link check — 44 links, none
broken.

### 5.4 `SECURITY.md` — a gate-list correction

The release-discipline list enumerates 19 gates and **omits `self-test`**, the
job RFC-040 added. CI runs 21 jobs; the document describes 20 of them.

Add it — `cargo run -p xtask -- self-test`, which proves each of the four static
gates fails against a deliberate violation fixture (RFC-040 §3.2).

This is small and it is exactly the drift M5 exists to eliminate: a release is
measured against that list, so a list missing a gate understates what the
release passed.

### 5.5 Release procedure

Per SECURITY.md, with one step that is new since 0.18.0:

1. CI green on the release commit — push and confirm before tagging.
2. Clean-room verification via `git archive` of the release commit.
3. **Tag `0.19.0`** (annotated, no `v` prefix, message `0.19.0`) and push it.
4. **Wait for `release-gates.yml` to run and go green.** It triggers on the tag
   and runs `cargo deny check advisories` against the tagged commit. This is the
   gate that makes the non-blocking pull-request advisory policy safe, and this
   is the first release it applies to. **A red release-gates run blocks
   publication.**
5. `cargo publish --dry-run -p codlet`.

**Stop there and hand back.** Do not publish. Note that
`cargo publish --dry-run -p codlet-sqlx` will fail to resolve `codlet ^0.19.0`
until `codlet` is actually published — that is structural, expected, and was
documented at 0.18.0.

## 6. Required tests

Full CI suite green on the release commit, `release-gates.yml` green on the tag,
and the clean-room verification.

## 7. Acceptance criteria

1. `Cargo.toml` version and both internal pins read `0.19.0`.
2. `CHANGELOG.md` has a dated `[0.19.0]` and a fresh empty `[Unreleased]`.
3. All five M5 RFCs read `Implemented (v0.19.0)`; `rfcs/README.md` agrees.
4. Index link check passes — 44 links, none broken.
5. `SECURITY.md` lists `self-test`.
6. CI green on the release commit, run URL recorded.
7. **`release-gates.yml` green on the tag, run URL recorded.**
8. Clean-room verification passed.
9. `cargo publish --dry-run -p codlet` clean.
10. Nothing published.

## 8. Prohibited shortcuts

- Do not publish. Hand back first.
- Do not yank anything.
- Do not proceed past a red `release-gates` run — that gate exists precisely to
  stop a release carrying a known advisory.
- Do not edit CHANGELOG entry text while dating it.
- Do not bump the version in only one of the two places in `Cargo.toml`.

## 9. Required evidence

Diff; CI run URL for the release commit; **`release-gates.yml` run URL for the
tag**; clean-room output; `cargo publish --dry-run -p codlet` output; index link
check output.

## 10. Review request

`.git-exclude/review-request/release-0.19.0.md`; my result returns at
`.git-exclude/reviewed/release-0.19.0.md`.
