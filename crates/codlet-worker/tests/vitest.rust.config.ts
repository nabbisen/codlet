import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

// The real Rust D1 stores, built by `worker-build` from
// crates/codlet-worker-miniflare (RFC-054 stage C).
export default defineWorkersConfig({
  test: {
    name: "rust-d1",
    include: ["rust_d1.test.ts"],
    poolOptions: {
      workers: {
        wrangler: {
          configPath: "./wrangler.rust.toml",
        },
      },
    },
  },
});
