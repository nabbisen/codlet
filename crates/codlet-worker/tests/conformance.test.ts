/**
 * codlet-worker Miniflare integration tests (RFC-033 §14).
 *
 * Runs inside the Workers runtime via @cloudflare/vitest-pool-workers.
 * SELF.fetch() calls worker.js which executes the same SQL as the Rust stores.
 *
 * Coverage unique to these tests:
 *   - D1 binding API (prepare/bind/run/meta.changes) in the live Workers runtime
 *   - REAL timestamp storage and comparison in D1 (D1Type::Real semantics)
 *   - KV put/get/delete with TTL
 *   - COALESCE(bound_resource,'') in form-token consume
 *   - Concurrent UPDATE race → exactly one winner (INV-5, INV-6)
 */

import { SELF } from "cloudflare:test";
import { describe, it, expect, beforeEach } from "vitest";

const NOW = Math.floor(Date.now() / 1000);
const LATER = NOW + 3600;

async function post(path: string, body: unknown): Promise<unknown> {
  const res = await SELF.fetch(`http://worker${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${path} => ${res.status}: ${await res.text()}`);
  return res.json();
}

// ── Migration ─────────────────────────────────────────────────────────────────

describe("migration", () => {
  it("creates all three tables", async () => {
    const res = await post("/migrate", {}) as { ok: boolean };
    expect(res.ok).toBe(true);
  });

  it("is idempotent (IF NOT EXISTS semantics in D1)", async () => {
    // Run twice — must not throw
    await post("/migrate", {});
    const res = await post("/migrate", {}) as { ok: boolean };
    expect(res.ok).toBe(true);
  });

  it("RFC-044: adds last_seen_at to a pre-existing codlet_sessions table", async () => {
    // Simulate a database created before RFC-044 (no last_seen_at column),
    // then run the real /migrate path -- proving PRAGMA table_info and the
    // idempotent ALTER TABLE both work against a real D1 binding, not just
    // documented SQLite behaviour.
    await post("/sessions/simulate-pre-rfc-044", {});
    const res = await post("/migrate", {}) as { ok: boolean };
    expect(res.ok).toBe(true);

    // The column must now exist and be usable: insert a row (leaving
    // last_seen_at unset, as insert_session always does) and read it back.
    const id = `sess-migrated-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/sessions/insert", { id, lookup_key: lk, key_version: "v1", subject: "carol", created_at: NOW, expires_at: LATER });
    const row = await post("/sessions/find", { lookup_key: lk, now: NOW }) as { last_seen_at: number | null } | null;
    expect(row?.last_seen_at ?? null).toBeNull();

    // Migrating again (the ALTER path already applied) must still not throw.
    const res2 = await post("/migrate", {}) as { ok: boolean };
    expect(res2.ok).toBe(true);
  });
});

// ── D1CodeStore ───────────────────────────────────────────────────────────────

describe("D1CodeStore", () => {
  beforeEach(async () => { await post("/migrate", {}); });

  it("inserts and finds a redeemable code", async () => {
    const id = `code-find-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });
    const row = await post("/codes/find", { lookup_key: lk, now: NOW }) as { id: string } | null;
    expect(row?.id).toBe(id);
  });

  it("RFC-047: expired code is returned (not excluded), with expires_at intact — REAL timestamp (D1Type::Real)", async () => {
    // Inverted per RFC-047 §4.3: the store must return the row, not exclude
    // it — codlet's classify_code_lookup is what rejects it. A backend that
    // kept its old exclusion filter would return null here.
    const lk = `exp-${Date.now()}`.padEnd(64, "x");
    const expiresAt = NOW - 1; // stored as REAL (f64) per RFC-033 §6
    await post("/codes/insert", {
      id: `exp-${Date.now()}`, lookup_key: lk, key_version: "v1",
      created_at: NOW - 7200,
      expires_at: expiresAt,
    });
    const row = await post("/codes/find", { lookup_key: lk, now: NOW }) as { expires_at: number; used_at: number | null; revoked_at: number | null } | null;
    expect(row).not.toBeNull();
    expect(row?.expires_at).toBe(expiresAt);
    expect(row?.used_at ?? null).toBeNull();
    expect(row?.revoked_at ?? null).toBeNull();
  });

  it("RFC-047: used code is returned with used_at set, not excluded", async () => {
    const id = `code-used-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });
    const claim = await post("/codes/claim", { id, subject: "user-1", now: NOW }) as { changes: number };
    expect(claim.changes).toBe(1);

    const row = await post("/codes/find", { lookup_key: lk, now: NOW }) as { used_at: number | null } | null;
    expect(row).not.toBeNull();
    expect(row?.used_at).toBe(NOW);
  });

  it("RFC-047: revoked code is returned with revoked_at set, not excluded", async () => {
    const id = `code-revoked-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });
    await post("/codes/revoke", { id, now: NOW });

    const row = await post("/codes/find", { lookup_key: lk, now: NOW }) as { revoked_at: number | null } | null;
    expect(row).not.toBeNull();
    expect(row?.revoked_at).toBe(NOW);
  });

  it("claim_code: exactly one winner under concurrency (INV-5)", async () => {
    const id = `code-claim-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });

    const results = await Promise.all(
      Array.from({ length: 4 }, (_, i) =>
        post("/codes/claim", { id, subject: `user-${i}`, now: NOW })
      )
    ) as Array<{ changes: number }>;

    expect(results.filter(r => r.changes === 1).length).toBe(1);
    expect(results.filter(r => r.changes === 0).length).toBe(3);
  });

  it("RFC-047: claimed code is still findable afterward, with used_at set", async () => {
    // Inverted per RFC-047 §4.3 -- this duplicated the intent of the
    // "used code" test above under the old (exclusion) contract; kept as its
    // own test since it exercises claim_code's real UPDATE rather than
    // inserting used_at by hand.
    const id = `code-after-claim-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });
    await post("/codes/claim", { id, subject: "u1", now: NOW });
    const row = await post("/codes/find", { lookup_key: lk, now: NOW }) as { used_at: number | null } | null;
    expect(row).not.toBeNull();
    expect(row?.used_at).toBe(NOW);
  });
});

