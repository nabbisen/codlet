//! Compilable usage examples for codlet (RFC-016).
//!
//! Each binary in `src/bin/` demonstrates a complete, self-contained flow.
//! All examples follow the rules from RFC-016 §10.2:
//!
//! - No hard-coded production secrets (use environment variables or generated
//!   test material).
//! - No printing of generated codes to normal server logs.
//! - No storing of session secrets in JavaScript-accessible storage.
//! - Always show host authorization after authentication.
//! - Use safe defaults: 8+ character codes, `ProductionStrict` cookie policy.
