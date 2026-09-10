import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { AuditConfigSchema, loadConfig, validateConfig, type AuditConfig } from "../src/config.ts";
import { ConfigError } from "../src/errors.ts";

const REPO = join(import.meta.dirname, "..", "..", "..");

function minimal(): AuditConfig {
  return AuditConfigSchema.parse({
    version: 1,
    area: "alpha",
    root: "../../..",
    packages: [{ name: "alpha", path: "alpha/lib" }],
    routes: [{ id: "test", kind: "nextest", command: "just test", executes: true }],
    selections: [{ label: "lib-bare", packages: ["alpha"], features: [], routes: ["test"] }],
    cohorts: [{ id: "l1", description: "L1", population: "local", recipe: "just test", selections: ["lib-bare"] }],
    environments: [{ name: "ubuntu-latest", kind: "native", os: "linux", cells: [{ tier: "L1", package: "alpha" }] }],
  });
}

function write(config: unknown): string {
  const dir = mkdtempSync(join(tmpdir(), "test-audit-config-"));
  const path = join(dir, "audit.config.json");
  writeFileSync(path, typeof config === "string" ? config : JSON.stringify(config));
  return path;
}

describe("area configuration", () => {
  it("accepts a minimal valid configuration and applies defaults", () => {
    const config = minimal();
    expect(validateConfig(config)).toEqual([]);
    expect(config.junit.enforceTimeoutFloors).toBe(false);
    expect(config.cohorts[0]?.tiers).toEqual(["L1"]);
    expect(config.families).toBe("families.json");
  });

  it("rejects a selection that names an undeclared package or route", () => {
    const config = minimal();
    config.selections.push({ label: "ghost", packages: ["beta"], features: [], routes: ["nope"] });
    expect(validateConfig(config)).toEqual([
      "selection ghost: unknown package beta",
      "selection ghost: unknown route nope",
    ]);
  });

  it("rejects a cohort naming an undeclared selection and a cell naming an unknown package", () => {
    const config = minimal();
    config.cohorts.push({ id: "l2", description: "L2", population: "local", recipe: "just test-l2", selections: ["missing"], tiers: ["L2"] });
    config.environments[0]!.cells.push({ tier: "L2", package: "beta" });
    expect(validateConfig(config)).toEqual([
      "cohort l2: unknown selection missing",
      "environment ubuntu-latest: cell L2/beta names an unknown package",
    ]);
  });

  it("rejects duplicate ids, unexplained pending legs, executing absent routes, and exclusions on unknown legs", () => {
    const config = minimal();
    config.packages.push({ name: "alpha", path: "alpha/lib" });
    config.environments.push({ name: "wsl2-ubuntu", kind: "wsl", os: "linux", cells: [], pending: true });
    config.routes.push({ id: "nothing", kind: "absent", command: "-", executes: true });
    config.junit.platformExclusions = { "windows-latest": ["x"] };
    expect(validateConfig(config)).toEqual([
      "duplicate package: alpha",
      "environment wsl2-ubuntu: pending legs must state a reason",
      "junit.platformExclusions: unknown environment windows-latest",
      "route nothing: absent routes cannot claim to execute tests",
    ]);
  });

  it("loadConfig reports schema problems with their path and refuses invalid JSON", () => {
    expect(() => loadConfig(write("{not json"))).toThrow(ConfigError);
    const bad = { ...minimal(), version: 2 };
    expect(() => loadConfig(write(bad))).toThrow(/version/);
    const noSelections = { ...minimal(), selections: [] };
    expect(() => loadConfig(write(noSelections))).toThrow(/selections/);
  });

  it("resolves every path relative to the config file", () => {
    const path = write({ ...minimal(), root: "../repo", evidenceDir: "evidence", enumeration: "captures" });
    const resolved = loadConfig(path);
    const dir = join(path, "..");
    expect(resolved.repoRoot).toBe(join(dir, "..", "repo"));
    expect(resolved.enumerationDir).toBe(join(dir, "evidence", "captures"));
    expect(resolved.familiesPath).toBe(join(dir, "evidence", "families.json"));
    expect(resolved.sourceRoots).toEqual([{ package: "alpha", roots: [join(dir, "..", "repo", "alpha", "lib")] }]);
  });

  it.each([
    "darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json",
    "sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json",
    "claudine/fixes/2026-09-07-faster-claudine-tests/audit.config.json",
  ])("the shipped area configuration %s loads and points at the repository root", (relative) => {
    const config = loadConfig(join(REPO, relative));
    expect(config.repoRoot).toBe(REPO);
    expect(config.packages.length).toBeGreaterThan(0);
    for (const { roots } of config.sourceRoots) {
      for (const root of roots) expect(root.startsWith(REPO)).toBe(true);
    }
  });

  it("the Darkmatter configuration keeps the local and CI-selected L1 populations as separate cohorts", () => {
    const config = loadConfig(join(REPO, "darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json"));
    const local = config.cohorts.find((c) => c.id === "l1-local");
    const ci = config.cohorts.find((c) => c.id === "l1-ci");
    expect(local?.population).toBe("local");
    expect(ci?.population).toBe("ci");
    expect(ci?.env?.["BISCUIT_L1_INCLUDE_SLOW"]).toBe("1");
    expect(local?.filter).toContain("slow_");
    expect(ci?.filter).not.toContain("slow_");
    const routes = new Set(config.routes.map((r) => r.id));
    for (const id of ["check-zed", "zed-verify", "vscode-dmls", "test-browser", "test-l2"]) expect(routes.has(id)).toBe(true);
    expect(config.routes.find((r) => r.id === "vscode-dmls")?.executes).toBe(false);
  });

  it("the Sniff configuration distinguishes remote, network, and test-fixtures selections and declares the caching boundary", () => {
    const config = loadConfig(join(REPO, "sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json"));
    const features = new Map(config.selections.map((s) => [s.label, s.features]));
    expect(features.get("lib-remote")).toEqual(["remote"]);
    expect(features.get("lib-network")).toEqual(["network"]);
    expect(features.get("cli-fixtures")).toEqual(["test-fixtures"]);
    expect(features.get("cli-bare")).toEqual([]);
    expect(config.cohorts.find((c) => c.id === "sanity")?.tiers).toEqual(["sanity"]);
    expect(config.counters?.boundaries.map((b) => b.key)).toEqual(["production-caching-2026-07-22"]);
    expect(config.counters?.compatibilityKeys).toContain("platformKind");
    expect(config.environments.filter((e) => e.kind === "wsl").map((e) => e.name)).toEqual(["wsl2-ubuntu"]);
  });
});
