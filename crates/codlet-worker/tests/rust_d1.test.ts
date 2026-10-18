/**
 * RFC-054 stage C: the REAL Rust `D1CodeStore`, `D1SessionStore` and
 * `D1FormTokenStore` (crates/codlet-worker), driven by `codlet-conformance`'s
 * sequential suites, running inside Miniflare against a real D1 binding.
 *
 * The Worker under test is built from `crates/codlet-worker-miniflare`
 * (`npm run build:rust`). One vitest test per conformance test, so a failure
 * names the test; the panic message of a failing assertion is in the Worker's
 * console output above the failure.
 */
import { SELF } from "cloudflare:test";
import { describe, it, expect } from "vitest";

type Suites = { code: string[]; session: string[]; token: string[] };

async function get(path: string): Promise<Response> {
  return SELF.fetch(`http://worker${path}`);
}

async function run(suite: string, name: string, extra = ""): Promise<void> {
  const res = await SELF.fetch(`http://worker/run?suite=${suite}&name=${name}${extra}`, { method: "POST" });
  const body = await res.text();
  expect(res.status, `${suite}/${name} => ${res.status}: ${body}`).toBe(200);
}

// Test names are fetched at collection time, so a test added to the conformance
// crate appears here without editing this file.
const suites = (await (await get("/list")).json()) as Suites;

describe("D1CodeStore (real Rust)", () => {
  for (const name of suites.code) it(name, () => run("code", name));
});
describe("D1SessionStore (real Rust)", () => {
  for (const name of suites.session) it(name, () => run("session", name));
});
// RFC-057: the same session suite, against a database created before RFC-044 (no
// `last_seen_at`) and upgraded by the shipped `run_d1_migrations`.
describe("D1SessionStore (real Rust) on a pre-RFC-044 database upgraded by run_d1_migrations", () => {
  for (const name of suites.session) it(name, () => run("session", name, "&upgraded=1"));
});
describe("D1FormTokenStore (real Rust)", () => {
  for (const name of suites.token) it(name, () => run("token", name));
});

describe("run_d1_migrations (the shipped runner, real Rust) — RFC-057", () => {
  type Outcome = { ok: boolean; error?: string | null };
  const call = async (): Promise<Outcome> =>
    (await SELF.fetch("http://worker/run_d1_migrations", { method: "POST" })).json() as Promise<Outcome>;

  it("applies the shipped migration, called as a host calls it", async () => {
    const res = await call();
    expect(res.error ?? "").toBe("");
    expect(res.ok).toBe(true);
  });

  it("two consecutive calls both succeed (including once last_seen_at exists)", async () => {
    expect((await call()).ok).toBe(true);
    expect((await call()).ok).toBe(true);
  });

  it("adds last_seen_at to a pre-RFC-044 codlet_sessions, and stays idempotent afterwards", async () => {
    const res = (await (await SELF.fetch("http://worker/pre_rfc_044_upgrade", { method: "POST" })).json()) as {
      ok: boolean; error?: string | null; before: string[]; after: string[];
    };
    expect(res.before, "setup: the table must start without the column").not.toContain("last_seen_at");
    expect(res.error ?? "").toBe("");
    expect(res.ok).toBe(true);
    expect(res.after).toContain("last_seen_at");
    expect((await call()).ok).toBe(true); // already upgraded: no "duplicate column name"
  });
});
