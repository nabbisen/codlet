# RFC-011: SQLx and In-Memory Adapters

- **Status:** Partially implemented (v0.5.0 — SQLite + in-memory done; PostgreSQL adapter not yet written)
- **Target milestone:** M4
- **Primary crate(s):** codlet-sqlx + codlet-test
- **Source basis:** zinnias-ciao v0.36.1 service handoff

## 1. Summary

Add portable storage backends to validate codlet outside Cloudflare Workers.

## 2. Motivation

A library abstraction should not be shaped only by D1. SQLx adapters prove the core model works for conventional Rust services and normal transaction support.

## 3. Decision

Build an in-memory conformance adapter early and SQLx SQLite/Postgres adapters before v1.0.

## 4. Detailed design


In-memory adapter:

- deterministic, test-only or dev-only;
- supports concurrency tests with locks;
- must not be recommended for production.

SQLx adapter:

- schema based on `codlet_codes`, `codlet_sessions`, and `codlet_form_tokens`;
- use transactions or conditional updates;
- expose migrations;
- satisfy Send requirements;
- run the same conformance suite as D1.

SQLite claim uses conditional update and affected rows. PostgreSQL may additionally use transactions or `RETURNING`.


## 5. Security considerations

The in-memory adapter can be misused. It must have explicit naming and docs. SQLx adapters must enforce the same single-winner guarantees as D1.

## 6. Host application responsibilities

The host must apply migrations and choose an isolation mode appropriate for its workload. It should not use in-memory stores for multi-instance production deployments.

## 7. Tests and release gates


- Shared conformance suite for in-memory, SQLite, PostgreSQL.
- Concurrent claim test under real async tasks.
- Session expiry/revocation tests.
- Form-token replay tests.
- Migration smoke tests.


## 8. Migration notes

No existing application migration is required beyond adopting the new codlet API. 

## 9. Open questions

Whether Redis rate limiting belongs in v1.0 or later. 


## 10. Expanded technical design

### 10.1 SQL adapter scope

`codlet-sqlx` should start with SQLite because it is practical for local development and small embedded services. Postgres can follow once the trait contract stabilizes. MySQL/MariaDB should not be promised until SQL semantics are reviewed.

### 10.2 Schema portability principles

The SQL schema should use common concepts:

- text lookup keys and key versions;
- integer or timestamp fields with documented timezone/precision;
- partial indexes where available, but not required for correctness;
- conditional updates for atomic claim/consume;
- uniqueness constraints preventing duplicate active lookup rows where supported.

### 10.3 In-memory adapter status

The in-memory adapter is a test and example tool unless explicitly documented otherwise. It must be safe for deterministic tests, not advertised for production. It should support simulated races to test exactly-one-winner behavior.

### 10.4 Migration ownership

`codlet-sqlx` may ship migration files, but host applications own migration application order and naming. The adapter should support configurable table prefix/schema where practical, but table-name customization must not make conformance impossible.

### 10.5 Concrete acceptance checklist

- [x] SQLite adapter passes all store conformance tests.
- [x] In-memory adapter is documented as non-production.
- [x] Schema includes key version columns from first migration.
- [x] Atomic operations do not rely on read-then-write without conditional update.
- [x] Migration docs explain host-owned domain tables remain separate.


## References

- zinnias-ciao service handoff for codlet, v0.36.1, 2026-06-13.
- NIST SP 800-63B Digital Identity Guidelines: Authentication and Authenticator Management, SP 800-63-4 edition.
- OWASP Authentication Cheat Sheet.
- OWASP Session Management Cheat Sheet.
- OWASP Cross-Site Request Forgery Prevention Cheat Sheet.
- OWASP REST Security Cheat Sheet.
