# RFC-007: Form Tokens, CSRF Protection, and Idempotency Replay

- **Status:** Implemented (v0.2.0)
- **Target milestone:** M2
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define single-use form-token issue/consume semantics for CSRF protection and double-submit idempotency.

## 2. Motivation

The service uses form tokens as both CSRF defense and replay control. The pure consume classifier already exists and should become a codlet primitive.

## 3. Decision

codlet will provide `FormTokenManager`, `FormTokenStore`, and `TokenConsumeOutcome::{Proceed, Replay, Invalid}`. v1 must support result replay using `result_ref`.

## 4. Detailed design


Issue:

- generate plaintext token;
- hash under `SecretDomain::FormToken`;
- store subject binding, purpose, optional bound resource, issued_at, expires_at;
- return plaintext token for hidden form field or short-lived cookie.

Consume:

```sql
UPDATE codlet_form_tokens
SET consumed_at = :now
WHERE lookup_key = :key
  AND subject_kind = :kind
  AND subject_value IS :value
  AND purpose = :purpose
  AND COALESCE(bound_resource, '') = :bound
  AND expires_at > :now
  AND consumed_at IS NULL
```

Classification:

- affected rows `1` → `Proceed`
- affected rows `0` + matching consumed row → `Replay(result_ref)`
- otherwise → `Invalid`

Pre-auth support:

- `TokenSubject::Anonymous` for join forms;
- `TokenSubject::Ephemeral` for join tickets;
- `TokenSubject::Subject` for authenticated users.


## 5. Security considerations

CSRF defense must not rely solely on SameSite. A token with wrong subject, purpose, resource, expiry, or consumed status must not proceed. `changed == 0` must never proceed.

## 6. Host application responsibilities

The host must include tokens in all state-changing forms and handle `Replay` safely. It should not reuse a form token for unrelated operations.

## 7. Tests and release gates


- Winner proceeds.
- Loser sees replay.
- Unknown token invalid.
- Binding mismatch invalid.
- Purpose mismatch invalid.
- Expired unconsumed invalid.
- `changed == 0` never proceeds.
- Result ref is returned on replay once implemented.


## 8. Migration notes

The service already has `form_tokens.result_ref` and `set_result`; codlet should complete that deferred path. 

## 9. Open questions

None at this stage. 


## 13. Expanded technical design

### 13.1 Token purpose namespace

Token purposes should be host-extensible but structured. Recommended representation:

```text
codlet reserved: codlet.redeem_code, codlet.session_logout, codlet.flow_continue
host custom: host.<application-defined>
```

Purpose values must be stable strings or strongly typed wrappers; they are persisted and used in consume queries.

### 13.2 Bound resource design

`bound_resource` should be a lookup-safe binding value, not a plaintext domain object. For example, a profile form token may be bound to `HMAC(ticket_value)` rather than storing `invite_id:community_id` directly. This reduces accidental leakage and avoids token reuse across resources.

### 13.3 Anonymous flow model

The source service uses `user_id = ""` before authentication. codlet should avoid an API that makes empty string special. Better design options:

```text
TokenSubject::Anonymous
TokenSubject::Authenticated(Subject)
TokenSubject::Flow(FlowId)
```

This prevents accidental collision between anonymous and real subject IDs.

### 13.4 Replay classification table

| First consume state | Second consume state | Outcome | Recommended host behavior |
|---|---|---|---|
| Proceed and side effect complete | consumed, optional result exists | Replay | redirect to result if safe |
| Proceed but response lost | consumed, no result | Replay | generic refresh/retry guidance |
| Token expired before use | unconsumed expired | Invalid | reload form |
| Binding mismatch | unconsumed or consumed | Invalid | generic failure, possible event |

### 13.5 Concrete acceptance checklist

- [x] Empty-string anonymous token use is replaced by explicit anonymous/flow type in design.
- [x] Purpose and bound resource are part of consume preconditions.
- [x] Replays cannot trigger side effects.
- [x] Token plaintext is never stored or logged.
- [x] CSRF docs avoid exposing framework-specific jargon to end users.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
