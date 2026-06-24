//! Pure lifecycle classifiers (RFC-005, RFC-006, RFC-007).
//!
//! Each sub-module encodes a single state machine as a pure function. No I/O,
//! no `async`. Adapters supply the inputs; these functions produce the outcome.

pub mod claim;
pub mod session;
pub mod token;

pub use claim::{ClaimOutcome, classify_claim};
pub use session::{SessionValidationOutcome, classify_session};
pub use token::{TokenConsumeOutcome, classify_token_consume};
