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
import { parseProvenance, type Provenance } from "../junit/provenance.ts";
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
): { cells: MeasuredCell[]; violations: AttributionViolation[]; provenance: Provenance } {
  const violations: AttributionViolation[] = [];
  const cells: MeasuredCell[] = [];

  const manifestPath = join(legDir, "manifest.jsonl");
  if (!existsSync(manifestPath)) {
    violations.push({
      kind: "missing-artifact",
      detail: `${label}: manifest.jsonl absent; the tree is not a \`_stage_junit\` staging directory`,
    });
    return { cells, violations, provenance: null };
  }

  let records: ManifestRecord[];
  try {
    records = parseManifest(readFileSync(manifestPath, "utf8"), `${label}/manifest.jsonl`);
  } catch (error) {
    violations.push({ kind: "malformed-report", detail: (error as Error).message });
    return { cells, violations, provenance: null };
  }

  for (const record of records) {
    if (record.environment !== env.name) {
      violations.push({
        kind: "provenance-mismatch",
        detail: `${label}: ${record.tier}/${record.package} records environment '${record.environment}' inside the '${env.name}' tree`,
      });
    }
  }

  // Read provenance if present
  let provenance: Provenance = null;
  const provenancePath = join(legDir, "provenance.json");
  if (existsSync(provenancePath)) {
    try {
      provenance = parseProvenance(readFileSync(provenancePath, "utf8"), `${label}/provenance.json`);
    } catch (error) {
      violations.push({ kind: "malformed-report", detail: (error as Error).message });
      return { cells, violations, provenance: null };
    }
  }

  if (env.cells.length === 0) {
    violations.push({
      kind: "missing-leg",
      detail: `${label}: the leg declares no cells, so it measures nothing`,
    });
    return { cells, violations, provenance };
  }

  // Check for exact one-to-one manifest-cell matching
  const seen = new Map<string, number>();
  for (const record of records) {
    const key = `${record.tier}/${record.package}`;
    const count = seen.get(key) ?? 0;
    seen.set(key, count + 1);
    if (count > 0) {
      violations.push({
        kind: "duplicate-manifest-cell",
        detail: `${label}: manifest lists ${key} ${count + 1} times; each cell must appear exactly once`,
      });
    }
  }

  for (const record of records) {
    const key = `${record.tier}/${record.package}`;
    const expected = env.cells.some((c) => c.tier === record.tier && c.package === record.package);
    if (!expected) {
      violations.push({
        kind: "unexpected-manifest-cell",
        detail: `${label}: manifest lists ${key} but the environment declares no such cell`,
      });
    }
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

  return { cells, violations, provenance };
}

/**
 * Join stored staging trees to families and emit `perLegFamilySummed`.
 *
 * Provenance kind is derived from stamped data: a tree with CI provenance on
 * all counted runs is `ci`, otherwise `local`. All counted samples must share
 * one source revision and represent ordered consecutive runs.
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
  const allProvenance: Array<{ run: string; leg: string; provenance: Provenance }> = [];

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

      // Record provenance from this successful leg
      allProvenance.push({ run, leg: env.name, provenance: leg.provenance });

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

  // Derive provenance kind: `ci` only if all counted samples have CI provenance
  const ciProvenance = allProvenance.filter((p) => p.provenance !== null);
  const provenanceKind: "ci" | "local" = ciProvenance.length === allProvenance.length ? "ci" : "local";

  // Validate CI provenance consistency
  if (provenanceKind === "ci") {
    // All samples must share one source revision
    const shas = new Set(ciProvenance.map((p) => p.provenance!.sha));
    if (shas.size > 1) {
      violations.push({
        kind: "mixed-source-revisions",
        detail: `samples span ${shas.size} source revisions: ${[...shas].join(", ")}; all must be from one revision`,
      });
    }

    // All samples must be ordered consecutive runs
    // Group by workflow+ref, then check run numbers are consecutive
    const byWorkflowRef = new Map<string, Array<{ run: string; leg: string; runNumber: number }>>();
    for (const { run, leg, provenance } of ciProvenance) {
      const key = `${provenance!.workflow}/${provenance!.ref}`;
      const entry = byWorkflowRef.get(key) ?? [];
      entry.push({ run, leg, runNumber: provenance!.runNumber });
      byWorkflowRef.set(key, entry);
    }

    for (const [key, entries] of byWorkflowRef) {
      const sorted = [...entries].sort((a, b) => a.runNumber - b.runNumber);
      for (let i = 1; i < sorted.length; i++) {
        if (sorted[i]!.runNumber !== sorted[i - 1]!.runNumber + 1) {
          violations.push({
            kind: "non-consecutive-runs",
            detail: `${key}: run numbers ${sorted[i - 1]!.runNumber} → ${sorted[i]!.runNumber} are not consecutive`,
          });
        }
      }
    }
  }

  const runsPerLeg: Record<string, number> = {};
  for (const leg of legs) runsPerLeg[leg] = countedRuns[leg]!.length;

  return {
    budgetInput: {
      provenance: {
        kind: provenanceKind,
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
