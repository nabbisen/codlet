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
//!     // The concurrent single-winner test is a separate entry point (it needs
//!     // tokio, behind the default `concurrency` feature). Run it too wherever
//!     // you can.
//!     codlet_conformance::run_code_store_concurrency_conformance(|| async {
//!         MyCodeStore::new_for_test().await
//!     }).await;
//! }
//! ```

pub mod code;
pub mod fixtures;
pub mod session;
pub mod token;

#[cfg(feature = "concurrency")]
pub use code::run_code_store_concurrency_conformance;
pub use code::{CODE_STORE_TESTS, run_code_store_conformance, run_code_store_test};
pub use session::{SESSION_STORE_TESTS, run_session_store_conformance, run_session_store_test};
pub use token::{
    FORM_TOKEN_STORE_TESTS, run_form_token_store_conformance, run_form_token_store_test,
};
