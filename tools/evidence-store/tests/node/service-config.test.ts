// The service configuration through its public parser: the schema's own
// example is the control, then one edit per case over the field shapes, then
// the strict loader's document shapes once.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parseServiceConfig, serviceConfig, type ServiceConfig } from "../../src/config.ts";
import { ApiError } from "../../src/http.ts";

const schema = JSON.parse(readFileSync(new URL("../../api/schemas/service-config.schema.json", import.meta.url), "utf8")) as {
  examples: Record<string, unknown>[];
};
const example = (): Record<string, any> => structuredClone(schema.examples[0]!) as Record<string, any>;

function problem(document: unknown): { code: string; location: string } {
  const result = parseServiceConfig(typeof document === "string" ? document : JSON.stringify(document));
  if (result.ok) throw new Error("expected the configuration to be refused");
  const [first] = result.problems;
  return { code: first!.code, location: first!.location };
}

function accepted(document: unknown): ServiceConfig {
  const result = parseServiceConfig(JSON.stringify(document));
  if (!result.ok) throw new Error(JSON.stringify(result.problems));
  return result.value;
}

describe("parseServiceConfig", () => {
  it("accepts the schema's example document", () => {
    const config = accepted(example());
    expect(config.repositories).toEqual([{ repository_id: "873451209", owner_id: "4417321", slug: "example-owner/rusty-biscuit" }]);
    expect(config.oidc.trusted_workflows[0]).toMatchObject({ workflow: ".github/workflows/evidence-store-demo.yml", roles: ["producer", "reader"] });
    expect(config.quotas.capacity.refuse_ratio).toBe(0.8);
  });

  it("accepts empty allowlists, which deny everything rather than nothing", () => {
    const document = example();
    document["repositories"] = [];
    document["oidc"].trusted_workflows = [];
    expect(accepted(document)).toMatchObject({ repositories: [], oidc: { trusted_workflows: [] } });
    const emptyRoles = example();
    emptyRoles["oidc"].trusted_workflows[0].roles = [];
    expect(accepted(emptyRoles).oidc.trusted_workflows[0]!.roles).toEqual([]);
  });

  const cases: [string, (document: Record<string, any>) => void, string][] = [
    ["absent repositories", (d) => delete d["repositories"], "/repositories"],
    ["null repositories", (d) => (d["repositories"] = null), "/repositories"],
    ["scalar repositories", (d) => (d["repositories"] = "873451209"), "/repositories"],
    ["a repository that is not an object", (d) => (d["repositories"] = [7]), "/repositories/0"],
    ["a non-numeric repository_id", (d) => (d["repositories"][0].repository_id = "rusty"), "/repositories/0/repository_id"],
    ["a null owner_id", (d) => (d["repositories"][0].owner_id = null), "/repositories/0/owner_id"],
    ["a repository listed twice", (d) => d["repositories"].push({ ...d["repositories"][0] }), "/repositories/1/repository_id"],
    ["an unknown repository key", (d) => (d["repositories"][0].trusted = true), "/repositories/0/trusted"],
    ["a wrong issuer", (d) => (d["oidc"].issuer = "https://example.com"), "/oidc/issuer"],
    ["a wrong audience", (d) => (d["oidc"].audience = "anything"), "/oidc/audience"],
    ["a fractional clock tolerance", (d) => (d["oidc"].clock_tolerance_seconds = 1.5), "/oidc/clock_tolerance_seconds"],
    ["a clock tolerance above 60", (d) => (d["oidc"].clock_tolerance_seconds = 61), "/oidc/clock_tolerance_seconds"],
    ["null trusted_workflows", (d) => (d["oidc"].trusted_workflows = null), "/oidc/trusted_workflows"],
    ["a workflow outside .github/workflows", (d) => (d["oidc"].trusted_workflows[0].workflow = "ci.yml"), "/oidc/trusted_workflows/0/workflow"],
    ["a pull_request event", (d) => (d["oidc"].trusted_workflows[0].event_name = "pull_request"), "/oidc/trusted_workflows/0/event_name"],
    ["a ref outside refs/heads or refs/tags", (d) => (d["oidc"].trusted_workflows[0].ref = "main"), "/oidc/trusted_workflows/0/ref"],
    ["a verifier OIDC role", (d) => (d["oidc"].trusted_workflows[0].roles = ["reader", "verifier"]), "/oidc/trusted_workflows/0/roles/1"],
    ["a repeated OIDC role", (d) => (d["oidc"].trusted_workflows[0].roles = ["reader", "reader"]), "/oidc/trusted_workflows/0/roles/1"],
    ["absent quotas", (d) => delete d["quotas"], "/quotas"],
    ["a mutation rate above 60", (d) => (d["quotas"].rate_mutations_per_minute = 61), "/quotas/rate_mutations_per_minute"],
    ["a string read rate", (d) => (d["quotas"].rate_reads_per_minute = "300"), "/quotas/rate_reads_per_minute"],
    ["a refuse ratio above 0.8", (d) => (d["quotas"].capacity.refuse_ratio = 0.9), "/quotas/capacity/refuse_ratio"],
    ["a zero alert ratio", (d) => (d["quotas"].capacity.alert_ratio = 0), "/quotas/capacity/alert_ratio"],
    ["a negative allowance", (d) => (d["quotas"].capacity.d1_database_bytes = -1), "/quotas/capacity/d1_database_bytes"],
    ["an allowance above the free tier", (d) => (d["quotas"].capacity.r2_class_a_per_month = 800_001), "/quotas/capacity/r2_class_a_per_month"],
    ["another API version", (d) => (d["api_version"] = "v2"), "/api_version"],
    ["an unknown top-level key", (d) => (d["unrestricted"] = true), "/unrestricted"],
  ];
  for (const [name, edit, location] of cases) {
    it(`refuses ${name}, located at ${location}`, () => {
      const document = example();
      edit(document);
      expect(problem(document)).toEqual({ code: "invalid-config", location });
    });
  }

  it("refuses a top-level array", () => {
    expect(problem([example()])).toEqual({ code: "invalid-config", location: "" });
  });

  it("refuses the strict loader's document shapes", () => {
    const text = JSON.stringify(example());
    expect(problem(`{"api_version":"v1","api_version":"v1"}`)).toEqual({ code: "duplicate-key", location: "/api_version" });
    expect(problem(`${text} {}`).code).toBe("trailing-content");
    expect(problem("").code).toBe("empty-document");
    expect(problem(`﻿${text}`).code).toBe("byte-order-mark");
  });
});

describe("serviceConfig", () => {
  it("answers 503 for a missing or invalid binding without quoting the document", () => {
    const refusal = (text: unknown) => {
      try {
        serviceConfig({ SERVICE_CONFIG: text as string });
      } catch (error) {
        return error as ApiError;
      }
      throw new Error("expected a refusal");
    };
    const missing = refusal(undefined);
    expect(missing.code).toBe("unavailable");
    const invalid = refusal(JSON.stringify({ ...example(), secret: "do-not-echo" }));
    expect(invalid.code).toBe("unavailable");
    expect(invalid.message).toContain('"/secret"');
    expect(invalid.message).not.toContain("do-not-echo");
    expect(serviceConfig({ SERVICE_CONFIG: JSON.stringify(example()) }).api_version).toBe("v1");
  });
});
