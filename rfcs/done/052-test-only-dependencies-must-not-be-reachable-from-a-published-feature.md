# RFC-052: Test-Only Dependencies Must Not Be Reachable From a Published Feature

- **Status:** Implemented (Unreleased) — reviewed and approved 2026-10-18
- **Target milestone:** M7 — **required before the next release tag**
- **Primary crate(s):** `codlet-sqlx`, `xtask`, CI
- **Source basis:** CI run 39005981164; four RUSTSEC advisories; verified independently 2026-10-17

## 1. Summary

Four `rustls` advisories are blocking the release gate. The lockfile bump that
clears them is one command. The reason they were reachable at all is the part
worth an RFC: `codlet-sqlx` exposes Docker test tooling through a **published
feature**, so a consumer can pull a test-only dependency tree into production.

## 2. The advisories

`cargo deny check advisories` reports, all against `rustls v0.23.41`, all fixed
in `>= 0.23.46`:

| ID | Summary |
|---|---|
| RUSTSEC-2026-0285 | TLS 1.3 handshake messages accepted across encryption level boundaries |
| RUSTSEC-2026-0346 | `Debug` impls for client session values expose TLS secrets |
| RUSTSEC-2026-0348 | Brotli certificate decompression can allocate 1 GiB pre-authentication |
| RUSTSEC-2026-0349 | Unauthenticated ServerHello can downgrade later connections to classical key exchange |

**Exposure to users of codlet: none in the normal configuration.** Verified:
`cargo tree -p codlet-sqlx --features postgres --no-default-features -e normal`
contains no `rustls`, no `bollard`, no `testcontainers` — no TLS stack at all.
`rustls` enters only via `testcontainers-modules`, behind `postgres-test`.

**The gate was working.** A prior diagnosis held that the advisories job was
broken and had been failing open. It was not: `cargo deny` ran and reported
these four. A toolchain error in the same log appears in the blocking
`supply chain (bans, licenses, sources)` job too, which passes — so it is noise.
Recorded here because the proposed remedy for the misdiagnosis was to change the
workflow, which would have silenced four real advisories and produced exactly
the fail-open RFC-039 §3.1's empty `ignore` list exists to prevent.

## 3. The underlying defect

`crates/codlet-sqlx/Cargo.toml`:

```toml
[features]
postgres      = ["sqlx/postgres"]
postgres-test = ["postgres", "dep:testcontainers-modules"]

[dependencies]
testcontainers-modules = { workspace = true, features = ["postgres"], optional = true }
```

`testcontainers-modules` is an **optional regular dependency**, not a
dev-dependency. Consequences:

1. A consumer who enables `postgres-test` compiles and ships a Docker client,
   an HTTP stack and a TLS stack into production.
2. `postgres` and `postgres-test` sit adjacent in the feature list and differ by
   one word. A host wanting PostgreSQL *with tests* — or simply reading the two
   names and picking the more complete-sounding one — enables the wrong one and
   gets no error, no warning, and a much larger attack surface. The names do not
   distinguish "a backend" from "test scaffolding".
3. `cargo-deny-action` defaults to `--all-features`, so every advisory in that
   tree lands on this project's gate forever, regardless of whether any user
   could reach it. The gate reports truthfully; the signal is about code we do
   not ship.

(2) is the one that matters most: a feature name is API, and this pair is a trap.

### 3.1 This was already known, and accepted

`deny.toml`'s `ISC` entry carries the reasoning in full:

> Covers `ring`, `rustls-webpki`, `untrusted` — reached via `codlet-sqlx`'s
> `postgres-test` feature (testcontainers -> bollard -> rustls). `postgres-test`
> is a published, optional *normal* dependency feature of `codlet-sqlx`, not a
> dev-dependency, so a consumer who enables it receives this subtree: **in scope
> because it ships.**

So RFC-039 identified the condition precisely, reached the correct conclusion
that the subtree ships, and responded by widening the **license allowlist** to
accommodate it. That was the wrong lever. The right response to "a test-only
subtree ships to consumers" is to stop it shipping, not to accept its licences.

This RFC reverses that choice. It is a sharper lesson than the advisories:
the analysis was already right and written down, and nothing acted on it.

## 4. Decision

### 4.1 Bump `rustls`

`cargo update -p rustls` → `rustls 0.23.46`, `rustls-webpki 0.103.15`. Verified
to resolve, and 0.23.46 is the exact fix version for all four advisories.
Lockfile only; no manifest change.

### 4.2 `testcontainers-modules` becomes a dev-dependency

```toml
[features]
postgres      = ["sqlx/postgres"]
# A test gate only: enables the Docker-backed integration tests. It pulls no
# dependency into a consumer's graph.
postgres-test = ["postgres"]

[dev-dependencies]
testcontainers-modules = { workspace = true, features = ["postgres"] }
```

