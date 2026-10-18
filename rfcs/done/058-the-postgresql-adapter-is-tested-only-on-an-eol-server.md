# RFC-058: The PostgreSQL Adapter Is Tested Only On an End-of-Life Server

- **Status:** Implemented (Unreleased) — PR #2, `7b08aa7`; reviewed and approved 2026-10-19
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet-sqlx`, CI, `docs/`
- **Source basis:** noticed in the RFC-056 review request's closing note; established 2026-10-19

## 1. Summary

Every PostgreSQL conformance test runs against **PostgreSQL 11**, which reached
end of life in November 2023. Nothing in codlet chose that version — it is a
dependency's hardcoded default. And no document states which PostgreSQL versions
codlet supports, so there is no yardstick against which the gap could have been
noticed.

## 2. Evidence

- `testcontainers-modules` 0.15.0, `src/postgres/mod.rs:6`:
  `const TAG: &str = "11-alpine";`
- No `with_tag` or image override anywhere in `crates/codlet-sqlx/tests/`.
- `grep` across `docs/src/`, `crates/codlet-sqlx/README.md`, `README.md` and
  `SECURITY.md`: **no stated PostgreSQL version requirement.**
- `migrations/0002_postgres.sql` reasons about per-feature minimums — "IF NOT
  EXISTS on all CREATE TABLE and CREATE INDEX (PostgreSQL 9.1+)",
  "`ADD COLUMN IF NOT EXISTS` (9.6+)" — which shows the question was considered
  *per statement* and never answered *per product*.

What runs only on PostgreSQL 11:

- the concurrent single-winner claim test that proves **INV-5** for this
  adapter;
- the `READ COMMITTED` row-lock behaviour the adapter matrix cites by name;
- `run_postgres_migrations`, including RFC-057's new pre-RFC-044 upgrade tests;
- RFC-056's TLS test.

Versions 12 through 18 are untested. Meanwhile
`docs/src/adapter-matrix-and-config.md` presents `PostgresStore` as
"Multi-instance production" with ✓ for atomic claim and atomic consume.

## 3. Why this matters more than a version bump

**The adapter is almost certainly fine.** It uses no exotic SQL — RFC-034
deliberately avoided `RETURNING` and `FOR UPDATE` — so the probability that
PostgreSQL 16 breaks it is low.

That is not the point. The point is the same one RFC-038 and RFC-054 made:
a guarantee table is only worth what its evidence covers, and here the evidence
covers a server no host should be running. Nobody deploying codlet on a managed
PostgreSQL today is running 11; providers no longer offer it. So the tested
configuration and every real configuration are disjoint.

**The version is also not a decision.** It arrives from a dependency's default,
which means a routine `testcontainers-modules` bump can change what codlet is
tested against, in either direction, with no diff that mentions PostgreSQL. That
is the hazard RFC-039 hit when `cargo-deny-action`'s `--all-features` default
turned out to be doing load-bearing work: an inherited default nobody chose.

## 4. Decision (proposed)

### 4.1 Declare a supported range

State it once, in `docs/src/adapter-matrix-and-config.md`, with the reason:
the oldest version is set by the SQL the adapter actually uses, and the newest
by what is tested.

This is the root cause. Without a declared range, "is this tested on the right
version?" has no answer, which is why three years passed.

**Decision: support PostgreSQL 15 and newer.**

The owner agreed to a floor of 13 on 2026-10-19, on my recommendation that 13
was "the oldest release still receiving security updates for most of this
project's horizon". **That was wrong**, and I checked it before writing the
handoff rather than after:

| Version | Upstream support ends | Status on 2026-10-19 |
|---|---|---|
| 13 | 2025-11-13 (final release 13.23) | **already end of life** |
| 14 | 2026-11-12 | ends in ~4 weeks |
| 15 | 2027-11-11 | supported |
| 16 | 2028-11-09 | supported |
| 17 | 2029-11-08 | supported |
| 18 | 2030-11-14 | supported, newest GA |
| 19 | — | RC 1 on 2026-10-15; pre-release |

A floor of 13 would have codlet declare support for an already-EOL server,
which is the defect this RFC exists to remove, merely two years less badly. A
floor of 14 expires within a month of being written.

So **15**, which is supported until November 2027, and **18** as the upper end —
PostgreSQL 19 is still a release candidate and is not a pin target.

The SQL imposes no constraint either way: every construct
`0002_postgres.sql` relies on is 9.6-or-older. This is a support-policy
decision, and the correction follows the owner's stated intent — "no reason to
use PostgreSQL 11" — more faithfully than my own number did.

**Declared support is not a claim that older versions break.** A host on 13 or
14 will almost certainly work, for the same reason the adapter is almost
certainly fine on 18: the SQL is unexceptional. The range says what is tested
and promised, not what is forbidden, and §4.4 should word it that way.

### 4.2 Pin the image explicitly, and stop inheriting it

Override the tag in the test fixtures rather than taking
`testcontainers-modules`' default, so the version under test is visible in
codlet's own source and a dependency bump cannot move it.

### 4.3 Test both ends of the range: 15 and 18

Run the PostgreSQL conformance suite against **15** and **18**. Two jobs, or one
job with a matrix.

Both ends, not just the newest: the floor is what the declared range promises,
and an untested floor is the same defect in a different place.

The TLS test needs to run on at least one of them, not necessarily both.

### 4.3.1 The pin must verify itself

Pinning a tag is not enough on its own, because the failure this RFC is about
was an inherited default doing work nobody saw. If the override is dropped, or a
matrix variable goes unset, or a future `testcontainers-modules` changes
behaviour, the suite would quietly test one version twice and still be green.

So the test must **assert the major version of the server it actually connected
to** matches the one requested — `SELECT current_setting('server_version')` or
equivalent — and fail if it does not. That turns the pin from a declaration into
a checked property, and it is the same discipline as RFC-057's "assert the column
is absent first" and RFC-056's control connection: prove the test could fail.

### 4.4 Say what is proven, where the claim is made

The adapter matrix's `PostgresStore` row should name the versions its ✓ marks
were established on — as the RFC-038 note already does for the CI run that
first executed this suite, and as RFC-054's D1 note now does for Miniflare.

## 5. Non-goals

- No change to the adapter's SQL, to `READ COMMITTED` reliance, or to the
  decision to avoid `RETURNING`/`FOR UPDATE`.
- No change to SQLite or D1 coverage. SQLite is a linked library at a known
  version; D1 is covered by RFC-054's note.
- No attempt to test every supported version. Two endpoints is the point;
  a full matrix is CI time without a matching increase in information.
- No change to `run_postgres_migrations`.

## 6. Security considerations

INV-5's PostgreSQL evidence is the concurrent single-winner test, and it has
only ever run on PostgreSQL 11. The invariant is enforced by a conditional
UPDATE whose semantics are stable across versions, so this is a gap in evidence
rather than a known weakness — but INV-5 is the invariant RFC-048's SQL
injection and RFC-005's whole design turn on, and "we believe the semantics are
stable" is weaker than "we ran it".

RFC-056's TLS evidence is also PostgreSQL 11 only. TLS negotiation is handled by
rustls and the server's OpenSSL, both of which differ across versions.

## 7. Maintenance consequence

A declared range and a pinned pair of versions both go stale. PostgreSQL 15
leaves support in November 2027 and a new major arrives roughly every September.

This RFC deliberately does **not** build automation for that. A yearly review
item is the honest answer, and it belongs with the M7 release checklist rather
than in a cron job nobody reads. What matters is that §4.3.1 makes a stale pin
*visible* — the version is named in codlet's own source and checked against the
running server — rather than silently inherited, which is the whole defect.
