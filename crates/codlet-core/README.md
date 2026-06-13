# codlet-core

Runtime-neutral authentication primitives for [codlet](https://github.com/nabbisen/codlet).

This crate holds pure types, policy objects, cryptographic lookup-key
derivation, lifecycle state machines, and storage traits. It has no web
framework, database, or async-executor dependencies.

codlet authenticates a subject; the host application authorizes that subject.
This crate never decides membership, roles, permissions, or resource access.

> **Status: pre-release skeleton (v0.0.0).** The security primitives are being
> implemented RFC-by-RFC. See the workspace `rfcs/` directory. Do not depend on
> this crate for production authentication yet.

## License

Apache-2.0
