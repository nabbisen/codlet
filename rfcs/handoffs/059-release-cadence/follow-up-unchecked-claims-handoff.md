# Follow-Up Handoff — Two Documented Things Nothing Checks

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-20
- **Context:** queued during the 0.21.0 release; `.git-exclude/reviewed/056-postgres-tls.md` and `.git-exclude/reviewed/057-d1-migration-runner.md` §9
- **Size:** small, two unrelated items.
- **0.21.0 is published**, so nothing here is release-blocking.

## 1. Item 1 — `SECURITY.md` promises half of what we do

Line 69 reads:

> Advisories will be published via the GitHub Security Advisory tab after a fix
> is available.

That is now incomplete. The owner accepted on 2026-10-20 that advisories go to
**both** the GitHub Security Advisory tab **and** `rustsec/advisory-db`, because
`cargo audit` and `cargo deny check advisories` read RustSec, not GitHub's
database — so a GHSA alone is invisible to the Rust-native tooling this project
itself runs in `release-gates.yml`.

Amend the sentence to name both and say, in one clause, why there are two: one
reaches Dependabot, the other reaches `cargo audit`/`cargo deny`.

Keep it to a sentence or two. Do not restate the reasoning in
`.git-exclude/reviewed/ADVISORY-DRAFT-rfc-048.md` §B — `SECURITY.md` is a policy
document, not an explainer.

**Do not touch anything else in `SECURITY.md`**, in particular not the no-yank
policy or the gate list.

## 2. Item 2 — `codlet-worker`'s rustdoc is only checked by hand

From the RFC-057 review: a rustdoc intra-doc link to a private module broke
`cargo doc --target wasm32-unknown-unknown`, and **CI never runs that**. The
`docs` job runs `cargo doc --workspace --no-deps` on the native target, which
never compiles `codlet-worker`'s wasm32-only code, so its rustdoc is unchecked
except when someone thinks to run it.

You found that by hand and fixed it by hand. The next one will not be found.

Add a CI step — extend the existing wasm job rather than creating a new one if
that is cleaner — running:

```
RUSTDOCFLAGS="-D warnings" cargo doc -p codlet-worker --no-deps --target wasm32-unknown-unknown
```

Include `codlet-worker-miniflare` if it has rustdoc worth checking; use your
judgement and say what you decided.

**Then prove it fires**: reintroduce a broken intra-doc link on a branch,
confirm the job fails, revert, quote the failure. A doc gate that has never
rejected anything is the shape RFC-036 was about.

If the wasm doc build turns out to need a toolchain component the job does not
have, report that rather than working around it.

## 3. Non-change scope

- No change to the native `docs` job's scope.
- No change to any rustdoc content; this is about checking it, not editing it.
- No `#![allow]` or `--cap-lints` to make the new job pass. If it finds real
  warnings, report them — that is the job working.
- Nothing about the advisory itself: publishing the GHSA and the RustSec entry
  are the owner's actions, already written up in the advisory draft's appendix.

## 4. Required verification

| Check | Expectation |
|---|---|
| `grep -n -i 'rustsec\|advisory' SECURITY.md` | the amended sentence names both destinations |
| the new doc command, locally | clean on the current tree |
| the §2 trial | the job fails on a broken link; failure quoted |
| `cargo doc --workspace --no-deps` with `-D warnings` | still clean |
| full CI | all blocking jobs green |

## 5. Acceptance criteria

1. `SECURITY.md` names both the GHSA tab and `rustsec/advisory-db`, with the
   one-clause reason.
2. CI checks `codlet-worker`'s rustdoc on `wasm32-unknown-unknown` with
   `-D warnings`.
3. That check was demonstrated to fail against a deliberately broken link.
4. No rustdoc content changed to make it pass.
5. All blocking CI jobs green.

## 6. Required evidence

The `SECURITY.md` diff; the new CI step; the trial's failure output; CI run URL.

## 7. Review request

`.git-exclude/review-request/059-followup-unchecked-claims.md`; result returns at
`.git-exclude/reviewed/059-followup-unchecked-claims.md`.
