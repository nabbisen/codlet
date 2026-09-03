# RFC-039: Supply-Chain Scanning

- **Status:** Proposed
- **Target milestone:** M5
- **Primary crate(s):** workspace-wide (CI and policy configuration)
- **Source basis:** `ops-security.md` operational risk; RFC-015; `ROADMAP.md` M5-1

## 1. Summary

Add `cargo-deny` to CI with four checks — `advisories`, `licenses`, `bans`,
`sources` — under a split blocking policy: deterministic checks block pull
requests; advisory findings are reported on pull requests and block at release.

## 2. Motivation

codlet is a security library with 137 distinct packages in its workspace
dependency graph, and no automated visibility into any of them. The v0.17.1
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
