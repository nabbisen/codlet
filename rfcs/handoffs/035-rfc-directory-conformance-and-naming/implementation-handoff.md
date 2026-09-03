# Implementation Handoff — RFC-035 RFC Directory Conformance

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-08-31
- **Milestone:** M4 (`ROADMAP.md`)
- **Governing RFC:** [`../../accepted/035-rfc-directory-conformance-and-naming.md`](../../accepted/035-rfc-directory-conformance-and-naming.md)
- **Priority:** after the RFC-036 handoff is merged.

## 1. Purpose

Make the `rfcs/` directory conform to the lifecycle policy it publishes. One
atomic change, as RFC-000 §Adoption guidance prescribes for an existing
directory.

## 2. Background

Five verified defects, all detailed in RFC-035 §2: missing `proposed/` and
`accepted/` folders, non-conforming filenames, RFC-000 in the wrong location,
RFC-000 carrying stale and self-contradictory content, and all 33 index links
broken.

Read RFC-035 in full before starting. It is the authority; this handoff only
directs execution. If execution uncovers a conflict with RFC-035, **stop and
escalate** — do not resolve it in the handoff.

## 3. Change scope

- `rfcs/**` — folder layout, filenames, `README.md`, and the
  `accepted/ → done/` transitions in §5.6. **Not** RFC-000's content — already
  placed by the architect (§5.3).
- `README.md` (repository root) — two RFC links, plus the official crate list (§5.7)
- `CONTRIBUTING.md` — one path reference
- `docs/src/rfc-process.md` — one path reference
- `ROADMAP.md` — mark the `codlet-axum` carried-forward item withdrawn (§5.7)
- `CHANGELOG.md` — `[Unreleased]` entry only

## 4. Non-change scope — do not touch

- **The body of any RFC other than 000.** Implemented RFCs are historical
  records. Do not correct their prose, their `codlet-core` references, or their
  outdated crate lists. RFC-002's divergence from as-built is recorded in
  `ROADMAP.md`, not patched.
- **RFC numbers.** No renumbering, no gap-filling, no reuse. RFC-018 stays 018.
- **`CHANGELOG.md` history.** Its references to `rfcs/proposed/RFC-033` are an
  accurate record of where files were at the time. Rewriting them is
  falsification.
- **`.git-exclude/rules/000-rfc-lifecycle-policy.md`** — the portable upstream
  copy. Read it, copy from it, do not edit it.
- Anything under `crates/`, `examples/`, `xtask/`, `.github/`.

## 5. Required implementation

Work in this order.

### 5.1 Folders

`rfcs/proposed/`, `rfcs/accepted/`, and `rfcs/handoffs/` already exist. Confirm
that **both** `rfcs/proposed/.gitkeep` and `rfcs/accepted/.gitkeep` are
committed — Git does not track empty directories, and by the end of this handoff
both folders are empty (§5.6). Without the placeholders the layout vanishes on
clone.

Do **not** create `draft/` (RFC-035 §3.1.1).

`rfcs/handoffs/` now holds developer handoffs in-repo, keyed to the governing
RFC number, per RFC-000 §Companion handoffs and the owner's direction of
2026-08-31. Handoffs have no lifecycle state of their own; they inherit it from
the matching RFC. Do not create `proposed/`, `done/`, or `archive/` subfolders
inside `handoffs/`.

### 5.2 Filename normalization

Drop the `RFC-` prefix from every RFC body. Use `git mv` so history follows.

```sh
cd rfcs
for f in done/RFC-*.md archive/RFC-*.md; do
  git mv "$f" "$(dirname "$f")/$(basename "$f" | sed 's/^RFC-//')"
done
```

Then apply the one slug fix — RFC-018's malformed triple hyphen:

```sh
git mv archive/018-future-server---idp-crate-strategy.md \
       archive/018-future-server-idp-crate-strategy.md
```

Verify afterwards that every file matches `^[0-9]{3}-[a-z0-9-]+\.md$` and that
no filename contains a double hyphen.

### 5.3 RFC-000 — already done; verify only

**Do not perform this step.** The architect placed
`rfcs/done/000-rfc-lifecycle-policy.md` on 2026-09-03 (the authoritative
617-line text plus three project corrections: the project Status-header form, an
"Adoption in codlet" subsection recording the 5-folder variant, and a corrected
Self-application section) and removed the stale root copy in the same change.
The governing policy had to be authoritative in-repo before the rest of this
migration could be checked against it.

Verify only:

```sh
test -f rfcs/done/000-rfc-lifecycle-policy.md
test ! -e rfcs/000-rfc-lifecycle-policy.md
diff .git-exclude/rules/000-rfc-lifecycle-policy.md rfcs/done/000-rfc-lifecycle-policy.md
```

