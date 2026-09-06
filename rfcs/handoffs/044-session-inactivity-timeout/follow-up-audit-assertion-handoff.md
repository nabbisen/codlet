# Follow-Up Handoff — RFC-044: A Test That Does Not Test Half of What It Claims

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-09-06
- **Governing RFC:** [`../../done/044-session-inactivity-timeout.md`](../../done/044-session-inactivity-timeout.md) — this restores coverage of that RFC's §4.5 contract, so it lives beside its implementation handoff rather than acquiring an RFC of its own.
- **Found by:** you, while designing RFC-045's audit tests. Reported out of scope rather than fixed — correctly.

## 1. The defect

`crates/codlet/tests/rfc_044_idle_timeout.rs`:

```rust
async fn failed_touch_leaves_session_authenticated_and_emits_audit_event() {
    …
    let audit = CollectingAuditSink::new();
    let mgr = SessionManager::new(store, hasher(), clock.clone(), audit, cookie())
        …
    assert!(outcome.is_authenticated(), "…");
}
```

The sink is constructed, moved into the manager, and never read. **The test
asserts the first half of its own name and nothing else.**

RFC-044 §4.5 makes both halves the contract: a failed `touch_session` *leaves
the session authenticated **and** emits an audit event*. Only the first is
currently proven.

**I approved this test**, and cited it in the RFC-044 review as one of the three
that mattered. The name read as an assertion and I did not check the body
against it.

## 2. Change scope

- `crates/codlet/tests/rfc_044_idle_timeout.rs` — that one test
- `CHANGELOG.md` — only if you judge it worth a line; a test-coverage fix
  arguably is not

## 3. Non-change scope

- **Any production code.** `touch_session`'s behaviour is correct; only its
  test is incomplete. If you find yourself changing `auth/session.rs`, stop —
  that would mean the defect is not what this handoff says it is, and that is
  an escalation.
- The other four RFC-044 tests.
- RFC-045's tests, which already do this correctly.

## 4. Required implementation

Assert the audit event. Use the `SharedAuditSink` pattern you used for RFC-045
— an `Arc<Mutex<Vec<CodeAuthEvent>>>` that stays readable after being cloned
into the manager — rather than `CollectingAuditSink`, which is what made the
original unassertable.

Assert the specific variant (`SessionTouchFailed`) and that it names the right
session, not merely that some event was recorded.

## 5. Prove it can fail

The point of this handoff is that a test passed while proving half its claim.
A replacement that cannot fail would be the same defect wearing a fix.

Temporarily suppress the `SessionTouchFailed` emission in `auth/session.rs`,
confirm the test **fails**, revert, confirm clean. Record both outputs.

Without that trial this is unverified in exactly the way the original was.

## 6. Acceptance criteria

1. The test asserts the specific audit event and its session id.
2. Observed failing with the emission suppressed; reverted; output recorded.
3. `auth/session.rs` unchanged in the final diff.
4. The other four RFC-044 tests untouched and passing.
5. Full CI green.

## 7. Prohibited shortcuts

- Do not rename the test to match its weaker body. The name states the contract;
  the body should meet it.
- Do not change production code.
- Do not skip §5's trial.

## 8. Worth a look while you are there

You found this one by needing the same pattern for RFC-045. If a quick scan
turns up other tests whose names promise more than their bodies assert,
**report them — do not fix them.** A list is more useful than a scattered set of
edits, and it would tell us whether this is one oversight or a habit worth a
gate.

## 9. Required evidence

Diff; the suppressed-emission trial output; `git diff` on `auth/session.rs`
showing no change; CI run URL.

## 10. Review request

`.git-exclude/review-request/044-followup-audit-assertion.md`; my result returns
at `.git-exclude/reviewed/044-followup-audit-assertion.md`.
