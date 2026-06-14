# RFC-014: zinnias-ciao Migration and Compatibility Plan

- **Status:** Implemented (v0.6.0)
- **Target milestone:** M5
- **Primary crate(s):** codlet-core + codlet-worker
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define an incremental extraction path from zinnias-ciao to codlet without destabilizing the service.

## 2. Motivation

The service is the reference implementation and should remain functional during extraction. Big-bang auth rewrites are risky.

## 3. Decision

Migrate in layers: pure functions first, then cookie/session helpers, then form-token classification, then Worker/D1 adapter operations.

## 4. Detailed design


Step 1: Pure extraction

- move code alphabet, length policy, generation, normalization, validation;
- move HMAC helpers and test vectors;
- keep service storage unchanged.

Step 2: Session/cookie extraction

- replace cookie builder and clearing cookie;
- keep `sessions` table unchanged.

Step 3: Form-token classification

- replace pure `classify_token_consume` with codlet;
- keep `form_tokens` SQL unchanged.

Step 4: Worker adapter compatibility

- configure D1 adapter table names: `invite_codes`, `sessions`, `form_tokens`;
- map service columns to codlet concepts;
- keep service-specific `community_id`, `created_by_membership_id`, and `grants_role` outside core.

Step 5: Key-version migration

- add key-version columns;
- backfill current version;
- update key provider.

Step 6: Optional table rename later

- move to codlet-owned table names only if useful.


## 5. Security considerations

Each migration step must preserve existing security invariants. The dangerous points are key-provider behavior, claim outcome handling, and form-token replay classification.

## 6. Host application responsibilities

zinnias-ciao must continue to own community authorization, display names, role grants, UI copy, and audit vocabulary.

## 7. Tests and release gates


- Existing 223 service tests remain green after each step.
- Full join flow integration test.
- Concurrent double-submit test.
- Admin invite generation/revocation tests.
- No plaintext code/session/form-token in database tests.


## 8. Migration notes

This RFC is itself the migration plan. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Migration strategy

zinnias-ciao migration should be staged to reduce risk:

| Phase | Change | Rollback risk |
|---|---|---|
| 1 | Move pure code generation/normalization/HMAC tests to codlet. | Low |
| 2 | Use codlet for new code generation while preserving schema. | Low |
| 3 | Add key-version columns and compatibility values. | Medium |
| 4 | Replace form token state machine with codlet. | Medium |
| 5 | Replace invite claim with codlet Worker/D1 adapter. | High |
| 6 | Replace session cookie/session lookup helpers. | Medium |
| 7 | Remove duplicated service code after soak. | Medium |

### 10.2 Compatibility requirements

During migration:

- Existing unexpired invite codes must remain redeemable or be intentionally expired with admin communication.
- Existing sessions should remain valid unless the service chooses a forced logout release.
- Japanese UI copy must not change as a side effect of library extraction.
- Service-specific tests remain the final gate even after codlet tests pass.

### 10.3 Shadow validation option

Before switching redemption fully, zinnias-ciao can run codlet derivation/classification in shadow mode for generated test fixtures: service result and codlet result must agree, but codlet does not control production outcome yet.

### 10.4 Concrete acceptance checklist

- [ ] Migration plan identifies database migrations and rollback points.
- [ ] Existing security release gates move into codlet or remain in service as appropriate.
- [ ] Service tests verify full join flow before and after extraction.
- [ ] Admin UI behavior remains unchanged unless deliberately redesigned.
- [ ] Deployment notes state whether existing sessions/codes survive the migration.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
