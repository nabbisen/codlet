//! Storage traits (RFC-005, RFC-006, RFC-007).
//!
//! These traits define the contract between `codlet-core` and any storage
//! backend. Adapters (in-memory, SQLx, Workers D1) implement them and must
//! pass the conformance suite (RFC-023) before being considered production-ready.

pub mod code;
pub mod error;
pub mod session;
pub mod token;

pub use code::{ClaimRequest, CodeRecord, CodeStore, RedeemableCode};
pub use error::{PublicAuthError, StoreError};
pub use session::{ActiveSessionRecord, SessionRecord, SessionStore};
pub use token::{FormTokenRecord, FormTokenStore, TokenSubject};
