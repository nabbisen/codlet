# Implementation Handoff — RFC-049 Supply-Chain Carried Items

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-07
- **Milestone:** M7 preamble — carried from M5
- **Governing RFC:** [`../../accepted/049-supply-chain-carried-items.md`](../../accepted/049-supply-chain-carried-items.md)

Read RFC-049 before starting, **§4 in particular**. If execution conflicts with
it, **stop and escalate**.

## 1. Purpose

Close RFC-039 §8.1's two deferred questions, and add the one check the audit
showed was worth adding.

## 2. Change scope

- `Cargo.lock` — two dependency updates, §4.1
- `deny.toml` — a comment recording the settled decision, §4.2
- `.github/workflows/ci.yml` — the `core-deps` extension, §4.3
- `docs/src/adapter-matrix-and-config.md` — `codlet-sqlx`'s duplicate crypto
  generations, §4.4
- `CHANGELOG.md`

## 3. Non-change scope

- **`deny.toml`'s `bans.multiple-versions` value.** It stays `warn`. Only a
  comment is added. If you find yourself adding `skip` entries, stop — that is
  the outcome RFC-049 §3 rejects.
- **`deny.toml`'s licences, sources, or advisories sections.**
- **Any dependency version other than `chacha20` and `spin`**, and no
  `Cargo.toml` dependency changes at all.
- **No attempt to deduplicate `codlet-sqlx`.** Its duplicates are upstream
  (RFC-049 §4); documenting them is the whole of the work here.
- No sixth `xtask` gate. This extends `core-deps`, which lives in `ci.yml`.

## 4. Required implementation

### 4.1 The yanked crates

```sh
cargo update -p chacha20 -p spin
```

Expected: `chacha20 0.10.0 → 0.10.2`, `spin 0.9.8 → 0.9.9`. **Confirm nothing
else moved** — `git diff Cargo.lock` should show only these two crates'
version and checksum lines. If other packages shifted, revert and report; that
would mean the update resolved more broadly than intended.

Then confirm `cargo deny check advisories` reports no yanked warnings.

### 4.2 `deny.toml` — record the decision, change no value

Add a comment at `bans.multiple-versions` stating: the graph was audited under
RFC-049 (12 duplicates, workspace-wide, listed in that RFC); moving to `deny`
would require twelve `skip` entries, which RFC-039 §3.2 identifies as a gate
switched off while appearing to run; the question is **settled at `warn`**, not
deferred again.

The point is that the next reviewer inherits the evidence instead of repeating
the audit. Write it for them.

### 4.3 Extend `core-deps` — `codlet` only

The gate already asserts no framework/database/executor crate has entered
`cargo tree -p codlet -e normal`. Add: **that tree carries no duplicate
versions.**

Keep `set -euo pipefail` and the existing non-empty and root-node assertions
(RFC-036 §3.5) — the new check must not become the reason a future failure goes
unnoticed. A working shape:

```sh
dupes="$(echo "$tree" | sed 's/ (\*)$//' | grep -E '^[a-zA-Z0-9_-]+ v' \
  | sort -u | awk '{print $1}' | uniq -d)"
if [ -n "$dupes" ]; then
  echo "::error::duplicate versions in codlet's dependency tree: $dupes"
  exit 1
fi
```

**Scoped to `codlet` only.** Do not extend it to `codlet-sqlx` — that fails
today for an upstream reason (§4.4), and a check that fails on introduction is
the trap RFC-039 §3.2 describes.

### 4.4 Document `codlet-sqlx`'s duplicate crypto generations

In `docs/src/adapter-matrix-and-config.md`, record that `codlet-sqlx` builds two
generations of the crypto stack — `sha2`, `digest`, `crypto-common`,
`block-buffer` — because `sqlx` 0.9 depends on the 0.10 generation while codlet
uses 0.11.

State plainly what it is and is not: **compile-time weight, not a vulnerability.**
Both generations are maintained; codlet's own HMAC path uses only the newer.
Name the condition that resolves it — `sqlx` moving to the 0.11 generation — so
a future reader knows what to watch for rather than re-deriving it.

Do not describe it as a defect in codlet. It is a fact about a dependency.

## 5. Prove the new check can fail

Introduce a duplicate into `codlet`'s normal tree, confirm `core-deps` fails,
revert, confirm clean.

A dependency pulling the older digest generation works — `sha1 = "0.10"` depends
on `digest 0.10` while codlet uses `digest 0.11`. Any equivalent is fine; what
matters is that the duplicate is real and the gate catches it.

Record the output. A check added to a gate without being seen to fire is the
defect class this project has spent two milestones removing.

**Revert completely.** `Cargo.toml` and `Cargo.lock` must show only §4.1's two
updates when you are done.

## 6. Acceptance criteria

1. `chacha20` and `spin` updated; **only those two** changed in `Cargo.lock`
   beyond §5's reverted trial; no yanked warnings remain.
2. `bans.multiple-versions` still `warn`; comment records the audit and that the
   question is settled.
3. `core-deps` asserts no duplicates in `codlet`'s tree, scoped to `codlet`, with
   `set -euo pipefail` and the existing assertions intact.
4. The new check observed failing against a real duplicate; output recorded;
   trial fully reverted.
5. `codlet-sqlx`'s duplicate generations documented as weight, not defect, with
   the resolving condition named.
6. No published crate's behaviour changed.
7. Full CI green.

## 7. Prohibited shortcuts

- Do not add `skip` entries to `deny.toml`.
- Do not change `multiple-versions` to `deny`.
- Do not extend the duplicate check to `codlet-sqlx`.
- Do not pin `sha2`/`digest` backwards to deduplicate anything.
- Do not skip §5.

## 8. Required evidence

`git diff Cargo.lock` showing only the two updates; `cargo deny check
advisories` clean; the §5 trial output and the clean revert; `core-deps` passing
afterward; CI run URL.

## 9. Review request

`.git-exclude/review-request/049-supply-chain-carried-items.md`; my result
returns at `.git-exclude/reviewed/049-supply-chain-carried-items.md`.
