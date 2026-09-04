# RFC-039: Supply-Chain Scanning

- **Status:** Implemented (v0.19.0)
- **Target milestone:** M5
- **Primary crate(s):** workspace-wide (CI and policy configuration)
- **Source basis:** `ops-security.md` operational risk; RFC-015; `ROADMAP.md` M5-1

## 1. Summary

Add `cargo-deny` to CI with four checks — `advisories`, `licenses`, `bans`,
`sources` — under a split blocking policy: deterministic checks block pull
requests; advisory findings are reported on pull requests and block at release.

## 2. Motivation

codlet is a security library with **137 distinct package names** in its normal
(non-dev) workspace dependency graph — **188 packages in total** once
dev-dependencies are included — and no automated visibility into any of them.

*(Corrected 2026-09-03 by the RFC-039 review. The original text gave only the
137 figure while §3.2's licence table was derived from the full 188-package
graph, presenting two measurements of different graphs as one. The distinction
matters: `cargo-deny`'s `licenses` check excludes dev-dependencies by default,
so the enforced scope and the audited scope diverge unless `include-dev` is
set — see §3.2.)* The v0.17.1
handoff bundle records this as an open operational risk: *"security gates are
project-internal (no `cargo audit`/`deny` yet)"*.

The five `xtask` gates check codlet's own source. Nothing checks what codlet
depends on. A vulnerability published against `hmac`, `sha2`, `subtle`, or
`getrandom` — the crates DEC-011 deliberately delegates cryptography to —
would reach a codlet release with no signal at all.

## 3. Decision

### 3.1 Checks and blocking policy

| Check | On pull requests | At release |
|---|---|---|
| `bans` | **blocking** | blocking |
| `licenses` | **blocking** | blocking |
| `sources` | **blocking** | blocking |
| `advisories` | reported, non-blocking | **blocking** |

The split is by *what triggers a failure*, not by severity.

`bans`, `licenses`, and `sources` change state only when **we** change
dependencies. A failure always names something a developer just did, in the
change that did it. Blocking is correct and it cannot fire spontaneously.

`advisories` change state when a third party publishes a RUSTSEC entry, at an
arbitrary time, against a dependency we may not have touched in months.
Blocking those on pull requests means an upstream publication can freeze every
unrelated PR in the repository. That pressure does not produce prompt upgrades;
it produces a blanket `ignore` list, and a gate full of permanent ignores is a
gate that has been switched off while still appearing to run. Reporting on PRs
and blocking at release means nothing ships with a known advisory, while
day-to-day work keeps moving.

**The release gate is the part that protects users.** The PR gate protects
developers from each other.

### 3.2 The policy must pass on the day it lands

A gate that fails on introduction gets disabled or ignored — this is how
supply-chain checks die. The allow-list below was derived from the workspace's
actual dependency graph, not from a template.

Every licence currently present:

