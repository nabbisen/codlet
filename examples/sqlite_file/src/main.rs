//! File-based SQLite example (RFC-016).
//!
//! Demonstrates `codlet-sqlx` backed by a real SQLite file rather than an
//! in-memory database, showing:
//!
//! - opening a persistent SQLite file with WAL journal mode;
//! - running codlet migrations (idempotent — safe to call on every start);
//! - issuing codes and sessions that survive process restarts.
//!
//! # Running
//!
//! ```sh
//! # First run — issues a code and prints it, then exits.
//! cargo run  # from examples/sqlite_file/
//!
//! # Second run — the DB file still exists; paste the code from the first run.
//! # Run in examples/sqlite_file/
//! cargo run -- <CODE>
//! ```
//!
//! The database is written to `codlet-example.db` in the current directory.
//! Delete it to start fresh.
//!
//! **Never use hard-coded secrets in production.** Load key bytes from an
//! environment variable or secret manager.

use std::path::Path;
use std::time::Duration;

use codlet_core::CodePolicy;
use codlet_core::audit::NoopAuditSink;
use codlet_core::auth::{CodeAuth, SessionManager};
use codlet_core::clock::SystemClock;
use codlet_core::cookie::CookiePolicy;
use codlet_core::hashing::{SecretHasher, StaticKeyProvider};
use codlet_core::rng::SystemRandom;
use codlet_core::secret::{CodeId, SessionId, SubjectId};
use codlet_core::state::SessionValidationOutcome;
use codlet_sqlx::{SqliteStore, run_migrations};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

const DB_FILE: &str = "codlet-example.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Storage — file-based SQLite with WAL mode ─────────────────────────────
    // WAL (Write-Ahead Logging) allows concurrent reads while a write is in
    // progress. Recommended for any production SQLite deployment.
    let opts = SqliteConnectOptions::new()
        .filename(Path::new(DB_FILE))
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;

    // Migrations are idempotent (IF NOT EXISTS) — safe to run on every startup.
    run_migrations(&pool).await?;
    let store = SqliteStore::new(pool);

    // ── Crypto ────────────────────────────────────────────────────────────────
    // In production: load from an environment variable or secret manager.
    let key_bytes = b"EXAMPLE-32-byte-key-change-me!!".to_vec();
    let hasher = SecretHasher::new(
        StaticKeyProvider::single("v1", key_bytes).expect("non-empty key required"),
    );
    let mut rng = SystemRandom::new();

    // ── Managers ─────────────────────────────────────────────────────────────
    let policy = CodePolicy::default_human(Duration::from_secs(24 * 3600))?;
    let cookie_policy =
        CookiePolicy::production_strict("app_sid", Duration::from_secs(30 * 24 * 3600));

    let code_auth = CodeAuth::without_rate_limit(
        store.clone(),
        hasher.clone(),
        SystemClock::new(),
        NoopAuditSink,
        policy,
    );
    let session_mgr = SessionManager::new(
        store,
        hasher,
        SystemClock::new(),
        NoopAuditSink,
        cookie_policy,
    );

    // ── Dispatch on command-line argument ────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        // ── No argument: issue a new invite code ─────────────────────────────
        None => {
            let code_id = CodeId::new("invite-file-01".into());
            let (_, plain_code) = code_auth
                .issue_code(
                    &mut rng,
                    code_id,
                    None,                       // purpose
                    None,                       // scope
                    Some("role:member".into()), // grant returned after claim
                )
                .await?;

            println!("Database: {DB_FILE}");
            println!();
            println!("One-time code (deliver out-of-band — do not log in production):");
            println!();
            println!("    {}", plain_code.expose());
            println!();
            println!("Re-run with the code to redeem it");
            println!("in examples/sqlite_file/:");
            println!("    cargo run -- {}", plain_code.expose());
        }

        // ── Code supplied: redeem it and issue a session ──────────────────────
        Some(raw_input) => {
            // Step 1: validate format + look up the record.
            let found = match code_auth.find(raw_input, None).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Code not valid or already used: {e:?}");
                    eprintln!("Did you run the first step first? DB file: {DB_FILE}");
                    std::process::exit(1);
                }
            };

            // Step 2: host resolves or creates its subject, then claims.
            // codlet does not create users — the host owns that step.
            let subject = SubjectId::new("user-42".into());
            let redeem = code_auth.claim(&found, subject, None).await?;
            println!("Code claimed. Grant: {:?}", redeem.grant);

            // Step 3: issue a session.
            let issued = session_mgr
                .issue(&redeem, SessionId::new("sess-file-01".into()), &mut rng)
                .await?;
            println!("Session issued. Set-Cookie header (not logged in production).");

            // Step 4: validate the session (as a subsequent request would).
            // Extract the cookie value from the Set-Cookie header.
            let cookie_val: &str = issued
                .set_cookie
                .split(';')
                .next()
                .and_then(|kv| kv.split_once('=').map(|(_, v)| v))
                .expect("well-formed Set-Cookie");

            match session_mgr.validate(cookie_val).await? {
                SessionValidationOutcome::Authenticated { subject, .. } => {
                    println!(
                        "Session validated. Authenticated subject: {}",
                        subject.as_str()
                    );
                }
                SessionValidationOutcome::Unauthenticated => {
                    println!("Session not valid.");
                }
            }

            println!();
            println!("The session record is persisted in {DB_FILE}.");
            println!("It will survive process restarts until it expires or is revoked.");
        }
    }

    Ok(())
}
