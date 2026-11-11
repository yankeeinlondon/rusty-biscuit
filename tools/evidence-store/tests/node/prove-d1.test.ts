import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { isSpikeNotFound, missingEnvironment, parseDatabaseId, parseWorkerUrl } from "../../cli/prove-d1.ts";

describe("prove-d1 preconditions", () => {
  it("names every missing credential in declaration order", () => {
    expect(missingEnvironment({})).toEqual(["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"]);
    expect(missingEnvironment({ CLOUDFLARE_API_TOKEN: "", CLOUDFLARE_ACCOUNT_ID: "abc" })).toEqual([
      "CLOUDFLARE_API_TOKEN",
    ]);
    expect(missingEnvironment({ CLOUDFLARE_API_TOKEN: "t", CLOUDFLARE_ACCOUNT_ID: "a" })).toEqual([]);
  });

  it("exits 2 without credentials, names them, and records no result", () => {
    const script = fileURLToPath(new URL("../../cli/prove-d1.ts", import.meta.url));
    const tsx = fileURLToPath(new URL("../../node_modules/tsx/dist/cli.mjs", import.meta.url));
    const environment = { ...process.env };
    delete environment.CLOUDFLARE_API_TOKEN;
    delete environment.CLOUDFLARE_ACCOUNT_ID;
    const run = spawnSync(process.execPath, [tsx, script], { env: environment, encoding: "utf8", timeout: 30_000 });
    expect(run.status).toBe(2);
    expect(run.stderr).toContain("CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID");
    expect(run.stderr).toContain("No feasibility result was recorded");
    expect(run.stdout).toBe("");
  });
});

describe("prove-d1 Wrangler output parsing", () => {
  it("reads database_id from JSONC and TOML snippets", () => {
    const id = "0f9e8d7c-6b5a-4321-8fed-cba987654321";
    expect(parseDatabaseId(`{\n  "d1_databases": [{ "binding": "DB", "database_id": "${id}" }]\n}`)).toBe(id);
    expect(parseDatabaseId(`[[d1_databases]]\ndatabase_id = "${id}"`)).toBe(id);
    expect(parseDatabaseId("no snippet here")).toBeNull();
  });

  it("reads the workers.dev URL and nothing else", () => {
    expect(parseWorkerUrl("Deployed evidence-spike triggers\n  https://evidence-spike-1a2b.acct.workers.dev\n")).toBe(
      "https://evidence-spike-1a2b.acct.workers.dev",
    );
    expect(parseWorkerUrl("https://example.com")).toBeNull();
  });
});

describe("isSpikeNotFound", () => {
  it("accepts only the spike's JSON 404, not Cloudflare's HTML page for a workers.dev address that is not routing yet", () => {
    expect(isSpikeNotFound({ status: 404, body: { error: "not_found" } })).toBe(true);
    expect(isSpikeNotFound({ status: 404, body: "<!DOCTYPE html>\n<html class=\"no-js\">" })).toBe(false);
    expect(isSpikeNotFound({ status: 200, body: { error: "not_found" } })).toBe(false);
    expect(isSpikeNotFound(null)).toBe(false);
  });
});
