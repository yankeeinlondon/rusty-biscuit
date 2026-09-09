/**
 * Compatibility proof: run this tool's reconciliation over the preserved
 * Claudine listings and reproduce the universe and source-scan counts the
 * first-generation scripts recorded.
 *
 * Everything here is frozen at revision `9fc5151a0`: the listings under
 * `enumeration/9fc5151a0/`, and the classifier and expectations under
 * `fixtures/claudine-compat/`. Whether the *live* tree reconciles is the
 * consuming area's own gate (`test-audit reconcile`), not this suite's claim —
 * see `fixtures/claudine-compat/README.md`.
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { buildUniverse, parseMetadata } from "../src/reconcile/listings.ts";
import { parseFamilyFile, reconcileFamilies } from "../src/reconcile/families.ts";
import { scanSourcePopulation } from "../src/reconcile/sources.ts";
import { diffSourceAgainstRunner, checkExclusions } from "../src/reconcile/gate.ts";
import {
  ERA_ENUMERATION_DIR,
  ERA_FAMILIES_PATH,
  ERA_REVISION,
  ERA_WORKTREE,
  claudinePackageRoots,
  expectations,
  haveEraWorktree,
} from "./claudine-compat-inputs.ts";

const recorded = expectations();

// Scanning the live tree in these four would report real, expected drift
// against era listings rather than a scanner defect, so they read the era
// worktree and skip when it is absent. See `ERA_WORKTREE`.
const haveBaseline = haveEraWorktree();

function eraUniverse() {
  const metadata = parseMetadata(readFileSync(join(ERA_ENUMERATION_DIR, "captures.json"), "utf8"));
  return { metadata, universe: buildUniverse(ERA_ENUMERATION_DIR, metadata.captures.map((c) => c.label)) };
}

describe("Claudine compatibility proof", () => {
  it("the preserved bundle is the revision its directory name claims", () => {
    const metadata = parseMetadata(readFileSync(join(ERA_ENUMERATION_DIR, "captures.json"), "utf8"));
    expect(metadata.revision).toMatch(new RegExp(`^${ERA_REVISION}`));
    expect(recorded.revision).toMatch(new RegExp(`^${ERA_REVISION}`));
  });

  it("every shipped capture parses and declares a package kind and binary id", () => {
    const { metadata } = eraUniverse();
    for (const capture of metadata.captures) {
      const parsed = JSON.parse(readFileSync(join(ERA_ENUMERATION_DIR, `${capture.label}.json`), "utf8"));
      expect(parsed["rust-suites"]).toBeDefined();
    }
  });

  it("every shipped capture is declared in the manifest and vice versa", () => {
    expect(eraUniverse().universe.violations).toEqual([]);
  });

  it("the era families assign every era identity exactly once", () => {
    const { universe } = eraUniverse();
    const { families } = parseFamilyFile(readFileSync(ERA_FAMILIES_PATH, "utf8"));
    const { violations, counts } = reconcileFamilies(universe, families);
    expect(violations).toEqual([]);
    const assigned = [...counts.values()].reduce((a, b) => a + b, 0);
    expect(assigned).toBe(universe.identities.size);
  });

  it("universe counts per package match the old tool's recorded table", () => {
    const { universe } = eraUniverse();
    const byPackage: Record<string, number> = {};
    for (const identity of universe.identities.values()) {
      byPackage[identity.pkg] = (byPackage[identity.pkg] ?? 0) + 1;
    }
    expect(byPackage).toEqual(recorded.universe.byPackage);
    expect(universe.identities.size).toBe(recorded.universe.total);
  });

  it.skipIf(!haveBaseline)(
    "the tree-sitter scan reproduces the old lexer's per-package source counts at the captured revision",
    async () => {
      const population = await scanSourcePopulation(ERA_WORKTREE, claudinePackageRoots(ERA_WORKTREE), []);
      const counts: Record<string, number> = {};
      for (const test of population.tests) counts[test.pkg] = (counts[test.pkg] ?? 0) + 1;
      expect(counts).toEqual(recorded.sourceScan.byPackage);
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "grammar gaps surface as parse-error diagnostics without losing a test",
    async () => {
      // tree-sitter-rust 0.24 does not parse `&raw`/`raw` as an ordinary
      // identifier or `unsafe extern` blocks; each produces a local ERROR node.
      // The count assertion above proves no test is lost to them, so only the
      // kind and an upper bound on the affected files are pinned here — the
      // exact set is a grammar property, not a suite property.
      const population = await scanSourcePopulation(ERA_WORKTREE, claudinePackageRoots(ERA_WORKTREE), []);
      expect(population.diagnostics.length).toBeGreaterThan(0);
      expect(population.diagnostics.every((d) => d.kind === "parse-error")).toBe(true);
      expect(new Set(population.diagnostics.map((d) => d.file)).size).toBeLessThan(
        recorded.sourceScan.maxFilesWithParseErrors,
      );
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "every source only test at the captured revision is a declared exclusion",
    async () => {
      const { universe } = eraUniverse();
      const population = await scanSourcePopulation(ERA_WORKTREE, claudinePackageRoots(ERA_WORKTREE), []);
      expect(population.tests.length).toBeGreaterThan(7000);
      const diff = diffSourceAgainstRunner(population.tests, universe);
      const { exclusions } = parseFamilyFile(readFileSync(ERA_FAMILIES_PATH, "utf8"));
      expect(checkExclusions(diff, exclusions)).toEqual([]);
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "the captured revision has no runner identity without a source definition",
    async () => {
      const { universe } = eraUniverse();
      const population = await scanSourcePopulation(ERA_WORKTREE, claudinePackageRoots(ERA_WORKTREE), []);
      expect(diffSourceAgainstRunner(population.tests, universe).runnerOnly).toEqual([]);
    },
    30000,
  );
});
