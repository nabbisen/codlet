/**
 * Test harness Worker for codlet-worker Miniflare integration tests.
 *
 * This Worker mirrors the SQL that codlet-worker's Rust stores execute,
 * running inside the real Miniflare/Workers runtime with real D1 and KV
 * bindings. Tests call it via SELF.fetch() in the vitest environment.
 *
 * Why JavaScript rather than compiled Rust:
 *   codlet-worker is a library crate (no fetch handler). Adding a fetch
 *   handler to the production crate would pollute it. A JS harness that
 *   executes the same SQL is the correct architecture: it tests the D1
 *   runtime binding behaviour (the layer unique to Miniflare) while the
 *   wasm32 compile CI job verifies the Rust type constraints.
 *
 * Each route corresponds to one store operation. All SQL is copied verbatim
 * from the Rust implementation so any divergence is a test failure.
 */

const MIGRATION_SQL = `
CREATE TABLE IF NOT EXISTS codlet_codes (
  id TEXT NOT NULL PRIMARY KEY,
  lookup_key TEXT NOT NULL UNIQUE,
  key_version TEXT NOT NULL,
  purpose TEXT, scope TEXT, grant_payload TEXT,
  created_at REAL NOT NULL, expires_at REAL NOT NULL,
  used_at REAL, used_by_subject TEXT, revoked_at REAL
);
CREATE TABLE IF NOT EXISTS codlet_sessions (
  id TEXT NOT NULL PRIMARY KEY,
  lookup_key TEXT NOT NULL UNIQUE,
  key_version TEXT NOT NULL,
  subject TEXT NOT NULL,
  created_at REAL NOT NULL, expires_at REAL NOT NULL,
  revoked_at REAL,
  last_seen_at REAL
);
CREATE TABLE IF NOT EXISTS codlet_form_tokens (
  lookup_key TEXT NOT NULL PRIMARY KEY,
  key_version TEXT NOT NULL,
  subject_kind TEXT NOT NULL,
  purpose TEXT NOT NULL,
  bound_resource TEXT,
  issued_at REAL NOT NULL, expires_at REAL NOT NULL,
  consumed_at REAL, result_ref TEXT
);
`;

async function migrate(db) {
  // D1's exec() only accepts one statement at a time.
  // Split on semicolons and run each statement via prepare().run().
  for (const stmt of MIGRATION_SQL.split(';')) {
    const s = stmt.replace(/--[^\n]*/g, '').trim();
    if (s) await db.prepare(s).run();
  }
  await ensureSessionLastSeenAtColumn(db);
}

