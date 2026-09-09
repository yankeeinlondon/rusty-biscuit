/**
 * The JUnit → family join that turns stored CI staging trees into the budget
 * input `deriveBudgets` consumes.
 *
 * `attribute` reads nextest console logs; CI evidence is JUnit XML in
 * `<run-id>/<env>/<tier>/<package>.xml` trees with a `manifest.jsonl`. Nothing
 * joined the two, so `perLegFamilySummed` stayed empty and every budget was
 * refused for the wrong reason. This module walks those trees, projects each
 * cell through `invocationFromJunit`, and totals summed duration per family
 * using the reconciler's own matcher — a join, not a second classifier.
 *
 * ## Notes
 *
 * One run contributes **one sample per family per leg**, and only when that
 * leg's evidence is clean: a red run, a malformed report, a missing artifact,
 * or an identity no family (or two families) claims disqualifies the leg for
 * that run rather than being averaged away. The command never decides whether
 * enough samples exist — that judgment stays in `deriveBudgets`.
 *
 * Samples inside `perLegFamilySummed[leg][family]` are positionally anonymous:
 * a family that appeared in only two of three runs holds two values, not three
 * with a hole. `deriveBudgets` reads that shortfall as `insufficient-runs`,
 * which is the intended reading — a family whose membership moved between runs
 * has no comparable worst-of-three.
 */

import { existsSync, readFileSync, statSync } from "node:fs";
import { basename, join } from "node:path";

import type { EnvironmentConfig } from "../config.ts";
import { MalformedInput } from "../errors.ts";
import { parseJunit } from "../junit/parse.ts";
import { parseManifest, type ManifestRecord } from "../junit/manifest.ts";
import type { Family } from "../reconcile/families.ts";
import type { Invocation } from "../nextest-log.ts";
import {
  attribute,
  checkInvocation,
  invocationFromJunit,
  type AttributionViolation,
  type BudgetInput,
} from "./index.ts";

/** A `<tier>/<package>.xml` cell projected onto the console-log shape. */
interface MeasuredCell {
  tier: string;
  package: string;
  invocation: Invocation;
}

/** One cell that survived every check and contributed to a leg's sample. */
export interface AggregatedCell {
  run: string;
  leg: string;
  tier: string;
  package: string;
  tests: number;
  summedSeconds: number;
}

export interface AggregateResult {
  /** Exactly the shape `deriveBudgets` reads, plus the run ids it came from. */
  budgetInput: BudgetInput & { provenance: { runs: string[] } };
  violations: AttributionViolation[];
  /** `<run>/<leg>` pairs skipped because the leg is declared `pending`. */
  pending: string[];
  cells: AggregatedCell[];
  /** Per leg, the run ids whose evidence was clean enough to count. */
  countedRuns: Record<string, string[]>;
}

export interface AggregateInput {
  /** One directory per CI run, each holding `<env>/` staging trees. */
  runDirs: string[];
  environments: EnvironmentConfig[];
  families: Family[];
  /** Provenance stamped on the output; `local` is refused by `deriveBudgets`. */
  provenanceKind?: "ci" | "local";
  headroom?: number;
}

function directoryExists(path: string): boolean {
  return existsSync(path) && statSync(path).isDirectory();
}

/**
 * Read one leg's cells into invocations, reporting anything that makes the
 * leg unusable as evidence.
 *
 * Returns `undefined` when the leg cannot be measured at all; an empty
 * violation list is the caller's signal that the leg may contribute a sample.
 */
function readLeg(
  legDir: string,
  label: string,
  env: EnvironmentConfig
): { cells: MeasuredCell[]; violations: AttributionViolation[] } {
  const violations: AttributionViolation[] = [];
  const cells: MeasuredCell[] = [];

  const manifestPath = join(legDir, "manifest.jsonl");
  if (!existsSync(manifestPath)) {
    violations.push({
      kind: "missing-artifact",
      detail: `${label}: manifest.jsonl absent; the tree is not a \`_stage_junit\` staging directory`,
    });
    return { cells, violations };
  }

  let records: ManifestRecord[];
  try {
    records = parseManifest(readFileSync(manifestPath, "utf8"), `${label}/manifest.jsonl`);
  } catch (error) {
    violations.push({ kind: "malformed-report", detail: (error as Error).message });
    return { cells, violations };
  }

  for (const record of records) {
    if (record.environment !== env.name) {
      violations.push({
        kind: "provenance-mismatch",
        detail: `${label}: ${record.tier}/${record.package} records environment '${record.environment}' inside the '${env.name}' tree`,
      });
    }
  }

  if (env.cells.length === 0) {
    violations.push({
      kind: "missing-leg",
      detail: `${label}: the leg declares no cells, so it measures nothing`,
    });
    return { cells, violations };
  }

  for (const cell of env.cells) {
    const relative = `${cell.tier}/${cell.package}.xml`;
    const record = records.find((r) => r.tier === cell.tier && r.package === cell.package);
    if (!record) {
      violations.push({
        kind: "missing-artifact",
        detail: `${label}: no manifest record for ${relative}`,
      });
      continue;
    }
    if (record.exit_code !== 0) {
      violations.push({
        kind: "failed-run",
        detail: `${label}: ${relative} exited ${record.exit_code}; a red run measures nothing`,
      });
    }
    if (!record.report_present) {
      violations.push({
        kind: "missing-artifact",
        detail: `${label}: ${relative} recorded report_present=false (nextest emitted no XML)`,
      });
      continue;
    }
    const xmlPath = join(legDir, record.xml);
    if (!existsSync(xmlPath)) {
      violations.push({
        kind: "missing-artifact",
        detail: `${label}: ${record.xml} named by the manifest is absent from the tree`,
      });
      continue;
    }
    const cellLabel = `${label}/${record.xml}`;
    try {
      const report = parseJunit(readFileSync(xmlPath, "utf8"), cellLabel);
      const invocation = invocationFromJunit(report, cellLabel);
      violations.push(...checkInvocation(invocation, cellLabel));
      cells.push({ tier: cell.tier, package: cell.package, invocation });
    } catch (error) {
      if (error instanceof MalformedInput) {
        violations.push({ kind: "malformed-report", detail: error.message });
        continue;
      }
      throw error;
    }
  }

  return { cells, violations };
}

