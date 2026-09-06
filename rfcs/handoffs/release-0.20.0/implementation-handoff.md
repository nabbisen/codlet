# Implementation Handoff — Release 0.20.0

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-06
- **Governing RFCs:** RFC-044, RFC-045, RFC-046, RFC-047 (all `done/`)
- **Owner decisions:** breaking changes accepted (2026-09-06); no yank.

## 1. Purpose

Publish the M6 session-lifecycle work as 0.20.0.

## 2. Why a minor

Two breaking changes to the public API:

1. `SessionValidationOutcome::Unauthenticated` is now a struct variant carrying
   `reason` (RFC-046) — exhaustive matches must change.
2. `SessionValidationOutcome::Authenticated` is now `#[non_exhaustive]`
   (RFC-045) — external construction is blocked; exhaustive patterns need `..`.

Both are accepted pre-v1. Both already have migration lines in the CHANGELOG.

## 3. Change scope

- `Cargo.toml` — `[workspace.package].version` and both `[workspace.dependencies]` pins → `0.20.0`
- `CHANGELOG.md` — convert `[Unreleased]` to `[0.20.0] — <date>`, fresh empty `[Unreleased]` above
- `rfcs/done/{044,045,046,047}-*.md` — Status → `Implemented (v0.20.0)`
- `rfcs/README.md` — Version column for those four rows

## 4. Non-change scope

- Any code. If something under `crates/` needs to change, this is not a release.
- **Do not yank any version.**
- The RFC-048 entry already released as 0.19.1 — leave it under its own heading.

## 5. CHANGELOG

`[Unreleased]` currently holds RFC-044, 045, 046, 047 and the two RFC-048
follow-ups. **All of it ships in 0.20.0** — unlike 0.19.1, there is nothing here
to hold back, so this is a straight heading conversion plus a fresh empty
`[Unreleased]`.

Check before dating: **both breaking changes must be findable by someone
scanning for breaks**, each with its migration line. They were reviewed as
written; if either has drifted, report rather than rewrite.

## 6. Release procedure

Unchanged, with both gates:

1. CI green on the release commit.
2. Clean-room verification via `git archive`.
3. Tag `0.20.0` (annotated, no `v` prefix, message `0.20.0`), push.
4. **`release-gates.yml` green on the tag** — blocking.
5. `cargo publish --dry-run -p codlet`.
6. **Stop and hand back.** Do not publish.

`codlet-sqlx`'s dry-run will fail to resolve `codlet ^0.20.0` until `codlet` is
published — structural, expected, documented since 0.18.0.

## 7. Acceptance criteria

1. `0.20.0` in all three places in `Cargo.toml`.
2. CHANGELOG dated; fresh empty `[Unreleased]`; both breaking changes legible
   with migration lines.
3. RFC-044/045/046/047 read `Implemented (v0.20.0)`; index agrees; link check
   passes.
4. CI green on the release commit, URL recorded.
5. `release-gates.yml` green on the tag, URL recorded.
6. Clean-room verification passed.
7. `cargo publish --dry-run -p codlet` clean.
8. Nothing published; nothing yanked.

## 8. Required evidence

Diff; both CI run URLs; clean-room output; dry-run output; index link check.

## 9. Review request

`.git-exclude/review-request/release-0.20.0.md`; my result returns at
`.git-exclude/reviewed/release-0.20.0.md`.
