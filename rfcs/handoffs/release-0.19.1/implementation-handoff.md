# Implementation Handoff — Release 0.19.1 (Security Patch)

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-05
- **Governing RFC:** RFC-048 (`done/` on release)
- **Owner decisions:** 0.19.1 authorised; **no yank**; advisory wording option (a); advisory published with the release

Read the decision record at
`.git-exclude/reviewed/DECISION-REQUEST-rfc-048-release-and-disclosure.md`
before starting.

## 1. Purpose

Publish the RFC-048 security fix as 0.19.1.

## 2. Keep this release minimal

**Nothing but the version mechanics.** No test additions, no refactors, no
opportunistic cleanups, however small.

A security release is small and fast on purpose: every extra change is another
thing that can fail CI and delay disclosure while a confirmed-exploitable
vulnerability sits unpatched on crates.io. The conformance-suite hardening
identified in the decision record is queued as a **separate follow-up**, not
part of this.

## 3. Change scope

- `Cargo.toml` — `[workspace.package].version` and both `[workspace.dependencies]` pins → `0.19.1`
- `CHANGELOG.md` — convert `[Unreleased]` to `[0.19.1] — <date>`, fresh empty `[Unreleased]` above
- `rfcs/accepted/048-*.md` → `rfcs/done/`, Status → `Implemented (v0.19.1)`
- `rfcs/README.md` — move RFC-048 from Accepted to Implemented, version `v0.19.1`

## 4. Non-change scope

- Any code. If something under `crates/` needs to change, this is not a release.
- **Do not yank any version.** Owner decision, recorded.
- RFC-044/046/047, which remain `Implemented (Unreleased)` — they ship with a
  later release, not this one. **Do not sweep them into this version.** This
  release is the security fix; conflating it with unrelated M6 work makes the
  advisory harder to read and the diff harder to audit.

## 5. CHANGELOG wording — this one is load-bearing

The `### Security` entry is what a reader diffs the advisory against. It must
state exactly what is confirmed and exactly what is not:

- SQLite: **confirmed exploitable** before this fix.
- D1: **confirmed exploitable** before this fix.
- PostgreSQL: **affected by the same defect; exploitability not confirmed.**
  Upgrade regardless.

Do not write that PostgreSQL was unaffected. That claim rests on analysis, not a
test, and the owner chose the conservative wording precisely so no operator
concludes they were safe.

If the existing entry already says this, leave it and say so — the entry was
reviewed as written.

## 6. Release procedure

Unchanged from 0.19.0, with both gates:

1. CI green on the release commit.
2. Clean-room verification via `git archive`.
3. Tag `0.19.1` (annotated, no `v` prefix, message `0.19.1`), push.
4. **`release-gates.yml` green on the tag** — blocking.
5. `cargo publish --dry-run -p codlet`.
6. **Stop and hand back.** Do not publish.

`codlet-sqlx`'s dry-run will fail to resolve `codlet ^0.19.1` until `codlet` is
published — structural, expected, documented at 0.18.0 and 0.19.0.

## 7. Acceptance criteria

1. Version `0.19.1` in all three places.
2. CHANGELOG dated, `### Security` entry stating confirmed vs. unconfirmed per §5.
3. RFC-048 in `done/` at `Implemented (v0.19.1)`; index agrees; link check passes.
4. RFC-044/046/047 still `Implemented (Unreleased)` — untouched.
5. CI green on the release commit, URL recorded.
6. `release-gates.yml` green on the tag, URL recorded.
7. Clean-room verification passed.
8. `cargo publish --dry-run -p codlet` clean.
9. Nothing published; nothing yanked.

## 8. Required evidence

Diff; both CI run URLs; clean-room output; dry-run output; index link check.

## 9. Review request

`.git-exclude/review-request/release-0.19.1.md`; my result returns at
`.git-exclude/reviewed/release-0.19.1.md`.
