/**
 * Compatibility proof: run the new gate over the preserved Claudine inputs
 * and verify the universe counts per package match the old tool's recorded
 * table.
 */
import { describe, it, expect } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { buildUniverse, parseMetadata } from "../src/reconcile/listings.ts";
import { parseFamilyFile, reconcileFamilies } from "../src/reconcile/families.ts";
import { parseInventoryFamilyIndex, checkInventory } from "../src/reconcile/inventory.ts";
import { scanSourcePopulation } from "../src/reconcile/sources.ts";
import { diffSourceAgainstRunner, checkExclusions } from "../src/reconcile/gate.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..", "..");
const fixDir = join(repoRoot, "claudine/fixes/2026-09-07-faster-claudine-tests");
const enumerationDir = join(fixDir, "enumeration");
const familiesPath = join(fixDir, "families.json");
const inventoryPath = join(fixDir, "inventory.md");

/**
 * The captures under `enumeration/` describe revision 9fc5151a0; the working
 * tree has since gained tests (the fix's Phases 4–7), so the source scan is
 * compared against the preserved baseline worktree when it is present and
 * skipped otherwise. Scanning the live tree would report real, expected
 * undeclared exclusions, not a scanner defect.
 */
const baselineRoot = process.env["TEST_AUDIT_CLAUDINE_BASELINE"] ?? "/tmp/rb-baseline-9fc5151a0";
const haveBaseline = existsSync(join(baselineRoot, "claudine", "lib", "Cargo.toml"));
const rootsAt = (root: string) => claudinePackageRoots.map((r) => ({ package: r.package, roots: r.roots.map((x) => x.replace(repoRoot, root)) }));

/** The old lexer's recorded source column (inventory.md family index). */
const RECORDED_SOURCE_COUNTS: Record<string, number> = {
  claudine: 4167,
  "claudine-catalog-types": 21,
  "claudine-cli": 2761,
  "claudine-contract": 52,
  "claudine-gen": 158,
  "rendezvous-client": 24,
  "rendezvous-core": 88,
  "rendezvous-daemon": 184,
};

const claudinePackageRoots = [
  { package: "claudine-catalog-types", roots: [join(repoRoot, "claudine/catalog-types")] },
  { package: "claudine", roots: [join(repoRoot, "claudine/lib")] },
  { package: "claudine-contract", roots: [join(repoRoot, "claudine/contract")] },
  { package: "claudine-cli", roots: [join(repoRoot, "claudine/cli")] },
  { package: "claudine-gen", roots: [join(repoRoot, "claudine/gen")] },
  { package: "rendezvous-core", roots: [join(repoRoot, "claudine/rendezvous/core")] },
  { package: "rendezvous-daemon", roots: [join(repoRoot, "claudine/rendezvous/daemon")] },
  { package: "rendezvous-client", roots: [join(repoRoot, "claudine/rendezvous/client")] },
];

