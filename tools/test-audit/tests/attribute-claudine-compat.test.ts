/**
 * Compatibility proof: replay Claudine's recorded gate logs through this tool
 * and reproduce the numbers the first-generation scripts published.
 *
 * Every side of the replay is frozen at revision `9fc5151a0` — the logs in the
 * consuming fix directory, and the classifier and expectations under
 * `fixtures/claudine-compat/`. See that directory's README for why the live
 * `families.json` and `attribution.md` are deliberately not read here.
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";

import { parseFamilyFile } from "../src/reconcile/families.ts";
import { parseAndCheckLogs, attribute } from "../src/attribute/index.ts";
import { ERA_FAMILIES_PATH, ERA_LOG_PATHS, expectations } from "./claudine-compat-inputs.ts";

const recorded = expectations().attribution;

function replay() {
  const families = parseFamilyFile(readFileSync(ERA_FAMILIES_PATH, "utf8")).families;
  const { invocations, violations } = parseAndCheckLogs(ERA_LOG_PATHS);
  return { families, invocations, parseViolations: violations, result: attribute(invocations, families) };
}

describe("Claudine attribution compatibility", () => {
  it("reproduces the recorded family attribution counts and totals", () => {
    const { result, parseViolations } = replay();
    expect(parseViolations).toHaveLength(0);
    expect(result.violations).toHaveLength(0);

    expect(result.count).toBe(recorded.count);
    expect(result.summed).toBeCloseTo(recorded.summed, 1);
    expect(result.elapsed).toBeCloseTo(recorded.elapsed, 1);
    expect(result.families[0]!.id).toBe(recorded.topFamily);
  });

  it("family counts match the recorded family index family for family", () => {
    const { result } = replay();
    const replayed = new Map(result.families.filter((f) => f.count > 0).map((f) => [f.id, f.count]));
    expect(Object.fromEntries([...replayed].sort())).toEqual(
      Object.fromEntries(Object.entries(recorded.familyCounts).sort()),
    );
  });

  it("exits clean with no violations from the recorded logs", () => {
    const { result, parseViolations } = replay();
    const allViolations = [...parseViolations, ...result.violations];
    expect(allViolations.map((v) => `[${v.kind}] ${v.detail}`)).toEqual([]);
  });
});
