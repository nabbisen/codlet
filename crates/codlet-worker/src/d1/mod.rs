//! D1-backed store implementations (RFC-033).
//!
//! All stores share:
//! - Timestamps as `D1Type::Real(t as f64)` (RFC-033 §6).
//! - Affected-row count from `result.meta()?.changes` (RFC-033 §8).
//! - Conditional UPDATE for atomic single-winner operations (INV-5/INV-6).
//!
//! Stores hold `std::rc::Rc<D1Database>` because `D1Database` is not `Clone`
//! and Workers are single-threaded (no `Send` requirement).

pub mod code;
pub mod session;
pub mod token;

pub use code::D1CodeStore;
pub use session::D1SessionStore;
pub use token::D1FormTokenStore;

use codlet_core::store::error::StoreError;
use worker::d1::{D1PreparedStatement, D1Type};

/// Bind a `u64` Unix-second timestamp as `REAL` for D1 (RFC-033 §6).
///
/// `f64` has 53-bit integer precision; Unix seconds fit until year 285 million.
pub(crate) fn ts(t: u64) -> D1Type<'static> {
    D1Type::Real(t as f64)
}

/// Extract the affected-row count from a D1 `run()` result (RFC-033 §8).
pub(crate) fn changes(result: &worker::d1::D1Result) -> Result<usize, StoreError> {
    Ok(result
        .meta()
        .map_err(|e| StoreError::Backend(e.to_string()))?
        .and_then(|m| m.changes)
        .unwrap_or(0))
}

/// Convert a `worker::Error` to [`StoreError::Backend`].
pub(crate) fn to_store_err(e: worker::Error) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Bind a slice of [`D1Type`] values to a prepared statement via `bind_refs`.
///
/// Uses `D1PreparedStatement::bind_refs` which accepts `IntoIterator<Item = &D1Type>`
/// directly, avoiding any `JsValue` conversion in calling code.
pub(crate) fn bind<'a>(
    stmt: D1PreparedStatement,
    args: &'a [D1Type<'a>],
) -> Result<D1PreparedStatement, StoreError> {
    stmt.bind_refs(args)
        .map_err(|e| StoreError::Backend(e.to_string()))
}
