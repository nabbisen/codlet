import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

// The JavaScript harness (worker.js): D1 binding behaviour and the concurrent
// claim route. The real Rust stores run in the `rust-d1` project
// (vitest.rust.config.ts).
export default defineWorkersConfig({
  test: {
    name: "js-harness",
    include: ["conformance.test.ts"],
    poolOptions: {
      workers: {
        wrangler: {
          configPath: "./wrangler.toml",
        },
      },
    },
  },
});
