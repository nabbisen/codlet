//! Minimal login / logout web app (RFC-016 example).
//!
//! A tiny [Axum](https://github.com/tokio-rs/axum) service that demonstrates
//! the full codlet auth cycle: issue a one-time code → user enters it in a
//! browser form → session is set → user can log out.
//!
//! # Running
//!
//! ```sh
//! cargo run  # from examples/axum_login_logout/
//! ```
//!
//! The server starts on <http://127.0.0.1:3000>.  A one-time code is printed
//! to stdout — paste it into the browser form to sign in.
//!
//! # What this covers
//!
//! - `CodeAuth::issue_code` + `CodeAuth::find` + `CodeAuth::claim`
//! - `SessionManager::issue` + `SessionManager::validate` + `SessionManager::revoke`
//! - Reading the session cookie from an incoming request
//! - Sending `Set-Cookie` / clear-cookie headers in responses
//!
//! **Never use hard-coded key material in production.**
//! This example uses a fixed byte string for readability only.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    Form, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use codlet_core::audit::NoopAuditSink;
use codlet_core::auth::{CodeAuth, SessionManager};
use codlet_core::clock::SystemClock;
use codlet_core::cookie::CookiePolicy;
use codlet_core::hashing::{SecretHasher, StaticKeyProvider};
use codlet_core::rng::SystemRandom;
use codlet_core::secret::{CodeId, SessionId, SubjectId};
use codlet_core::state::SessionValidationOutcome;
use codlet_core::{CodePolicy, IssuedSession};
use codlet_sqlx::{SqliteStore, run_migrations};
use serde::Deserialize;

// ── Application state ─────────────────────────────────────────────────────────

/// Shared state injected into every handler.
#[derive(Clone)]
struct AppState {
    code_auth: Arc<
        CodeAuth<
            SqliteStore,
            codlet_core::auth::NoRateLimit,
            StaticKeyProvider,
            SystemClock,
            NoopAuditSink,
        >,
    >,
    session_mgr: Arc<SessionManager<SqliteStore, StaticKeyProvider, SystemClock, NoopAuditSink>>,
    cookie_name: String,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Storage ───────────────────────────────────────────────────────────────
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect("sqlite::memory:")
        .await?;
    run_migrations(&pool).await?;
    let store = SqliteStore::new(pool);

    // ── Crypto ────────────────────────────────────────────────────────────────
    // IMPORTANT: load key bytes from an environment variable or secret manager
    // in production.  Never commit real key material to source control.
    let key_bytes = b"EXAMPLE-32-byte-key-change-me!!".to_vec();
    let hasher = SecretHasher::new(
        StaticKeyProvider::single("v1", key_bytes).expect("non-empty key bytes required"),
    );

    // ── Policies ──────────────────────────────────────────────────────────────
    let code_policy = CodePolicy::default_human(Duration::from_secs(15 * 60))?;

    // In production use `CookiePolicy::production_strict`.  We use `lax` here
    // so the redirect after login carries the cookie in most browsers.
    let cookie_policy = CookiePolicy::production_lax("app_sid", Duration::from_secs(24 * 3600));
    let cookie_name = cookie_policy.name().to_string();

    // ── Managers ─────────────────────────────────────────────────────────────
    let code_auth = Arc::new(CodeAuth::without_rate_limit(
        store.clone(),
        hasher.clone(),
        SystemClock::new(),
        NoopAuditSink,
        code_policy,
    ));
    let session_mgr = Arc::new(SessionManager::new(
        store,
        hasher,
        SystemClock::new(),
        NoopAuditSink,
        cookie_policy,
    ));

