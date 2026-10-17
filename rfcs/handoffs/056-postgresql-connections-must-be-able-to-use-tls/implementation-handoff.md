# Developer Handoff — RFC-056: PostgreSQL Connections Must Be Able To Use TLS

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **RFC:** [`../../accepted/056-postgresql-connections-must-be-able-to-use-tls.md`](../../accepted/056-postgresql-connections-must-be-able-to-use-tls.md)
- **Baseline:** `8e9d68c`
- **Needs Docker.** §5's demonstration cannot be done without it — see §8.

## 1. Purpose

Give `codlet-sqlx` an opt-in TLS feature for PostgreSQL. Today there is no TLS
stack, so a remote database connection is either impossible or in cleartext,
and an attacker on that path can forge session rows.

## 2. Change scope — indicative, not authoritative

`crates/codlet-sqlx/Cargo.toml`, `Cargo.lock`, `deny.toml`,
`.github/workflows/ci.yml`, `docs/src/adapter-matrix-and-config.md`,
`CHANGELOG.md`, and the PostgreSQL test harness. §9 is binding.

## 3. The feature

```toml
postgres-rustls = ["postgres", "sqlx/tls-rustls-ring-webpki"]
```

Name the resolved sqlx feature, not the `tls-rustls` alias — I verified in
sqlx 0.9.0's manifest that `tls-rustls` → `tls-rustls-ring` →
`tls-rustls-ring-webpki`, and pinning the resolved name means an upstream
alias change cannot silently swap our crypto provider or root store.

**One thing to establish rather than assume.** sqlx's `_tls-*` features look
mutually exclusive. Find out what happens when a host enables **both**
`codlet-sqlx/postgres-rustls` and, in their own manifest,
`sqlx/tls-rustls-ring-native-roots` — a build error, a silent precedence, or
something worse. An operator whose provider CA is in the OS trust store rather
than the webpki set will try exactly that. Report what you find; do not design
around a guess. If it breaks, say so and I will decide whether a second feature
is warranted.

## 4. The gate must be loosened first, or CI goes red

`.github/workflows/ci.yml`'s `core-deps` job bans `rustls` on
`--all-features`. Adding this feature trips it on a legitimate dependency.

Remove `rustls`, `hyper-rustls` and `tokio-rustls` from the alternation.
**Keep** `testcontainers`, `testcontainers-modules`, `bollard`,
`bollard-stubs`, `bollard-buildkit-proto` and `ureq`, and keep both feature
passes and the capture/`test -n`/marker/match ordering exactly as they are.

I verified the reduced list still works, with RFC-052's original defect
reintroduced:

```
reduced list, defect present → [--all-features] BAN HIT, exit 1
    bollard, bollard-stubs, bollard-buildkit-proto,
    testcontainers, testcontainers-modules, ureq
reduced list, defect absent  → clean, exit 0
```

Re-demonstrate it yourself after your edit — the point of RFC-052's review was
that a gate nobody fired is not a gate, and that applies to my verification as
much as to the original.

## 5. Required: prove TLS actually works

This is the criterion that matters. A feature flag that compiles proves nothing.

Using the existing testcontainers PostgreSQL harness, add a test that connects
with `sslmode=require` to a TLS-enabled server and performs at least one real
store operation. The standard `postgres` image needs `ssl=on` plus a
certificate and key; generating a self-signed pair in the test fixture is fine,
and with `sslmode=require` the client does not verify the chain, so no custom
root is needed.

**If you want chain verification too** (`sslmode=verify-full`), that needs the
test CA in the client's root store, which the webpki feature does not allow.
Do not fight it — record it as a limitation and let `require` be the test.
That distinction is itself worth documenting: `require` encrypts,
`verify-full` authenticates the server, and only the latter stops an active
attacker. Say which one the feature enables out of the box.

Then establish §9's item 3: what `sslmode=require` does **without** the
feature. Observe it; quote the error or the absence of one. I believe sqlx
errors that TLS support is not compiled in, but I have not run it and you
should not take my word.

## 6. Required: the documentation gets a transport row

`docs/src/adapter-matrix-and-config.md` has no row for transport, which is why
this was invisible. Add one to the guarantee-definitions table and state per
adapter what is encrypted.

Say plainly that a remote PostgreSQL connection without `postgres-rustls` is
not production-safe, and why — an attacker on that path can return a forged
session row, and RFC-047 left the classifier as the only guard. One short
paragraph, in the existing voice. Do not reproduce RFC-056 §3.

## 7. Required: `deny.toml`

Beside the `ISC` comment you corrected in RFC-052, record that `rustls` is now
reachable from a published feature, so a `rustls` advisory is user-relevant and
cannot be dismissed the way RFC-052's four were.

Check whether `ring` and `webpki` introduce licences not in the `allow` list;
`ring`'s licensing is not a plain MIT/Apache pair, so read what `cargo deny`
reports rather than predicting it. If the list needs widening, say which
licence and why in the review request — do not widen it silently.

## 8. If you have no Docker

§5 is the core of this handoff and needs it. If Docker is unavailable, **stop
and report** rather than delivering the feature flag alone: a TLS feature whose
only evidence is that it compiles is the shape of defect RFC-038 and RFC-054
are both about. Tell me and I will decide whether to split it or wait.

Everything else — §3, §4, §6, §7 — can proceed, and a partial delivery that
says so clearly is fine.

## 9. Acceptance criteria

Binding; §2 yields to these.

1. `postgres-rustls` exists, implies `postgres`, enables
   `sqlx/tls-rustls-ring-webpki`.
2. A real `sslmode=require` connection to a TLS-enabled PostgreSQL succeeds
   with the feature, **demonstrated in a test**, and a real store operation runs
   over it.
3. The without-feature behaviour of `sslmode=require` is established by
   observation and quoted.
4. The gate no longer bans the rustls crates, still bans the test-only markers,
   and was re-demonstrated against a reintroduced violation.
5. `cargo tree -p codlet-sqlx --features postgres --no-default-features -e normal`
   still has no TLS crate.
6. The adapter matrix has a transport row and the production-safety statement.
7. `deny.toml` updated per §7, with any licence addition named and justified.
8. `cargo deny check advisories bans licenses sources` clean.
9. A CI job exercises `--features postgres-rustls`; all blocking jobs green.
10. The §3 both-features question answered with observed behaviour.

## 10. Prohibited shortcuts

- Do not enable TLS by default, or add it to the `postgres` feature.
- Do not use the `tls-rustls` alias, `aws-lc-rs`, or `native-tls`.
- Do not parse connection strings or handle `sslmode` in codlet. That is sqlx's
  job; a second implementation is a second source of truth.
- Do not widen the licence allowlist without naming what and why.
- Do not claim TLS works on the strength of a successful `cargo build`.
- Do not weaken the gate beyond §4's three removals.

## 11. Required evidence

`git diff --stat`; the new feature stanza quoted; the TLS test quoted in full
with its output; the without-feature `sslmode=require` observation; the gate
re-demonstration output; `cargo deny` output; the transport row quoted; CI run
URL.

## 12. Review request

`.git-exclude/review-request/056-postgres-tls.md`; result returns at
`.git-exclude/reviewed/056-postgres-tls.md`.
