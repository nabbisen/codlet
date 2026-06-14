# codlet-core

Runtime-neutral authentication primitives for [codlet](https://github.com/nabbisen/codlet).

This crate holds pure types, policy objects, cryptographic lookup-key
derivation, lifecycle state machines, and storage traits. It has no web
framework, database, or async-executor dependencies.

codlet authenticates a subject; the host application authorizes that subject.
This crate never decides membership, roles, permissions, or resource access.

> **Status: early pre-release (v0.1.0).** The cryptographic primitives —
> code policy/generation/normalization/validation (RFC-003) and HMAC
> lookup-key derivation, key providers, domain separation, and key versioning
> (RFC-004) — are implemented and tested. Storage traits, session/form-token
> lifecycle, and adapters are still to come. Do not yet rely on this crate for
> a complete production authentication flow.

## License

Apache-2.0
