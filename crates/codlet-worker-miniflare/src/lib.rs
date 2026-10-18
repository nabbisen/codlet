//! Test-only Worker (RFC-054 stage C): runs `codlet-conformance` against the
//! **real** `D1CodeStore`, `D1SessionStore` and `D1FormTokenStore` inside
//! Miniflare, so the Rust D1 adapter code executes in a test instead of being
//! type-checked and mirrored in JavaScript.
//!
//! It is a separate crate so `codlet-worker` can stay a library with no fetch
//! handler. Routes (called from `crates/codlet-worker/tests/*.test.ts`):
//!
//! - `GET  /list` — the names of every sequential test, per suite.
//! - `POST /run?suite=<code|session|token>&name=<test>` — run one test. A failed
//!   assertion panics, which traps the wasm instance, so the request fails and the
//!   panic message goes to the console via the panic hook.
//! - `POST /run_d1_migrations` — call the shipped runner and report its result.
#![cfg(target_arch = "wasm32")]

use std::rc::Rc;

use codlet_conformance::{
    CODE_STORE_TESTS, FORM_TOKEN_STORE_TESTS, SESSION_STORE_TESTS, run_code_store_test,
    run_form_token_store_test, run_session_store_test,
};
use codlet_worker::{D1CodeStore, D1FormTokenStore, D1SessionStore, D1TableConfig};
use worker::{Context, Env, Request, Response, Result, d1::D1Database, event};

/// The migration codlet ships, embedded at build time — never a copy.
const MIGRATION_SQL: &str = include_str!("../../codlet-worker/migrations/0001_initial.sql");

/// Apply the shipped migration statement by statement.
///
/// `run_d1_migrations` cannot be used here: it hands the whole file to
/// `D1Database::exec`, which D1 rejects (see `/run_d1_migrations`). Comments are
/// stripped *before* splitting on `;` because the shipped file has a `;` inside
/// a `--` comment.
async fn apply_migration(db: &D1Database) {
    let stripped: String = MIGRATION_SQL
        .lines()
        .map(|l| l.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        db.prepare(stmt).run().await.expect("migration statement");
    }
}

/// A clean database for one test: shipped schema, no rows.
async fn fresh(db: &D1Database) {
    apply_migration(db).await;
    for table in ["codlet_codes", "codlet_sessions", "codlet_form_tokens"] {
        db.prepare(format!("DELETE FROM {table}"))
            .run()
            .await
            .expect("clear table");
    }
}

fn json(value: &serde_json::Value) -> Result<Response> {
    Response::from_json(value)
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let url = req.url()?;
    let db = Rc::new(env.d1("DB")?);

    match url.path() {
        "/list" => json(&serde_json::json!({
            "code": CODE_STORE_TESTS,
            "session": SESSION_STORE_TESTS,
            "token": FORM_TOKEN_STORE_TESTS,
        })),
        "/run_d1_migrations" => {
            // The runner the adapter ships, called exactly as a host would.
            match codlet_worker::run_d1_migrations(&db).await {
                Ok(()) => json(&serde_json::json!({ "ok": true })),
                Err(e) => json(&serde_json::json!({ "ok": false, "error": e.to_string() })),
            }
        }
        "/run" => {
            let param = |k: &str| {
                url.query_pairs()
                    .find(|(key, _)| key == k)
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_default()
            };
            let (suite, name) = (param("suite"), param("name"));
            let tables = D1TableConfig::default;
            let known = match suite.as_str() {
                "code" => {
                    let factory = || {
                        let db = Rc::clone(&db);
                        async move {
                            fresh(&db).await;
                            D1CodeStore::new(db, tables())
                        }
                    };
                    run_code_store_test(&name, &factory).await
                }
                "session" => {
                    let factory = || {
                        let db = Rc::clone(&db);
                        async move {
                            fresh(&db).await;
                            D1SessionStore::new(db, tables())
                        }
                    };
                    run_session_store_test(&name, &factory).await
                }
                "token" => {
                    let factory = || {
                        let db = Rc::clone(&db);
                        async move {
                            fresh(&db).await;
                            D1FormTokenStore::new(db, tables())
                        }
                    };
                    run_form_token_store_test(&name, &factory).await
                }
                _ => false,
            };
            if known {
                json(&serde_json::json!({ "ok": true, "suite": suite, "name": name }))
            } else {
                Response::error(format!("unknown test: suite={suite} name={name}"), 404)
            }
        }
        other => Response::error(format!("no route: {other}"), 404),
    }
}