    // ── Issue an invite code (admin side) ─────────────────────────────────────
    // In a real service this would be an admin endpoint or a CLI command.
    // Here we print it to stdout so you can paste it into the browser form.
    let mut rng = SystemRandom::new();
    let (_, plain_code) = code_auth
        .issue_code(
            &mut rng,
            CodeId::new("example-invite".into()),
            None,                       // purpose
            None,                       // scope
            Some("role:member".into()), // grant returned to host after claim
        )
        .await?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  codlet example — one-time login code:");
    println!();
    println!("      {}", plain_code.expose());
    println!();
    println!("  Open http://127.0.0.1:3000 and paste this code to sign in.");
    println!("  The code expires in 15 minutes.");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // ── Router ────────────────────────────────────────────────────────────────
    let state = AppState {
        code_auth,
        session_mgr,
        cookie_name,
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a named cookie value from the `Cookie` request header.
fn extract_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /` — show either the login form or the "you are signed in" page.
async fn home(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    // Try to validate the session cookie.
    if let Some(cookie_val) = extract_cookie(&headers, &state.cookie_name) {
        if let Ok(SessionValidationOutcome::Authenticated { subject, .. }) =
            state.session_mgr.validate(cookie_val).await
        {
            return Html(page_signed_in(subject.as_str()));
        }
    }
    Html(page_login(None))
}

/// `POST /login` — accept the one-time code and start a session.
#[derive(Deserialize)]
struct LoginForm {
    code: String,
}

async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    // Step 1: find the redeemable code record.
    let found = match state.code_auth.find(&form.code, None).await {
        Ok(r) => r,
        Err(_) => return Html(page_login(Some("Invalid or expired code."))).into_response(),
    };

    // Step 2: host resolves the subject from the grant (or any business logic).
    // Here we just use the grant payload directly as the subject identifier.
    let subject_str = found.grant.as_deref().unwrap_or("member").to_string();
    let subject = SubjectId::new(subject_str);

    // Step 3: claim the code atomically (single-winner, INV-5).
    let redeem = match state.code_auth.claim(&found, subject, None).await {
        Ok(r) => r,
        Err(_) => return Html(page_login(Some("Code already used or expired."))).into_response(),
    };

    // Step 4: issue a session.
    let mut rng = SystemRandom::new();
    let session_id = SessionId::new(uuid_v4());
    let IssuedSession { set_cookie, .. } =
        match state.session_mgr.issue(&redeem, session_id, &mut rng).await {
            Ok(s) => s,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Session error").into_response(),
        };

    // Redirect to home with the session cookie set.
    ([(header::SET_COOKIE, set_cookie)], Redirect::to("/")).into_response()
}

/// `POST /logout` — revoke the session and clear the cookie.
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(cookie_val) = extract_cookie(&headers, &state.cookie_name) {
        if let Ok(SessionValidationOutcome::Authenticated { session_id, .. }) =
            state.session_mgr.validate(cookie_val).await
        {
            // Revoke the session record and get the clear-cookie header value.
            if let Ok(clear_cookie) = state.session_mgr.revoke(&session_id).await {
                return ([(header::SET_COOKIE, clear_cookie)], Redirect::to("/")).into_response();
            }
        }
    }
    Redirect::to("/").into_response()
}

// ── Tiny UUID v4 (no extra dependency) ───────────────────────────────────────

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Good enough for a non-cryptographic session record identifier.
    // In production, use the `uuid` crate.
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("sess-{t:08x}-{:04x}", t.wrapping_mul(0xdeadbeef) & 0xffff)
}

// ── HTML pages (inline, no template engine dependency) ────────────────────────

fn page_login(error: Option<&str>) -> String {
    let error_html = error
        .map(|e| format!(r#"<p class="error">{e}</p>"#))
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Sign in</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: system-ui, sans-serif;
      background: #f8f8f7;
      min-height: 100dvh;
      display: grid;
      place-items: center;
      color: #1a1a1a;
    }}
    .card {{
      background: #fff;
      border: 1px solid #e0e0dc;
      border-radius: 8px;
      padding: 2rem 2.5rem;
      width: min(360px, 90vw);
    }}
    h1 {{ font-size: 1.25rem; font-weight: 600; margin-bottom: 0.5rem; }}
    p.hint {{ font-size: 0.875rem; color: #666; margin-bottom: 1.5rem; }}
    label {{ display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 0.4rem; }}
    input[type=text] {{
      width: 100%;
      padding: 0.6rem 0.75rem;
      border: 1px solid #ccc;
      border-radius: 5px;
      font-size: 1.125rem;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      font-family: monospace;
    }}
    input[type=text]:focus {{
      outline: 2px solid #3b6ef0;
      outline-offset: 1px;
      border-color: transparent;
    }}
    button {{
      margin-top: 1rem;
      width: 100%;
      padding: 0.65rem;
      background: #1a1a1a;
      color: #fff;
      border: none;
      border-radius: 5px;
      font-size: 0.9375rem;
      font-weight: 500;
      cursor: pointer;
    }}
    button:hover {{ background: #333; }}
    .error {{
      margin-top: 1rem;
      padding: 0.6rem 0.75rem;
      background: #fff3f3;
      border: 1px solid #f5c6c6;
      border-radius: 5px;
      color: #c0392b;
      font-size: 0.875rem;
    }}
  </style>
</head>
<body>
  <div class="card">
    <h1>Sign in</h1>
    <p class="hint">Enter the one-time code you received.</p>
    <form method="post" action="/login">
      <label for="code">One-time code</label>
      <input id="code" name="code" type="text" autocomplete="one-time-code"
             autocapitalize="characters" autofocus required>
      <button type="submit">Continue</button>
      {error_html}
    </form>
  </div>
</body>
</html>"#
    )
}

fn page_signed_in(subject: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Signed in</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: system-ui, sans-serif;
      background: #f8f8f7;
      min-height: 100dvh;
      display: grid;
      place-items: center;
      color: #1a1a1a;
    }}
    .card {{
      background: #fff;
      border: 1px solid #e0e0dc;
      border-radius: 8px;
      padding: 2rem 2.5rem;
      width: min(360px, 90vw);
    }}
    h1 {{ font-size: 1.25rem; font-weight: 600; margin-bottom: 0.5rem; }}
    p.sub {{ font-size: 0.875rem; color: #555; margin-bottom: 1.5rem; }}
    code {{
      background: #f4f4f2;
      padding: 0.15em 0.4em;
      border-radius: 3px;
      font-size: 0.875rem;
    }}
    form {{ margin-top: 1.5rem; }}
    button {{
      padding: 0.6rem 1.25rem;
      background: transparent;
      color: #1a1a1a;
      border: 1px solid #ccc;
      border-radius: 5px;
      font-size: 0.875rem;
      cursor: pointer;
    }}
    button:hover {{ background: #f4f4f2; }}
  </style>
</head>
<body>
  <div class="card">
    <h1>You are signed in</h1>
    <p class="sub">Subject: <code>{subject}</code></p>
    <p class="sub">
      Your host application would now load user data and
      check authorization based on this subject identifier.
      codlet's responsibility ends here.
    </p>
    <form method="post" action="/logout">
      <button type="submit">Sign out</button>
    </form>
  </div>
</body>
</html>"#
    )
}
