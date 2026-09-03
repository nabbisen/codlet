# Implementation Handoff — RFC-039 Supply-Chain Scanning

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-03
- **Milestone:** M5 (`ROADMAP.md`, M5-1)
- **Governing RFC:** [`../../accepted/039-supply-chain-scanning.md`](../../accepted/039-supply-chain-scanning.md)
- **Priority:** may run in parallel with the RFC-040 handoff. No dependency between them.

Read RFC-039 before starting. If execution conflicts with it, **stop and
escalate** — a handoff may not override an RFC decision.

## 1. Purpose

Give codlet automated visibility into its 137-package dependency graph, which
currently has none.

## 2. Change scope

- `deny.toml` — **new**, workspace root
- `.github/workflows/ci.yml` — new job(s)
- `.github/workflows/release-gates.yml` — **new** (see §3.3)
- `SECURITY.md` — release-discipline list
- `CHANGELOG.md` — `[Unreleased]`

## 3. Required implementation

### 3.1 `deny.toml`

Four sections: `advisories`, `licenses`, `bans`, `sources`.

**Licences — allow exactly these**, derived in RFC-039 §3.2 from the real graph:

```
Apache-2.0, MIT, BSD-3-Clause, Unicode-3.0, Zlib, Unlicense, BSL-1.0
```

plus the `Apache-2.0 WITH LLVM-exception` variant.

Two traps RFC-039 §3.2 calls out, both of which will fail a copied allow-list:

- `Unicode-3.0` appears in 18 packages (the ICU crates reached via `sqlx`).
- Two expressions use `AND` (`MIT AND BSD-3-Clause`,
  `(MIT OR Apache-2.0) AND Unicode-3.0`) and require **every** named licence to
  be allowed, not just one.

**Sources:** restrict to the crates.io registry. No git or alternate registries
are in use today.

**Bans:** duplicate-version detection at `warn`, **not** `deny` — RFC-039 §3.4.
Do not add specific crate bans; there is no reason for one yet.

**Advisories:** no `ignore` entries. If the check fails on the current graph,
that is a finding to report, not something to silence.

### 3.2 It must pass on the day it lands

Before wiring CI, run `cargo deny check` locally against the current graph. If
anything fails, **stop and report** rather than adding an exception. RFC-039 §3.2
is explicit: a gate that fails on introduction gets disabled, and an allow-list
padded to force a pass is the same thing more slowly.

The licence data in RFC-039 §3.2 says it should pass. If it does not, either the
graph changed or the allow-list is wrong — both are worth knowing.

### 3.3 CI wiring — the split blocking policy

**Blocking on pull requests**, in `ci.yml`:

```yaml
  supply-chain:
    name: supply chain (bans, licenses, sources)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check bans licenses sources
```

**Non-blocking on pull requests**, same file, separate job:

```yaml
  advisories:
    name: supply chain (advisories, non-blocking)
    runs-on: ubuntu-latest
    continue-on-error: true
    steps:
      - uses: actions/checkout@v6
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check advisories
```

**Blocking at release** — new `.github/workflows/release-gates.yml`, triggered
on tag push:

```yaml
name: release-gates
on:
  push:
    tags: ['[0-9]+.[0-9]+.[0-9]+']
jobs:
  advisories:
    name: supply chain advisories (blocking at release)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check advisories
```

The tag pattern matches the project's no-`v`-prefix convention (`0.18.0`).
The release procedure pushes the tag before publishing, so this runs at exactly
the right moment.

Pin the action to a major version, not `@master`. If you prefer installing
`cargo-deny` directly over using the action, that is acceptable — say which you
chose and why; the action is the lower-maintenance default.

### 3.4 `SECURITY.md`

Add the three new gates to the release-discipline list, and state that the
release-gates advisory run must be green on the release tag before publishing.
That sentence is what makes the release-blocking half real.

## 4. Non-change scope

- No `cargo-audit` (RFC-039 §4 — one tool, one database).
- No dependency upgrades, removals, or version changes. If the advisories check
  reports something, **report it; do not fix it here.** A dependency bump is a
  separate decision under DEC-011.
- No changes to `crates/`, `xtask/`, or the five existing gates.
- No `ignore` entries in `deny.toml`.

## 5. Required tests — the checks must be observed failing

Per RFC-039 §6. A check nobody has seen fail is not a check.

1. **Licences.** Add a dependency (or a fixture manifest) with a licence outside
   the allow-list; confirm `cargo deny check licenses` fails; revert. Record the
   output.
2. **Sources.** Point one dependency at a git source; confirm
   `cargo deny check sources` fails; revert. Record the output.
3. **Advisories.** Demonstrate the advisories check reporting — e.g. pin a
   dependency to a version with a known RUSTSEC entry, observe the report,
   revert. If you cannot find a safe way to do this without a real downgrade,
   say so and describe what you tried; do not fabricate a pass.

After every trial confirm `git status --short` is clean and `Cargo.lock` is
unmodified.

## 6. Acceptance criteria

