// Fixture for `xtask self-test`, gate `no-fallback-key` (RFC-040 §5.2).
//
// Not compiled into the workspace: this file lives outside `crates/` and is
// never referenced by a `mod` declaration, so `cargo build`/`test` never sees
// it. It exists only to be read as text by the gate's check function.
//
// Deliberately violates the gate: a `*-change-in-production`-style key
// literal in non-comment code.
const FALLBACK_KEY: &[u8] = b"dev-pepper-change-in-production";