/**
 * Join stored staging trees to families and emit `perLegFamilySummed`.
 *
 * ## Errors
 *
 * `MalformedInput` when two run directories carry the same run id — the same
 * run counted twice would inflate `runsPerLeg` without new evidence.
 */
export function aggregate(input: AggregateInput): AggregateResult {
  const violations: AttributionViolation[] = [];
  const pending: string[] = [];
  const cells: AggregatedCell[] = [];
  const perLegFamilySummed: Record<string, Record<string, number[]>> = {};
  const countedRuns: Record<string, string[]> = {};
  const runs: string[] = [];

  for (const runDir of input.runDirs) {
    const run = basename(runDir);
    if (runs.includes(run)) {
      throw new MalformedInput(`run ${run} was supplied twice; one run is one sample`);
    }
    runs.push(run);
  }

  const legs = input.environments.filter((env) => !env.pending).map((env) => env.name);
  for (const leg of legs) countedRuns[leg] = [];

  for (const runDir of input.runDirs) {
    const run = basename(runDir);
    if (!directoryExists(runDir)) {
      violations.push({
        kind: "missing-artifact",
        detail: `${run}: no such staging tree: ${runDir}`,
      });
      continue;
    }

    for (const env of input.environments) {
      const label = `${run}/${env.name}`;
      const legDir = join(runDir, env.name);
      if (!directoryExists(legDir)) {
        if (env.pending) {
          pending.push(label);
          continue;
        }
        violations.push({
          kind: "missing-leg",
          detail: `${label}: declared but absent from the staging tree`,
        });
        continue;
      }
      if (env.pending) {
        pending.push(label);
        continue;
      }

      const leg = readLeg(legDir, label, env);
      violations.push(...leg.violations);

      const attribution = attribute(
        leg.cells.map((cell) => cell.invocation),
        input.families
      );
      violations.push(...attribution.violations);

      if (leg.violations.length > 0 || attribution.violations.length > 0) continue;
      if (attribution.count === 0) {
        violations.push({
          kind: "missing-leg",
          detail: `${label}: no test result was attributed, so the leg contributes no sample`,
        });
        continue;
      }

      const families = (perLegFamilySummed[env.name] ??= {});
      for (const family of attribution.families) {
        (families[family.id] ??= []).push(family.summed);
      }
      for (const cell of leg.cells) {
        cells.push({
          run,
          leg: env.name,
          tier: cell.tier,
          package: cell.package,
          tests: cell.invocation.results.length,
          summedSeconds: cell.invocation.results.reduce((total, r) => total + r.durationS, 0),
        });
      }
      countedRuns[env.name]!.push(run);
    }
  }

  const runsPerLeg: Record<string, number> = {};
  for (const leg of legs) runsPerLeg[leg] = countedRuns[leg]!.length;

  return {
    budgetInput: {
      provenance: {
        kind: input.provenanceKind ?? "ci",
        legs,
        runsPerLeg,
        runs,
      },
      perLegFamilySummed,
      headroom: input.headroom ?? 0.25,
    },
    violations,
    pending,
    cells,
    countedRuns,
  };
}

export function renderAggregateTable(result: AggregateResult): string {
  const lines = [
    "| Run | Leg | Cell | Tests | Summed |",
    "|---|---|---|---:|---:|",
  ];
  for (const cell of result.cells) {
    lines.push(
      `| ${cell.run} | ${cell.leg} | \`${cell.tier}/${cell.package}\` | ${cell.tests} | ${cell.summedSeconds.toFixed(2)} s |`
    );
  }
  lines.push("");
  lines.push("| Leg | Runs counted | Families | Samples per family |");
  lines.push("|---|---:|---:|---|");
  for (const leg of result.budgetInput.provenance.legs) {
    const families = result.budgetInput.perLegFamilySummed[leg] ?? {};
    const counts = new Set(Object.values(families).map((samples) => samples.length));
    const spread = counts.size === 0 ? "—" : [...counts].sort((a, b) => a - b).join("/");
    lines.push(
      `| ${leg} | ${result.budgetInput.provenance.runsPerLeg[leg] ?? 0} | ${Object.keys(families).length} | ${spread} |`
    );
  }
  if (result.pending.length > 0) {
    lines.push("");
    lines.push(`pending legs (declared pending, not measured): ${result.pending.join(", ")}`);
  }
  return lines.join("\n");
}
