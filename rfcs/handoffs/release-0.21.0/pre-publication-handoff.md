# Pre-Publication Handoff — Release 0.21.0

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-20
- **Review result:** `.git-exclude/reviewed/release-0.21.0.md`
- **Publication:** authorized by the owner (2026-10-20). The owner runs the publish commands; you prepare and verify.

## 1. Purpose

Three fixes to the 0.21.0 release notes and front page, then move the tag, then
re-run the release gate. The release mechanics are already approved.

## 2. CHANGELOG — three edits, wording supplied

Apply these exactly. Each `old` string appears once in `CHANGELOG.md`.

### 2.1 RFC-055: label the break

old:
```
  (RFC-055). Behavioural change, pre-v1.** `CodeAuth::find` and `claim` applied
```
new:
```
  (RFC-055). Breaking and behavioural, pre-v1.** `CodeAuth::find` and `claim` applied
```

### 2.2 RFC-053: say what a host must do

old:
```
  not check). `KeyError::InvalidKeyMaterial`'s message now names the minimum.
```
new:
```
  not check). `KeyError::InvalidKeyMaterial`'s message now names the minimum.
  **A host whose key is shorter now fails at construction, not at first use:
  supply at least 32 bytes.**
```

### 2.3 RFC-052: label the break and say what to do

old:
```
- **`codlet-sqlx`'s `postgres-test` feature no longer pulls any dependency into a
  consumer's graph (RFC-052).** `testcontainers-modules` — and with it a Docker
```
new:
```
- **`codlet-sqlx`'s `postgres-test` feature no longer pulls any dependency into a
  consumer's graph (RFC-052). Breaking, pre-v1.** `testcontainers-modules` — and with it a Docker
```

and, in the same entry, old:
```
  dev-dependency, and `postgres-test = ["postgres"]` only gates the
  Docker-backed tests.
```
new:
```
  dev-dependency, and `postgres-test = ["postgres"]` only gates the
  Docker-backed tests. **If you enabled `postgres-test` to obtain
  `testcontainers-modules`, add it to your own `[dev-dependencies]`.**
```

If any `old` string does not match exactly — line wrapping, a stray space —
**stop and report** rather than approximating. I built these from the file at
`5cb6412`; if it has moved, so have they.

## 3. README — five version references

`README.md`, lines 79–96 at `5cb6412`:

| Line | From | To |
|---|---|---|
| 79 | `codlet = "0.20"` | `codlet = "0.21"` |
| 82 | `# codlet-sqlx = "0.20"` | `# codlet-sqlx = "0.21"` |
| 83 | `# codlet-sqlx = { version = "0.20", … }` | `"0.21"` |
| 93 | `tag = "0.20.0"` | `tag = "0.21.0"` |
| 96 | `tag = "0.20.0"` | `tag = "0.21.0"` |

Lines 93 and 96 are the `codlet`/`codlet-worker` git examples from the RFC-016
sweep; they must name the tag this release creates, which is why this has to
happen before the tag is moved, not after.

Check for any other version-bearing line I have missed — `grep -rn '0\.20'`
across `README.md`, `docs/src/` and the crate READMEs — and report anything you
find rather than guessing whether it is deliberate. The crate READMEs carried no
version numbers at `5cb6412`; confirm that is still true.

## 4. Move the tag

My call, overriding your correct general instinct. Nothing has consumed this
tag: nothing is on crates.io and the README's own examples still point at
0.20.0. See the review result §4.

1. Commit §2 and §3 together on `main`, message along the lines of
   `Release 0.21.0: label the remaining breaking changes; bump README versions`.
2. CI green on that commit.
3. Move the tag to it — annotated, message `0.21.0`, no `v` prefix — and
   force-push it.
4. **`release-gates.yml` green on the moved tag.** Not optional: the gate must
   be green on the commit that actually ships.
5. `cargo publish --dry-run -p codlet` clean again.

If moving the tag fails or looks wrong at any step, stop and report. A
half-moved tag is worse than the mismatch we are fixing.

## 5. Then hand back for publication

The owner publishes, in this order:

```
cargo publish -p codlet
# wait for the index to update, then
cargo publish -p codlet-sqlx
```

`codlet-worker` is unpublished by design; `codlet-conformance` is
`publish = false`. Two crates.

Leave the commands ready and state plainly that nothing has been published yet.
Do not run them.

## 6. Non-change scope

- No other CHANGELOG wording. The three edits in §2 and nothing else.
- No code. If a change under `crates/` appears necessary, stop — it is not a
  release.
- No yank.
- No attempt to fix `codlet-sqlx`'s dry-run resolution failure.
- Do not create a `0.21.1` or any other tag.

## 7. Required verification

| Check | Expectation |
|---|---|
| `grep -c 'Breaking' CHANGELOG.md` within the `[0.21.0]` section | 4 — one per breaking change |
| `grep -rn '0\.20' README.md` | no hits |
| `grep -rn '0\.20\.0' Cargo.toml` | no hits |
| `git cat-file -t 0.21.0` | `tag` |
| `git rev-list -n1 0.21.0` | the new commit |
| CI on the new commit | 25 jobs green |
| `release-gates.yml` on the moved tag | green |
| `cargo publish --dry-run -p codlet` | clean |
| `cargo test --workspace` | 264 or higher |

The first check is the point of §2: four breaking changes, four labels, findable
by someone scanning.

## 8. Acceptance criteria

1. The three §2 edits applied verbatim; no other entry wording changed.
2. No `0.20` anywhere in `README.md`.
3. Tag `0.21.0` is annotated and points at the new commit.
4. `release-gates.yml` green **on that tag**.
5. `cargo publish --dry-run -p codlet` clean.
6. Nothing published; the two commands handed back ready to run.
7. Nothing under `crates/` changed.

## 9. Required evidence

`git diff` for the CHANGELOG and README (small enough to quote in full); the
`[0.21.0]` section's four "Breaking" labels quoted; `git rev-list -n1 0.21.0`
before and after; the CI and `release-gates.yml` run URLs for the moved tag; the
dry-run output; explicit confirmation that nothing was published.

## 10. Review request

`.git-exclude/review-request/release-0.21.0-prepublication.md`; result returns
at `.git-exclude/reviewed/release-0.21.0-prepublication.md`.
