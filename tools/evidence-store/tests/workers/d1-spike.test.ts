// Local run of the D1 feasibility scenarios against Miniflare's SQLite-backed
// D1. It proves the spike Worker and its scenarios are correct before they are
// pointed at a deployed database; it is never the feasibility result itself.
import { applyD1Migrations } from "cloudflare:test";
import { env, exports } from "cloudflare:workers";
import { beforeAll, describe, expect, it } from "vitest";
import { runScenarios, type SpikePost } from "../../spike/d1-guard/scenarios.ts";

const post: SpikePost = async (path, body) => {
  const response = await exports.default.fetch(`https://spike.local${path}`, {
    method: "POST",
    headers: { authorization: `Bearer ${env.SPIKE_TOKEN}` },
    body: typeof body === "string" ? body : JSON.stringify(body),
  });
  return { status: response.status, body: await response.json() };
};

describe("D1 guard spike (local Miniflare)", () => {
  beforeAll(async () => {
    await applyD1Migrations(env.SPIKE_DB, env.SPIKE_MIGRATIONS);
  });

  it("passes every scenario with a reduced pair count", async () => {
    const checks = await runScenarios(post, {
      pairs: 4,
      cpuObservations: 16,
      cpuRequestBytes: 64 * 1024,
    });
    const failed = checks.filter((check) => !check.ok);
    expect(failed).toEqual([]);
    expect(checks.map((check) => check.name)).toContain(
      "rejected batch leaves no sibling, manifest, member, event, or revision",
    );
  });

  it("refuses an unauthenticated request without touching D1", async () => {
    const response = await exports.default.fetch("https://spike.local/state", {
      method: "POST",
      body: JSON.stringify({ repository_id: "x" }),
    });
    expect(response.status).toBe(401);
  });
});
