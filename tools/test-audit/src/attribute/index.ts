/**
 * Cost attribution over nextest run logs.
 *
 * Joins every test result line to a family declaration, reports summed/mean/max
 * duration per family, and derives per-family budgets from CI baseline runs.
 * Each gate semantic from Phase 3 is preserved: local-derived budgets are
 * refused, insufficient runs are refused, count mismatches are violations.
 */

import { readFileSync } from "node:fs";

import { MalformedInput, type Violation } from "../errors.ts";
import {
  parseLog,
  readLog,
  packageOf,
  type Invocation,
  type TerminalStatus,
  type TestResult,
} from "../nextest-log.ts";
import {
  familiesMatching,
  type Family,
} from "../reconcile/families.ts";
import { parseJunit, type Report } from "../junit/parse.ts";

export type AttributionViolationKind =
  | "truncated-log"
  | "malformed-report"
  | "count-mismatch"
  | "failed-run"
  | "duplicate-identity"
  | "unassigned-identity"
  | "double-assigned-identity"
  | "local-derived-budget"
  | "insufficient-runs"
  | "missing-leg"
  | "invalid-headroom"
  | "missing-artifact"
  | "provenance-mismatch";

export type AttributionViolation = Violation<AttributionViolationKind>;

interface Identity {
  id: string;
  pkg: string;
  binaryId: string;
  kind: "test";
  name: string;
  captures: string[];
}

function toIdentity(result: TestResult): Identity {
  return {
    id: `${result.binaryId} ${result.name}`,
    pkg: result.pkg,
    binaryId: result.binaryId,
    kind: "test",
    name: result.name,
    captures: [],
  };
}

/**
 * Verify that an invocation can serve as evidence.
 *
 * Rejects runs with count mismatches (result lines vs summary), duplicate
 * identities at the same attempt, and non-passing statuses.
 */
export function checkInvocation(
  inv: Invocation,
  label: string
): AttributionViolation[] {
  const violations: AttributionViolation[] = [];
  const decided = new Map<string, TestResult>();

  for (const result of inv.results) {
    const key = `${result.binaryId} ${result.name}`;
    const previous = decided.get(key);
    if (previous === undefined) {
      decided.set(key, result);
      continue;
    }
    if (previous.attempt === result.attempt) {
      violations.push({
        kind: "duplicate-identity",
        detail: `${label}: ${result.binaryId} ${result.name} reported twice at attempt ${result.attempt}`,
      });
      continue;
    }
    if (result.attempt > previous.attempt) decided.set(key, result);
  }

  if (decided.size !== inv.summary.run) {
    violations.push({
      kind: "count-mismatch",
      detail: `${label}: ${decided.size} decided result line(s) but the summary claims ${inv.summary.run} tests run`,
    });
  }

  for (const result of decided.values()) {
    if (result.status !== "PASS") {
      violations.push({
        kind: "failed-run",
        detail: `${label}: ${result.binaryId} ${result.name} ended ${result.status}; a red run measures nothing`,
      });
    }
  }

  return violations;
}

/** The final attempt for each identity, preserving log order. */
export function decidedResults(inv: Invocation): TestResult[] {
  const decided = new Map<string, TestResult>();
  for (const result of inv.results) {
    const key = `${result.binaryId} ${result.name}`;
    const previous = decided.get(key);
    if (previous === undefined || result.attempt > previous.attempt) {
      decided.set(key, result);
    }
  }
  return [...decided.values()];
}

export interface FamilyCost {
  id: string;
  package: string;
  count: number;
  summed: number;
  mean: number;
  max: number;
  min: number;
  slowest: { name: string; seconds: number };
  share: number;
}

export interface Attribution {
  families: FamilyCost[];
  violations: AttributionViolation[];
  summed: number;
  elapsed: number;
  count: number;
}

/**
 * Join decided results to families and total the cost of each.
 *
 * Uses the reconciler's matcher, so the two gates cannot drift on family
 * membership. An identity matched by zero or more than one family is a
 * violation and contributes to no total.
 */
export function attribute(
  invocations: Invocation[],
  families: Family[]
): Attribution {
  const violations: AttributionViolation[] = [];
  const perFamily = new Map<
    string,
    { family: Family; durations: { name: string; seconds: number }[] }
  >();
  let summed = 0;
  let elapsed = 0;
  let count = 0;

  for (const inv of invocations) {
    elapsed += inv.summary.elapsedS;
    for (const result of decidedResults(inv)) {
      const hits = familiesMatching(toIdentity(result), families);
      if (hits.length === 0) {
        violations.push({
          kind: "unassigned-identity",
          detail: `${result.binaryId} ${result.name} matches no family`,
        });
        continue;
      }
      if (hits.length > 1) {
        violations.push({
          kind: "double-assigned-identity",
          detail: `${result.binaryId} ${result.name} matches ${hits.length} families: ${hits.map((h) => h.id).join(", ")}`,
        });
        continue;
      }
      const family = hits[0]!;
      const bucket = perFamily.get(family.id) ?? { family, durations: [] };
      bucket.durations.push({
        name: `${result.binaryId} ${result.name}`,
        seconds: result.durationS,
      });
      perFamily.set(family.id, bucket);
      summed += result.durationS;
      count += 1;
    }
  }

  const costs: FamilyCost[] = [];
  for (const { family, durations } of perFamily.values()) {
    const seconds = durations.map((d) => d.seconds);
    const familySummed = seconds.reduce((a, b) => a + b, 0);
    const slowest = durations.reduce((a, b) =>
      b.seconds > a.seconds ? b : a
    );
    costs.push({
      id: family.id,
      package: family.package,
      count: durations.length,
      summed: familySummed,
      mean: familySummed / durations.length,
      max: Math.max(...seconds),
      min: Math.min(...seconds),
      slowest,
      share: summed === 0 ? 0 : familySummed / summed,
    });
  }
  costs.sort((a, b) => b.summed - a.summed);
  return { families: costs, violations, summed, elapsed, count };
}

