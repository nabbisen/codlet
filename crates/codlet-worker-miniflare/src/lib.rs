//! Test-only Worker (RFC-054 stage C): runs `codlet-conformance` against the
//! **real** `D1CodeStore`, `D1SessionStore` and `D1FormTokenStore` inside
//! Miniflare, so the Rust D1 adapter code executes in a test instead of being
//! type-checked and mirrored in JavaScript.
//!
//! It is a separate crate so `codlet-worker` can stay a library with no fetch
//! handler. Routes (called from `crates/codlet-worker/tests/*.test.ts`):
//!
//! - `GET  /list` — the names of every sequential test, per suite.
//! - `POST /run?suite=<code|session|token>&name=<test>[&upgraded=1]` — run one test
//!   (`upgraded=1`, session suite only: against a pre-RFC-044 database upgraded by the runner). A failed
//!   assertion panics, which traps the wasm instance, so the request fails and the
//!   panic message goes to the console via the panic hook.
//! - `POST /run_d1_migrations` — call the shipped runner and report its result.
//! - `POST /pre_rfc_044_upgrade` — drop `last_seen_at`, call the runner, and report the
//!   table's columns before and after.
#![cfg(target_arch = "wasm32")]

use std::rc::Rc;

use codlet_conformance::{
    CODE_STORE_TESTS, FORM_TOKEN_STORE_TESTS, SESSION_STORE_TESTS, run_code_store_test,
    run_form_token_store_test, run_session_store_test,
};
use codlet_worker::{D1CodeStore, D1FormTokenStore, D1SessionStore, D1TableConfig};
use worker::{Context, Env, Request, Response, Result, d1::D1Database, event};

/// A clean database for one test: the shipped schema applied by the shipped
/// runner, no rows. Calling the runner for every test also exercises its
/// idempotence on every test after the first.
async fn fresh(db: &D1Database) {
    codlet_worker::run_d1_migrations(db)
        .await
        .expect("run_d1_migrations");
    for table in ["codlet_codes", "codlet_sessions", "codlet_form_tokens"] {
        db.prepare(format!("DELETE FROM {table}"))
            .run()
            .await
            .expect("clear table");
    }
}

/// Like [`fresh`], but the database is first taken back to its pre-RFC-044
/// shape (no `codlet_sessions.last_seen_at`) and then **upgraded by the shipped
/// runner** — so a suite run against it proves the upgrade path, not just a
/// fresh schema (RFC-057).
async fn fresh_pre_rfc_044_upgraded(db: &D1Database) {
    fresh(db).await;
    db.prepare("ALTER TABLE codlet_sessions DROP COLUMN last_seen_at")
        .run()
        .await
        .expect("simulate a pre-RFC-044 table");
    codlet_worker::run_d1_migrations(db)
        .await
        .expect("run_d1_migrations must upgrade a pre-RFC-044 table");
}

/// Column names of `codlet_sessions`, as D1 reports them.
async fn session_columns(db: &D1Database) -> Vec<String> {
    let rows: Vec<serde_json::Value> = db
        .prepare("SELECT name FROM pragma_table_info('codlet_sessions')")
        .all()
        .await
        .expect("pragma_table_info")
        .results()
        .expect("pragma rows");
    rows.iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect()
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
        "/pre_rfc_044_upgrade" => {
            // Take the shipped table back to its pre-RFC-044 shape, run the
            // shipped runner, and show what it did.
            fresh(&db).await;
            db.prepare("ALTER TABLE codlet_sessions DROP COLUMN last_seen_at")
                .run()
                .await?;
            let before = session_columns(&db).await;
            let outcome = codlet_worker::run_d1_migrations(&db).await;
            let after = session_columns(&db).await;
            json(&serde_json::json!({
                "ok": outcome.is_ok(),
                "error": outcome.err().map(|e| e.to_string()),
                "before": before,
                "after": after,
            }))
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
                    // `upgraded=1`: run against a pre-RFC-044 database that the
                    // shipped runner has upgraded.
                    let upgraded = param("upgraded") == "1";
                    let factory = || {
                        let db = Rc::clone(&db);
                        async move {
                            if upgraded {
                                fresh_pre_rfc_044_upgraded(&db).await;
                            } else {
                                fresh(&db).await;
                            }
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
