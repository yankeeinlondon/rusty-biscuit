import { cloudflareTest, readD1Migrations } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

// Two projects: `node` runs tooling (CLI, fixtures, API documents) under
// Node, `workers` runs Worker code inside workerd through Miniflare, with a
// local D1 binding. Local D1 is SQLite: it proves code paths, never the
// deployed-D1 feasibility checkpoint.
export default defineConfig({
  test: {
    testTimeout: 30_000,
    projects: [
      {
        test: {
          name: "node",
          include: ["tests/node/**/*.test.ts"],
          environment: "node",
        },
      },
      {
        plugins: [
          cloudflareTest(async () => ({
            main: "./spike/d1-guard/worker.ts",
            miniflare: {
              compatibilityDate: "2026-11-01",
              compatibilityFlags: ["nodejs_compat"],
              d1Databases: ["SPIKE_DB"],
              bindings: {
                SPIKE_TOKEN: "local-spike-token",
                SPIKE_MIGRATIONS: await readD1Migrations("./spike/d1-guard"),
              },
            },
          })),
        ],
        test: {
          name: "workers",
          include: ["tests/workers/**/*.test.ts"],
        },
      },
    ],
  },
});
