# RFC-059: Release Cadence

- **Status:** In force (architect, under delegated authority, 2026-10-20) — applied to 0.21.0
- **Target milestone:** standing policy
- **Primary crate(s):** none — process
- **Source basis:** ten RFCs unreleased for six weeks, four of them security fixes

## 1. Summary

The owner assigned release scheduling to the architect on 2026-10-20: *"make
schedule and manage release cycles is your role. Mine is to authorize them."*
This records how that will be run, and the rule that stops the failure which
prompted it.

## 2. The failure being fixed

0.20.0 was tagged 2026-09-06. By 2026-10-20 — **six weeks** — ten RFCs were
`Implemented (Unreleased)`: 049, 050, 051, 052, 053, 054, 055, 056, 057, 058.

**Four are security fixes:** a test-only dependency subtree reachable from a
published feature (052), a documented rate-limiting requirement that nothing
enforced (053), a configured rate limit that silently did nothing (055), and a
PostgreSQL adapter with no TLS at all (056).

Nothing was blocked and nothing was decided wrongly. The releases simply had no
owner: each one was treated as something the project owner would initiate, and
the architect's reviews ended at "approved" rather than at "and this should
ship".

## 3. Policy

### 3.1 A security fix is released on its own, within seven days

A fix for a security defect in a released version gets a patch release — or a
minor, if the fix is breaking — **within seven days of landing on `main`**, and
does **not** wait for feature work in flight.

This is the rule that matters. Batching a security fix behind unrelated work is
how four of them aged six weeks.

### 3.2 Everything else batches, with a ceiling

Non-security work accumulates until a coherent body is done — a milestone, or a
thread like RFC-050's surface clean-up. The ceiling is **four weeks or five
RFCs, whichever comes first**: at that point the architect proposes a release
even if the body of work feels unfinished.

A ceiling, not a calendar. Releasing on a date nobody needs is its own waste;
the ceiling exists so "not yet" cannot run indefinitely.

### 3.3 Version choice, pre-1.0

- any breaking change to a public API, or a behavioural change a host must
  react to → **minor** (`0.X+1.0`);
- fixes and additions only → **patch** (`0.X.Y+1`).

Breaking changes are accepted pre-1.0 (owner, 2026-09-06) and this does not
revisit that. After 1.0 this section is superseded.

### 3.4 The trigger lives inside the review

**A review result that approves a security fix must state whether it triggers a
release, and recommend the version.** Not a separate step, not a reminder — part
of a step already performed every time.

That is the whole mechanism. §3.1 and §3.2 are the intent; this is what makes
them happen, and it is the same reasoning the project applies to gates: tie the
check to something that already runs, or it will be forgotten.

Equally, a finished-work report that leaves something releasable on `main` says
so and recommends a version.

### 3.4a Version-bearing documentation is part of release scope

Every release must update the places that name a version in prose or in a
copy-pasteable example — at minimum the root `README.md`'s dependency examples
and any git-dependency `tag = "…"` lines.

This is in the policy because it has now been missed twice. The 0.20.0 handoff
omitted it, which let `README.md` go stale enough for the **owner** to notice
(`codlet = "0.17"`, 2026-10-17); the 0.21.0 handoff inherited the omission and
the dev team caught it at the last moment. A release checklist that does not
include the front page will keep producing a front page advertising the previous
release.

The RFC-016 rule still holds — do not pin a version in a prose *status* line —
but a dependency example must name a version to be usable, so it has to be
maintained rather than avoided.

### 3.4b The tag is cut once, after the notes are audited

A release runs in this order, and the tag is the **last** step before the gate:

1. version bumps, CHANGELOG heading conversion, RFC statuses;
2. version-bearing documentation (§3.4a);
3. **the release-notes audit** — every breaking change labelled and carrying a
   migration line — with any gap fixed *before* step 4;
4. CI green, then tag, then `release-gates.yml` on the tag, then the dry-run.

The audit must not be a report delivered after the tag exists. 0.21.0's handoff
asked for the audit and for the tag in one pass, so three labelling gaps were
correctly reported *and* the tag was correctly cut, and fixing them then required
force-pushing a moved tag — which needed the owner, because the tooling treats a
tag force-push as destructive and rightly so.

Moving a tag is defensible only while nothing has consumed it. Not needing to is
better. If a release-notes gap is found after tagging, the gap is fixed and the
tag moved **only** before publication; after publication it is a note in the
next release.

### 3.5 Authorization is unchanged

Each release is authorized by the owner. The architect proposes version, timing
and contents; the owner says yes or no. **No release is cut without
authorization**, and nothing here delegates that.

## 4. Non-goals

- **No automation.** No cron, no release bot, no scheduled workflow. The
  trigger is §3.4, inside work that is already happening.
- **No fixed calendar.** §3.2 is a ceiling, not a timetable.
- No change to the release procedure itself: CI green, clean-room `git archive`
  verification, annotated tag with no `v` prefix, `release-gates.yml` green on
  the tag, `cargo publish --dry-run`, then hand back to the owner to publish.
- No change to the no-yank policy (SECURITY.md).
- No change to who publishes crates or advisories: the owner.

## 5. Security considerations

§3.1 is a security control. A fixed defect that is not released protects
nobody, and the gap between "fixed on `main`" and "available on crates.io" is
time during which the defect is public in the commit history and unavailable as
a remedy.

That cuts less sharply for codlet today, with no production users — but the
reasoning should not depend on that, because the audience will change and the
habit will not. Arguing urgency from "no users" is how the six weeks happened.

## 6. Acceptance criteria

Process, so the criteria are observational:

1. The next review result approving a security fix states the release
   consequence.
2. No security fix sits on `main` unreleased for more than seven days without
   an authorization request having been made.
3. No more than five RFCs, or four weeks, pass without a release proposal.
4. Every release is authorized before it is cut.
