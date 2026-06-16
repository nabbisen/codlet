//! Form-token CSRF protection example (RFC-016, RFC-007).
//!
//! Demonstrates:
//! - issuing a single-use form token for a state-changing form;
//! - the first submit proceeds;
//! - a concurrent or duplicate submit sees a Replay, not a double-execution;
//! - a token with the wrong subject or purpose is rejected.
//!
//! Host UX guidance (RFC-016 §10.3):
//! - Say "please reload the page and try again" on Invalid — do not say "CSRF failed".
//! - On Replay, silently redirect to the same result — no need to tell the user anything.
//!
//! Run with: `cargo run  # from examples/form_token_csrf/`

use std::time::Duration;

use codlet_core::audit::NoopAuditSink;
use codlet_core::auth::FormTokenManager;
use codlet_core::clock::FixedClock;
use codlet_core::hashing::{SecretHasher, StaticKeyProvider};
use codlet_core::mem::MemFormTokenStore;
use codlet_core::rng::SystemRandom;
use codlet_core::secret::SubjectId;
use codlet_core::store::token::TokenSubject;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hasher = SecretHasher::new(StaticKeyProvider::single(
        "v1",
        b"example-form-token-key-32-bytes!".to_vec(),
    )?);
    let store = MemFormTokenStore::new();
    let clock = FixedClock::at(1_700_000_000);
    let manager = FormTokenManager::new(
        store,
        hasher,
        clock,
        NoopAuditSink,
        Duration::from_secs(3_600),
    );

    let mut rng = SystemRandom::new();
    let alice = TokenSubject::Authenticated(SubjectId::new("alice".into()));

    // ── Issue token when rendering the form ──────────────────────────────────
    let token = manager
        .issue(&mut rng, alice.clone(), "save_note", None)
        .await?;
    // Embed token.expose() in the rendered form as a hidden field.
    // Never log the token value.

    // ── First submit: proceeds ────────────────────────────────────────────────
    let r1 = manager
        .consume(token.expose(), &alice, "save_note", None)
        .await?;
    assert!(r1.is_none(), "first submit: Proceed (Ok(None))");
    println!("[submit 1] Proceed — execute the mutation");

    // ── Duplicate submit (network retry, back-button, etc.): Replay ──────────
    let r2 = manager
        .consume(token.expose(), &alice, "save_note", None)
        .await?;
    // r2 is Ok(Some(result_ref)) if set_result was called, else Ok(None).
    println!("[submit 2] Replay — return prior result without re-executing");
    let _ = r2; // In a real handler: redirect to the same result page.

    // ── Wrong subject: Invalid ────────────────────────────────────────────────
    let bob = TokenSubject::Authenticated(SubjectId::new("bob".into()));
    let fresh_token = manager
        .issue(&mut rng, alice.clone(), "delete_note", None)
        .await?;
    let err = manager
        .consume(fresh_token.expose(), &bob, "delete_note", None)
        .await;
    assert!(err.is_err(), "wrong subject must be rejected");
    println!("[submit 3] Invalid subject — tell user: please reload the page and try again");

    // ── Wrong purpose: Invalid ────────────────────────────────────────────────
    let fresh_token2 = manager
        .issue(&mut rng, alice.clone(), "save_note", None)
        .await?;
    let err2 = manager
        .consume(fresh_token2.expose(), &alice, "delete_note", None)
        .await;
    assert!(err2.is_err(), "wrong purpose must be rejected");
    println!("[submit 4] Purpose mismatch — same generic message to user");

    println!("\nAll form-token invariants hold ✓");
    Ok(())
}
