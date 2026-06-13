# Security Policy

codlet is a security-sensitive authentication library. This policy is a
pre-release skeleton; it will be finalized before the first published release
per RFC-028.

## Supported versions

codlet has not had a stable release yet. No version is currently supported for
production use. Do not depend on codlet for production authentication until a
`>= 0.1` release is published with a completed version of this policy.

## Reporting a vulnerability

Until a dedicated security contact is published here, report suspected
vulnerabilities privately to the maintainer via the repository's private
vulnerability reporting on GitHub, rather than opening a public issue.

Please include: affected crate and version/commit, a description of the issue,
and a minimal reproduction if possible. Do not include live secrets (HMAC keys,
session cookies, plaintext codes) in a report.

## What counts as a security bug

Non-exhaustive examples treated as security bugs in codlet (RFC-028):

- persistence of a plaintext code, session secret, or form-token secret;
- any fallback HMAC key, or HMAC operation succeeding with missing key material;
- RNG failure producing a deterministic or empty secret instead of an error;
- a one-time code claim or single-use form-token consume that can succeed more
  than once under concurrency;
- a redemption failure path that reveals whether a code exists, is expired,
  revoked, or already used;
- a session cookie built without `HttpOnly`, `Secure`, or `SameSite=Strict`
  under a production policy;
- a secret value appearing in `Debug`/`Display` output, logs, audit events, or
  error messages.

## Disclosure

Coordinated disclosure. Specific response and disclosure timelines will be
stated here before the first public release.
