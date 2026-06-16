# codlet-examples

Compilable usage examples for [codlet](https://crates.io/crates/codlet-core).
Each binary is self-contained and runs against an in-memory SQLite database —
no database setup required.

## Examples

### `axum_login_logout` — minimal login / logout web app

A working [Axum](https://github.com/tokio-rs/axum) HTTP service that walks
through the full codlet authentication cycle end-to-end in a browser:

- An invite code is issued on startup and printed to stdout.
- The user opens `http://127.0.0.1:3000`, enters the code in a form, and
  receives a session cookie (`Set-Cookie`).
- The session is validated on every subsequent request.
- The user can sign out; codlet revokes the session record and clears the
  cookie.

**Practices demonstrated:**

- `CodeAuth::issue_code` — issue a one-time invite code with a grant payload
- `CodeAuth::find` + `CodeAuth::claim` — two-step atomic redemption (INV-5)
- `SessionManager::issue` — start a session after a won claim
- `SessionManager::validate` — authenticate a cookie on each request
- `SessionManager::revoke` — sign out and clear the cookie
- Sharing `SqliteStore` across managers via `Clone`
- Reading the `Set-Cookie` / clear-cookie header values from codlet and writing
  them into Axum responses
- Inline HTML without a template engine dependency

```sh
cd examples/axum_login_logout && cargo run
# Open http://127.0.0.1:3000 and paste the printed code to sign in.
```

---

### `sqlite_file` — file-based SQLite with WAL mode

Shows how to connect to a real SQLite file (rather than in-memory), with WAL
journal mode enabled for production-grade concurrent access. The example is
split into two steps that you run separately to observe persistence across
process restarts:

- **First run (no argument):** opens or creates `codlet-example.db`, runs
  migrations, issues an invite code, and prints it.
- **Second run (code as argument):** opens the same file, redeems the code
  with `find()` + `claim()`, issues a session, and validates it — demonstrating
  that the issued code record survived the process restart.

**Practices demonstrated:**

- `SqliteConnectOptions` with `filename`, `journal_mode(Wal)`, and
  `create_if_missing(true)` for a persistent, production-suitable connection
- `run_migrations` called on every startup (idempotent — safe with `IF NOT EXISTS`)
- Records written in one process and read in another via the same DB file

```sh
# Step 1 — issue a code (creates codlet-example.db)
cd examples/sqlite_file && cargo run

# Step 2 — redeem the printed code
# Run in examples/sqlite_file
cargo run -- <CODE>

# Clean up
rm codlet-example.db codlet-example.db-wal codlet-example.db-shm
```

---

### `sqlite_quickstart` — core primitives walkthrough

Runs entirely in Rust (no HTTP server). Steps through code issuance,
normalization, HMAC lookup-key derivation, atomic claim, session issuance, and
session validation using `codlet-sqlx` with an in-memory SQLite pool.

```sh
cd examples/sqlite_quickstart && cargo run
```

---

### `key_rotation` — HMAC key rotation grace period

Shows how to configure an active key and a previous key so that sessions
issued under the old key remain valid during the rotation grace period, and
how to retire the old key once it is safe to do so.

```sh
cd examples/key_rotation && cargo run
```

---

### `form_token_csrf` — single-use CSRF form tokens

Demonstrates `FormTokenManager`: issuing a single-use form token for a
state-changing form, verifying that the first submit proceeds and a replay
is detected, and testing the `bound_resource` binding that ties a token to a
specific resource.

```sh
cd examples/form_token_csrf && cargo run
```

---

## Notes

- **No production secrets here.** All examples use hard-coded key bytes for
  readability. In production, load key material from a secret manager or
  environment variable.
- **In-memory SQLite.** All examples use `sqlite::memory:` so they start
  instantly and leave nothing on disk.
- All examples are marked `publish = false` and are not published to crates.io.
- Each example is a standalone Cargo project; `cd` into its directory and
  run `cargo run` directly.
