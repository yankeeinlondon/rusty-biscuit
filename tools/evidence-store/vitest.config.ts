import { cloudflareTest, readD1Migrations } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

const TEST_SERVICE_CONFIG = {
  api_version: "v1",
  repositories: [],
  oidc: {
    issuer: "https://token.actions.githubusercontent.com",
    audience: "rusty-biscuit-evidence-store",
    clock_tolerance_seconds: 60,
    trusted_workflows: [],
  },
  quotas: {
    rate_mutations_per_minute: 60,
    rate_reads_per_minute: 300,
    operator_rate_mutations_per_minute: 30,
    capacity: {
      alert_ratio: 0.7,
      refuse_ratio: 0.8,
      r2_live_plus_staging_bytes: 8589934592,
      r2_class_a_per_month: 800000,
      r2_class_b_per_month: 8000000,
      d1_database_bytes: 500000000,
      d1_rows_read_per_day: 5000000,
      d1_rows_written_per_day: 100000,
      worker_requests_per_day: 100000,
    },
  },
};

// Three projects (plus the opt-in `real` below): `node` runs tooling (CLI, fixtures, API documents) under
// Node; `workers` runs the D1 feasibility spike inside workerd through
// Miniflare; `service` runs the evidence-store Worker and its `src/db` layer
// inside workerd against a local D1 built from `migrations/`. Local D1 is
// SQLite: it proves code paths, never the deployed-D1 feasibility checkpoint.
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
      {
        plugins: [
          cloudflareTest(async () => ({
            main: "./src/worker.ts",
            miniflare: {
              compatibilityDate: "2026-11-01",
              compatibilityFlags: ["nodejs_compat"],
              d1Databases: ["EVIDENCE_DB"],
              r2Buckets: ["EVIDENCE_BUCKET"],
              // Generous limits: rate refusal is tested with an injected limiter.
              ratelimits: {
                READ_LIMITER: { namespace_id: "1001", simple: { limit: 100_000, period: 60 } },
                MUTATION_LIMITER: { namespace_id: "1002", simple: { limit: 100_000, period: 60 } },
                OPERATOR_LIMITER: { namespace_id: "1003", simple: { limit: 100_000, period: 60 } },
              },
              bindings: {
                CURSOR_HMAC_KEY: "local-test-cursor-key",
                // A valid configuration with empty allowlists: it denies every
                // repository and OIDC token. Tests that need grants pass their own.
                SERVICE_CONFIG: JSON.stringify(TEST_SERVICE_CONFIG),
                TEST_MIGRATIONS: await readD1Migrations("./migrations"),
              },
            },
          })),
        ],
        test: {
          name: "service",
          include: ["tests/service/**/*.test.ts"],
          setupFiles: ["./tests/service/apply-migrations.ts"],
        },
      },
      // `real` sends the acceptance cases to a deployed Worker (`just
      // test-real`). It exists only when asked for, so `vitest run` never
      // contacts a deployment.
      ...(process.env["EVIDENCE_STORE_REAL"] === "1"
        ? [
            {
              test: {
                name: "real",
                include: ["tests/real/**/*.test.ts"],
                environment: "node",
                testTimeout: 600_000,
                hookTimeout: 60_000,
              },
            },
          ]
        : []),
    ],
  },
});
