import { readFileSync } from "node:fs";

import { parseArgv, requireString, optionalString, hasFlag } from "../args.ts";
import type { Command, CommandIo } from "../command.ts";
import { loadConfig } from "../config.ts";
import { EXIT, MalformedInput, type Violation } from "../errors.ts";
import { versionBanner } from "../version.ts";
import { buildUniverse, parseMetadata } from "./listings.ts";
import { parseFamilyFile, reconcileFamilies, type Family } from "./families.ts";
import { parseInventoryFamilyIndex, checkInventory } from "./inventory.ts";
import { scanSourcePopulation } from "./sources.ts";
import { diffSourceAgainstRunner, checkExclusions } from "./gate.ts";

const USAGE = `usage: test-audit reconcile --config <audit.config.json> [--json|--markdown]

Prove the family inventory is total and non-overlapping:
  1. Rebuild the runner universe from nextest captures
  2. Match every identity against families.json
  3. Cross-check inventory.md against families and counts
  4. Diff source attributes against the runner universe
  5. Check that every source-only test is a declared exclusion

Exit 0 clean, 1 violations, 2 usage error.

Options:
  --config <path>      Audit configuration file (required)
  --enumeration <dir>  Override enumeration directory
  --families <path>    Override families.json path
  --inventory <path>   Override inventory.md path
  --json               Output violations as JSON
  --markdown           Output tables as markdown (default)
`;

export const reconcileCommand: Command = {
  name: "reconcile",
  summary: "prove the family inventory is total and non-overlapping",
  usage: USAGE,
  async run(argv: string[], io: CommandIo): Promise<number> {
    const parsed = parseArgv(argv, ["json", "markdown"]);
    const configPath = requireString(parsed, "config", USAGE);
    const config = loadConfig(configPath);

    const enumerationDir = optionalString(parsed, "enumeration") ?? config.enumerationDir;
    const familiesPath = optionalString(parsed, "families") ?? config.familiesPath;
    const inventoryPath = optionalString(parsed, "inventory") ?? config.inventoryPath;

    let labels: string[];
    try {
      const metadataPath = `${enumerationDir}/captures.json`;
      const metadata = parseMetadata(readFileSync(metadataPath, "utf8"));
      labels = metadata.captures.map((c) => c.label);
    } catch (error) {
      if (error instanceof MalformedInput) {
        io.err(`1 violation(s):`);
        io.err(`  [malformed-capture] ${error.message}`);
        io.err("GATE EXIT=1");
        return EXIT.violations;
      }
      throw error;
    }

    const universe = buildUniverse(enumerationDir, labels);
    const familyFile = parseFamilyFile(readFileSync(familiesPath, "utf8"));
    const { violations: familyViolations, counts } = reconcileFamilies(universe, familyFile.families);
    const rows = parseInventoryFamilyIndex(readFileSync(inventoryPath, "utf8"));
    const inventoryViolations = checkInventory(rows, familyFile.families, counts);
    const population = await scanSourcePopulation(config.repoRoot, config.sourceRoots, config.sourceScan.exclude);
    const source = population.tests;
    const diff = diffSourceAgainstRunner(source, universe);
    const exclusionViolations = checkExclusions(diff, familyFile.exclusions);

    const violations = [...universe.violations, ...familyViolations, ...inventoryViolations, ...exclusionViolations];

    const json = hasFlag(parsed, "json");
    if (!json) {
      io.out(renderTable(familyFile.families, counts, source, universe));
      io.out("");
      if (diff.sourceOnly.length > 0) {
        io.out(`cfg/feature exclusions (source-defined, runner never lists): ${diff.sourceOnly.reduce((a, b) => a + b.count, 0)}`);
      }
      if (diff.runnerOnly.length > 0) {
        io.out(`runner identities with no plain source definition: ${diff.runnerOnly.length}`);
      }
      if (diff.macroExpanded.length > 0) {
        io.out(`macro-expanded source tests matched by prefix (runner listing is the authority): ${diff.macroExpanded.length}`);
      }
      if (population.diagnostics.length > 0) {
        io.out(`source diagnostics (${population.diagnostics.length}):`);
        for (const d of population.diagnostics) io.out(`  [${d.kind}] ${d.file}:${d.line} ${d.detail}`);
      }
      if (universe.emptySuites.length > 0) {
        io.out(`build targets listing no test: ${universe.emptySuites.map((s) => s.binaryId).join(", ")}`);
      }
    }

    if (violations.length > 0) {
      if (json) {
        io.out(JSON.stringify({ violations }, null, 2));
      } else {
        io.err(`${violations.length} violation(s):`);
        for (const v of violations) io.err(`  [${v.kind}] ${v.detail}`);
        io.err("GATE EXIT=1");
      }
      return EXIT.violations;
    }

    if (!json) io.out("GATE EXIT=0");
    else io.out(JSON.stringify({ violations: [] }, null, 2));
    return EXIT.clean;
  },
};

function renderTable(families: Family[], counts: Map<string, number>, source: any[], universe: any): string {
  const byPackage = new Map<string, number>();
  for (const identity of universe.identities.values()) {
    byPackage.set(identity.pkg, (byPackage.get(identity.pkg) ?? 0) + 1);
  }
  const lines: string[] = [];
  lines.push("| Family | Package | Identities |");
  lines.push("|---|---|---:|");
  for (const family of families) {
    lines.push(`| \`${family.id}\` | \`${family.package}\` | ${counts.get(family.id) ?? 0} |`);
  }
  lines.push("");
  lines.push("| Package | Runner identities | Source attributes |");
  lines.push("|---|---:|---:|");
  const srcByPackage = new Map<string, number>();
  for (const t of source) srcByPackage.set(t.pkg, (srcByPackage.get(t.pkg) ?? 0) + 1);
  let runnerTotal = 0;
  let sourceTotal = 0;
  for (const pkg of [...byPackage.keys()].sort()) {
    const r = byPackage.get(pkg) ?? 0;
    const s = srcByPackage.get(pkg) ?? 0;
    runnerTotal += r;
    sourceTotal += s;
    lines.push(`| \`${pkg}\` | ${r} | ${s} |`);
  }
  lines.push(`| **total** | **${runnerTotal}** | **${sourceTotal}** |`);
  return lines.join("\n");
}
