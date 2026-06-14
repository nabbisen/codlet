# RFC-001: Project Scope, Product Shape, and Non-goals

- **Status:** Implemented (v0.0.0)
- **Target milestone:** M0
- **Primary crate(s):** workspace-wide
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define codlet as an embedded one-time-code authentication library, not a standalone identity provider or application authorization system.

## 2. Motivation

The source service demonstrates that short human-friendly codes can remove major onboarding friction for non-technical users. If codlet starts as an IdP, adopters inherit multi-service operations, redirect/callback configuration, and cross-service trust management. The reusable value is the safe authentication primitive, not a second server.

## 3. Decision

codlet will ship first as a library family. `codlet-core` contains primitives and contracts; adapters provide runtime integration. A future `codlet-server` may be built later, but it must depend on the same stable core and may not distort the embedded API.

## 4. Detailed design


Scope includes:

- one-time code generation, normalization, validation, and HMAC lookup;
- atomic one-time redemption;
- session issuance, validation, revocation, and secure cookie construction;
- form tokens for CSRF and idempotency;
- rate-limit contracts;
- security events and audit hooks;
- adapters for selected runtimes.

Scope excludes:

- user profile storage;
- display names;
- roles, permissions, groups, organizations, communities;
- email/SMS sending;
- UI rendering;
- account recovery;
- OIDC/OAuth provider behavior in v1;
- business audit schema.

Naming convention:

- package names use `codlet-*`;
- Rust module paths use `codlet_*`;
- the root package may re-export stable core APIs.


## 5. Security considerations

A narrow scope reduces confused-deputy risk. codlet must not blur authentication with authorization; host services must always perform their own resource checks after codlet returns a subject.

## 6. Host application responsibilities

The host application owns identity records, membership, role checks, UI copy, code delivery, and all authorization decisions. It must not pass user-supplied subject IDs as trusted values.

## 7. Tests and release gates


- Documentation tests showing authentication followed by host-side authorization.
- Static review gate: no `Role`, `Community`, `Organization`, `Permission`, or service-specific concepts in `codlet-core` public API.
- Examples must not imply codlet grants access by itself.


## 8. Migration notes

zinnias-ciao should continue to own `community_memberships`, `grants_role` interpretation, display names, and Japanese UI. Only authentication primitives move. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Product boundary contract

codlet's public contract is intentionally narrower than the authentication surface of a complete application. A host application must be able to replace its local invite-code/login machinery with codlet without adopting a new user model. Therefore the stable contract is expressed in four neutral objects:

| Object | Owned by | Purpose | Must not contain |
|---|---|---|---|
| `Subject` | Host | The authenticated principal identifier returned after successful authentication. | Role, permissions, community membership, profile fields. |
| `CredentialRecord` / `CodeRecord` | codlet store | One-time code metadata and lifecycle state. | Plaintext code, delivery channel content. |
| `SessionRecord` | codlet store or host store | Server-side session validation state. | Authorization grants, UI preferences. |
| `SecurityEvent` | codlet event vocabulary | Redacted authentication/security lifecycle observation. | Business audit meaning such as event creation or membership approval. |

A host should be able to choose `Subject = String`, `Subject = Uuid`, or a domain-specific wrapper. codlet may validate type shape but must not interpret business meaning.

### 10.2 Accepted product shapes

The following product shapes are accepted in v1 design:

1. **Primitive-only use:** host uses codlet for code generation, normalization, HMAC lookup derivation, and state-machine classification but keeps all storage code local.
2. **Store-integrated use:** host uses codlet storage traits and adapters for code/session/token tables.
3. **Framework-integrated use:** host uses Axum/Worker helpers for request extraction, cookies, and response mapping.
4. **Orchestrated use:** host uses a high-level `CodeAuth` service that coordinates primitives and stores, while still delegating user creation and authorization to host callbacks.

The following product shapes are rejected before post-v1 work:

- mandatory redirect-based login;
- mandatory standalone auth server;
- mandatory central tenant model;
- library-owned user tables;
- library-owned role/permission model.

### 10.3 Non-goal enforcement gates

The non-goals must be enforced mechanically where possible:

- Public API review fails if core exposes `Role`, `Permission`, `Community`, `Organization`, `TenantAdmin`, `GroupMember`, or equivalent service-domain types.
- Examples must show host authorization after codlet authentication.
- Documentation must use the phrase "authentication" for codlet success, not "access granted" unless immediately followed by host authorization.
- `codlet-core` dependency review fails if web framework or database crates appear in its normal dependency tree.
- Test utility fallback keys must be unavailable unless a clearly named test feature/module is used.

### 10.4 Design consequences

This scope deliberately makes codlet less convenient than a full SaaS IdP in some cases. That is acceptable. The value proposition is low operational overhead and embeddability. If an adopter wants SSO, centralized policy, federation, cross-service session introspection, or hosted account recovery, that is a separate product track. The embedded library should remain stable even if the future `codlet-server` becomes popular.

### 10.5 Concrete acceptance checklist

- [x] README states "embedded one-time-code authentication library" in the first paragraph.
- [x] Requirements document includes a non-goals table.
- [x] External design has a host-owned authorization section.
- [x] Every RFC that returns a subject/session states that the host must still authorize.
- [x] RFC-018 remains post-v1 and cannot add requirements back into RFC-001 without an explicit revision.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
