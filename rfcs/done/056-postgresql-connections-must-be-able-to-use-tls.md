# RFC-056: PostgreSQL Connections Must Be Able To Use TLS

- **Status:** Implemented (v0.21.0) — PR #1, `6359643` and `bfbf028`; reviewed and approved 2026-10-19
- **Target milestone:** M7 — **before the API freeze**
- **Primary crate(s):** `codlet-sqlx`, CI, `docs/`
- **Source basis:** found while verifying RFC-052's dependency graph, 2026-10-18

## 1. Summary

`codlet-sqlx` has no TLS stack at all. A host pointing the PostgreSQL adapter at
any non-local database either cannot connect or connects in plaintext — and the
traffic carries session lookup keys, code hashes, subject identifiers and
host-owned grant payloads.

## 2. Evidence

The workspace pins `sqlx = { version = "0.9", default-features = false }`
(`Cargo.toml:38`), and `codlet-sqlx` enables only `runtime-tokio` plus a
backend. Verified on the published graph:

```
cargo tree -p codlet-sqlx --features postgres --no-default-features -e normal
→ 222 packages, zero TLS crates
```

The only match for `tls|ring|webpki` is `stringprep`, a false positive on the
substring. There is no `rustls`, no `native-tls`, no `openssl`.

This was found while verifying RFC-052, where the *absence* of `rustls` from
the production graph was the good news. It is also the bad news.

## 3. Threat

Two distinct problems, and the second is the serious one.

**Disclosure.** Subject identifiers, purposes, scopes, expiry times and `grant`
payloads cross the network in cleartext. `grant` is host-owned and opaque to
codlet, so its sensitivity is the host's to judge — which is precisely why
codlet must not force it over plaintext.

**Tampering.** An attacker on the database path can modify query results. They
can return a fabricated session row for an arbitrary subject, or suppress a
`revoked_at` value so a revoked session classifies as authenticated. RFC-047
made the classifier the sole guard for sessions — "there is no second
enforcement point downstream of this call" — and the classifier's input is
whatever the wire delivers. **That is a complete authentication bypass for an
attacker positioned between the service and its database.**

Note what the lookup-key design does and does not protect. Observing a
`lookup_key` does not let an attacker forge a session, because authentication
requires the raw secret that HMACs to it. The HMAC scheme holds. It is the
integrity of the *result rows* that is unprotected, and no amount of hashing
addresses that.

Managed providers make this immediate: RDS, Cloud SQL, Neon and Supabase all
expect or require TLS, so today a host either fails to connect or has
configured something that silently is not encrypted.

## 4. Why it was not noticed

`sqlx`'s TLS features are opt-in and `default-features = false` was set — for
good reasons, recorded in RFC-034 — so nothing ever errored. The adapter matrix
documents atomicity, consistency and expiry semantics and says nothing about
transport, so there was no row for this fact to be wrong in. A property with no
place in the documentation cannot be audited.

## 5. Decision

### 5.1 One opt-in feature, named for its backend

```toml
[features]
postgres        = ["sqlx/postgres"]
postgres-rustls = ["postgres", "sqlx/tls-rustls-ring-webpki"]
```

**Naming deviation, flagged for the owner.** The approved recommendation said
`postgres-tls`. I am proposing `postgres-rustls` instead, because
`postgres-test` already exists and `postgres-tls` differs from it by one
transposition while sitting adjacent in the same list — the exact confusion
class RFC-052 §3 identified as the underlying defect there. `postgres-rustls`
also names the backend, leaving room for `postgres-native-tls` without renaming
anything. Say the word and it becomes `postgres-tls`; this is the only point in
this RFC that departs from what was approved.

`tls-rustls-ring-webpki` rather than the `tls-rustls` alias: the alias resolves
to the same thing today, and naming the resolved feature means a change in
sqlx's alias cannot silently change our crypto provider or root store. `ring`
rather than `aws-lc-rs` for build portability — `aws-lc-rs` needs a C toolchain
on some targets, and a security library should not make TLS the reason a build
breaks.

### 5.2 Opt-in, not default

Enabling TLS by default would add `rustls` and `ring` to every PostgreSQL
consumer's build, including those connecting over a Unix socket or to
`localhost`, where it buys nothing.

The compensating control is documentation that states the consequence plainly
rather than listing a feature: a remote PostgreSQL connection without
`postgres-rustls` is not production-safe, for the reasons in §3. That belongs in
the adapter matrix as a property with a row, so it can be audited next time.

### 5.3 RFC-052's gate must stop banning `rustls`

RFC-052 §4.3's gate bans, on `--all-features`:

```
testcontainers|testcontainers-modules|bollard|bollard-stubs|bollard-buildkit-proto|ureq|rustls|hyper-rustls|tokio-rustls
```

With `postgres-rustls` in the manifest, `--all-features` contains `rustls` and
**the gate turns CI red on a legitimate dependency.**

The list conflated a marker with a symptom. The gate's purpose is "test-only
tooling must not reach a consumer"; `rustls` was only how that tooling became
visible. Drop `rustls`, `hyper-rustls` and `tokio-rustls`; keep the test-only
markers.

Verified in both directions before writing this, with the original defect
reintroduced:

```
reduced list, defect present → [--all-features] BAN HIT, exit 1
    bollard v0.20.2, bollard-stubs, bollard-buildkit-proto,
    testcontainers v0.27.3, testcontainers-modules v0.15.0, ureq v3.3.0
reduced list, defect absent  → clean, exit 0
```

So the gate keeps its discriminating power. RFC-052 §4.3 is amended
accordingly.

### 5.4 The advisory posture changes

Today `rustls` advisories reach no consumer, which is why RFC-052 could
classify four of them as unreachable. Once `postgres-rustls` exists, a `rustls`
advisory is **user-reachable for anyone who enables it**, and the release gate's
judgement on such a finding changes with it.

Record this in `deny.toml` beside the `ISC` comment that RFC-052 just corrected,
so the next reader does not reuse the old reasoning. This is the second time
that comment has had to change; it is load-bearing documentation.

## 6. Non-goals

- TLS is not enabled by default — §5.2.
- No `native-tls` and no `aws-lc-rs` feature in this RFC. Add later if asked for.
- No change to SQLite or D1, neither of which has a network transport.
- No change to the `sqlx` version pin or to `default-features = false`.
- No connection-string parsing or `sslmode` handling in codlet. That is sqlx's
  job and duplicating it would create a second source of truth.

## 7. Acceptance criteria

1. `postgres-rustls` exists, implies `postgres`, and enables
   `sqlx/tls-rustls-ring-webpki`.
2. A real TLS connection to PostgreSQL is **demonstrated**, not assumed:
   `sslmode=require` against a TLS-enabled server succeeds with the feature and
   fails without it. The existing testcontainers harness is the natural place.
3. The behaviour of `sslmode=require` **without** the feature is established by
   observation and recorded — error or silent plaintext. Do not assume which.
4. RFC-052's gate no longer bans `rustls`/`hyper-rustls`/`tokio-rustls`, still
   bans the test-only markers, and was re-demonstrated to fail against a
   reintroduced violation.
5. `cargo tree -p codlet-sqlx --features postgres --no-default-features -e normal`
   still contains no TLS crate — TLS must not leak into the non-TLS feature set.
6. The adapter matrix has a transport row stating what is and is not encrypted,
   and that a remote connection without the feature is not production-safe.
7. `deny.toml`'s advisory-scope note updated per §5.4.
8. `cargo deny check advisories bans licenses sources` clean with the new
   feature — including any new licences `ring`/`webpki` introduce.
9. All blocking CI jobs green; a CI job exercises the TLS feature.
