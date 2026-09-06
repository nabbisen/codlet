# Follow-Up Handoff #2 — RFC-044: One Blessed Audit Sink, and the Two Tests That Needed It

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-06
- **Governing RFC:** [`../../done/044-session-inactivity-timeout.md`](../../done/044-session-inactivity-timeout.md) — this closes the scan its first follow-up (§8) commissioned.
- **Prerequisite:** follow-up #1 merged.

## 1. Purpose

Fix the two remaining tests your scan found, and remove the reason all three
existed.

## 2. Why this is not three patches

Three tests moved a `CollectingAuditSink` into a manager and never read it back.
That is not three oversights — it is one API shape producing the same workaround
three times.

`CollectingAuditSink` holds `Mutex<Vec<CodeAuthEvent>>`, so it is inherently
shareable. But every manager constructor takes `A` **by value**, so the caller
loses its handle. Faced with that, three separate test files each wrote a private
`SharedAuditSink`.

**And a comment in `rfc_013_acceptance.rs` states, incorrectly, that fixing this
"would require refactoring the API to accept `&A` instead of `A`."** It would
not — the `Arc`-based handle those three files already wrote proves it. That
false claim is currently the documented justification for a test asserting
nothing. Leave it in place and it will justify the next one.

## 3. Change scope

- `crates/codlet/src/audit.rs` — a shareable, assertable sink under the existing
  `test-utils` gating
- `crates/codlet/tests/rfc_013_acceptance.rs` — Finding 1
- `crates/codlet/tests/rfc_045_session_rotation.rs` — Finding 2
- `crates/codlet/tests/rfc_044_idle_timeout.rs`,
  `crates/codlet/tests/rfc_046_session_failure_reasons.rs` — collapse their
  private `SharedAuditSink` copies onto the blessed one
- `CHANGELOG.md` — only if you judge a test-utils addition warrants a line

## 4. Non-change scope

- **Any production code path.** No manager constructor changes signature. The
  point is that the `&A` refactor the comment invokes is *unnecessary*; do not
  perform it.
- `AuditSink`'s trait definition.
- The behaviour any of these tests exercise.

## 5. Required implementation

### 5.1 One sink, shareable and assertable

Provide a single helper in `audit.rs` — `test-utils`-gated, like
`CollectingAuditSink` — that can be cloned into a manager and still read
afterward. Mechanism is yours: an `AuditSink` impl for a shared handle, a
`shared()` constructor returning a clonable pair, whatever reads best beside the
existing code.

Requirement: **one helper, in one place**, that the four test files use. If
`CollectingAuditSink` can simply become that, prefer it — a second near-identical
helper is how this started.

### 5.2 Finding 1

`rfc_013_acceptance.rs::audit_events_emitted_through_complete_flow` must assert
the events its name and leading comment promise: `CodeIssued`, `CodeRedeemed`,
`SessionIssued`.

**Delete the comment.** Not amend it — it is wrong, and a wrong explanation left
in place is worse than none.

If asserting all three turns out to be genuinely blocked by something other than
the sink handle, **stop and report** — that would mean the comment was right for
a reason it did not state, and I want to know before you work around it.

### 5.3 Finding 2

`rfc_045_session_rotation.rs::failed_revoke_returns_ok_old_session_still_valid_and_audit_event_fires`
must assert `SessionRotationRevokeFailed`.

The sibling test already proves the behaviour, so you may find the honest fix is
to merge the two rather than duplicate the assertion. Either is acceptable —
what is not acceptable is a name promising an assertion that lives in a
different test.

### 5.4 Prove each one can fail

For **both** repaired tests: suppress the relevant emission in production code,
confirm the test fails, revert, confirm clean. Record both.

Follow-up #1 established this standard for exactly this defect class. A repaired
test that has not been seen to fail is the original defect with better wording.

## 6. Acceptance criteria

1. One shareable sink in `audit.rs`; no private `SharedAuditSink` copies remain
   in any test file.
2. Finding 1 asserts all three events; the false comment is gone.
3. Finding 2 asserts its event, or is merged with its sibling.
4. Both repaired tests observed failing under suppressed emission; output
   recorded.
5. No production code path changed; no constructor signature changed.
6. Full CI green.

## 7. Prohibited shortcuts

- Do not refactor manager constructors to take `&A`. The comment claiming that
  is necessary is the thing being removed.
- Do not rename either test to match a weaker body.
- Do not leave a second near-identical sink helper behind.
- Do not skip §5.4 for either test.

## 8. Required evidence

Diff; both suppressed-emission trials; `git diff` showing no production path
changed; a grep showing no `SharedAuditSink` definitions remain outside
`audit.rs`; CI run URL.

## 9. Review request

`.git-exclude/review-request/044-followup-2-shared-audit-sink.md`; my result
returns at `.git-exclude/reviewed/044-followup-2-shared-audit-sink.md`.