The diff must show changes in exactly three regions — the header, the
folder-variant section, and Self-application. Anything else is a finding: report
it, do not repair it.

### 5.4 Rebuild `rfcs/README.md`

Regenerate from the filesystem, with four state sections in lifecycle order:
**Proposed**, **Accepted**, **Implemented**, **Archive**. Every link carries its
folder prefix, e.g.:

```markdown
| [000](./done/000-rfc-lifecycle-policy.md) | RFC Lifecycle Policy | v0.0.0 |
```

Requirements:

- RFC-000 is listed as an Implemented RFC, not as loose front matter above the
  tables. It is an RFC and belongs in the table.
- Preserve the existing "Version" column values and the `*(partial)*` markers on
  RFC-014 and RFC-015 exactly as they are.
- Counts in the section headings must match the files on disk. **End state after
  §5.6:** Proposed (0), Accepted (0), Implemented (38), Archive (1) — the
  existing 33, plus RFC-000, plus RFC-035/036/037/038. If RFC-038 is held back
  per §5.6, it is Accepted (1) / Implemented (37); say so in the index note.
- Mark RFC-018's Archive row reason as `Deferred post-v1`, unchanged, and add
  RFC-037's withdrawal of `codlet-axum` to the notes below the tables.
- Note which RFCs have a companion handoff under `rfcs/handoffs/`, per RFC-000
  §README integrity. A handoff is not a separate lifecycle item — do not give it
  its own row or status.
- Add one line under the Implemented table noting that RFC-002 describes
  `crates/codlet-axum` and `crates/codlet-test`, neither of which was built, and
  pointing to RFC-037 and `ROADMAP.md` for the disposition. Record the
  divergence; do not edit RFC-002.

### 5.5 Inbound links

| File | Current | Becomes |
|------|---------|---------|
| `README.md` line 131 | `./rfcs/done/RFC-001-project-scope-product-shape-non-goals.md` | `./rfcs/done/001-project-scope-product-shape-non-goals.md` |
| `README.md` line 132 | `./rfcs/done/RFC-002-crate-architecture-feature-flags-runtime-matrix.md` | `./rfcs/done/002-crate-architecture-feature-flags-runtime-matrix.md` |

The link text in `README.md` reads `` `rfcs/done/RFC-001` ``; update the visible
text to match the new filename too.

`CONTRIBUTING.md` and `docs/src/rfc-process.md` already point at
`rfcs/done/000-rfc-lifecycle-policy.md` — the architect repaired them when
moving the file (§5.3). Leave them alone.

There are no path cross-links between RFC bodies — verified by grep. If you find
one, report it.

### 5.6 Shipping RFC-035, RFC-036, RFC-037, and RFC-038

All four M4 RFCs transition to `done/` in this change — RFC-036's
implementation merged at `7142f72`, RFC-038's in the handoff that runs
immediately before this one, and RFC-037's acceptance criteria are satisfied by
§5.7 below.

For each of `035-rfc-directory-conformance-and-naming.md`,
`036-gate-integrity-ci-conformance-and-msrv.md`,
`037-withdraw-codlet-axum-framework-adapter.md`, and
`038-migration-runner-must-not-parse-sql.md`:

```sh
git mv rfcs/accepted/<file> rfcs/done/<file>
```

and set each `Status` to `Implemented (vX.Y.Z)` for the release it ships in.
Update `rfcs/README.md` in the same commit — RFC-000 requires the index to move
with the file.

**Order matters for RFC-037:** it must not ship before RFC-036, because RFC-037
§4.2 depends on the `test-send-compat` job actually running again. RFC-036
merged at `7142f72`; confirm that job is green before moving RFC-037.

**RFC-038 ships only if verified.** Move it to `done/` only when its handoff is
merged *and* the PostgreSQL conformance suite has actually executed. If that
verification is still outstanding, leave RFC-038 in `accepted/`, ship the other
three, and note why — do not mark an RFC Implemented on the strength of an
unverified fix.

`accepted/` is then empty; this is expected, and `.gitkeep` preserves it.

### 5.7 RFC-037 consequential edits

1. **`README.md` — official crate list.** RFC-037 §4.4 records that
   `codlet-axum`, `codlet-worker`, and `codlet-conformance` are unreserved on
   crates.io. Add a short subsection to the existing Quick start or Design notes
   area stating which crates are published by this project — currently `codlet`
   and `codlet-sqlx` — and that no other `codlet-*` crate on crates.io is
   official. Keep it to two or three sentences; README brevity is a project
   rule. Do not speculate about future crates.
