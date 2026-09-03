# RFC-013: High-Level Orchestration API and Host Application Boundary

- **Status:** Implemented (v0.4.0)
- **Target milestone:** M5
- **Primary crate(s):** codlet-core + adapters
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Define ergonomic APIs for common flows while preserving the host application boundary.

## 2. Motivation

Low-level traits are safe but verbose. A useful library should provide high-level flows without taking over user management or authorization.

## 3. Decision

Provide composable managers and optional orchestration helpers. The helpers stop at subject/session boundaries and require host callbacks for subject creation/resolution.

## 4. Detailed design


Managers:

- `CodeAuth` for code issue and redemption;
- `SessionManager` for sessions;
- `FormTokenManager` for form tokens;
- `RateLimiter` wrapper for policy application.

First-time join helper:

```rust
pub async fn redeem_with_subject<F, Fut>(
    &self,
    raw_code: &str,
    rate_key: Option<RateLimitKey>,
    create_subject: F,
) -> Result<RedeemAndSessionOutcome, RedeemError>
where
    F: FnOnce(CodeRecord) -> Fut,
    Fut: Future<Output = Result<SubjectId, HostError>>;
```

The callback receives safe record metadata and opaque grant, creates the host subject, and returns a subject ID. codlet then attempts the claim and may issue a session only on `Won`.

Two-stage flow remains available for profile forms where subject creation requires additional user input.


## 5. Security considerations

High-level helpers must not hide `ClaimOutcome::Lost`. If host subject creation happens before claim and claim loses, the host may need compensation. Docs must make the transaction boundary explicit.

## 6. Host application responsibilities

The host supplies callbacks for user creation/resolution and handles compensation if its storage cannot atomically combine host writes with codlet claim.

## 7. Tests and release gates


- Example first-time join flow compiles.
- Example returning login flow compiles.
- Lost claim does not issue a session.
- Host callback errors do not claim code.
- Public errors remain generic.


## 8. Migration notes

No existing application migration is required beyond adopting the new codlet API. 

## 9. Open questions

Whether high-level APIs should claim before or after host subject creation by default. The answer may differ for invite join versus returning login. 


## 10. Expanded technical design

### 10.1 Orchestration layers

codlet should expose multiple layers rather than one giant helper:

| Layer | Example responsibility | Audience |
|---|---|---|
| Primitive | normalize, generate, derive lookup | security-conscious custom apps |
| Store service | issue code, claim code, issue session | apps using custom routing |
| Flow service | redeem code and call host hooks | apps wanting safe default flow |
| Framework adapter | Axum/Worker route helpers | quick integration |

### 10.2 Host hook points

High-level flows need host hooks:

```text
on_code_claim_won(claim_context) -> host subject or domain action
on_session_issued(session_context) -> optional host audit
on_public_error(error_context) -> host UX mapping
```

The hook that creates a user/member must be explicitly host-owned. codlet must not hide it behind a misleading "create account" default.

### 10.3 Safe orchestration failure order

A high-level join/login flow should enforce this order:

1. rate-limit check;
2. input normalization/validation;
3. CSRF/form token consume if configured;
4. code claim;
5. host hook;
6. session issue;
7. audit/security event;
8. response helper.

If any step before claim fails, no claim occurs. If claim wins but host hook fails, no session is issued. Recovery is host operational responsibility.

### 10.4 Concrete acceptance checklist

- [x] High-level API names do not imply authorization.
- [x] Host subject creation is a callback or explicit caller step.
- [x] Error mapping remains generic.
- [x] Session issuance cannot occur before claim success.
- [x] Examples show both low-level and high-level integration paths. (`sqlite_quickstart.rs`: two-step `find`+`claim` is low-level; `callback_flow_example` uses `redeem_with_callback` for high-level.)


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
