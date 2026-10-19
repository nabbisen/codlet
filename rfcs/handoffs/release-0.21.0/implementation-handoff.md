# Implementation Handoff — Release 0.21.0

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-20
- **Governing RFCs:** 049, 050, 051, 052, 053, 054, 055, 056, 057, 058 (all `done/`)
- **Owner decisions:** 0.21.0 authorized (2026-10-20); breaking changes accepted (2026-09-06); no yank.
- **Baseline:** `56835e5`

## 1. Purpose

Publish six weeks of accumulated work as 0.21.0. Ten RFCs, four of them
security fixes. **No code changes** — if something under `crates/` needs to
change, this is not a release.

## 2. Why a minor

Four breaking changes, each already carrying a CHANGELOG migration line:

1. **RFC-050** — four renames and three signatures:
   `find_redeemable` → `lookup_code`, `RedeemableCode` → `StoredCode`,
   `find_active_session` → `lookup_session`, `ActiveSessionRecord` →
   `StoredSession`; `now` removed from both lookups;
   `classify_code_lookup` now takes `&StoredCode`.
2. **RFC-055** — `RedeemError::Misconfigured` added. `RedeemError` is not
   `#[non_exhaustive]`, so an exhaustive match without a wildcard stops
   compiling. Also behavioural: a configured policy with `rate_key: None` now
   errors instead of silently skipping the limiter.
3. **RFC-053** — `StaticKeyProvider::new` rejects keys under 32 bytes, in both
   the active and previous positions. A host with a shorter key now fails at
   construction. `CodeAuth::without_rate_limit` is deprecated (a warning, not a
   break).
4. **RFC-052** — `postgres-test` no longer pulls `testcontainers-modules`
   (it moved to `[dev-dependencies]`). A consumer who enabled that feature to
   get the Docker tooling no longer receives it.

## 3. Change scope

- `Cargo.toml` — `[workspace.package].version`, and the `codlet` and
  `codlet-sqlx` pins in `[workspace.dependencies]` → `0.21.0` (three places;
  there are no other own-crate pins — check rather than trust me)
- `CHANGELOG.md` — convert `[Unreleased]` to `[0.21.0] — <date>`, with a fresh
  empty `[Unreleased]` above it
- `rfcs/done/{049,050,051,052,053,054,055,056,057,058}-*.md` — Status →
  `Implemented (v0.21.0)`, keeping any commit references already there
- `rfcs/README.md` — the Version column for those ten rows, `*(Unreleased)*` →
  `v0.21.0`

## 4. Non-change scope

- **Any code.** See §1.
- **Do not yank any version.**
- Do not edit the `[Unreleased]` entries' wording. They were reviewed as
  written; if one looks wrong, report rather than rewrite.
- Do not touch RFC-059 — it is process policy and ships with nothing.
- Do not add a `v` prefix to the tag.

## 5. CHANGELOG

Everything currently in `[Unreleased]` ships. There is nothing to hold back, so
this is a heading conversion plus a fresh empty section — as for 0.20.0, not as
for 0.19.1.

Before dating it, check that **all four breaking changes in §2 are findable by
someone scanning for breaks**, each with its migration line. Report a gap; do
not fill it silently.

One thing to verify rather than assume: the RFC-048 entry under its own `0.19.1`
heading must stay where it is.

## 6. Release procedure

Unchanged:

1. CI green on the release commit.
2. Clean-room verification via `git archive`.
3. Tag `0.21.0` — annotated, no `v` prefix, message `0.21.0` — and push it.
4. **`release-gates.yml` green on the tag** — blocking. This runs
   `cargo deny check advisories`, which is the gate that caught the four
   `rustls` advisories RFC-052 cleared; it must pass against the tagged commit.
5. `cargo publish --dry-run -p codlet`.
6. **Stop and hand back. Do not publish.**

`codlet-sqlx`'s dry-run will fail to resolve `codlet ^0.21.0` until `codlet` is
published. Structural, expected, documented since 0.18.0 — do not try to fix it.

## 7. Required verification

| Check | Expectation |
|---|---|
| `grep -rn '0\.20\.0' Cargo.toml` | no hits |
| `grep -c 'Unreleased' rfcs/README.md` | 0 |
| `grep -rn 'Unreleased' rfcs/done/` | no hits for the ten RFCs |
| `cargo test --workspace` | green, 264 or higher |
| `cargo build --workspace` | green, lockfile unchanged except the version lines |
| `cargo run -p xtask -- release-check` | 5 gates ok |
| `cargo deny check advisories bans licenses sources` | all ok |
| CI on the release commit | all blocking jobs green, 25 jobs |
| `release-gates.yml` on the tag | green |

`Cargo.lock` will change — its own `codlet`/`codlet-sqlx`/`codlet-worker`
version entries. That is expected; anything else changing in the lockfile is
not, and should be reported.

## 8. Acceptance criteria

1. `0.21.0` in all three manifest places, and nowhere is `0.20.0` left.
2. `CHANGELOG.md` has `[0.21.0] — <date>` and an empty `[Unreleased]`.
3. All ten RFCs read `Implemented (v0.21.0)`; `rfcs/README.md` agrees.
4. No file under `crates/` changed.
5. The annotated tag `0.21.0` exists and is pushed; `release-gates.yml` green
   on it.
6. `cargo publish --dry-run -p codlet` succeeds.
7. Nothing published to crates.io.

## 9. Required evidence

`git diff --stat`; the four §2 breaking changes quoted from the CHANGELOG as
they will ship; each §7 check with its exit status; the `release-gates.yml` run
URL for the tag; the dry-run output. Confirm explicitly that nothing under
`crates/` changed.

## 10. Review request

`.git-exclude/review-request/release-0.21.0.md`; result returns at
`.git-exclude/reviewed/release-0.21.0.md`.
