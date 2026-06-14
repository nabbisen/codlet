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

  it("expired code not returned — REAL timestamp comparison (D1Type::Real)", async () => {
    const lk = `exp-${Date.now()}`.padEnd(64, "x");
    await post("/codes/insert", {
      id: `exp-${Date.now()}`, lookup_key: lk, key_version: "v1",
      created_at: NOW - 7200,
      expires_at: NOW - 1,   // stored as REAL (f64) per RFC-033 §6
    });
    const row = await post("/codes/find", { lookup_key: lk, now: NOW });
    expect(row).toBeNull();
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

  it("claimed code is not findable afterward", async () => {
    const id = `code-after-claim-${Date.now()}`;
    const lk = id.padEnd(64, "x");
    await post("/codes/insert", { id, lookup_key: lk, key_version: "v1", created_at: NOW, expires_at: LATER });
    await post("/codes/claim", { id, subject: "u1", now: NOW });
    const row = await post("/codes/find", { lookup_key: lk, now: NOW });
    expect(row).toBeNull();
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
