# RFC-035: RFC Directory Conformance, Naming, and Lifecycle-Policy Placement

- **Status:** Implemented (v0.18.0)
- **Target milestone:** M4
- **Primary crate(s):** none — repository governance only
- **Source basis:** RFC-000 (lifecycle policy); owner direction 2026-08-31

## 1. Summary

Bring the `rfcs/` directory into conformance with the lifecycle policy that
governs it: adopt the 5-folder variant, rename every RFC file to the
`NNN-slug.md` convention the policy mandates, move RFC-000 into `done/` and
replace it with the current authoritative text, and repair the index and every
inbound link.

No crate code changes. No public API changes.

## 2. Motivation

The RFC directory currently violates the policy it publishes, in five distinct
ways. Each was verified against the working tree at commit `0962ca4`:

1. **Folder layout.** The project adopts the 5-folder variant, but only
   `done/` and `archive/` exist. There is no `proposed/` and no `accepted/` —
   there is literally nowhere to file a new RFC. `rfcs/README.md` nonetheless
   documents `proposed/` and renders a "Proposed (0)" section for a folder that
   does not exist.

2. **Filenames.** RFC-000 §Naming mandates `NNN-slug.md`. Every file except
   `000-rfc-lifecycle-policy.md` is named `RFC-NNN-slug.md`. The archived
   RFC-018 additionally carries a malformed slug with a triple hyphen
   (`future-server---idp-crate-strategy`).

3. **RFC-000's location.** RFC-000 has `Status: Implemented` and sits at
   `rfcs/` root, not in `done/`. The policy states that **the folder is the
   source of truth for state**; a root-level RFC has no state.

4. **RFC-000's content.** The in-repo copy is 500 lines and stale. The
   authoritative text is 617 lines and adds the `rfcs/handoffs/` companion-
   document convention, which the repository copy lacks entirely. The stale
   copy also contains three statements that are false for this project:
   - "This RFC is written for the 4-folder variant" — the project adopted the
     5-folder variant;
   - its Self-application section asserts "it lives in `rfcs/done/`" — it does
     not;
   - its anti-patterns section argues *against* formalising `accepted/`, which
     contradicts the adopted variant.

   Its `Status` header also uses `**Status.** Implemented`, while every other
   RFC in the project uses `- **Status:** Implemented (vX.Y.Z)`.

5. **Index integrity.** All 33 links in `rfcs/README.md` point at
   `rfcs/NNN-….md` while the files live in `rfcs/done/`. Every link in the
   index is dead. RFC-000 names this exact failure — "letting cross-references
   rot" — as an anti-pattern, and requires the index to be updated in the same
   change that moves an RFC.

A governance directory that does not follow its own published rules cannot be
used as evidence of anything. This RFC closes that gap in one atomic change, as
RFC-000 §Adoption guidance recommends for an existing directory.

## 3. Decision

### 3.1 Folder layout — 5-folder variant

```text
rfcs/
  README.md        index; every RFC, grouped by state
  draft/           NOT created (see §3.1.1)
  proposed/        open for review
  accepted/        review complete; implementer may start
  done/            Implemented
  archive/         Withdrawn or Superseded
  handoffs/        NNN-slug/ companion execution docs (see §3.5)
```

`accepted/` is meaningful for this project because design and implementation
are performed by different actors: the architect accepts, the dev team
implements. This is the condition RFC-000 sets for choosing the variant.

#### 3.1.1 Empty folders

Git does not track empty directories. Both `proposed/` and `accepted/` carry a `.gitkeep` placeholder so the layout
survives a clone. `accepted/` needs one because it is empty again as soon as the
M4 RFCs ship to `done/`. `draft/` is **not** created: RFC-000 marks it optional,
and this project has no multi-author drafting need.

### 3.2 Naming

Every RFC file is renamed to `NNN-slug.md`, dropping the `RFC-` prefix:

```text
rfcs/done/RFC-001-project-scope-product-shape-non-goals.md
  → rfcs/done/001-project-scope-product-shape-non-goals.md
rfcs/archive/RFC-018-future-server---idp-crate-strategy.md
  → rfcs/archive/018-future-server-idp-crate-strategy.md   (triple hyphen collapsed)
```