// Mirrors codlet-worker's Rust migration.rs: RFC-044's last_seen_at column is
// additive, so a codlet_sessions table created before this ran (this
// function's own CREATE TABLE IF NOT EXISTS is a no-op on it) needs the
// column added explicitly. D1 has no `ADD COLUMN IF NOT EXISTS`, so this
// checks column presence first via PRAGMA table_info -- proving that pragma
// is actually usable against a real D1 binding, not assumed from SQLite docs.
async function ensureSessionLastSeenAtColumn(db) {
  const { results } = await db.prepare("PRAGMA table_info('codlet_sessions')").run();
  const hasColumn = results.some((r) => r.name === 'last_seen_at');
  if (!hasColumn) {
    await db.prepare('ALTER TABLE codlet_sessions ADD COLUMN last_seen_at REAL').run();
  }
}

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    const db = env.DB;
    const kv = env.CODLET_RL;

    // POST /migrate
    if (url.pathname === '/migrate') {
      await migrate(db);
      // Run twice to verify idempotency
      await migrate(db);
      return Response.json({ ok: true });
    }

    // POST /codes/insert  body: {id, lookup_key, key_version, created_at, expires_at, scope?}
    if (url.pathname === '/codes/insert' && req.method === 'POST') {
      const b = await req.json();
      // Timestamps stored as REAL (f64) — same as D1Type::Real(t as f64) in Rust
      await db.prepare(
        'INSERT INTO codlet_codes (id, lookup_key, key_version, created_at, expires_at, scope) VALUES (?,?,?,?,?,?)'
      ).bind(b.id, b.lookup_key, b.key_version, b.created_at, b.expires_at, b.scope ?? null).run();
      return Response.json({ ok: true });
    }

    // POST /codes/find  body: {lookup_key, now}
    // RFC-047: matches on lookup_key only -- no expiry/revocation/use
    // predicate. Returns used_at/revoked_at so the caller (classify_code_lookup
    // in Rust; this test file just asserts the raw fields) can classify it.
    if (url.pathname === '/codes/find' && req.method === 'POST') {
      const b = await req.json();
      const row = await db.prepare(
        `SELECT id, key_version, grant_payload, scope, expires_at, used_at, revoked_at
         FROM codlet_codes
         WHERE lookup_key = ? LIMIT 1`
      ).bind(b.lookup_key).first();
      return Response.json(row ?? null);
    }

    // POST /codes/claim  body: {id, subject, now, scope?}
    // Mirrors D1CodeStore::claim_code — conditional UPDATE + meta.changes.
    // RFC-047 explicitly does not touch this: it remains the sole
    // enforcement point for expiry/revocation/use at claim time (INV-5).
    // RFC-048: when `scope` is supplied it is always bound as a parameter,
    // never interpolated -- a fixed pair of complete SQL templates, exactly
    // mirroring D1CodeStore::claim_code's fix.
    if (url.pathname === '/codes/claim' && req.method === 'POST') {
      const b = await req.json();
      const result = b.scope !== undefined
        ? await db.prepare(
            `UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
             WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL
               AND expires_at > ? AND scope = ?`
          ).bind(b.now, b.subject, b.id, b.now, b.scope).run()
        : await db.prepare(
            `UPDATE codlet_codes SET used_at = ?, used_by_subject = ?
             WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL AND expires_at > ?`
          ).bind(b.now, b.subject, b.id, b.now).run();
      return Response.json({ changes: result.meta.changes });
    }

    // POST /codes/revoke  body: {id, now}
    // Mirrors D1CodeStore::revoke_code.
    if (url.pathname === '/codes/revoke' && req.method === 'POST') {
      const b = await req.json();
      await db.prepare(
        `UPDATE codlet_codes SET revoked_at = ?
         WHERE id = ? AND used_at IS NULL AND revoked_at IS NULL`
      ).bind(b.now, b.id).run();
      return Response.json({ ok: true });
    }

    // POST /sessions/insert  body: {id, lookup_key, key_version, subject, created_at, expires_at}
    if (url.pathname === '/sessions/insert' && req.method === 'POST') {
      const b = await req.json();
      await db.prepare(
        'INSERT INTO codlet_sessions (id, lookup_key, key_version, subject, created_at, expires_at) VALUES (?,?,?,?,?,?)'
      ).bind(b.id, b.lookup_key, b.key_version, b.subject, b.created_at, b.expires_at).run();
      return Response.json({ ok: true });
    }

    // POST /sessions/find  body: {lookup_key, now}
    if (url.pathname === '/sessions/find' && req.method === 'POST') {
      const b = await req.json();
      const row = await db.prepare(
        `SELECT id, subject, created_at, expires_at, last_seen_at FROM codlet_sessions
         WHERE lookup_key = ? AND revoked_at IS NULL AND expires_at > ? LIMIT 1`
      ).bind(b.lookup_key, b.now).first();
      return Response.json(row ?? null);
    }

    // POST /sessions/touch  body: {id, now}
    // Mirrors D1SessionStore::touch_session (RFC-044).
    if (url.pathname === '/sessions/touch' && req.method === 'POST') {
      const b = await req.json();
      await db.prepare(
        'UPDATE codlet_sessions SET last_seen_at = ? WHERE id = ?'
      ).bind(b.now, b.id).run();
      return Response.json({ ok: true });
    }

    // POST /sessions/simulate-pre-rfc-044  body: {}
    // Drops and recreates codlet_sessions in its pre-RFC-044 shape (no
    // last_seen_at), so /migrate's idempotent-ALTER path can be exercised
    // against a table that predates this column, exactly like the Rust
    // migration.rs test does against a real SQLite pool.
    if (url.pathname === '/sessions/simulate-pre-rfc-044' && req.method === 'POST') {
      await db.prepare('DROP TABLE IF EXISTS codlet_sessions').run();
      await db.prepare(
        `CREATE TABLE codlet_sessions (
          id TEXT NOT NULL PRIMARY KEY,
          lookup_key TEXT NOT NULL UNIQUE,
          key_version TEXT NOT NULL,
          subject TEXT NOT NULL,
          created_at REAL NOT NULL, expires_at REAL NOT NULL,
          revoked_at REAL
        )`
      ).run();
      return Response.json({ ok: true });
    }

    // POST /tokens/insert  body: {lookup_key, key_version, subject_kind, purpose, issued_at, expires_at}
    if (url.pathname === '/tokens/insert' && req.method === 'POST') {
      const b = await req.json();
      await db.prepare(
        `INSERT INTO codlet_form_tokens
         (lookup_key, key_version, subject_kind, purpose, issued_at, expires_at)
         VALUES (?,?,?,?,?,?)`
      ).bind(b.lookup_key, b.key_version, b.subject_kind, b.purpose, b.issued_at, b.expires_at).run();
      return Response.json({ ok: true });
    }

    // POST /tokens/consume  body: {lookup_key, subject_kind, purpose, now}
    // Mirrors D1FormTokenStore::consume_form_token — conditional UPDATE + meta.changes
    if (url.pathname === '/tokens/consume' && req.method === 'POST') {
      const b = await req.json();
      const result = await db.prepare(
        `UPDATE codlet_form_tokens SET consumed_at = ?
         WHERE lookup_key = ? AND subject_kind = ? AND purpose = ?
           AND COALESCE(bound_resource,'') = ''
           AND expires_at > ? AND consumed_at IS NULL`
      ).bind(b.now, b.lookup_key, b.subject_kind, b.purpose, b.now).run();
      return Response.json({ changes: result.meta.changes });
    }

    // POST /kv/record_failure  body: {key}
    if (url.pathname === '/kv/record_failure' && req.method === 'POST') {
      const b = await req.json();
      const kvKey = `codlet:rl:${b.key}`;
      const current = parseInt(await kv.get(kvKey) ?? '0');
      await kv.put(kvKey, String(current + 1), { expirationTtl: 300 });
      return Response.json({ count: current + 1 });
    }

    // POST /kv/clear  body: {key}
    if (url.pathname === '/kv/clear' && req.method === 'POST') {
      const b = await req.json();
      await kv.delete(`codlet:rl:${b.key}`);
      return Response.json({ ok: true });
    }

    // POST /kv/check  body: {key}
    if (url.pathname === '/kv/check' && req.method === 'POST') {
      const b = await req.json();
      const val = await kv.get(`codlet:rl:${b.key}`);
      return Response.json({ count: parseInt(val ?? '0') });
    }

    return new Response('not found', { status: 404 });
  }
};
