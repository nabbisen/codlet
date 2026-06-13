# codlet

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](./LICENSE)
[![Status: pre-release](https://img.shields.io/badge/status-pre--release-orange.svg)](#status)

**Embedded one-time-code authentication primitives for Rust web services.**

## Overview

codlet lets a Rust web service exchange a short, human-friendly one-time code
for an application-defined subject and session — without passwords, email
verification, OAuth redirects, or a separately operated identity-provider
server. It is a library of auditable security primitives plus optional runtime
adapters, designed to be embedded directly in a single-service deployment.

codlet authenticates a subject. **The host application authorizes that
subject.** codlet has no concept of users, roles, permissions, communities, or
organizations, and never makes access-control decisions.

## Why / when

Use codlet when you want private, invite-only access for non-technical users
and want to keep operational complexity low: one service, one database, no
second auth server to run. It suits neighborhood groups, hobby clubs, small
team tools, event sign-ups, and similar invite-driven membership systems. It is
**not** an IdP, a user-management system, or an authorization framework, and it
does not try to make short codes look stronger than they are.

## Quick start

> **Not yet.** codlet is at the pre-release bootstrap stage (v0.0.0). The
> security primitives are being implemented RFC-by-RFC and are not ready to
> depend on. Watch the `rfcs/` directory and `CHANGELOG.md` for progress.

When the first primitives land, the minimal path will be: configure a
`KeyProvider` with real HMAC key material, generate a code with a `CodePolicy`,
and redeem it through a `CodeStore` whose atomic claim guarantees single use.

## Design notes

- **Security by default.** Secrets are stored only as keyed HMAC lookup values;
  missing key material and RNG failure both fail closed; redemption failures map
  to a single generic public error; session cookies are `HttpOnly; Secure;
  SameSite=Strict` by default.
- **Small, runtime-neutral core.** `codlet-core` carries no web-framework,
  database, or async-executor dependencies. Runtime support (Cloudflare
  Workers/D1/KV, SQLx, Axum) lives in separate adapter crates.
- **Storage proves its own atomicity.** One-time claim and single-use
  form-token consume are trait-level contracts backed by conditional writes;
  every adapter must pass a shared conformance suite.

## More detail

- Architecture, scope, and non-goals: [`rfcs/done/RFC-001`](./rfcs/done/RFC-001-project-scope-product-shape-non-goals.md),
  [`rfcs/done/RFC-002`](./rfcs/done/RFC-002-crate-architecture-feature-flags-runtime-matrix.md).
- All design proposals and their lifecycle: [`rfcs/README.md`](./rfcs/README.md).
- Security policy and how to report a vulnerability: [`SECURITY.md`](./SECURITY.md).
- Full documentation lives under [`docs/src`](./docs/src) (mdBook-compatible).

## Status

Pre-release (v0.0.0). Phase 0 bootstrap: workspace, CI, RFC process, and
`codlet-core` skeleton. RFC-001 and RFC-002 are accepted; cryptographic
primitives (RFC-003/004) are next.

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](./LICENSE) and
[`NOTICE`](./NOTICE).