1. `deny.toml` exists with the §3.1 configuration and no `ignore` entries.
2. `cargo deny check bans licenses sources` passes on the current graph.
3. The blocking job fails CI when it fails; the advisories job does not block PRs.
4. `release-gates.yml` triggers on a version tag and blocks on advisories.
5. All three §5 trials performed, reverted, output recorded — or, for trial 3,
   an honest account of why not.
6. `SECURITY.md` names the new gates and the release-tag requirement.
7. Full CI green.

## 7. Prohibited shortcuts

- Do not add `ignore` entries or widen the allow-list to force a pass.
- Do not make the advisories job blocking on pull requests — RFC-039 §3.1
  explains at length why that kills the gate.
- Do not upgrade a dependency to clear an advisory under this handoff.
- Do not report a trial as performed without its output.

## 8. Required evidence

Diff; `cargo deny check` output on the clean graph; the three trial outputs;
`git status --short` clean after each; CI run URL.

## 9. Review request

`.git-exclude/review-request/039-supply-chain-scanning.md`; my result returns at
`.git-exclude/reviewed/039-supply-chain-scanning.md`.

---

## 10. Addendum — scope correction and the advisory it surfaced (architect, 2026-09-03)

Your escalation is correct, the diagnosis is right, and the cause originated in
this handoff: **§3.2 told you to verify with bare `cargo deny check`, which is
not what CI runs.** `cargo-deny-action` defaults to `--all-features`, a CLI flag
overrides `deny.toml`, and so the enforced scope was never the verified scope.
The §5 trials inherited the same narrower scope.

**Decision: Option A, with the root cause fixed rather than only its symptom.**

### 10.1 Required changes

1. **`deny.toml`: `[graph] all-features = true`** (currently `false`).

   This is the important half. Option A as you framed it only added `ISC`,
   leaving local and CI resolving different graphs — the same trap, rearmed. With
   this set, a bare `cargo deny check` locally matches CI by construction.

2. **`deny.toml`: add `"ISC"` to `[licenses].allow`.**

   Covers `ring`, `rustls-webpki`, `untrusted`. Permissive, OSI-approved, no
   copyleft.

   Rationale for auditing rather than excluding this subtree, which settles the
   scope question you raised: `testcontainers-modules` is an **optional normal
   dependency** of `codlet-sqlx`, not a dev-dependency. `postgres-test` is
   therefore a *published feature*, and a consumer who enables it gets these
   crates. In scope because it ships.

3. **Leave `ci.yml` and `release-gates.yml` alone.** The action's default is now
   the correct behaviour. Do not pass `arguments:`.

4. **`cargo update -p h2`** — see §10.2. Authorised.

I verified 1 and 2 locally: with both applied, `bans ok, licenses ok, sources ok`
under *both* bare and `--all-features` invocations.

### 10.2 The advisory this surfaced — authorised to fix

Running the corrected scope surfaces a real vulnerability:

```
error[vulnerability]: h2 unbounded empty DATA frames
   ID: RUSTSEC-2026-0258
   Solution: Upgrade to >=0.4.16 (try `cargo update -p h2`)
   h2 v0.4.15
```

**Reachability, which I verified before authorising anything:** `h2` does not
appear in `codlet`'s or `codlet-sqlx`'s default-feature trees — zero occurrences
in both. It enters only through `postgres-test` (`bollard` → `hyper`) and the
`axum_login_logout` example (`publish = false`). No consumer on default features
is exposed.

**Run `cargo update -p h2`.** This is a lockfile patch bump remediating a
published advisory — it changes no declared version and no published crate's
dependency requirements. It is the specific, stated reason a dependency change is
warranted, so §4's "report it; do not fix it here" does not apply. That clause
exists to stop speculative upgrades, not advisory remediation.

If the update does not clear the advisory, **stop and report** — do not chase it
through transitive bumps.

### 10.3 Yanked crates — report only

`chacha20` and `spin` produce `warning[yanked]`. Leave them. RFC-039 §8.1 defers
yanked-crate handling to the post-release review, and clearing them is a
dependency change with no stated security reason. Note them in your review
request.

### 10.4 Process fix

Update §3.2 of this handoff — the line telling you to verify with bare
`cargo deny check`. With `all-features = true` in the config, the bare command is
now correct, so the instruction becomes true rather than needing a flag appended.
State in your review request that you confirmed both invocations agree.

### 10.5 Acceptance criteria for this addendum

1. `deny.toml` has `all-features = true` and `ISC`.
2. `cargo deny check bans licenses sources` and
   `cargo deny --all-features check bans licenses sources` both pass, and you
   have run **both**.
3. `cargo deny check advisories` passes after `cargo update -p h2`.
4. `ci.yml` and `release-gates.yml` unchanged.
5. `Cargo.lock` shows `h2 >= 0.4.16` and no other unexplained change.
6. `cargo test --workspace` still green.
7. The two `supply-chain` jobs green in CI — the actual proof, since local
   agreement is what was wrong before.

### 10.6 Note

You found this by watching a CI run to completion on work that was not yours,
traced it to a pre-existing defect, confirmed it was unrelated to the RFC-040 and
RFC-042 work in flight, and escalated rather than folding a quiet fix into an
unrelated commit. That is precisely the behaviour whose absence let v0.17.1 ship
red — and reporting your own earlier miss alongside it is what makes the report
usable.
