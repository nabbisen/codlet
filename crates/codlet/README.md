# codlet

Runtime-neutral authentication primitives for [codlet](https://github.com/nabbisen/codlet).

This crate holds pure types, policy objects, cryptographic lookup-key
derivation, lifecycle state machines, and storage traits. It has no web
framework, database, or async-executor dependencies.

codlet authenticates a subject; the host application authorizes that subject.
This crate never decides membership, roles, permissions, or resource access.

> **Status: pre-1.0.** The primitive, orchestration, and adapter layers are
> all implemented: code generation and HMAC derivation, lifecycle classifiers,
> storage traits, cookie policy, rate limiting, error model, audit events, and
> the `auth` managers for complete end-to-end flows. SQLite and PostgreSQL
> adapters (`codlet-sqlx`), a Cloudflare Workers D1/KV adapter, and in-memory
> stores for tests all pass a shared conformance suite. Breaking changes are
> permitted between minor releases; see `CHANGELOG.md` for the current version
> and history.

## License

Apache-2.0