**Numbers are not changed.** This is a filename normalization, not a
renumbering; RFC-000's permanence rule concerns numbers, and every number
keeps its identity. Renaming is performed with `git mv` so history follows the
file.

### 3.3 RFC-000 — executed by the architect, 2026-09-03

- **Content:** replaced with the current authoritative 617-line text, which
  includes the `rfcs/handoffs/` convention absent from the repository copy.
- **Location:** moved to `rfcs/done/000-rfc-lifecycle-policy.md`. Its state is
  Implemented and the folder is the source of truth.
- **Project-specific corrections**, applied on top of the authoritative text —
  these are the *adoption record*, not edits to the portable policy:
  - the Status header adopts the project's `- **Status:** Implemented (vX.Y.Z)`
    form;
  - an "Adoption in this project" section records that codlet uses the
    **5-folder** variant, omits `draft/`, and files RFC bodies as `NNN-slug.md`;
  - the Self-application section is corrected to describe the file's real
    location and this migration.
- The duplicate under `.git-exclude/rules/` remains the portable upstream copy
  and is not edited by this RFC.

**Status: done.** The owner directed this step be performed directly rather than
delegated, so `rfcs/done/000-rfc-lifecycle-policy.md` is in place with the three
corrections applied, and the stale root copy is removed. The governing policy
had to become authoritative in-repo before the rest of the migration could be
checked against it. The remainder of §3 is still the dev team's work.

### 3.4 Index and inbound links

- `rfcs/README.md` rebuilt from the filesystem: `Proposed`, `Accepted`,
  `Implemented`, `Archive` sections, each link carrying its folder prefix.
  RFC-000 is listed as an Implemented RFC, not as loose front matter.
- Inbound references updated: `README.md` (2 links to RFC-001/RFC-002),
  `CONTRIBUTING.md` and `docs/src/rfc-process.md` (both cite
  `rfcs/000-rfc-lifecycle-policy.md`).
- **`CHANGELOG.md` is not touched.** Its references to `rfcs/proposed/RFC-033`
  and similar are an accurate historical record of where those files were at
  the time. Rewriting history to match a later layout would be falsification.

### 3.5 Handoffs move in-repo; review artefacts do not

**Owner decision, 2026-08-31: follow RFC-000.** Developer handoffs move from the
untracked `.git-exclude/tasks/dev-team/` into `rfcs/handoffs/NNN-slug/`, keyed
to the governing RFC number, with `implementation-handoff.md` as the entry
point.

**Corrected 2026-09-03.** An earlier revision of this RFC also placed the dev
team's review request in `rfcs/handoffs/NNN-slug/review-request.md`. That was an
over-extension of the policy: RFC-000 names `implementation-handoff.md`,
`task-breakdown-pr-plan.md`, `acceptance-qa-checklist.md`, and an optional
`README.md` as handoff companions, and does not include review artefacts. The
owner corrected it by moving the RFC-036 review request out. The rule is now:

| Artefact | Location | Tracked |
|---|---|---|
| Developer handoff | `rfcs/handoffs/NNN-slug/implementation-handoff.md` | yes |
| Developer review request | `.git-exclude/review-request/NNN-slug.md` | no |
| Architect review result | `.git-exclude/reviewed/NNN-slug.md` | no |

A handoff is a durable design companion and belongs with the RFC. A review
request and its result are exchange documents between two actors on one
iteration; they are workflow correspondence, not design record. The workflow
remains file-based throughout — every exchange is a file at a known path, not a
conversation.

Consequences, all intended:

- A handoff becomes citable evidence in a review. An untracked handoff cannot be
  referenced from a commit, an RFC, or a release record. What a review concludes
  is instead carried into the record by the CHANGELOG entry and the RFC's
  transition to `done/`.
- A handoff has **no lifecycle state of its own**. It inherits the state of the
  matching RFC number. `handoffs/` is deliberately not subdivided into
  `proposed/`, `done/`, or `archive/` — RFC-000 names that as an anti-pattern.
