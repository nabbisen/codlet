# Developer Handoff — RFC-058: The PostgreSQL Adapter Is Tested Only On an End-of-Life Server

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-19
- **RFC:** [`../../accepted/058-the-postgresql-adapter-is-tested-only-on-an-eol-server.md`](../../accepted/058-the-postgresql-adapter-is-tested-only-on-an-eol-server.md)
- **Baseline:** `cffe10e`
- **Needs Docker for the tests.** CI has it; you do not. §8 says how to handle that.
- **Credit:** your closing note on RFC-056 — "whatever `postgres:11-alpine` and rustls agree on" — is what surfaced this.

## 1. Purpose

Every PostgreSQL test runs on **PostgreSQL 11**, which has been end of life
since November 2023, because `testcontainers-modules` 0.15 hardcodes
`TAG = "11-alpine"` and codlet never overrides it. Declare a supported range,
pin the versions in our own source, and test both ends.

## 2. The decided range: 15 to 18

| Version | Upstream support ends |
|---|---|
| 13 | 2025-11-13 — **already EOL** |
| 14 | 2026-11-12 — ends in ~4 weeks |
| **15** | 2027-11-11 — **the floor** |
| 16 | 2028-11-09 |
| 17 | 2029-11-08 |
| **18** | 2030-11-14 — **newest GA, the ceiling** |
| 19 | RC 1 on 2026-10-15 — pre-release, not a pin target |

I checked these rather than recalling them. If you find any of them wrong, say
so before building on them — the floor is the one decision everything else
hangs off.

**Declared support is not a claim that 13 or 14 break.** Everything
`0002_postgres.sql` uses is 9.6-or-older, so older servers will very likely
work. The range states what is tested and promised. Word the documentation that
way: a range, not a prohibition.

## 3. Pin the tag in our source

Override the image tag in the test fixtures instead of inheriting it. You
already use `ImageExt` in `crates/codlet-sqlx/tests/postgres_tls.rs`, so
`.with_tag(...)` is the mechanism.

Drive it from one place — an env var read by the fixtures with the **floor as
the default** — so a local `cargo test` runs a supported version and CI sets it
per job. Do not scatter tag strings across test files.

## 4. The pin must verify itself — this is the criterion that matters

A pinned tag alone does not fix this defect, because the defect was an inherited
default doing work nobody saw. If the override is dropped, a matrix variable
goes unset, or a future `testcontainers-modules` changes behaviour, the suite
would test one version twice and stay green.

So **assert the major version of the server actually connected to** matches the
one requested, and fail if it does not —
`SELECT current_setting('server_version')` or `SHOW server_version`, major
component only (the minor will drift as images are rebuilt, so do not pin that).

That makes the pin a checked property rather than a declaration. It is the same
discipline as your "assert the column is absent first" in RFC-057 and your
control connection in RFC-056: prove the test could fail.

**Trial it**: set the env var to one version while forcing the fixture to
request another, confirm the assertion fires, revert. Quote the failure.

## 5. Run both ends in CI

The PostgreSQL jobs must cover **15 and 18**. A matrix on the existing jobs is
cleaner than duplicating them; use your judgement on whether
`test-postgres-tls` runs on both or just one — RFC-058 §4.3 allows one, and TLS
negotiation does differ by server OpenSSL, so covering both has some value if it
is free.

Do not let the matrix silently collapse: if both legs end up testing the same
version, §4's assertion is what catches it, which is why §4 comes first.

## 6. Documentation

`docs/src/adapter-matrix-and-config.md`:

- state the supported range once, with the reason (floor by support policy,
  ceiling by what is tested);
- the `PostgresStore` row should name the versions its ✓ marks were established
  on, as the RFC-038 note does for its CI run and RFC-054's note does for
  Miniflare;
- say that older servers are untested rather than unsupported-by-design.

`crates/codlet-sqlx/README.md` gets the range too — it is the crates.io front
page and the place a host chooses the adapter.

## 7. Non-change scope

- No change to the adapter's SQL, to `READ COMMITTED` reliance, or to the
  decision to avoid `RETURNING`/`FOR UPDATE`.
- No change to `run_postgres_migrations`.
- No change to SQLite or D1 coverage.
- Do not test every version in the range. Two endpoints is the design; a full
  matrix is CI time without more information.
- Do not pin PostgreSQL 19. It is a release candidate.
- No automation for keeping the range current — RFC-058 §7.

## 8. If a test cannot be run locally

Same as RFC-056: these need Docker. Commit, read the CI log, and report what CI
shows. If you need to iterate on the matrix, a pull request is **pre-authorised**
for this RFC — you do not need to ask again.

Do not land the §4 assertion without having seen it fire, in CI or locally. That
is the one thing in this handoff that must not be taken on faith, because it is
the thing that protects everything else.

## 9. Acceptance criteria

Binding.

1. The image tag is set in codlet's own source, from one place, defaulting to
   the floor (15).
2. The tests assert the connected server's **major version** matches the
   requested one, and this was **demonstrated to fire** against a mismatch.
3. CI runs the PostgreSQL conformance suite on **15 and 18**, both green.
4. No test inherits `testcontainers-modules`' default tag any more.
5. The supported range is stated in `adapter-matrix-and-config.md` and
   `crates/codlet-sqlx/README.md`, as a tested-and-promised range rather than a
   prohibition.
6. The `PostgresStore` matrix row names the versions behind its guarantees.
7. All blocking CI jobs green.

## 10. Required evidence

`git diff --stat`; the fixture change quoted; the version assertion quoted; the
**failure output from the §4 trial**; the CI matrix configuration; CI run URLs
showing both 15 and 18 green, with the version assertion visible in at least one
log; the documentation wording quoted.

## 11. Review request

`.git-exclude/review-request/058-postgres-version-range.md`; result returns at
`.git-exclude/reviewed/058-postgres-version-range.md`.