| Licence expression | Packages |
|---|---:|
| `MIT OR Apache-2.0` | 96 |
| `MIT` | 29 |
| `Unicode-3.0` | 18 |
| `Apache-2.0 OR MIT` | 16 |
| `Apache-2.0` | 14 |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 3 |
| `MIT/Apache-2.0` (legacy syntax) | 3 |
| `MIT AND BSD-3-Clause` | 2 |
| `Apache-2.0 OR BSL-1.0`, `Apache-2.0/MIT`, `BSD-3-Clause`, `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, `(MIT OR Apache-2.0) AND Unicode-3.0`, `Unlicense OR MIT`, `Zlib` | 1 each |

**No package lacks a licence, and no package is copyleft-only.** The single
LGPL appearance is one arm of an `OR` that also offers MIT and Apache-2.0.

Resulting allow-list: `Apache-2.0`, `MIT`, `BSD-3-Clause`, `Unicode-3.0`,
`Zlib`, `Unlicense`, `BSL-1.0`, and the `LLVM-exception` variant of Apache-2.0.

Note `Unicode-3.0` (18 packages, the ICU crates reached through `sqlx`) and the
two `AND` expressions, which require *every* named licence to be allowed rather
than any one. Both are common omissions from a copied allow-list and both would
have failed on day one.

### 3.2.1 Scope: `all-features`, and why the audited and enforced scopes must be the same

*(Added 2026-09-03 after a post-merge escalation. See §10.)*

`cargo-deny-action`'s default `arguments` is `--all-features`, and a CLI flag
overrides `deny.toml`. The original configuration set
`[graph] all-features = false`, so CI checked a **broader** graph than any local
`cargo deny check` invocation — and broader than the §3.2 survey. The result was
a blocking job red from the moment RFC-039 merged.

Two corrections:

1. **`[graph] all-features = true`.** Local and CI now resolve the same graph by
   construction, rather than by remembering to pass a flag. The divergence, not
   the licence list, was the defect: a policy whose enforced scope differs from
   its verified scope is unverified.
2. **`ISC` joins the allow-list**, covering `ring`, `rustls-webpki`, and
   `untrusted`, reached via `testcontainers-modules`.

**Why audit that subtree rather than exclude it.** `testcontainers-modules` is an
*optional normal dependency* of `codlet-sqlx`, not a dev-dependency — so
`postgres-test` is a **published feature** and its dependencies are reachable by
any consumer who enables it. Whether or not anyone should, they can. That
settles it: the subtree is in scope because it ships, not merely because broader
auditing feels safer.

`ISC` is OSI-approved, FSF Free, permissive, and carries no copyleft or
attribution obligation beyond those already accepted. Adding it does not widen
the policy in any way that matters.

### 3.3 `sources`

Restrict to the crates.io registry. codlet has no git or alternate-registry
dependencies today, and a dependency silently acquiring one is a supply-chain
event that should require a deliberate policy change.

### 3.4 `bans`

Start with duplicate-version detection at `warn`, not `deny`. The graph has not
been audited for duplicates and a first-day failure violates §3.2. Specific
crate bans are added when there is a reason, not pre-emptively.

## 4. Non-goals

- Not a replacement for the `xtask` gates. Those check codlet's own source;
  this checks what codlet depends on. Disjoint.
- No `cargo-audit` in addition to `cargo-deny` — `deny`'s `advisories` check
  consumes the same RUSTSEC database, and running two tools against one
  database is duplicated maintenance.
- No vendoring, no lockfile-pinning policy change, no dependency reduction
  campaign.
- No change to DEC-011 (cryptography via audited upstreams).

## 5. Security considerations

This RFC adds detection, not enforcement of any new invariant. Its security
value is entirely in whether the signal is *read*, which the release-blocking
half of §3.1 is designed to guarantee.

Explicitly acknowledged: `advisories` being non-blocking on pull requests means
a window exists between an advisory's publication and the next release in which
`main` carries a known-vulnerable dependency without CI going red. That window
is bounded by the release cadence and is a deliberate trade against the
ignore-list failure mode described in §3.1.

## 6. Testing and verification

Per the M5 standard, the checks must be **observed failing**, not merely
observed passing:

1. Introduce a dependency with a disallowed licence; confirm `licenses` fails;
   revert. Record the output.
2. Point a dependency at a git source; confirm `sources` fails; revert. Record
   the output.
3. Add a RUSTSEC advisory id to a deliberately vulnerable pinned version, or use
   `cargo deny --deny warnings` against a known-advisory fixture, to confirm
   `advisories` reports. Record the output.

A check nobody has seen fail is not a check. This is the standard RFC-036 §3.4
established and RFC-040 generalises.

## 7. Alternatives considered

1. **Blocking advisories on pull requests.** Rejected — §3.1.
2. **Advisory-only everywhere, blocking nowhere.** Rejected: it relies on
   someone choosing to read a log, which is the exact failure that let v0.17.1
   ship red.
3. **`cargo-audit` instead of `cargo-deny`.** Rejected: `audit` covers only
   advisories; `deny` covers advisories plus licences, bans, and sources in one
   tool and one configuration file.

## 8. Open questions

1. Should `bans` duplicate detection move from `warn` to `deny` once the graph
   is audited? Recommend revisiting after one release cycle of warnings.
2. Should the release-time advisory check run in the release workflow or as a
   manual pre-publish step? Recommend CI, so it cannot be skipped by whoever
   cuts the release.

## 9. Acceptance criteria

- `deny.toml` at the workspace root, with the §3.2 allow-list.
- `cargo deny check bans licenses sources` passes on the current graph and
  blocks on failure in CI.
- `cargo deny check advisories` runs on pull requests without blocking, and
  blocks in the release path.
- All three §6 negative trials performed, reverted, and their output recorded.
- SECURITY.md's release-discipline list names the new gates.

## 10. Post-merge amendment record (2026-09-03)

RFC-039 §3.2 claimed the policy "must pass on the day it lands" and the
implementation was verified against that claim. **The claim was true only for
the scope local verification checked, and false for the scope CI enforces.**
The `supply-chain` job was red from `da5297a` onward.

**Cause, and where it originated.** The implementation handoff instructed local
verification with bare `cargo deny check`. `cargo-deny-action` defaults to
`--all-features`. Those are different graphs, and the handoff — mine — specified
the one that does not match CI. The §6 negative trials were run in the same
narrower scope, so they too proved less than they appeared to.

**Consequence beyond the licence failure.** Running the broadened scope surfaced
a genuine advisory that the narrow scope had been hiding:

> **RUSTSEC-2026-0258** — *h2 unbounded empty DATA frames*, `h2 v0.4.15`,
> fixed in `>= 0.4.16`.

Reachability, verified: **not present** in `codlet`'s or `codlet-sqlx`'s
default-feature dependency trees. It enters only via `postgres-test`
(`bollard` → `hyper`) and via the `axum_login_logout` example, which is
`publish = false`. No consumer on default features is exposed.

This is the first real finding the supply-chain policy has produced, and it was
produced by fixing the scope rather than by the policy as originally merged —
which is the argument for §3.2.1's first correction being the important one.
