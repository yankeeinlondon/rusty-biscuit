/**
 * Markdown table and JSON rendering for JUnit gate results.
 */
import { versionBanner } from "../version.ts";
import type { JunitExpectations } from "../config.ts";
import type { CellMeasurement, CollectionResult, PlatformExclusion } from "./collect.ts";
import type { EnvironmentComparison } from "./compare.ts";

const fixed = (n: number) => n.toFixed(1);

export function renderTable(result: CollectionResult): string {
  const lines: string[] = [];
  lines.push(`### ${result.label}`, "");
  lines.push(
    "| Environment | Tier | Package | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips | Retries |",
  );
  lines.push("|---|---|---|---:|---:|---:|---:|---:|---:|---:|");
  for (const m of result.measurements) {
    lines.push(
      `| \`${m.environment}\` | ${m.tier} | \`${m.package}\` | ${fixed(m.buildSeconds)} s | ` +
        `${fixed(m.runnerSeconds)} s | ${fixed(m.summedSeconds)} s | ${m.tests} | ${m.failures} | ${m.skips} | ${m.retries} |`,
    );
  }
  for (const env of result.pending) {
    lines.push(`| \`${env}\` | — | — | pending | pending | pending | — | — | — | — |`);
  }
  return lines.join("\n");
}

export function renderTimeoutFloors(
  result: CollectionResult,
  expectations: JunitExpectations,
  environmentKinds: Map<string, "native" | "wsl">,
): string {
  const floors = Object.entries(expectations.timeoutFloors);
  if (floors.length === 0) return "";

  const lines: string[] = [];
  lines.push("", "### Timeout-shaped tests", "");
  lines.push("| Environment | Test | Observed | Bound | Verdict |");
  lines.push("|---|---|---:|---:|---|");

  for (const [env, cases] of Array.from(result.casesByEnvironment)) {
    const kind = environmentKinds.get(env) ?? "native";
    const allowance = kind === "wsl" ? 2 : 1;
    for (const [name, floor] of floors) {
      const found = cases.find((c) => c.name === name);
      const bound = floor.budget + floor.tick + allowance;
      if (!found) {
        const excluded = result.exclusions.some(
          (e) => e.environment === env && (e.test === name || e.test.endsWith(`::${name}`)),
        );
        const verdict = excluded ? "excluded" : "ABSENT";
        lines.push(`| \`${env}\` | \`${name}\` | absent | ${fixed(bound)} s | ${verdict} |`);
        continue;
      }
      const verdict = found.seconds <= bound ? "ok" : "MISS";
      lines.push(`| \`${env}\` | \`${name}\` | ${fixed(found.seconds)} s | ${fixed(bound)} s | ${verdict} |`);
    }
  }
  return lines.join("\n");
}

export function renderExclusions(exclusions: PlatformExclusion[]): string {
  if (exclusions.length === 0) return "";
  const lines = ["", "### Platform exclusions (declared, and absent as declared)", ""];
  lines.push("| Environment | Test |", "|---|---|");
  for (const e of exclusions) {
    lines.push(`| \`${e.environment}\` | \`${e.test}\` |`);
  }
  return lines.join("\n");
}

export function renderComparison(comparisons: EnvironmentComparison[]): string {
  const lines = ["", "### Matched tests, each environment against its own baseline", ""];
  if (comparisons.length === 0) {
    lines.push("No environment is present in both sets.");
    return lines.join("\n");
  }
  lines.push(
    "| Environment | Matched | Baseline summed | Candidate summed | Ratio | Added | Removed |",
    "|---|---:|---:|---:|---:|---:|---:|",
  );
  for (const c of comparisons) {
    const ratio =
      c.baselineMatchedSeconds > 0 ? (c.candidateMatchedSeconds / c.baselineMatchedSeconds).toFixed(3) : "—";
    lines.push(
      `| \`${c.environment}\` | ${c.matched} | ${fixed(c.baselineMatchedSeconds)} s | ` +
        `${fixed(c.candidateMatchedSeconds)} s | ${ratio} | ${c.added.length} ` +
        `(${fixed(c.addedSeconds)} s) | ${c.removed.length} (${fixed(c.removedSeconds)} s) |`,
    );
  }
  for (const c of comparisons) {
    if (c.added.length === 0 && c.removed.length === 0) continue;
    lines.push("", `#### \`${c.environment}\``, "");
    for (const identity of c.added) lines.push(`- added \`${identity}\``);
    for (const identity of c.removed) lines.push(`- removed \`${identity}\``);
  }
  return lines.join("\n");
}

export function renderViolations(result: CollectionResult): string {
  if (result.violations.length === 0) return "";
  const lines = ["", `${result.violations.length} violation(s) in ${result.label}:`];
  for (const v of result.violations) {
    lines.push(`  [${v.kind}] ${v.environment}: ${v.detail}`);
  }
  return lines.join("\n");
}

export function renderNote(): string {
  return [
    "",
    "**Note:** Build/setup, runner elapsed, and summed duration are three separate columns:",
    "- **Build/setup**: manifest `duration_s` minus runner time (compile + setup overhead)",
    "- **Runner elapsed**: nextest wall time for the run (from `<testsuites time>`)",
    "- **Summed duration**: Σ of all test case times (exceeds elapsed under parallelism)",
    "",
    "The summed column is comparable across runner core counts; the others are not.",
  ].join("\n");
}

interface JsonOutput {
  tool: string;
  label: string;
  measurements: CellMeasurement[];
  pending: string[];
  exclusions: PlatformExclusion[];
  violations: Array<{ environment: string; kind: string; detail: string }>;
  baseline?: {
    label: string;
    measurements: CellMeasurement[];
    pending: string[];
    exclusions: PlatformExclusion[];
    violations: Array<{ environment: string; kind: string; detail: string }>;
  } | undefined;
  comparison?: EnvironmentComparison[] | undefined;
}

export function renderJson(
  candidate: CollectionResult,
  baseline: CollectionResult | undefined,
  comparisons: EnvironmentComparison[] | undefined,
): string {
  const summary = (r: CollectionResult) => ({
    label: r.label,
    measurements: r.measurements,
    pending: r.pending,
    exclusions: r.exclusions,
    violations: r.violations,
  });

  const output: JsonOutput = {
    tool: versionBanner(),
    ...summary(candidate),
    ...(baseline ? { baseline: summary(baseline), comparison: comparisons } : {}),
  };

  return JSON.stringify(output, null, 2);
}