2. **`ROADMAP.md`.** In the "Carried-forward item requiring a decision" section,
   record that Option 1 (withdraw) was chosen by the owner on 2026-08-31 and
   that RFC-037 holds the decision and its risk assessment. Leave the option
   analysis in place as the rationale.

## 6. Required tests

None — no executable code changes. Verification is by §8.

## 7. Required documentation updates

`CHANGELOG.md` `[Unreleased]`: the folder-variant adoption, the renaming
convention, RFC-000's relocation and refresh, and the index repair.

## 8. Acceptance criteria

Run these and paste the output:

```sh
# 1. every RFC body matches NNN-slug.md  (state folders only -- handoff files
#    are companion docs and are deliberately not named NNN-slug.md)
find rfcs/proposed rfcs/accepted rfcs/done rfcs/archive -name '*.md' \
  | sed 's|.*/||' | grep -vE '^[0-9]{3}-[a-z0-9-]+\.md$'   # expect: no output

# 2. no RFC number appears in two state folders
find rfcs/proposed rfcs/accepted rfcs/done rfcs/archive -name '[0-9][0-9][0-9]-*.md' \
  | sed 's|.*/||;s|-.*||' | sort | uniq -d   # expect: no output

# 2b. every handoff directory matches an existing RFC number
for d in rfcs/handoffs/*/; do n=$(basename "$d" | cut -d- -f1); \
  ls rfcs/*/"$n"-*.md >/dev/null 2>&1 || echo "ORPHAN HANDOFF: $d"; done   # expect: no output

# 3. every link in the index resolves -- and the count proves links were found
#    (a zero-match grep would otherwise "pass" silently)
links=$(grep -o '](\.\?/\?[^)]*\.md)' rfcs/README.md | sed 's|](||;s|)||;s|^\./||')
echo "$links" | wc -l                                  # expect: 39 (38 if RFC-038 held back)
echo "$links" | while read -r p; do [ -f "rfcs/$p" ] || echo "BROKEN: $p"; done   # expect: no output

# 4. no stale RFC- path references remain outside CHANGELOG.md
grep -rn 'rfcs/[a-z]*/RFC-\|rfcs/000-' --include='*.md' . | grep -v CHANGELOG.md | grep -v '.git-exclude'   # expect: no output

# 5. history survived the renames
git log --follow --oneline rfcs/done/001-project-scope-product-shape-non-goals.md | tail -3   # expect: pre-rename commits
```

Additionally:

- Each RFC's `Status` field matches its containing folder.
- `rfcs/` contains exactly `proposed/`, `accepted/`, `done/`, `archive/`,
  `handoffs/`, `README.md` — no loose RFC at root.
- `git ls-files rfcs/proposed rfcs/accepted` is non-empty (placeholders committed).
- `git diff --stat` shows no change to any RFC body except 000.

## 9. Prohibited shortcuts

- Do not use `mv` instead of `git mv` — history matters here, and criterion 5
  will catch it.
- Do not edit an RFC body to make a link or a claim look right.
- Do not renumber anything, for any reason.
- Do not create empty `draft/` or `handoffs/` folders "for later".
- Do not rewrite CHANGELOG history.
- Do not hand-maintain the index counts — derive them from the filesystem and
  state that you did.

## 10. Security constraints

None directly. Do not let the sweep touch `crates/`, `xtask/`, or `.github/`; a
wide `sed -i` across the repository is the realistic way this handoff could
cause harm. Scope every bulk edit to the paths in §3.

## 11. Known risks

| Risk | Mitigation |
|------|------------|
| A repo-wide rename sweep hits CHANGELOG or source | Restrict every bulk command to the §3 paths; criterion 4 excludes CHANGELOG deliberately |
| `git mv` loop misfires on an unexpected filename | Run the loop with `echo` first and read the output before executing |
| The refreshed RFC-000 loses the three corrections when copied | Apply §5.3(a)–(c) after copying, and diff against the source to confirm only those three regions differ |
| `proposed/.gitkeep` omitted, folder disappears on clone | Criterion: `git ls-files rfcs/proposed` is non-empty |

## 12. Required evidence

- Full diff, with `git diff --stat` showing renames as renames (`R100`).
- Output of all five acceptance commands.
- Confirmation that RFC bodies other than 000 are byte-identical to their
  pre-migration content.

## 13. Required review-request format

Per §9.2 of `ai-multi-agent-software-development-organization-and-workflow.md`.
File the review request at
`.git-exclude/review-request/035-rfc-directory-conformance-and-naming.md`.
The architect's review result is returned at
`.git-exclude/reviewed/035-rfc-directory-conformance-and-naming.md`.
Review artefacts stay out of `rfcs/handoffs/`, which holds design companions
only (RFC-035 §3.5).

Report to the owner only the path of this file and your review-request file.
