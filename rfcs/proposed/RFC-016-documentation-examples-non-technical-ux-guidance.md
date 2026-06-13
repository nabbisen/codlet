# RFC-016: Documentation, Examples, and Non-Technical UX Guidance

- **Status:** Proposed
- **Target milestone:** M5
- **Primary crate(s):** workspace-wide
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define documentation and examples that help developers build safe, low-friction flows for non-technical users.

## 2. Motivation

codlet exists because the user experience of passwords and IdP redirects is too heavy for some communities. The library docs must preserve this motivation without mixing UI code into core.

## 3. Decision

Ship developer docs, security docs, and examples. Include UX guidance for host applications, but do not provide a UI framework in codlet v1.

## 4. Detailed design


Required docs:

- README quick start;
- threat model;
- secure configuration guide;
- key rotation guide;
- code entropy and rate-limit guide;
- adapter guarantee matrix;
- migration guide from zinnias-ciao-style schema;
- examples for Worker and Axum.

UX guidance:

- one field for the code;
- allow spaces/hyphens in entered code;
- avoid jargon such as token, HMAC, IdP, OAuth, expired credential;
- use generic but helpful failure copy;
- show codes in grouped format for readability if host chooses;
- provide clear admin revocation workflow.

Examples must keep app authorization visible after authentication.


## 5. Security considerations

Docs must not encourage unsafe short-code deployments without rate limits. Examples are security-sensitive because developers copy them.

## 6. Host application responsibilities

The host owns all user-facing copy, accessibility, language, and forms. codlet provides primitives and safe patterns.

## 7. Tests and release gates


- All README and example code compiles.
- Examples use secure defaults.
- Documentation lints check for banned misleading wording such as "authorization by codlet".


## 8. Migration notes

zinnias-ciao Japanese copy stays in the service. codlet docs may describe UX principles in English. 

## 9. Open questions

None at this stage. 


## 10. Expanded technical design

### 10.1 Documentation audiences

Docs should target three audiences:

| Audience | Needs |
|---|---|
| Application developer | quick start, adapter examples, host responsibility boundary. |
| Security reviewer | threat model, invariants, key management, conformance tests. |
| Operator | configuration checklist, rotation, incident response, rate-limit tuning. |

### 10.2 Example rules

Examples are security-sensitive and should follow these rules:

- Never show hard-coded production secrets.
- Never print generated codes to normal server logs except in explicitly local examples.
- Never store session secrets in JavaScript-readable storage.
- Always show host authorization after authentication.
- Use safe defaults: 8+ character codes, secure cookies, generic public failures.
- Mark shorter zinnias-ciao-style codes as compatibility/small-community examples with rate limits.

### 10.3 UX guidance boundary

codlet can recommend UX patterns but must not own UI rendering. Recommended language principles:

- Say "code" instead of "token" to users.
- Say "please check the code" instead of "invalid credential".
- Say "please reload and try again" instead of "CSRF failed".
- Avoid showing whether a code was expired, already used, revoked, or nonexistent.

### 10.4 Concrete acceptance checklist

- [ ] README has a safe quickstart and a security note.
- [ ] Threat model is linked from README.
- [ ] Adapter guarantee matrix is visible.
- [ ] All example code compiles in CI.
- [ ] User-facing copy guidance avoids jargon.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
