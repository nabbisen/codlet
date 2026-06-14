//! Adapter conformance test suite (RFC-023).
//!
//! Every codlet storage adapter must pass all tests in this crate before being
//! described as production-ready. The tests are parameterised over an async
//! factory function so the same suite runs against in-memory stores, SQLite,
//! PostgreSQL, and D1 without duplication.
//!
//! # Usage
//!
//! ```rust,ignore
//! #[tokio::test]
//! async fn my_store_conforms() {
//!     codlet_conformance::run_code_store_conformance(|| async {
//!         MyCodeStore::new_for_test().await
//!     }).await;
//! }
//! ```

pub mod code;
pub mod fixtures;
pub mod session;
pub mod token;

pub use code::run_code_store_conformance;
pub use session::run_session_store_conformance;
pub use token::run_form_token_store_conformance;
