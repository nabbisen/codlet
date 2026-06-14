# codlet-core

Runtime-neutral authentication primitives for [codlet](https://github.com/nabbisen/codlet).

This crate holds pure types, policy objects, cryptographic lookup-key
derivation, lifecycle state machines, and storage traits. It has no web
framework, database, or async-executor dependencies.

codlet authenticates a subject; the host application authorizes that subject.
This crate never decides membership, roles, permissions, or resource access.

> **Status: pre-release (v0.4.0).** The complete primitive and orchestration
> layers are implemented: code generation/HMAC derivation, lifecycle
> classifiers, storage traits, cookie policy, rate limiting, error model,
> audit events, and the `auth` managers for complete end-to-end flows.
> Production adapters (Workers/D1, SQLx) are the next step.

## License

Apache-2.0