`postgres-test` keeps its name and its job — gating `#[cfg(feature = ...)]` test
code — but stops carrying a `dep:`. A dev-dependency cannot reach a consumer's
graph at all, which makes the confusion in §3(2) *harmless* rather than merely
documented: a host who picks the wrong feature now gets some extra test code
compiled, not a TLS stack in production.

Accepted cost: `cargo test -p codlet-sqlx` builds `testcontainers` whether or
not the Docker tests run, because dev-dependencies are unconditional. This is
build time in CI and on developer machines, traded for a structural guarantee
about what consumers receive. The right trade.

### 4.3 A gate, not a convention

Extend the `core-deps` CI job — which already asserts that `codlet`'s published
tree carries no runtime, no framework and no duplicate versions — with the
corresponding assertion for `codlet-sqlx`:

**Amended 2026-10-18 — the first draft of this gate could not fail.** It is
recorded rather than quietly replaced, because the mistake is instructive.

The original text checked `--features postgres --no-default-features` only.
That graph never contains an optional dependency enabled by a *different*
feature — which is exactly the shape of the defect in §3. Reproduced against a
tree with the defect reintroduced:

```
--features postgres --no-default-features  → exit 0   (passed, violation present)
--all-features                             → exit 1   (fired)
```

So the gate must walk **every published feature**, not the production subset.
Both passes are kept: `--all-features` is the one that catches this class, and
the production-graph pass still catches a non-optional dependency added to
`[dependencies]` directly.

```sh
for features in "--features postgres --no-default-features" "--all-features"; do
  tree="$(cargo tree -p codlet-sqlx $features -e normal --prefix none --color never | sort -u)"
  test -n "$tree"
  echo "$tree" | grep -q '^codlet-sqlx ' || { echo "::error::unexpected cargo tree output ($features)"; exit 1; }
  if echo "$tree" | grep -Eiq '^(testcontainers|testcontainers-modules|bollard|bollard-stubs|bollard-buildkit-proto|ureq|rustls|hyper-rustls|tokio-rustls)( |$)'; then
    echo "::error::test-only dependency reachable from a published feature ($features)"; exit 1
  fi
done
```

Written in the shape RFC-036 and RFC-049 settled: capture to a variable, assert
non-empty, assert a known marker is present, **then** match — so a `cargo tree`
failure cannot pass as an empty grep. `--color never` because ANSI codes broke
RFC-049's first attempt at exactly this.

Add the matching `xtask self-test` fixture if the gate lands in `xtask` rather
than inline, per RFC-036's rule that every gate is proven against a violation.

### 4.4 Correct `deny.toml`'s comment — and keep the `ISC` entry

The reasoning quoted in §3.1 stops being true once §4.2 lands, so the comment
must be rewritten: the subtree is then reached only through dev-dependencies.

**Do not remove the `ISC` allow entry.** `include-dev = true` is set
deliberately (RFC-039 §3.2), so `cargo deny check licenses` still traverses
dev-dependencies and still needs `ISC` allowed. Removing it would turn the
blocking licenses gate red. The entry stays; only its justification changes.

## 5. Non-goals

- No `ignore` entry in `deny.toml`. RFC-039 §3.1 stands: a finding is reported,
  never silenced.
- No change to the advisories-job policy (non-blocking on PRs, blocking at
  release). It behaved correctly.
- No change to the workflow files to suppress the toolchain message. It is
  cosmetic and the blocking sibling job proves it is harmless; chasing it risks
  changing a working gate.
- No change to `codlet`'s own `core-deps` assertions.

## 6. Security considerations

The advisories are real but unreachable by users of the published feature set,
verified by dependency graph rather than argued. The durable improvement is
§4.2 plus §4.3: after this, "test tooling cannot reach a consumer" is a
machine-checked property of `codlet-sqlx` instead of a fact someone has to
re-derive from the manifest.

## 7. Acceptance criteria

1. `rustls >= 0.23.46` in `Cargo.lock`; `cargo deny check advisories` clean.
2. `testcontainers-modules` is a dev-dependency; `postgres-test` declares no
   `dep:`.
3. The Docker-backed PostgreSQL tests still run and pass under
   `--features postgres-test`.
4. Neither the `--features postgres --no-default-features` graph **nor the
   `--all-features` graph** contains any crate listed in §4.3, with `-e normal`.
   The production-only graph alone is **not** sufficient — see §4.3's
   amendment.
5. The §4.3 gate exists, is wired into CI, and **fails** against a deliberately
   reintroduced violation — demonstrated, not assumed.
6. Full blocking CI green; the advisories job green.
