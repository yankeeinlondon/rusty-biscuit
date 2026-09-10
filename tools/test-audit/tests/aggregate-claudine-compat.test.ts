/**
 * Passive corpus proof: the aggregator run against the real shipped
 * `baseline/34173378609/` staging tree.
 *
 * Nothing here is a fixture. The per-leg totals are asserted against the
 * numbers already published in `results.md` § CI baseline, so a change that
 * silently altered the join would have to also falsify the recorded evidence.
 *
 * The classifier is the frozen `fixtures/claudine-compat/families.json` rather
 * than the consuming fix's live one: both the staging tree and the recorded
 * totals are fixed, and judging fixed evidence by a moving classifier is what
 * made this suite red the last time a consumer renamed a family.
 */
import { describe, it, expect } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { loadConfig } from "../src/config.ts";
import { parseFamilyFile } from "../src/reconcile/families.ts";
import { aggregate } from "../src/attribute/aggregate.ts";
import { deriveBudgets } from "../src/attribute/index.ts";
import { ERA_FAMILIES_PATH, FIX_DIR } from "./claudine-compat-inputs.ts";

const CONFIG_PATH = join(FIX_DIR, "audit.config.json");
const BASELINE_RUN = join(FIX_DIR, "baseline/34173378609");

/** Summed test duration per leg, as `results.md` § CI baseline records it. */
const RECORDED_SUMMED: Record<string, number> = {
  "ubuntu-latest": 324.7,
  "macos-latest": 753.5,
  "windows-latest": 355.8,
  "wsl2-ubuntu": 692.2,
};

/** Identity counts per leg, as `baseline/README.md` § Platform exclusions records them. */
const RECORDED_TESTS: Record<string, number> = {
  "ubuntu-latest": 2466,
  "macos-latest": 2466,
  "windows-latest": 2105,
  "wsl2-ubuntu": 2466,
};

function eraFamilies() {
  return parseFamilyFile(readFileSync(ERA_FAMILIES_PATH, "utf8")).families;
}

function aggregateRealBaseline() {
  return aggregate({
    runDirs: [BASELINE_RUN],
    environments: loadConfig(CONFIG_PATH).environments,
    families: eraFamilies(),
  });
}

describe("Claudine JUnit → family aggregation over the shipped baseline", () => {
  it("has the shipped baseline tree to read", () => {
    expect(existsSync(BASELINE_RUN)).toBe(true);
  });

  it("joins every stored identity to exactly one family on all four legs", () => {
    const result = aggregateRealBaseline();
    expect(result.violations).toEqual([]);
    expect(result.pending).toEqual([]);
    expect(result.budgetInput.provenance.legs).toEqual([
      "ubuntu-latest",
      "macos-latest",
      "windows-latest",
      "wsl2-ubuntu",
    ]);
  });

  it("reproduces the summed duration and identity count recorded for each leg", () => {
    const result = aggregateRealBaseline();
    for (const cell of result.cells) {
      expect(cell.tier).toBe("L1");
      expect(cell.package).toBe("claudine-cli");
      expect(cell.tests).toBe(RECORDED_TESTS[cell.leg]);
      expect(cell.summedSeconds).toBeCloseTo(RECORDED_SUMMED[cell.leg]!, 1);
    }
    expect(result.cells).toHaveLength(4);
  });

  it("totals each leg's family samples back to that leg's summed duration", () => {
    const result = aggregateRealBaseline();
    for (const [leg, families] of Object.entries(result.budgetInput.perLegFamilySummed)) {
      const total = Object.values(families).reduce((sum, samples) => sum + samples[0]!, 0);
      expect(total).toBeCloseTo(RECORDED_SUMMED[leg]!, 1);
    }
  });

  it("carries families the classifier declares but the stored run never exercised", () => {
    // The stored run is one L1 cell per leg; the classifier covers every tier
    // and package in the area. So most declared families contribute no sample,
    // and the join has to stay total anyway — a declared-but-absent family is
    // not a violation, only an unassigned or twice-assigned identity is.
    const declared = new Set(eraFamilies().map((f) => f.id));
    const result = aggregateRealBaseline();
    const sampled = new Set(
      Object.values(result.budgetInput.perLegFamilySummed).flatMap((leg) => Object.keys(leg)),
    );
    expect(sampled.size).toBeGreaterThan(0);
    expect([...sampled].every((id) => declared.has(id))).toBe(true);
    expect(declared.size).toBeGreaterThan(sampled.size);
    expect(result.violations).toEqual([]);
  });

  it("reports Windows with fewer families than the Unix legs, which are identical", () => {
    const perLeg = aggregateRealBaseline().budgetInput.perLegFamilySummed;
    const unix = ["ubuntu-latest", "macos-latest", "wsl2-ubuntu"].map((leg) =>
      Object.keys(perLeg[leg] ?? {}).sort()
    );
    expect(unix[1]).toEqual(unix[0]);
    expect(unix[2]).toEqual(unix[0]);
    expect(Object.keys(perLeg["windows-latest"] ?? {}).length).toBeLessThan(unix[0]!.length);
  });

  it("produces one sample per leg, and deriveBudgets still refuses for want of three", () => {
    const result = aggregateRealBaseline();
    expect(result.budgetInput.provenance.runsPerLeg).toEqual({
      "ubuntu-latest": 1,
      "macos-latest": 1,
      "windows-latest": 1,
      "wsl2-ubuntu": 1,
    });

    // The historical baseline has no provenance.json (created before that feature),
    // so it's classified as 'local' and refused by deriveBudgets before checking run count.
    expect(result.budgetInput.provenance.kind).toBe("local");
    const { budgets, violations } = deriveBudgets(result.budgetInput);
    expect(budgets).toEqual([]);
    expect(violations.map((v) => v.kind)).toEqual(["local-derived-budget"]);
  });

  it("still refuses when the stored supplementary pull_request run is added", () => {
    // `34159725015` is the same tree on a `pull_request` event; it is stored as
    // a noise bracket and counts toward nothing. Two runs is still not three.
    const result = aggregate({
      runDirs: [BASELINE_RUN, join(FIX_DIR, "baseline/34159725015")],
      environments: loadConfig(CONFIG_PATH).environments,
      families: eraFamilies(),
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.runs).toEqual(["34173378609", "34159725015"]);
    for (const samples of Object.values(result.budgetInput.perLegFamilySummed)) {
      for (const values of Object.values(samples)) expect(values.length).toBeLessThanOrEqual(2);
    }
    expect(deriveBudgets(result.budgetInput).budgets).toEqual([]);
  });
});