export interface BinaryCost {
  binaryId: string;
  package: string;
  count: number;
  summed: number;
  mean: number;
  max: number;
  min: number;
}

/**
 * Total cost per test binary rather than per family.
 *
 * Nextest runs each test in its own process, so `count` is the number of times
 * setup was paid and `min` is close to the cost of minimal-work cases.
 */
export function attributeByBinary(invocations: Invocation[]): BinaryCost[] {
  const perBinary = new Map<string, number[]>();
  for (const inv of invocations) {
    for (const result of decidedResults(inv)) {
      const seconds = perBinary.get(result.binaryId) ?? [];
      seconds.push(result.durationS);
      perBinary.set(result.binaryId, seconds);
    }
  }
  const costs: BinaryCost[] = [];
  for (const [binaryId, seconds] of perBinary) {
    const summed = seconds.reduce((a, b) => a + b, 0);
    costs.push({
      binaryId,
      package: packageOf(binaryId),
      count: seconds.length,
      summed,
      mean: summed / seconds.length,
      max: Math.max(...seconds),
      min: Math.min(...seconds),
    });
  }
  costs.sort((a, b) => b.summed - a.summed);
  return costs;
}

export function renderAttributionTable(
  attribution: Attribution,
  top = 0
): string {
  const rows = top > 0 ? attribution.families.slice(0, top) : attribution.families;
  const lines = [
    "| Family | Package | Tests | Summed | Mean | Max | Share |",
    "|---|---|---:|---:|---:|---:|---:|",
  ];
  for (const family of rows) {
    lines.push(
      `| \`${family.id}\` | \`${family.package}\` | ${family.count} | ${family.summed.toFixed(2)} s | ` +
        `${family.mean.toFixed(3)} s | ${family.max.toFixed(2)} s | ${(family.share * 100).toFixed(1)}% |`
    );
  }
  lines.push("");
  lines.push(
    `attributed ${attribution.count} result(s); summed ${attribution.summed.toFixed(2)} s; ` +
      `runner elapsed ${attribution.elapsed.toFixed(2)} s`
  );
  return lines.join("\n");
}

export function renderBinaryTable(costs: BinaryCost[], top = 0): string {
  const rows = top > 0 ? costs.slice(0, top) : costs;
  const lines = [
    "| Binary | Processes | Summed | Mean | Max | Min |",
    "|---|---:|---:|---:|---:|---:|",
  ];
  for (const cost of rows) {
    lines.push(
      `| \`${cost.binaryId}\` | ${cost.count} | ${cost.summed.toFixed(2)} s | ` +
        `${cost.mean.toFixed(3)} s | ${cost.max.toFixed(3)} s | ${cost.min.toFixed(3)} s |`
    );
  }
  return lines.join("\n");
}

export interface BudgetProvenance {
  kind: "ci" | "local";
  legs: string[];
  runsPerLeg: Record<string, number>;
}

export interface BudgetInput {
  provenance: BudgetProvenance;
  perLegFamilySummed: Record<string, Record<string, number[]>>;
  headroom?: number;
}

export interface Budget {
  leg: string;
  family: string;
  observedMax: number;
  budget: number;
}

export const REQUIRED_RUNS_PER_LEG = 3;

/**
 * Derive per-family budgets, or refuse and say why.
 *
 * Phase 3's checkpoint requires that no budget be derived from a local run
 * alone and that every configured leg be represented by three consecutive green
 * runs, so `kind: 'local'`, a missing leg, and a short run count each yield a
 * violation and no budget.
 */
