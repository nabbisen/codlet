# RFC-006: Session Issuance, Validation, Revocation, and Cookie Policy

- **Status:** Implemented (v0.2.0)
- **Target milestone:** M2
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define opaque session secrets, HMAC-backed session storage, secure cookie construction, and validation.

## 2. Motivation

The source service correctly stores only session HMAC values and uses secure cookie attributes. codlet should preserve this as a reusable session primitive.

## 3. Decision

Create `SessionManager`, `SessionStore`, `SessionPolicy`, and `CookiePolicy`. Secure cookie attributes are mandatory defaults.

## 4. Detailed design


Session issue:

1. generate high-entropy secret;
2. hash with `SecretDomain::Session` and active key version;
3. insert record with subject ID, created_at, expires_at;
4. return `IssuedSession` containing secret for cookie creation.

Validation:

1. extract cookie secret;
2. hash using candidate key versions;
3. lookup active session where not revoked and not expired;
4. return `ActiveSession { subject_id, session_id }`.

Cookie builder:

```rust
pub struct CookiePolicy {
    pub name: String,
    pub path: String,
    pub max_age: Duration,
    pub same_site: SameSitePolicy, // default Strict
    pub secure: bool,              // default true
    pub http_only: bool,           // default true, cannot be false in production policy
    pub domain: Option<String>,
}
```

`HttpOnly`, `Secure`, and `SameSite=Strict` are the normal defaults. Relaxation, if any, must require explicit development-only configuration.


## 5. Security considerations

Session cookies are bearer credentials. Database storage must not contain plaintext. Cookie `Domain` should be omitted by default to avoid subdomain leakage. TTL must be configured by codlet/session policy, never derived from an unrelated upstream token expiration.

## 6. Host application responsibilities

The host must call authorization after session validation. It must provide HTTPS in production and a logout path that revokes the session and clears the cookie.

## 7. Tests and release gates


- Generated session secrets have required length/format.
- Plaintext session secret is not stored.
- Expired session is inactive.
- Revoked session is inactive.
- Cookie contains `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/` by default.
- Cookie omits `Domain` when domain is None.
- Clear cookie uses `Max-Age=0`.


## 8. Migration notes

zinnias-ciao can keep `ciao_sid` as a configured cookie name and map `sessions.user_id` to `SubjectId`. 

## 9. Open questions

None at this stage. 


## 13. Expanded technical design

### 13.1 Session secret representations

Session handling must separate:

| Representation | Location | Persistence |
|---|---|---|
| plaintext session secret | cookie / request only | never persisted |
| lookup key | DB | persisted |
| session ID | DB/audit/admin UI | persisted, not a bearer credential |
| subject | DB | persisted, host-owned meaning |

A session ID alone must not authenticate a request. Only possession of the session secret plus an active record authenticates.

### 13.2 Cookie builder policy profiles

Define named profiles rather than ad hoc booleans:

| Profile | Secure | HttpOnly | SameSite | Domain | Intended use |
|---|---:|---:|---|---|---|
| `ProductionStrict` | yes | yes | Strict | none by default | Default. |
| `ProductionLax` | yes | yes | Lax | explicit only | Needed for some top-level cross-site flows. |
| `LocalDevelopment` | configurable | yes | Lax/Strict | none | Must be opt-in and visibly non-production. |

A production profile should reject `Secure=false`.

### 13.3 Validation response shape

Session validation should return a structured result:

```text
Authenticated { subject, session_id, issued_at, expires_at, metadata_view }
Unauthenticated { reason: redacted internal classification }
StoreUnavailable
ConfigurationError
```

Framework adapters map this into extractors/middleware. Core should not force redirect decisions.

### 13.4 Revocation semantics

Revocation is monotonic. A revoked session cannot be unrevoked by codlet. If the host wants temporary suspension, it should implement authorization checks separately rather than toggling session validity.

### 13.5 Concrete acceptance checklist

- [ ] Cookie defaults are secure by construction.
- [ ] Clear-cookie helper mirrors path/domain/name.
- [ ] Session validation does not update mutable state unless renewal is explicitly configured.
- [ ] Revoked and expired sessions are indistinguishable to public callers.
- [ ] Examples demonstrate host authorization after session validation.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
