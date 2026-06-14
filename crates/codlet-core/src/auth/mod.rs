//! High-level orchestration managers (RFC-013).
//!
//! This module provides three composable managers that wrap the low-level
//! primitives into safe, ergonomic flows:
//!
//! - [`CodeAuth`] — code issuance, two-step redemption, callback-based
//!   redemption, and revocation.
//! - [`SessionManager`] — session issuance (requires a [`RedeemSuccess`]
//!   proof), validation, and revocation.
//! - [`FormTokenManager`] — form-token issuance and atomic consume with
//!   idempotency replay support.
//!
//! ## Layered design (RFC-013 §10.1)
//!
//! Primitive layer (`code`, `hashing`, `state`): security-conscious custom apps.
//! Store service layer (`store::*` traits): custom routing and special flows.
//! Flow service layer (`auth::*` managers): standard flows (this module).
//! Framework adapter layer: future crates for quick integration.
//!
//! ## Host application boundary
//!
//! codlet authenticates; the host authorizes. The managers never make access
//! control decisions. [`RedeemSuccess`] carries an opaque `grant` returned by
//! the host at issuance time; codlet does not interpret it.

pub mod code;
pub mod error;
pub mod norate;
pub mod session;
pub mod token;

pub use code::CodeAuth;
pub use error::{FormTokenError, IssuedSession, RedeemError, RedeemSuccess, SessionError};
pub use norate::NoRateLimit;
pub use session::SessionManager;
pub use token::FormTokenManager;