export function deriveBudgets(input: BudgetInput): {
  budgets: Budget[];
  violations: AttributionViolation[];
} {
  const violations: AttributionViolation[] = [];
  const headroom = input.headroom ?? 0.25;
  if (!Number.isFinite(headroom) || headroom < 0) {
    violations.push({
      kind: "invalid-headroom",
      detail: `headroom must be a finite fraction >= 0, got ${JSON.stringify(input.headroom)}`,
    });
    return { budgets: [], violations };
  }
  if (input.provenance.kind !== "ci") {
    violations.push({
      kind: "local-derived-budget",
      detail:
        "budgets are derived from the CI baseline only; local timing attributes cost and sets no target",
    });
    return { budgets: [], violations };
  }
  if (input.provenance.legs.length === 0) {
    violations.push({
      kind: "missing-leg",
      detail: "provenance declares no environment leg",
    });
    return { budgets: [], violations };
  }

  const budgets: Budget[] = [];
  for (const leg of input.provenance.legs) {
    const runs = input.provenance.runsPerLeg[leg] ?? 0;
    const families = input.perLegFamilySummed[leg];
    if (families === undefined) {
      violations.push({
        kind: "missing-leg",
        detail: `${leg}: declared but carries no measurements`,
      });
      continue;
    }
    if (runs < REQUIRED_RUNS_PER_LEG) {
      violations.push({
        kind: "insufficient-runs",
        detail: `${leg}: ${runs} green run(s); ${REQUIRED_RUNS_PER_LEG} consecutive are required`,
      });
      continue;
    }
    for (const [family, samples] of Object.entries(families)) {
      if (samples.length < REQUIRED_RUNS_PER_LEG) {
        violations.push({
          kind: "insufficient-runs",
          detail: `${leg}/${family}: ${samples.length} sample(s); ${REQUIRED_RUNS_PER_LEG} are required`,
        });
        continue;
      }
      const observedMax = Math.max(...samples);
      budgets.push({
        leg,
        family,
        observedMax,
        budget: Math.ceil(observedMax * (1 + headroom) * 100) / 100,
      });
    }
  }
  if (violations.length > 0) return { budgets: [], violations };
  return { budgets, violations };
}

export function renderBudgetTable(budgets: Budget[]): string {
  const lines = [
    "| Leg | Family | Worst observed | Budget |",
    "|---|---|---:|---:|",
  ];
  for (const budget of budgets) {
    lines.push(
      `| ${budget.leg} | \`${budget.family}\` | ${budget.observedMax.toFixed(2)} s | ${budget.budget.toFixed(2)} s |`
    );
  }
  return lines.join("\n");
}

/**
 * Project a nextest JUnit report onto the console-log invocation shape.
 *
 * This is what lets CI cells (`baseline/<run>/<env>/<tier>/<pkg>.xml`) be
 * attributed by family with the same matcher and the same gates as a local
 * console log. Skipped cases contribute no result line and are counted in the
 * summary; a case retried by nextest is reported at its final attempt
 * (`1 + retries`), so the retry disclosure survives the projection.
 * `wallSeconds` (`<testsuites time>`) is nextest's own runner elapsed for the
 * cell, the same quantity the console summary line carries.
 */
export function invocationFromJunit(report: Report, label: string): Invocation {
  const results: TestResult[] = [];
  let skipped = 0;
  let passed = 0;
  let failed = 0;
  let flaky = 0;
  for (const testCase of report.cases) {
    if (testCase.outcome === "skipped") {
      skipped += 1;
      continue;
    }
    const retries = testCase.retries ?? 0;
    if (retries > 0) flaky += 1;
    let status: TerminalStatus;
    if (testCase.outcome === "passed") {
      status = "PASS";
      passed += 1;
    } else if (testCase.outcome === "failed") {
      status = "FAIL";
      failed += 1;
    } else {
      status = "ABORT";
      failed += 1;
    }
    results.push({
      binaryId: testCase.binary,
      pkg: packageOf(testCase.binary),
      name: testCase.name,
      status,
      durationS: testCase.seconds,
      attempt: 1 + retries,
    });
  }
  if (report.declaredTests !== results.length + skipped) {
    throw new MalformedInput(
      `${label}: <testsuites tests> declares ${report.declaredTests} case(s) but ${results.length + skipped} were listed`
    );
  }
  return {
    declared: { tests: report.declaredTests, binaries: 0, skipped },
    results,
    summary: {
      run: results.length,
      passed,
      failed,
      timedOut: 0,
      skipped,
      slow: 0,
      leaky: 0,
      flaky,
      elapsedS: report.wallSeconds,
    },
    slowMarks: 0,
  };
}

/**
 * Parse all invocations from all inputs and check each one.
 *
 * A path ending in `.xml` is read as a nextest JUnit report (one invocation per
 * file); anything else goes through the console-log adapter.
 */
export function parseAndCheckLogs(
  logPaths: string[]
): {
  invocations: Invocation[];
  violations: AttributionViolation[];
} {
  const invocations: Invocation[] = [];
  const violations: AttributionViolation[] = [];
  for (const path of logPaths) {
    const label = path.replace(/\.(log(\.gz)?|xml)$/, "");
    const isReport = path.endsWith(".xml");
    try {
      const parsed = isReport
        ? [invocationFromJunit(parseJunit(readFileSync(path, "utf8"), path), label)]
        : parseLog(readLog(path), label);
      for (const inv of parsed) {
        violations.push(...checkInvocation(inv, label));
        invocations.push(inv);
      }
    } catch (error) {
      if (error instanceof MalformedInput) {
        violations.push({
          kind: isReport ? "malformed-report" : "truncated-log",
          detail: error.message,
        });
      } else {
        throw error;
      }
    }
  }
  return { invocations, violations };
}
