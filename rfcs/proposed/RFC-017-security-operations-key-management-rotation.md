# RFC-017: Security Operations, Key Management, and Rotation

- **Status:** Proposed
- **Target milestone:** M3
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define operational requirements for HMAC key management, rotation, and compromise response.

## 2. Motivation

The service handoff identifies missing key versioning as a weakness. If codlet adds key versions from the start, it can support practical rotation before v1.

## 3. Decision

codlet will model active and previous HMAC keys, store key versions, and document rotation and emergency invalidation procedures.

## 4. Detailed design


Key states:

- active: used for newly issued records;
- previous: accepted for validation until records expire or are migrated;
- retired: not accepted;
- compromised: trigger emergency invalidation.

Rotation plan:

1. deploy new key as active, old key as previous;
2. new codes/sessions/tokens use new version;
3. old sessions expire or are selectively reissued;
4. remove old key after max TTL;
5. run audit check for records still on old version.

Emergency compromise:

- revoke sessions issued under compromised key if database also suspected;
- invalidate short codes and form tokens if needed;
- rotate active key immediately;
- communicate service-level recovery outside codlet.

Adapter support:

- D1/SQLx schemas include key-version columns;
- key provider errors identify missing version without leaking key material.


## 5. Security considerations

Short human codes are vulnerable to offline guessing if both database and HMAC key leak. Key rotation reduces long-term exposure but does not erase compromise history.

## 6. Host application responsibilities

The host must manage secrets in its deployment platform and decide whether emergency rotation invalidates sessions or preserves previous keys during a grace period.

## 7. Tests and release gates


- Records issued with old key validate while previous key configured.
- Old records fail after key removed.
- New records use active key version.
- Missing key version fails closed.
- Rotation examples compile.


## 8. Migration notes

zinnias-ciao must add key-version columns to invite, session, and form-token tables before smooth rotation. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Operational key inventory

A production deployment should maintain a key inventory:

| Field | Purpose |
|---|---|
| version | matches persisted `key_version` |
| status | active/previous/retired/compromised |
| created_at | audit and rotation planning |
| activated_at | when new records started using it |
| retire_after | max TTL after deactivation |
| storage location | env secret, secret manager, KMS reference |

codlet may model this as config; operators own secure storage.

### 10.2 Planned rotation procedure

1. Generate new key outside application logs.
2. Deploy config with new key as active and old key as previous.
3. Verify new records use new version.
4. Wait longer than maximum lifetime of records issued with old key, or proactively reissue sessions.
5. Remove old key from previous list.
6. Verify no active records remain with old key version.

### 10.3 Emergency compromise procedure

If database and HMAC key are both suspected compromised, assume bearer secrets can be attacked offline. Recommended response:

- revoke active sessions or force re-login;
- revoke outstanding codes and form tokens;
- rotate key immediately;
- inspect audit/security events;
- communicate host-specific recovery steps.

### 10.4 Concrete acceptance checklist

- [ ] Key versions exist in all schemas.
- [ ] Active/previous validation behavior is tested.
- [ ] Missing previous key fails closed for affected records.
- [ ] Rotation guide includes max-TTL waiting rule.
- [ ] Emergency guide distinguishes DB-only leak from DB+key compromise.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