- **Every handoff now requires a governing RFC.** This is a real constraint, not
  a formality: the CI repair began as a plain defect fix with no RFC, and
  acquired RFC-036 precisely because it had nowhere legitimate to live and
  because it produced a durable rule worth recording.
- Role and onboarding tasks under `.git-exclude/tasks/` are **not** handoffs and
  do not move. `001-prepare.md` for each role stays where it is.

## 4. Non-goals

- No change to any RFC's number, status, or technical content.
- No edits to Implemented RFC bodies. RFC-002's divergence from as-built (it
  names `crates/codlet-axum` and `crates/codlet-test`, neither of which exists)
  is **recorded**, not corrected — Implemented RFCs are historical records. The
  divergence note belongs in `rfcs/README.md` and is carried in `ROADMAP.md`.
- No CI workflow changes. Those are a separate defect fix (M4-1).

## 5. Security considerations

None directly: no code, no dependency, no build change.

Indirectly, this RFC restores the auditability of the design record. A security
library whose stated design decisions cannot be navigated from its own index is
harder to review, and a reviewer who cannot follow a link is a reviewer who
skips the check.

## 6. Compatibility and migration effects

Published crates are unaffected — `rfcs/` is not packaged. The one external
effect is that deep links to `rfcs/done/RFC-NNN-….md` from outside the
repository (issues, commit messages, chat) will 404 after the rename. This is
accepted: RFC-000 anticipates it, the cost is bounded, and the alternative is
permanent non-conformance.

## 7. Tests and release gates

No automated gate is added by this RFC. The conformance checks RFC-000 §Optional
CI invariants proposes are deferred; at 35 RFCs a manual review pass is
proportionate. Verification is by the acceptance criteria in the Developer
Handoff:

- every file under `rfcs/**` matches `^[0-9]{3}-[a-z0-9-]+\.md$`;
- every `Status` field matches its containing folder;
- every relative link in `rfcs/README.md` resolves to an existing file;
- no RFC number appears in two folders;
- `git log --follow` still traces each renamed file.

## 8. Alternatives considered

1. **Leave the `RFC-` prefix, amend the policy instead.** Rejected: the prefix
   is redundant inside a directory named `rfcs/`, and amending a portable policy
   to match one project's drift inverts the relationship between rule and
   practice.
2. **Adopt the 4-folder variant** (no `accepted/`). Rejected by owner
   direction, and the roles here genuinely are separate — "the architect
   accepted it" and "the dev team finished it" are distinct events.
3. **Fix only the index links.** Rejected: it leaves four of the five defects
   in place and repeats the migration cost later.

## 9. Open questions

1. ~~Should developer handoffs move in-repo?~~ **Resolved by the owner on
   2026-08-31: follow RFC-000.** See §3.5.
2. Should the RFC-000 §Optional CI invariants become an `xtask check-rfcs`
   gate? Recommendation: revisit at roughly 50 RFCs, not now. The
   orphan-handoff check in the handoff's acceptance criteria is the one
   invariant worth automating early, since a handoff directory whose RFC number
   does not exist is silent rot.

## 10. Acceptance criteria

- `rfcs/` contains exactly `proposed/`, `accepted/`, `done/`, `archive/`,
  `handoffs/`, `README.md`.
- 39 RFC bodies present at end of M4: 38 in `done/` (the existing 33, plus 000,
  plus RFC-035/036/037/038), 1 in `archive/`, none left in `accepted/`.
- `rfcs/handoffs/NNN-slug/` contains handoffs only — no review requests or
  review results (§3.5).
- Every `rfcs/handoffs/NNN-slug/` directory corresponds to an existing RFC
  number, and none contains lifecycle subfolders.
- Every filename matches `NNN-slug.md`.
- `rfcs/README.md` lists every RFC with a working, folder-prefixed link.
- `README.md`, `CONTRIBUTING.md`, `docs/src/rfc-process.md` links resolve.
- `CHANGELOG.md` unmodified except for a new Unreleased entry.
