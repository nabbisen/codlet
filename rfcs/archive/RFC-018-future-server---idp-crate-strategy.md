# RFC-018: Future Server / IdP Crate Strategy

- **Status:** Proposed
- **Target milestone:** M6 / post-v1
- **Primary crate(s):** codlet-server future
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define how a future standalone server or IdP product may be built without distorting the embedded library.

## 2. Motivation

A standalone service may become useful after codlet stabilizes, especially for multi-service SSO or centralized audit. Starting there would impose unnecessary operations on initial adopters.

## 3. Decision

Defer `codlet-server` until after embedded v1.0. The future server must depend on public codlet APIs and must not require changes that make embedded use worse.

## 4. Detailed design


Possible future server capabilities:

- hosted code issue/redeem routes;
- session introspection endpoint;
- admin invite management;
- OIDC/OAuth provider exploration;
- audit and metrics;
- multi-tenant storage.

Constraints:

- no server-only concepts in `codlet-core`;
- no redirect/callback assumptions in core;
- embedded examples remain first-class;
- authorization remains application-owned unless a separate product explicitly defines it.

Decision gate for starting server work:

- at least one production-like embedded adopter;
- stable core API;
- clear demand for multi-service centralization;
- separate threat model for federation/assertions.


## 5. Security considerations

An IdP/server has a larger blast radius and different threats: redirect URI validation, client registration, token signing, federation metadata, and central audit. It needs a separate threat model.

## 6. Host application responsibilities

Host applications should not wait for server work unless they specifically need SSO or centralized auth operations.

## 7. Tests and release gates


- No v1 embedded tests depend on codlet-server.
- Future server RFC must include a separate OIDC/federation threat model.


## 8. Migration notes

No migration before post-v1 exploration. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Server/IdP decision gate

`codlet-server` work may start only when the following are true:

- embedded core has at least one production-like adopter;
- core API has survived Worker and one server adapter;
- there is explicit demand for multi-service centralization;
- a separate threat model covers federation, redirect flows, client registration, token signing, and service-to-service trust;
- server work does not require breaking embedded APIs.

### 10.2 Future server layers

A future server should still reuse codlet layers:

```text
HTTP/admin UI/API
  -> server-specific policy and tenant management
  -> codlet-core auth primitives
  -> codlet stores/adapters
```

If OIDC/OAuth provider behavior is added, it should live in a separate RFC series and likely separate crates. OIDC tokens and codlet session secrets must not be conflated.

### 10.3 Migration from embedded to server

A future migration path may exist, but v1 does not promise it. Likely migration issues:

- subject identifier mapping;
- session invalidation;
- code delivery/admin workflow changes;
- redirect/callback configuration;
- trust boundary and TLS/domain requirements;
- audit log centralization.

### 10.4 Concrete acceptance checklist

- [ ] No embedded v1 API depends on `codlet-server`.
- [ ] Server RFCs cannot modify RFC-001 scope without explicit revision.
- [ ] Future IdP work has its own security requirements.
- [ ] Embedded examples remain first-class after server exploration begins.
- [ ] OAuth/OIDC terminology is absent from core unless a future RFC justifies it.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
