//! Cloudflare Workers adapter for codlet (RFC-033).
//!
//! Provides D1-backed implementations of `CodeStore`, `SessionStore`,
//! `FormTokenStore`, and `CodeAdminStore`, plus a KV-backed
//! `RateLimitStore` and a `WorkerKeyProvider` that loads HMAC key
//! material from `Env` secrets.
//!
//! ## Crate target
//!
//! This crate is compiled for `wasm32-unknown-unknown` when deployed to
//! Cloudflare Workers. All public items are gated on
//! `#[cfg(target_arch = "wasm32")]`; on native targets the crate is an
//! empty rlib, allowing `cargo check --workspace` and `cargo doc` to
//! succeed without a wasm toolchain.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codlet_worker::{D1CodeStore, D1SessionStore, D1FormTokenStore,
//!                     KvRateLimitStore, WorkerKeyProvider, D1TableConfig,
//!                     run_d1_migrations};
//!
//! #[worker::event(fetch)]
//! async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
//!     // env.d1() returns a new handle each call (takes &self) — no Rc needed.
//!     run_d1_migrations(&env.d1("DB")?).await?;
//!
//!     let key_provider = WorkerKeyProvider::from_env(
//!         &env, "v1", "CODLET_HMAC_KEY_V1", &[],
//!     )?;
//!     let tables        = D1TableConfig::default();
//!     let code_store    = D1CodeStore::new(env.d1("DB")?, tables.clone());
//!     let session_store = D1SessionStore::new(env.d1("DB")?, tables.clone());
//!     let token_store   = D1FormTokenStore::new(env.d1("DB")?, tables);
//!     let kv            = env.kv("CODLET_RL")?;
//!     let rl_store      = KvRateLimitStore::new(kv);
//!     // … wire into CodeAuth, SessionManager, etc.
//!     todo!()
//! }
//! ```
//!
//!
//!
//!
//!
//!

#![forbid(unsafe_code)]

// All implementation is wasm32-only.
#[cfg(target_arch = "wasm32")]
pub mod d1;
#[cfg(target_arch = "wasm32")]
pub mod http;
#[cfg(target_arch = "wasm32")]
pub mod key_provider;
#[cfg(target_arch = "wasm32")]
pub mod kv;
#[cfg(target_arch = "wasm32")]
pub mod migration;
#[cfg(target_arch = "wasm32")]
mod table_config;

#[cfg(target_arch = "wasm32")]
pub use d1::{D1CodeStore, D1FormTokenStore, D1SessionStore};
#[cfg(target_arch = "wasm32")]
pub use key_provider::WorkerKeyProvider;
#[cfg(target_arch = "wasm32")]
pub use kv::KvRateLimitStore;
#[cfg(target_arch = "wasm32")]
pub use migration::run_d1_migrations;
#[cfg(target_arch = "wasm32")]
pub use table_config::D1TableConfig;
