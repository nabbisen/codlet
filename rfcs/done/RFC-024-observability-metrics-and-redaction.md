# RFC-024: Observability, Metrics, and Redaction

## Status

Implemented (v0.8.0). This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

Operators need to diagnose rate limiting, failed joins, expired sessions, and storage errors. But authentication observability can easily leak secrets or enable enumeration.

## Decision

codlet will expose structured, redacted observability hooks. Metrics and events use categories, not secret values.

## Metrics

Recommended counters:

- `codlet_code_issue_total`;
- `codlet_code_redeem_attempt_total`;
- `codlet_code_redeem_failure_total{reason="generic"}`;
- `codlet_code_claim_won_total`;
- `codlet_code_claim_lost_total`;
- `codlet_form_token_consume_total{outcome="proceed|replay|invalid"}`;
- `codlet_session_issue_total`;
- `codlet_session_validate_total{outcome="active|missing|expired|revoked"}`;
- `codlet_rate_limit_block_total`.

Do not label metrics with code IDs, subject IDs, IP addresses, user agents, or raw scopes by default.

## Tracing

Tracing spans may include:

- operation name;
- adapter name;
- outcome category;
- duration;
- error class.

They must not include plaintext secrets or lookup keys.

## Redaction policy

Every event payload must pass a redaction filter before reaching sinks. Forbidden field names include `code`, `plain_code`, `session_secret`, `token`, `pepper`, `key`, and raw request body fields.

## Acceptance criteria

- Unit tests verify debug/event payload redaction.
- Metrics examples do not use high-cardinality labels.
- Observability hooks are optional and disabled by default or no-op by default.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
