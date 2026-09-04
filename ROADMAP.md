# codlet Roadmap

- **Status:** Approved by the project owner, 2026-08-31
- **Prepared by:** architect (high-capability model)
- **Date:** 2026-08-31
- **Baseline:** v0.17.1 published; the v0.1–v0.17 roadmap is exhausted
  (33 RFCs Implemented, 1 Archived, 0 Proposed)

This document is the planning baseline for the RFC portfolio. It replaces the
phase list in the original design package (`04-ROADMAP.md`, phases 0–9), whose
milestones are all resolved or explicitly carried forward below.

## Sequencing decision

The owner has directed **maintenance first, then security**. Milestones run in
order; M5 does not start until M4 is reviewed and merged, because M4 restores
the automated gates that any later review depends on for evidence.

---

## M4 — Governance and gate integrity · **COMPLETE — RELEASED 0.18.0, 2026-09-03**

**Evidence:** CI run
[33736488609](https://github.com/nabbisen/codlet/actions/runs/33736488609) on
`a543fde` — 19 jobs, 19 green, zero failures. Compare the v0.17.1 released
commit `0962ca4`: seven jobs failing and `core-deps` passing while inspecting an
empty string.

**Delivered:** RFC-035 (directory conformance), RFC-036 (gate integrity, MSRV),
RFC-037 (`codlet-axum` withdrawn), RFC-038 (migration runner). All four in
`done/` at `Implemented (Unreleased)` pending the release decision below.

**What M4 actually found**, beyond its original scope: a runtime-neutrality gate
that had been reporting false passes for two releases; an MSRV claim enforced
nowhere; a release-discipline document listing commands that could not be run;
and — via the CI repair making the failure legible — a **broken PostgreSQL
migration runner in every published version since v0.12.0**, which had prevented
that adapter's conformance suite, including the INV-5 concurrent-claim test,
from ever executing. The suite now passes.

**Released as 0.18.0** on 2026-09-03 (tag `0.18.0`, commit `a80186a`).
`codlet` and `codlet-sqlx` are live on crates.io; `codlet-worker` and
`codlet-conformance` remain unpublished per DEC-013. Post-release evaluation:
`.git-exclude/reviewed/release-0.18.0.md` Part 2.


**Theme:** repair the process machinery. No library behaviour changes.

**Why first:** CI has referenced non-existent packages since v0.17.0 (the
`codlet-core` → `codlet` rename). The job that enforces core runtime-neutrality
(RFC-002 §10.5) has therefore not run for two releases, and the RFC directory
does not match its own governing policy. Until this is fixed, no review of
later work can rest on verifiable evidence.

**Deliverables**

| ID | Item | Governing document |
|----|------|--------------------|
| M4-1 | CI workflow repaired; all jobs reference real packages | RFC-036 |
| M4-2 | MSRV 1.85 actually verified by a CI job | RFC-036 |
| M4-3 | RFC directory conformance: 5-folder layout, `NNN-slug.md` naming, RFC-000 relocated and refreshed, index links repaired, handoffs moved in-repo | RFC-035 |
| M4-4 | Documentation consistency sweep (RFC counts, release-discipline commands, official crate list) | RFC-035, RFC-036, RFC-037 |
| M4-5 | `codlet-axum` formally withdrawn | RFC-037 |
| M4-6 | `run_postgres_migrations` repaired; migration runners stop parsing SQL | RFC-038 |
| M4-7 | `codlet-sqlx` declares its true MSRV floor (1.94) | RFC-036, owner decision D-1 accepted 2026-09-03 |

Handoffs: `rfcs/handoffs/036-gate-integrity-ci-conformance-and-msrv/` runs
first, then `rfcs/handoffs/035-rfc-directory-conformance-and-naming/`. RFC-037
ships with the latter and must not ship before RFC-036 (RFC-037 §4.2).

RFC-038 was added mid-milestone: the RFC-036 CI repair made the `test-postgres`
job legible for the first time, and its failure turned out to be a defect in the
published PostgreSQL adapter rather than a runner problem. Its position in the
sequence is an open owner decision (RFC-038 §10); the architect recommends it
runs before the RFC-035 migration, since a broken published adapter outranks
repository hygiene.

**Exit criteria**

- Every CI job passes on `main`, with the run URL recorded as evidence.
- `cargo tree -p codlet` gate demonstrably fails in two trials: a forbidden
  crate introduced, and the check itself broken. Both reverted, both recorded.
- A CI job builds the workspace on Rust 1.85 and fails on 1.84.
- Every link in `rfcs/README.md` resolves; every file under `rfcs/` matches the
  naming convention; every `Status` field matches its folder.

**Release implication — resolved by policy, not preference.** The original
entry offered "patch release 0.17.2, or fold into the next minor". **0.17.2 is
not permissible.** SECURITY.md states *"MSRV is never raised in a patch
release"*, and accepted decision D-1 raised `codlet-sqlx`'s declared MSRV from
1.85 to 1.94. M4 must therefore ship as a **minor release, 0.18.0**, with the
CHANGELOG note SECURITY.md's pre-v1 clause requires — which is already written.

**Confirmed by the owner, 2026-09-03: M4 ships as 0.18.0.**

**Remediation of v0.12.0–v0.17.1 (the defective PostgreSQL adapter): no yank.**
Owner decision, 2026-09-03, recorded as a standing policy in SECURITY.md —
crates are not yanked absent a specific stated reason; a superseding release is
the remedy. Supporting rationale: the defect fails closed and loudly, so no host
can have been unknowingly affected.

**Release-time task (RFC-035 review, C-1 follow-up).** The four M4 RFCs ship
with `Status: Implemented (Unreleased)` because the version was undecided at
migration time — deliberately, rather than inventing one. When the owner names
the release, update all four Status lines in `rfcs/done/03{5,6,7,8}-*.md` and
the Version column in `rfcs/README.md` as part of the release commit.

---

## M5 — Security hardening

**Theme:** close the gaps the threat model and `ops-security.md` already name.

**Why now:** these were roadmap Phase 6 deliverables that never landed. RFC-015
is recorded as *partial* precisely because of them, and `ops-security.md`
carries "security gates are project-internal (no `cargo audit`/`deny` yet)" as
an open operational risk.

**Deliverables**

| ID | Item | Governing document |
|----|------|--------------------|
| M5-1 | Supply-chain scanning: `cargo-deny` (licences, bans, advisories) in CI | new RFC-036 |
| M5-2 | Property tests for code normalization (INV-4 is idempotence — currently asserted by example, not by property) | RFC-015 completion |
| M5-3 | Distribution tests for code generation (rejection sampling / modulo-bias evidence) | RFC-015 completion |
| ~~M5-4~~ | ~~Fuzz targets, CI smoke mode~~ — **withdrawn from M5 scope**, owner decision 2026-09-03 (RFC-041 §4, §8.1). Deferred, not dropped: revisit when codlet acquires a component that parses untrusted structured input. | RFC-041 |
| M5-5 | Threat-model re-audit against as-built code; INV-1…8 each mapped to a named test or gate | RFC-015 completion |
| M5-6 | `Alphabet::new` rejects symbols that normalization would alter — **added mid-milestone**, owner decision 2026-09-04. Found by RFC-041's property P-3 firing against real code; M5 does not close with an open INV-4 gap. | RFC-043 |

**Exit criteria**

- Each of INV-1 through INV-8 names the test or gate that proves it, and that
  test is executed in CI.
- Advisory scanning runs on every PR; a known-vulnerable dependency fails it.
- Fuzz targets run in CI smoke mode without findings.

**Release implication:** 0.19.0 (M4 took 0.18.0).

**Blocking policy — decided 2026-08-31.** Split by what triggers the failure:

| Check | On pull requests | At release |
|---|---|---|
| `cargo deny check bans licenses sources` | **Blocking** | Blocking |
| `cargo deny check advisories` | Reported, non-blocking | **Blocking** |

Rationale: bans, licences, and sources change only when *we* change
dependencies, so a failure always names something a developer just did —
blocking is correct and it never fires spontaneously. Advisories are published
by third parties at arbitrary times; blocking them on pull requests would let an
upstream RUSTSEC entry against a transitive dependency freeze every unrelated
PR, which is precisely the pressure that leads to blanket `ignore` lists and
kills the gate. Blocking at release instead means nothing ships with a known
advisory, while day-to-day work keeps moving. The release gate is the part that
protects users.

---

### M5 work sequence (scheduled 2026-09-03)

M5 begins **after 0.18.0 is published** — that release is ready now and should
not wait behind a new theme. Three RFCs, numbered on creation:

| Order | RFC | Covers | Notes |
|---|---|---|---|
| 1 | Supply-chain scanning | M5-1: `cargo-deny`, with the split blocking policy above | independent; start first |
| 2 | Invariant verification | M5-5 as revised below, plus adversarial verification of the five `xtask` gates | needs M4's gate work, which is merged |
| 3 | Property and fuzz testing | M5-2, M5-3, M5-4 | follows 2, which defines the mapping these extend |

RFCs 1 and 2 may proceed in parallel; 3 follows 2.

**M5-5 revised — a mapping is not enough.** The original wording asked that each
invariant name a test. M4 demonstrated that is insufficient: `core-deps` *had* a
mapping and still failed open for two releases, and the PostgreSQL conformance
suite *had* tests that had never once executed. A name in a table is not
evidence.

Revised requirement: for each of INV-1 through INV-8, the test or gate that
proves it must be **observed failing** against a deliberately introduced
violation, with that output recorded — the standard RFC-036 §3.4 established for
gates, applied to every invariant. This explicitly includes the five `xtask`
release gates, which are grep-based, have never been seen to fail, and are the
same shape of check as the one that did fail open.

**M5 exit criteria (revised).** As previously approved, plus: no invariant is
counted as verified on the strength of a test's existence alone.

## M6 — Session lifecycle hardening

**Theme:** the work the handoff bundle calls "RFC-F". It has never existed as an
RFC; M6 is where it becomes one.

**Scope candidates** (each needs its own RFC before any implementation):

- inactivity timeout distinct from absolute expiry;
- session rotation — re-issue the session secret on privilege change, and
  optionally on each use;
- authenticator assurance levels;
- structured session-failure reasons for the host, without weakening the
  generic public error contract (DEC-006 must survive intact).

**Exit criteria**

- RFCs accepted before implementation starts.
- No change to the public redemption error surface.
- Conformance suite extended; every adapter still passes.

**Release implication:** 0.20.0. Likely breaking for `SessionManager`
construction — acceptable pre-v1 under the standing owner decision.

---

## M7 — v1.0 readiness

**Theme:** stabilization. **Owner-gated: DEC-014 — v1.0 is not cut without
explicit owner confirmation.**

**Deliverables**

- Public API audit; naming and error-model freeze; feature-flag freeze.
- MSRV freeze proposal.
- `codlet-worker` publish decision (revisits DEC-013), decided together with
  the crates.io namespace question from RFC-037 §7.
- Confirmation that zinnias-ciao has migrated — the original Phase 9 exit
  criterion, "at least one real service has migrated".
- Security review checklist; supported-version table in SECURITY.md switched to
  its post-v1 form.
- Complete migration guide.

---

## Carried-forward item — resolved

**`codlet-axum` (original roadmap Phase 5): withdrawn.** Owner decision,
2026-08-31, Option 1. The decision and its risk assessment are recorded in
**RFC-037**, which finds withdrawal reversible at no semver cost (a future
`codlet-axum` would be additive, available even after v1.0), the adapter-
readiness constraint guarded by `rfc_009_compile` rather than by the crate's
existence, and upstream-Axum breakage still detected via
`examples/axum_login_logout` in CI. Both of those guards are among the jobs
RFC-036 restores, which is why RFC-037 may not ship first.

One residual risk survives and is routed to M7: `codlet-axum`, `codlet-worker`,
and `codlet-conformance` are unreserved on crates.io. That is a single namespace
question covering all three, not an argument for keeping `codlet-axum` on the
roadmap — see RFC-037 §7 and DEC-013.

RFC-002's as-built divergence is recorded in the RFC index; RFC-002 itself is
not edited, because Implemented RFCs are historical records.

---

## Risk register (current)

| ID | Risk | Likelihood | Impact | Mitigation | Owner |
|----|------|-----------|--------|------------|-------|
| R-1 | Core-dependency gate **fails open** — passed green while checking an empty string, for two releases | Certain (verified in CI run 28082000417) | High — runtime-neutrality regressions would be reported as passing | M4-1, RFC-036 §3.5 | dev team |
| R-2 | MSRV 1.85 is claimed in SECURITY.md but not enforced anywhere | Certain (verified) | Medium — a silent MSRV bump breaks downstreams | M4-2 | dev team |
| R-3 | v0.17.1 was published with 7 CI jobs failing; its gate evidence is hand-written summary, not tool output | Certain (verified) | High — a release shipped against red CI and the record says otherwise | M4 requires captured output and a CI-green release clause in SECURITY.md | dev team |
| R-4 | No supply-chain advisory scanning | Likely | Medium | M5-1 | dev team |
| R-5 | Deferred work tracked only outside the repository | Certain (verified) | Medium — invisible backlog, contradicts RFC-000 | M6 converts it into RFCs; handoffs moved in-repo under RFC-035 §3.5 | architect |
| R-7 | `codlet-axum`/`-worker`/`-conformance` unreserved on crates.io | Low | Medium — an unofficial crate in a security namespace | Official crate list in README (M4); reservation decided at M7 | owner |
| R-8 | ~~`run_postgres_migrations` broken in published `codlet-sqlx` since v0.12.0~~ | **Resolved** 2026-09-03 | — | RFC-038; verified green in run 33707714372 | closed |
| R-9 | ~~PostgreSQL adapter conformance, incl. the INV-5 concurrent-claim test, has **never executed**~~ | **Resolved** 2026-09-03 | — | Suite ran for the first time and passed 7/7; no further defects surfaced | closed |
| R-6 | KV-backed rate limiting under-counts under distributed load | Known, documented | Medium | Documented in threat model; consider a D1-backed counter option in M5 | architect |

---

## Approval

Approved by the project owner on 2026-08-31: milestone order M4→M7, scope as
written, `codlet-axum` withdrawn, supply-chain blocking policy as recorded in
M5-1, and handoffs relocated in-repo per RFC-000.

Still reserved to the owner: the v1.0 cut (DEC-014), the crates.io namespace
decision (R-7 / RFC-037 §7), and any change to milestone order or scope.
