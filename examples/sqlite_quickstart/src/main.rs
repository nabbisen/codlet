//! SQLite quick-start example (RFC-016).
//!
//! Demonstrates a complete authentication flow using `codlet-sqlx`:
//!
//! 1. issue an invite code;
//! 2. simulate user submission (validate and normalize);
//! 3. find the redeemable record;
//! 4. claim it (atomic, single-winner);
//! 5. issue a session cookie;
//! 6. validate the session on a subsequent request.
//!
//! **Never use hard-coded secrets in production.** Load from a secret manager
//! or environment variable.  This example uses a generated test key for
//! illustration only.

use std::time::Duration;

use codlet::audit::NoopAuditSink;
use codlet::auth::{CodeAuth, SessionManager};
use codlet::clock::SystemClock;
use codlet::cookie::CookiePolicy;
use codlet::hashing::{SecretHasher, StaticKeyProvider};
use codlet::mem::MemRateLimitStore;
use codlet::rng::SystemRandom;
use codlet::secret::{CodeId, SessionId, SubjectId};
use codlet::state::SessionValidationOutcome;
use codlet::{CodePolicy, RateLimitKey, RateLimitPolicy};
use codlet_sqlx::{SqliteStore, run_migrations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Set up storage ────────────────────────────────────────────────────
    // In production: use a file path and connect with SqlitePoolOptions.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    run_migrations(&pool).await?;
    let store = SqliteStore::new(pool);

    // ── 2. Configure crypto ──────────────────────────────────────────────────
    // In production: load key bytes from an environment variable or secret manager.
    // Never commit real key material to source control.
    let key_bytes = b"EXAMPLE-32-byte-key-change-me!!!".to_vec();
    let hasher = SecretHasher::new(
        StaticKeyProvider::single("v1", key_bytes)
            .expect("a key of at least 32 bytes is required — use a secret manager in production"),
    );
    let mut rng = SystemRandom::new();

    // ── 3. Build managers ────────────────────────────────────────────────────
    let policy = CodePolicy::default_human(Duration::from_secs(24 * 3600))?;
    // Rate limiting is required for every code policy (see the threat model).
    // `MemRateLimitStore` counts in-process only: with more than one instance a
    // shared backend is needed (see the adapter matrix).
    let code_auth = CodeAuth::new(
        store.clone(),
        MemRateLimitStore::new(),
        hasher.clone(),
        SystemClock::new(),
        NoopAuditSink,
        policy,
        RateLimitPolicy::default_invite(),
    );
    // Key for the rate limit: in a web service, the verified client address.
    let rate_key = RateLimitKey::new("local-quickstart");
    let cookie_policy =
        CookiePolicy::production_strict("app_sid", Duration::from_secs(30 * 24 * 3600));
    let session_mgr = SessionManager::new(
        store.clone(),
        hasher,
        SystemClock::new(),
        NoopAuditSink,
        cookie_policy,
    );

    // ── 4. Issue a code (admin side) ─────────────────────────────────────────
    let code_id = CodeId::new("invite-001".into());
    let (_, plain_code) = code_auth
        .issue_code(
            &mut rng,
            code_id,
            None,                       // purpose
            None,                       // scope
            Some("role:member".into()), // grant: host-defined payload
        )
        .await?;

    // In a real service you would deliver plain_code.expose() out-of-band
    // (email, SMS, printed card). Never log it.
    println!("[admin] code issued — deliver out-of-band (do not log in production)");

    // ── 5. User submits the code ─────────────────────────────────────────────
    // The user may type spaces or hyphens; validate_code_input handles that.
    let raw_input = plain_code.expose().to_string(); // simulating user submission
    let found = code_auth.find(&raw_input, Some(&rate_key)).await?;

    // ── 6. Host creates or resolves its subject, then claims ─────────────────
    // The host owns all user/membership creation. codlet only records
    // which SubjectId claimed the code — it does not create users.
    let subject = SubjectId::new("user-42".into()); // host-assigned after its own DB write
    let redeem_success = code_auth.claim(&found, subject, Some(&rate_key)).await?;
    println!("[auth] code claimed by {:?}", redeem_success.grant);

    // ── 7. Issue a session ───────────────────────────────────────────────────
    let issued = session_mgr
        .issue(&redeem_success, SessionId::new("sess-001".into()), &mut rng)
        .await?;

    // issued.set_cookie is the Set-Cookie header value; the plaintext bearer
    // secret is inside it — do not log this value.
    println!("[auth] session issued — Set-Cookie header ready (not logged)");

    // ── 8. Validate on a subsequent request ──────────────────────────────────
    // Extract the cookie value from the request (e.g. `Cookie: app_sid=…`)
    // In this simulation we extract it from the Set-Cookie header.
    let cookie_val: &str = issued
        .set_cookie
        .split(';')
        .next()
        .and_then(|kv| kv.split_once('=').map(|(_, v)| v))
        .expect("well-formed Set-Cookie");

    let outcome = session_mgr.validate(Some(cookie_val)).await?;
    match outcome {
        SessionValidationOutcome::Authenticated { subject, .. } => {
            println!("[request] authenticated subject: {}", subject.as_str());
            // Host now performs its own authorization check:
            // e.g. load membership, check community role, etc.
        }
        SessionValidationOutcome::Unauthenticated { reason } => {
            // `reason` is host-only (RFC-046); do not render it to end users.
            println!("[request] no valid session ({reason:?}) — redirect to join page");
        }
    }

    Ok(())
}

/// Demonstrate the callback-based (high-level) flow.
///
/// This satisfies RFC-013 §10.4 item 5: "Examples show both low-level
/// and high-level integration paths." The two-step `find` + `claim` is the
/// low-level path above; this is the high-level path.
///
/// The callback only runs after a confirmed won claim (INV-7).
#[allow(dead_code)]
#[allow(deprecated)] // demonstrates the experimental redeem_with_callback API (RFC-D)
async fn callback_flow_example(
    code_auth: &codlet::auth::CodeAuth<
        SqliteStore,
        MemRateLimitStore,
        StaticKeyProvider,
        codlet::clock::SystemClock,
        codlet::audit::NoopAuditSink,
    >,
    raw_code: &str,
    rate_key: &RateLimitKey,
) -> Result<codlet::auth::RedeemSuccess, codlet::auth::RedeemError> {
    code_auth
        .redeem_with_callback(raw_code, Some(rate_key), |_record| async {
            // The host creates or resolves its subject here, *after* the claim
            // is won. codlet does not create users — the host owns that step.
            Ok::<_, std::convert::Infallible>(codlet::secret::SubjectId::new("user-99".into()))
        })
        .await
}
