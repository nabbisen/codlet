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

async function run(suite: string, name: string): Promise<void> {
  const res = await SELF.fetch(`http://worker/run?suite=${suite}&name=${name}`, { method: "POST" });
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
describe("D1FormTokenStore (real Rust)", () => {
  for (const name of suites.token) it(name, () => run("token", name));
});

describe("run_d1_migrations (the shipped runner, real Rust)", () => {
  // FINDING (RFC-054): the runner applies the shipped file with one
  // `D1Database::exec`, which D1 rejects. `it.fails` records that; when the
  // runner is fixed this flips and says to remove `.fails`.
  it.fails("applies the shipped migration", async () => {
    const res = await SELF.fetch("http://worker/run_d1_migrations", { method: "POST" });
    const body = (await res.json()) as { ok: boolean; error?: string };
    expect(body.error ?? "").toBe("");
    expect(body.ok).toBe(true);
  });
});
