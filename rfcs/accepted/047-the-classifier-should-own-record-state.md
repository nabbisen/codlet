# RFC-047: The Classifier Should Own Record State, Not the Adapter's WHERE Clause

- **Status:** Accepted
- **Target milestone:** M6
- **Primary crate(s):** `codlet`, all four adapters, `codlet-conformance`
- **Source basis:** RFC-046 review finding §5; RFC-044 §4.3's stated principle

## 1. Summary

Two enums in codlet document distinctions the code cannot make.
`SessionFailure::Expired` and `::Revoked` are never produced;
`RedemptionFailReason::Expired` and `::Revoked` are never produced. Both for the
same structural reason: the store's WHERE clause excludes those rows before any
classifier sees them, collapsing three causes into one `None`.

This RFC decides whether that changes — and finds that the diagnostic gap is the
smaller half of the problem.

## 2. The larger finding

RFC-044 established a principle, and gave the reason:

> Idle expiry is enforced in codlet, not in the adapter's WHERE clause. The
> store returns the record; `classify_session` decides. Pushing the rule into
> per-adapter SQL would give four implementations of one security decision, and
> the project already knows what that produces.

**Session expiry and revocation are already four implementations of one security
decision.** In-memory, SQLite, PostgreSQL, and D1 each carry their own predicate
for "is this session still valid". They agree today. Nothing structural makes
them agree tomorrow, and M4 found that the PostgreSQL adapter's conformance
suite had never once executed — so "they agree" rested, until recently, on
nobody having checked.

The unreachable enum variants are a symptom. The condition is that codlet
enforces one rule in one place for idle timeout and in four places for expiry
and revocation, having written down why the first arrangement is correct.

## 3. The risk is not where it first appears

The instinctive ordering — codes are higher-stakes than sessions, because INV-5
guards atomic single-use redemption — is **wrong**, and the difference decides
this RFC's shape.

**`find_redeemable` is a pre-filter, not the guard.** `claim_code`'s conditional
UPDATE independently enforces the same conditions:

```sql
WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL AND expires_at > ?
```

An expired code surviving `find_redeemable` still cannot be claimed. INV-5 rests
on the UPDATE, which is exactly why RFC-005 made it a conditional write rather
than a read-then-write.

**`find_active_session` has no second guard.** It *is* the enforcement point for
session expiry and revocation. Nothing downstream re-checks. Loosening it moves
a sole enforcement point into codlet, where a bug authenticates an expired or
revoked session.

So the session path — the one that looks safer because it carries no INV — is
the riskier change, and the code path is the safer one.

## 4. Decision

**Adopt the RFC-044 principle for both, and sequence by risk — codes first.**

### 4.1 Shape

The store selects by lookup key and returns the record with its state fields
(`expires_at`, `revoked_at`, and for codes `used_at`); the classifier decides.
The adapter stops deciding.

### 4.2 Sequencing: codes first, sessions second

Codes first, because `claim_code` remains an independent guard throughout: if
the new classifier logic is wrong, a code still cannot be redeemed. The blast
radius of a mistake is a diagnostic error, not an authentication bypass.

Sessions second, and only once the code path has shipped and its conformance
tests have run green in CI against all four adapters. The session change removes
a sole enforcement point and deserves to be made with the pattern already
proven.

**Each step is its own handoff.** They must not be implemented together — that
would forfeit the entire benefit of sequencing them.

### 4.3 The conformance suite must invert

Today the suite asserts the store *excludes* expired and revoked rows. After
this change it must assert the store **returns** them, and that the classifier
**rejects** them. Those are different tests, and the second is the one carrying
the security property.

**A conformance suite that still asserts exclusion will pass against an adapter
that has not been migrated**, silently leaving that backend enforcing in SQL
while codlet believes it enforces centrally. Getting this test change wrong is
more dangerous than getting the implementation wrong, because it is what would
hide the implementation being wrong.

### 4.4 What this buys, in order of importance

1. **One implementation of one security decision**, per RFC-044's own reasoning.
2. Reachable `Expired` and `Revoked` in both enums, closing the documentation
   gap RFC-046's C-1 currently papers over honestly.
3. A simpler adapter contract: adapters look records up; they do not adjudicate.

The diagnostic benefit is third. If it were the only benefit this RFC would not
be worth its risk.

## 5. Non-goals

- No change to `claim_code`'s conditional UPDATE. INV-5's guard is untouched —
  this RFC relies on it.
- No change to idle-timeout handling, which already works this way.
- No change to any public error type, or to what an end user sees. `NotFound`,
  `Expired`, and `Revoked` remain host-visible only.
- No change to the `Alphabet`, code-generation, or form-token paths.

## 6. Security considerations

**The risk, stated without softening:** this moves session expiry and revocation
enforcement from SQL that has been correct in production into Rust that has not
yet run anywhere. A defect authenticates a session that should have been
rejected — a direct authentication bypass, the most severe failure class codlet
has.

Three things make it acceptable, and all three must hold:

1. Codes first, where `claim_code` catches a classifier defect (§4.2).
2. The conformance suite inverted to test rejection, not exclusion (§4.3), run
   against all four adapters in CI including PostgreSQL and D1.
3. The classifier's rejection logic covered by tests observed failing against a
   deliberately broken classifier — the M5 standard.

If any of the three cannot be met, the session half should not proceed, and the
codebase should keep the honest documentation from RFC-046's C-1 instead.

**What this does not change:** an attacker's view. Every failure still collapses
to one response.

## 7. Alternatives considered

1. **Second, diagnostic-only query on the failure path.** Keeps the proven
   filter; costs a round trip on exactly the unauthenticated traffic an attacker
   controls the volume of. Rejected: it buys the third-most-important benefit
   while paying for it on the worst possible path, and leaves the four-way
   duplication intact.
2. **Remove the unreachable variants.** Honest and safe, and a second breaking
   change that forecloses the fix. Rejected — but this is the fallback if §6's
   three conditions cannot be met.
3. **Do nothing.** The status quo plus RFC-046's C-1 documentation. Defensible;
   the duplication is a latent risk, not a live defect. Rejected because the
   project has now written down twice why centralising this rule is correct.
4. **Sessions first.** Rejected — §3.

## 8. Open questions — resolved

**Both resolved by owner acceptance, 2026-09-05**, taking the RFC with its
recommendations.

1. **Decision order is fixed: revoked, then expired, then used, then idle.** A
   record can be several at once. Revoked comes first because it is the only
   state an operator caused deliberately, and it is the most useful thing to see
   in a log during an incident.
2. **Adapters do not keep their filters as defence in depth.** A filter that
   still excludes rows makes the classifier untestable against real data, and
   two enforcement points that can disagree is worse than one that is wrong.
   Flagged at acceptance as the least obvious call in this RFC; recorded here so
   a later reader finds the reasoning rather than re-deriving it.

## 9. Acceptance criteria

Per step, not in aggregate:

- Store returns state fields; adapters carry no expiry/revocation predicate.
- Conformance asserts the store returns expired and revoked rows, and that the
  classifier rejects them — all four adapters, CI-verified.
- Classifier rejection logic observed failing against a broken classifier.
- `Expired` and `Revoked` reachable, with real-condition tests, and the RFC-046
  C-1 "not currently produced" notes removed in the same change that makes them
  false.
- No change to `claim_code` or to any public error type.

## 10. Incidental

`SessionFailure`'s rustdoc states that the no-conversion test lives "in this
module's `tests`". It lives at
`crates/codlet/tests/rfc_046_no_public_conversion_compile_fail.rs`. A one-line
correction, folded into whichever handoff lands first — noted here so it is not
lost.