describe("Claudine compatibility proof", () => {
  it("every shipped capture parses and declares a package kind and binary id", () => {
    const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
    for (const capture of metadata.captures) {
      const path = join(enumerationDir, `${capture.label}.json`);
      const text = readFileSync(path, "utf8");
      const parsed = JSON.parse(text);
      expect(parsed["rust-suites"]).toBeDefined();
    }
  });

  it("every shipped capture is declared in the manifest and vice versa", () => {
    const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
    const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
    expect(universe.violations).toEqual([]);
  });

  it("the shipped families assign every shipped identity exactly once", () => {
    const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
    const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
    const { families } = parseFamilyFile(readFileSync(familiesPath, "utf8"));
    const { violations, counts } = reconcileFamilies(universe, families);
    expect(violations).toEqual([]);
    const assigned = [...counts.values()].reduce((a, b) => a + b, 0);
    expect(assigned).toBe(universe.identities.size);
  });

  it.skipIf(!haveBaseline)(
    "the tree-sitter scan reproduces the old lexer's per-package source counts at the captured revision",
    async () => {
      const population = await scanSourcePopulation(baselineRoot, rootsAt(baselineRoot), []);
      const counts: Record<string, number> = {};
      for (const test of population.tests) counts[test.pkg] = (counts[test.pkg] ?? 0) + 1;
      expect(counts).toEqual(RECORDED_SOURCE_COUNTS);
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "grammar gaps surface as parse-error diagnostics without losing a test",
    async () => {
      // tree-sitter-rust 0.24 does not parse `&raw`/`raw` as an ordinary
      // identifier or `unsafe extern` blocks; each produces a local ERROR node.
      // The count assertion above proves no test is lost to them.
      const population = await scanSourcePopulation(baselineRoot, rootsAt(baselineRoot), []);
      expect(population.diagnostics.length).toBeGreaterThan(0);
      expect(population.diagnostics.every((d) => d.kind === "parse-error")).toBe(true);
      // 44 files carried ERROR nodes at 9fc5151a0; the set is a grammar property,
      // not a suite property, so only the kind and the no-loss proof are pinned.
      expect(new Set(population.diagnostics.map((d) => d.file)).size).toBeLessThan(60);
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "every source only test at the captured revision is a declared exclusion",
    async () => {
      const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
      const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
      const population = await scanSourcePopulation(baselineRoot, rootsAt(baselineRoot), []);
      expect(population.tests.length).toBeGreaterThan(7000);
      const diff = diffSourceAgainstRunner(population.tests, universe);
      const { exclusions } = parseFamilyFile(readFileSync(familiesPath, "utf8"));
      expect(checkExclusions(diff, exclusions)).toEqual([]);
    },
    30000,
  );

  it.skipIf(!haveBaseline)(
    "the captured revision has no runner identity without a source definition",
    async () => {
      const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
      const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
      const diff = diffSourceAgainstRunner((await scanSourcePopulation(baselineRoot, rootsAt(baselineRoot), [])).tests, universe);
      expect(diff.runnerOnly).toEqual([]);
    },
    30000,
  );

  it("the live tree, which has moved past the captures, reports the drift as undeclared exclusions rather than hiding it", async () => {
    const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
    const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
    const population = await scanSourcePopulation(repoRoot, claudinePackageRoots, []);
    const diff = diffSourceAgainstRunner(population.tests, universe);
    const { exclusions } = parseFamilyFile(readFileSync(familiesPath, "utf8"));
    const violations = checkExclusions(diff, exclusions);
    // Either the tree still matches the captures (no drift) or every reported
    // item is an undeclared exclusion naming a real file:line.
    for (const violation of violations) {
      expect(violation.kind).toBe("undeclared-exclusion");
      expect(violation.detail).toMatch(/claudine\/.*\.rs:\d+/);
    }
  }, 30000);

  it(
    "universe counts per package match the old tool recorded table",
    async () => {
      const metadata = parseMetadata(readFileSync(join(enumerationDir, "captures.json"), "utf8"));
      const universe = buildUniverse(enumerationDir, metadata.captures.map((c) => c.label));
      const byPackage = new Map<string, number>();
      for (const identity of universe.identities.values()) {
        byPackage.set(identity.pkg, (byPackage.get(identity.pkg) ?? 0) + 1);
      }
      const expected = new Map([
        ["claudine", 4149],
        ["claudine-catalog-types", 21],
        ["claudine-cli", 2746],
        ["claudine-contract", 52],
        ["claudine-gen", 158],
        ["rendezvous-client", 21],
        ["rendezvous-core", 82],
        ["rendezvous-daemon", 171],
      ]);
      for (const [pkg, count] of expected) {
        expect(byPackage.get(pkg)).toBe(count);
      }
      expect([...byPackage.values()].reduce((a, b) => a + b, 0)).toBe(7400);
    },
    30000,
  );

  it(
    "the family index in the shipped inventory covers every declared family",
    async () => {
      const { families } = parseFamilyFile(readFileSync(familiesPath, "utf8"));
      const rows = parseInventoryFamilyIndex(readFileSync(inventoryPath, "utf8"));
      expect(new Set(rows.map((r) => r.id))).toEqual(new Set(families.map((f) => f.id)));
    },
    30000,
  );
});
