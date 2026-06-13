# RFC-022: Database Atomicity, Isolation, and Race Semantics

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

One-time code redemption and form-token consumption are only safe if storage enforces a single-winner transition. A trait that says `claim()` is not enough; adapters must implement it with precise atomic semantics.

## Decision

codlet store contracts will require conditional writes for one-time transitions. Read-then-write implementations are non-conformant unless protected by a transaction isolation level or lock that is proven equivalent.

## Code claim contract

The claim operation must update only rows that are currently unclaimed, unrevoked, and unexpired. Exactly one concurrent caller may observe `Won`.

## Form token consume contract

The consume operation must update only an unconsumed token matching lookup key, subject, purpose, bound resource, and expiry. Exactly one caller may observe `Proceed`.

## Isolation requirements

- SQLite/D1: use a single conditional `UPDATE` and affected-row count.
- PostgreSQL: same conditional `UPDATE`; optional `RETURNING` may be used.
- Redis-like stores: use atomic script/transaction semantics.
- In-memory store: use a mutex or equivalent atomic section.

## Race test

The conformance suite must start multiple concurrent tasks attempting the same claim/consume and verify exactly one winner.

## Acceptance criteria

- Store trait docs state the SQL-equivalent predicate.
- Adapter tests prove exactly-one-winner behavior.
- Any adapter without atomic conditional write is marked experimental or rejected.
- Host applications are instructed to create users/sessions only after `Won`.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