// ── D1SessionStore ────────────────────────────────────────────────────────────

describe("D1SessionStore", () => {
  beforeEach(async () => { await post("/migrate", {}); });

  it("inserts and finds an active session", async () => {
    const id = `sess-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/sessions/insert", { id, lookup_key: lk, key_version: "v1", subject: "alice", created_at: NOW, expires_at: LATER });
    const row = await post("/sessions/find", { lookup_key: lk, now: NOW }) as { id: string; subject: string } | null;
    expect(row?.id).toBe(id);
    expect(row?.subject).toBe("alice");
  });

  it("expired session not returned", async () => {
    const lk = `exp-sess-${Date.now()}`.padEnd(64, "x");
    await post("/sessions/insert", { id: `exp-${Date.now()}`, lookup_key: lk, key_version: "v1", subject: "bob", created_at: NOW - 7200, expires_at: NOW - 1 });
    const row = await post("/sessions/find", { lookup_key: lk, now: NOW });
    expect(row).toBeNull();
  });

  it("RFC-044: newly inserted session has no last_seen_at until touched", async () => {
    const id = `sess-touch-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/sessions/insert", { id, lookup_key: lk, key_version: "v1", subject: "dave", created_at: NOW, expires_at: LATER });
    const before = await post("/sessions/find", { lookup_key: lk, now: NOW }) as { last_seen_at: number | null };
    expect(before.last_seen_at ?? null).toBeNull();

    await post("/sessions/touch", { id, now: NOW + 10 });
    const after = await post("/sessions/find", { lookup_key: lk, now: NOW }) as { last_seen_at: number | null };
    expect(after.last_seen_at).toBe(NOW + 10);
  });
});

// ── D1FormTokenStore ──────────────────────────────────────────────────────────

describe("D1FormTokenStore", () => {
  beforeEach(async () => { await post("/migrate", {}); });

  it("consume: exactly one Proceed under concurrency (INV-6)", async () => {
    const lk = `tok-${Date.now()}`.padEnd(64, "x");
    await post("/tokens/insert", { lookup_key: lk, key_version: "v1", subject_kind: "anon", purpose: "logout", issued_at: NOW, expires_at: LATER });

    const results = await Promise.all(
      Array.from({ length: 4 }, () =>
        post("/tokens/consume", { lookup_key: lk, subject_kind: "anon", purpose: "logout", now: NOW })
      )
    ) as Array<{ changes: number }>;

    expect(results.filter(r => r.changes === 1).length).toBe(1);
    expect(results.filter(r => r.changes === 0).length).toBe(3);
  });

  it("second consume is a replay — changes == 0", async () => {
    const lk = `tok-replay-${Date.now()}`.padEnd(64, "x");
    await post("/tokens/insert", { lookup_key: lk, key_version: "v1", subject_kind: "anon", purpose: "logout", issued_at: NOW, expires_at: LATER });
    // First consume
    const r1 = await post("/tokens/consume", { lookup_key: lk, subject_kind: "anon", purpose: "logout", now: NOW }) as { changes: number };
    expect(r1.changes).toBe(1);
    // Second consume must be 0
    const r2 = await post("/tokens/consume", { lookup_key: lk, subject_kind: "anon", purpose: "logout", now: NOW }) as { changes: number };
    expect(r2.changes).toBe(0);
  });
});

// ── KvRateLimitStore ──────────────────────────────────────────────────────────

describe("KvRateLimitStore", () => {
  const KEY = `rl-${Date.now()}`;

  it("record_failure increments counter with TTL", async () => {
    await post("/kv/clear", { key: KEY });
    const r1 = await post("/kv/record_failure", { key: KEY }) as { count: number };
    const r2 = await post("/kv/record_failure", { key: KEY }) as { count: number };
    expect(r1.count).toBe(1);
    expect(r2.count).toBe(2);
  });

  it("clear_failures deletes counter", async () => {
    await post("/kv/record_failure", { key: KEY });
    await post("/kv/clear", { key: KEY });
    const r = await post("/kv/check", { key: KEY }) as { count: number };
    expect(r.count).toBe(0);
  });
});
