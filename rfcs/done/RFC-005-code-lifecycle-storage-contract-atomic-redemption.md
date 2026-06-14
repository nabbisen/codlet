# RFC-005: Code Lifecycle, Storage Contract, and Atomic Redemption

- **Status:** Implemented (v0.2.0)
- **Target milestone:** M2
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define the one-time code data model and the single-winner redemption contract.

## 2. Motivation

The most security-critical service behavior is the conditional `mark_used` update. codlet must make this an explicit trait-level guarantee rather than an implementation accident.

## 3. Decision

Introduce `CodeStore` with `find_redeemable`, `claim_code`, and `revoke_code`. `claim_code` returns `ClaimOutcome::Won` or `Lost`, and only `Won` permits the host to proceed.

## 4. Detailed design


Lifecycle:

```text
created → redeemable → claimed
                   ↘ revoked
                   ↘ expired
```

`find_redeemable` filters out used, revoked, and expired codes.

`claim_code` must be equivalent to:

```sql
UPDATE codlet_codes
SET used_at = :now, used_by_subject_id = :subject
WHERE id = :id
  AND used_at IS NULL
  AND revoked_at IS NULL
  AND expires_at > :now
```

Affected rows:

- `1` → `ClaimOutcome::Won`
- `0` → `ClaimOutcome::Lost`
- anything else → store invariant violation

Opaque grant:

- stored as optional bytes/string/JSON;
- returned to host;
- never interpreted by codlet.

Scope key:

- optional string used for scoped revoke or lookup;
- codlet does not define its semantics.


## 5. Security considerations

The primary race defense is the storage-level conditional update. Examples must never create sessions after `Lost`. Adapters must prove exactly-one-winner behavior under concurrency.

## 6. Host application responsibilities

The host creates or resolves its subject and passes the subject ID to `claim_code`. If `Lost`, it must abort user/session creation or compensate according to its own transaction model.

## 7. Tests and release gates


- `find_redeemable` rejects expired/used/revoked records.
- Concurrent claim test: exactly one `Won`.
- Claim after revoke returns `Lost`.
- Claim after expiry returns `Lost`.
- Wrong scope revoke does not revoke.
- Adapter conformance suite for every backend.


## 8. Migration notes

Existing `invite_codes` can map `used_by_membership_id` to `used_by_subject_id` and `grants_role` to opaque grant. 

## 9. Open questions

None at this stage. 


## 14. Expanded technical design

### 14.1 Claim operation contract

The `claim` operation is the most important persistence contract in codlet. It must be described as a compare-and-set state transition:

```text
precondition: record is redeemable at time T
transition: set used_at = T and optional used_by/claim_id
postcondition: no later claim can satisfy the same precondition
```

The adapter must use the database's atomic update/transaction mechanism. A read-then-write sequence without conditional write is not production-safe.

### 14.2 Claim request fields

A practical claim request should include:

| Field | Purpose |
|---|---|
| `lookup_key_candidates` | Active/previous key derived candidates. |
| `purpose` | Prevent cross-flow redemption. |
| `scope` | Optional host tenant/community boundary. |
| `now` | Single clock value for expiry and used_at. |
| `claim_actor` | Optional host opaque ID for audit/recovery. |
| `idempotency_key` | Optional future support; does not weaken single-use claim. |

### 14.3 Failure-mode mapping

Adapters may classify failures richly for internal events, but the high-level public response must collapse them:

```text
NotFound | Expired | Revoked | AlreadyUsed | PurposeMismatch | ScopeMismatch -> PublicAuthError::InvalidOrExpiredCode
StorageUnavailable -> PublicAuthError::TemporaryProblem
PolicyError -> configuration/internal error
```

This prevents code enumeration and avoids support copy such as "this code was already used" that reveals record existence.

### 14.4 Host-side transaction advice

If host record creation happens after claim, there is a risk of a claimed code without created user/membership if host creation fails. The design should expose enough metadata for operators to recover:

- claim ID;
- code record ID;
- timestamp;
- optional host actor/flow ID;
- internal failure event.

The library should document that fully atomic host creation requires shared transaction support or host-managed transaction integration.

### 14.5 Concrete acceptance checklist

- [ ] Adapter conformance test proves exactly one winner under concurrency.
- [ ] Public mapping collapses all non-success auth states.
- [ ] `changed > 1` is treated as invariant violation.
- [ ] Claim request includes purpose and optional scope.
- [ ] No high-level helper issues a session unless claim outcome is `Won`.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
