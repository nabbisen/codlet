# RFC-028: Security Policy, Advisory Process, MSRV, and Release Discipline

## Status

Draft. This RFC is intended for review before implementation.

## Context

codlet is extracted from the `zinnias-ciao` v0.36.1 one-time invite-code flow. The extraction preserves reusable authentication primitives while leaving membership, roles, UI, and authorization in the host application.

## Problem

A public authentication crate needs operational discipline: vulnerability reporting, versioning, release gates, and compatibility expectations.

## Decision

codlet will publish a security policy before its first public release and follow conservative versioning.

## Security policy

`SECURITY.md` must include:

- supported versions;
- vulnerability reporting address/process;
- expected response targets;
- disclosure policy;
- advisory format;
- what counts as a security bug.

## Security bug examples

- plaintext secret persistence;
- RNG fallback;
- missing key fallback;
- claim double-winner;
- token replay proceeding;
- public error enumeration;
- cookie builder omitting mandatory attributes;
- adapter falsely claiming conformance.

## MSRV

MSRV should be explicit. Do not raise MSRV in patch releases. Raising MSRV requires minor release notes before v1 and semver-compatible policy after v1.

## Release checklist

- all tests pass;
- conformance suite status is documented;
- changelog includes security-relevant changes;
- examples compile;
- docs build;
- static release gates pass;
- crate metadata is complete;
- security policy is present.

## Acceptance criteria

- `SECURITY.md` exists before publishing.
- `xtask release-check` exists before v0.1.
- MSRV is tested in CI or clearly documented.


## References

- NIST SP 800-63B, Authentication and Lifecycle Management: https://pages.nist.gov/800-63-4/sp800-63b.html
- OWASP Authentication Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- Source handoff: `SERVICE-HANDOFF-for-codlet.md` from `zinnias-ciao` v0.36.1.
