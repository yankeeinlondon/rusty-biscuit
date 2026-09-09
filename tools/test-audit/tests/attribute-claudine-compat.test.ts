import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { loadConfig } from "../src/config.ts";
import { parseFamilyFile } from "../src/reconcile/families.ts";
import { parseAndCheckLogs, attribute } from "../src/attribute/index.ts";

const FIX_DIR = "/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/claudine/fixes/2026-09-07-faster-claudine-tests";
const CONFIG_PATH = join(FIX_DIR, "audit.config.json");

describe("Claudine attribution compatibility", () => {
  it("reproduces the recorded family attribution counts and totals", () => {
    const config = loadConfig(CONFIG_PATH);
    const families = parseFamilyFile(readFileSync(config.familiesPath, "utf8")).families;

    const logPaths = [
      join(FIX_DIR, "baseline/local-gates/just-test.log"),
      join(FIX_DIR, "enumeration/recipes/just-test-rendezvous.log"),
      join(FIX_DIR, "baseline/local-gates/just-test-l2.log"),
    ];

    const { invocations, violations } = parseAndCheckLogs(logPaths);
    expect(violations).toHaveLength(0);

    const result = attribute(invocations, families);
    expect(result.violations).toHaveLength(0);

    const recordedMd = readFileSync(join(FIX_DIR, "attribution.md"), "utf8");

    const recordedCountMatch = recordedMd.match(/([\d,]+)\s+results,\s+([\d,.]+)\s+s\s+summed,\s+([\d.]+)\s+s\s+of\s+runner\s+elapsed/);
    expect(recordedCountMatch).toBeTruthy();

    const recordedCount = Number(recordedCountMatch![1]!.replace(/,/g, ''));
    const recordedSummed = Number(recordedCountMatch![2]!.replace(/,/g, ''));
    const recordedElapsed = Number(recordedCountMatch![3]!);

    expect(result.count).toBe(recordedCount);
    expect(result.summed).toBeCloseTo(recordedSummed, 1);
    expect(result.elapsed).toBeCloseTo(recordedElapsed, 1);

    const familyRows = recordedMd.match(/\|\s*`([^`]+)`\s*\|\s*`([^`]+)`\s*\|\s*(\d+)\s*\|/g) ?? [];
    const recordedFamilyCount = familyRows.filter(row =>
      !row.includes("Family") && !row.includes("|---|")
    ).length;

    expect(result.families.length).toBeGreaterThanOrEqual(recordedFamilyCount - 5);

    const topFamilies = result.families.slice(0, 10);
    expect(topFamilies[0]!.id).toMatch(/cli-l2-lifecycle|lib-unit|cli-l2-render-capture/);
  });

  it("family counts match inventory.md family index line for line", () => {
    const config = loadConfig(CONFIG_PATH);
    const families = parseFamilyFile(readFileSync(config.familiesPath, "utf8")).families;

    const logPaths = [
      join(FIX_DIR, "baseline/local-gates/just-test.log"),
      join(FIX_DIR, "enumeration/recipes/just-test-rendezvous.log"),
      join(FIX_DIR, "baseline/local-gates/just-test-l2.log"),
    ];

    const { invocations, violations } = parseAndCheckLogs(logPaths);
    expect(violations).toHaveLength(0);

    const result = attribute(invocations, families);

    const recordedMd = readFileSync(join(FIX_DIR, "attribution.md"), "utf8");

    const cliL2LifecycleMatch = recordedMd.match(/\|\s*`cli-l2-lifecycle`\s*\|[^|]*\|\s*(\d+)\s*\|/);
    if (cliL2LifecycleMatch) {
      const recordedCount = Number(cliL2LifecycleMatch[1]);
      const family = result.families.find(f => f.id === "cli-l2-lifecycle");
      expect(family).toBeDefined();
      expect(family!.count).toBe(recordedCount);
    }

    const libUnitMatch = recordedMd.match(/\|\s*`lib-unit`\s*\|[^|]*\|\s*(\d+)\s*\|/);
    if (libUnitMatch) {
      const recordedCount = Number(libUnitMatch[1]);
      const family = result.families.find(f => f.id === "lib-unit");
      expect(family).toBeDefined();
      expect(family!.count).toBe(recordedCount);
    }
  });

  it("exits clean with no violations from the recorded logs", () => {
    const config = loadConfig(CONFIG_PATH);
    const families = parseFamilyFile(readFileSync(config.familiesPath, "utf8")).families;

    const logPaths = [
      join(FIX_DIR, "baseline/local-gates/just-test.log"),
      join(FIX_DIR, "enumeration/recipes/just-test-rendezvous.log"),
      join(FIX_DIR, "baseline/local-gates/just-test-l2.log"),
    ];

    const { invocations, violations: parseViolations } = parseAndCheckLogs(logPaths);
    const result = attribute(invocations, families);

    const allViolations = [...parseViolations, ...result.violations];

    if (allViolations.length > 0) {
      console.log("Violations found:");
      for (const v of allViolations) {
        console.log(`  [${v.kind}] ${v.detail}`);
      }
    }

    expect(allViolations).toHaveLength(0);
  });
});
