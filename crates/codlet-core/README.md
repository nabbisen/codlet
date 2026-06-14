# codlet-core

Runtime-neutral authentication primitives for [codlet](https://github.com/nabbisen/codlet).

This crate holds pure types, policy objects, cryptographic lookup-key
derivation, lifecycle state machines, and storage traits. It has no web
framework, database, or async-executor dependencies.

codlet authenticates a subject; the host application authorizes that subject.
This crate never decides membership, roles, permissions, or resource access.

> **Status: pre-release (v0.2.0).** The full set of pure primitives is
> implemented: code generation/normalization/validation, HMAC lookup-key
> derivation, lifecycle classifiers (claim, session, form-token), storage
> traits, cookie policy, and in-memory stores for testing. Orchestration
> helpers, high-level API, and production adapters are still to come.

## License

Apache-2.0
