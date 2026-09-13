/**
 * Local-measurement gate: alternating-run reports and runner.
 *
 * Reads nextest console logs joined to a run manifest, produces spread tables
 * with drift brackets, cohort changes, and per-target ten-execution evidence.
 */

import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { MalformedInput, type Violation } from "../errors.ts";
import {
  parseLog,
  readLog,
  type Invocation,
  type TestResult,
} from "../nextest-log.ts";

export interface RunRecord {
  suite: string;
  revision: string;
  sha: string;
  sequence: number;
  role: "warmup" | "alternating" | "load";
  started: string;
  wallS: number;
  exitCode: number;
  log: string;
  loadavgBefore?: string;
  loadavgAfter?: string;
  cpuIdleBefore?: string;
  [extra: string]: unknown;
}

export interface CohortSpec {
  name: string;
  binaries?: string[];
  binaryPrefixes?: string[];
  prefixes?: { binary: string; prefix: string }[];
}

export interface TargetSpec {
  binary: string;
  name?: string;
  prefix?: string;
  suffix?: string;
  contract: string;
}

export function readManifest(path: string): RunRecord[] {
  return readFileSync(path, "utf8")
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .map((line, index) => {
      const record = JSON.parse(line) as RunRecord;
      for (const key of [
        "suite",
        "revision",
        "sha",
        "sequence",
        "role",
        "log",
        "wallS",
        "exitCode",
      ]) {
        if (!(key in record)) {
          throw new MalformedInput(`manifest line ${index + 1}: missing '${key}'`);
        }
      }
      return record;
    });
}

export interface RunMetrics {
  run: RunRecord;
  invocations: Invocation[];
  elapsedS: number;
  summedS: number;
  buildSetupS: number;
  tests: number;
  passed: number;
  failed: number;
  timedOut: number;
  skipped: number;
  leaks: number;
  leakFails: number;
  retries: number;
  slowMarks: number;
  identities: Set<string>;
  durations: Map<string, number>;
  violations: string[];
}

export function metricsForRun(run: RunRecord, invocations: Invocation[]): RunMetrics {
  const violations: string[] = [];
  const identities = new Set<string>();
  const durations = new Map<string, number>();
  let elapsedS = 0;
  let summedS = 0;
  let tests = 0;
  let passed = 0;
  let failed = 0;
  let timedOut = 0;
  let skipped = 0;
  let leaks = 0;
  let leakFails = 0;
  let retries = 0;
  let slowMarks = 0;

  for (const inv of invocations) {
    elapsedS += inv.summary.elapsedS;
    tests += inv.summary.run;
    passed += inv.summary.passed;
    failed += inv.summary.failed;
    timedOut += inv.summary.timedOut;
    skipped += inv.summary.skipped;
    slowMarks += inv.slowMarks;

    if (inv.results.length !== inv.summary.run) {
      violations.push(
        `[count-mismatch] ${run.suite} ${run.revision}#${run.sequence}: ${inv.results.length} result lines against '${inv.summary.run} tests run'`
      );
    }
    if (inv.declared && inv.declared.tests !== inv.summary.run) {
      violations.push(
        `[count-mismatch] ${run.suite} ${run.revision}#${run.sequence}: 'Starting ${inv.declared.tests}' against '${inv.summary.run} tests run'`
      );
    }

    for (const result of inv.results) {
      const identity = `${result.binaryId} ${result.name}`;
      if (identities.has(identity)) {
        violations.push(
          `[duplicate-identity] ${run.suite} ${run.revision}#${run.sequence}: ${identity}`
        );
      }
      identities.add(identity);
      durations.set(identity, result.durationS);
      summedS += result.durationS;
      retries += result.attempt - 1;
      if (result.status === "LEAK") leaks += 1;
      if (result.status === "LEAK-FAIL") leakFails += 1;
      if (result.status !== "PASS" && result.status !== "LEAK") {
        violations.push(
          `[non-passing] ${run.suite} ${run.revision}#${run.sequence}: ${result.status} ${identity} (${result.durationS}s)`
        );
      }
    }
  }

  if (run.exitCode !== 0) {
    violations.push(
      `[exit-code] ${run.suite} ${run.revision}#${run.sequence}: recipe exited ${run.exitCode}`
    );
  }

  return {
    run,
    invocations,
    elapsedS,
    summedS,
    buildSetupS: run.wallS - elapsedS,
    tests,
    passed,
    failed,
    timedOut,
    skipped,
    leaks,
    leakFails,
    retries,
    slowMarks,
    identities,
    durations,
    violations,
  };
}

export function median(values: number[]): number {
  if (values.length === 0) return NaN;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? sorted[mid]!
    : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

export interface Spread {
  n: number;
  min: number;
  median: number;
  max: number;
  driftRatio: number;
}

export function spread(values: number[]): Spread {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const med = median(values);
  return {
    n: values.length,
    min,
    median: med,
    max,
    driftRatio: med === 0 ? 0 : (max - min) / med,
  };
}

/**
 * A cross-revision delta is established only when it clears both revisions'
 * own same-code spread.
 */
export function established(baseline: Spread, candidate: Spread): boolean {
  const delta = Math.abs(baseline.median - candidate.median);
  const floor = Math.max(baseline.max - baseline.min, candidate.max - candidate.min);
  return delta > floor;
}

export function matchesCohort(spec: CohortSpec, binary: string, name: string): boolean {
  if (spec.binaries?.includes(binary)) return true;
  if (spec.binaryPrefixes?.some((p) => binary.startsWith(p))) return true;
  return (spec.prefixes ?? []).some((p) => p.binary === binary && name.startsWith(p.prefix));
}

export function matchesTarget(spec: TargetSpec, binary: string, name: string): boolean {
  if (spec.binary !== binary) return false;
  if (spec.name !== undefined) return spec.name === name;
  if (spec.prefix !== undefined) return name.startsWith(spec.prefix);
  if (spec.suffix !== undefined) return name.endsWith(spec.suffix);
  return true;
}

export interface Report {
  runs: RunMetrics[];
  violations: string[];
  markdown: string;
}

function fmt(seconds: number, digits = 2): string {
  return Number.isFinite(seconds) ? seconds.toFixed(digits) : "—";
}

function pct(ratio: number): string {
  return `${(ratio * 100).toFixed(1)}%`;
}

export function buildReport(
  manifest: RunRecord[],
  cohorts: CohortSpec[],
  targets: TargetSpec[],
  manifestDir: string
): Report {
  const runs: RunMetrics[] = [];
  const violations: string[] = [];

  for (const run of manifest) {
    const logPath = resolve(manifestDir, run.log);
    if (!existsSync(logPath)) {
      violations.push(
        `[missing-log] ${run.suite} ${run.revision}#${run.sequence}: ${run.log}`
      );
      continue;
    }
    let invocations: Invocation[];
    try {
      invocations = parseLog(readLog(logPath), run.log);
    } catch (error) {
      violations.push(`[malformed-log] ${(error as Error).message}`);
      continue;
    }
    const metrics = metricsForRun(run, invocations);
    violations.push(...metrics.violations);
    runs.push(metrics);
  }

  const lines: string[] = [];
  const suites = [...new Set(runs.map((r) => r.run.suite))];
  const revisions = [...new Set(runs.map((r) => r.run.revision))];

  lines.push("### Runs");
  lines.push("");
  lines.push(
    "| Suite | Revision | Seq | Role | Wall | Build/setup | Runner elapsed | Summed | Tests | Passed | Failed | Timed out | Skipped | Leaks | Retries | Slow | Load before |"
  );
  lines.push(
    "|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|"
  );
  for (const m of runs) {
    lines.push(
      `| \`${m.run.suite}\` | ${m.run.revision} | ${m.run.sequence} | ${m.run.role} | ${fmt(m.run.wallS, 1)} s | ${fmt(m.buildSetupS, 1)} s | ${fmt(m.elapsedS)} s | ${fmt(m.summedS)} s | ${m.tests} | ${m.passed} | ${m.failed} | ${m.timedOut} | ${m.skipped} | ${m.leaks + m.leakFails} | ${m.retries} | ${m.slowMarks} | ${m.run.loadavgBefore ?? "—"} |`
    );
  }
  lines.push("");

  const counted = runs.filter((r) => r.run.role === "alternating");
  lines.push("### Per-revision spread (alternating runs only)");
  lines.push("");
  lines.push(
    "| Suite | Revision | Runs | Elapsed min / median / max | Elapsed drift | Summed min / median / max | Summed drift | Identities |"
  );
  lines.push("|---|---|---:|---|---:|---|---:|---:|");

  const spreads = new Map<
    string,
    { elapsed: Spread; summed: Spread; identities: Set<string> }
  >();

  for (const suite of suites) {
    for (const revision of revisions) {
      const group = counted.filter(
        (r) => r.run.suite === suite && r.run.revision === revision
      );
      if (group.length === 0) continue;

      const elapsed = spread(group.map((r) => r.elapsedS));
      const summed = spread(group.map((r) => r.summedS));
      const first = group[0]!.identities;

      for (const other of group.slice(1)) {
        const missing = [...first].filter((id) => !other.identities.has(id));
        const extra = [...other.identities].filter((id) => !first.has(id));
        if (missing.length > 0 || extra.length > 0) {
          violations.push(
            `[unstable-identities] ${suite} ${revision}: run #${other.run.sequence} differs from #${group[0]!.run.sequence} by −${missing.length}/+${extra.length}`
          );
        }
      }

      spreads.set(`${suite} ${revision}`, { elapsed, summed, identities: first });
      lines.push(
        `| \`${suite}\` | ${revision} | ${group.length} | ${fmt(elapsed.min)} / ${fmt(elapsed.median)} / ${fmt(elapsed.max)} s | ${pct(elapsed.driftRatio)} | ${fmt(summed.min)} / ${fmt(summed.median)} / ${fmt(summed.max)} s | ${pct(summed.driftRatio)} | ${first.size} |`
      );
    }
  }
  lines.push("");

  if (revisions.includes("baseline") && revisions.includes("candidate")) {
    lines.push(
      "### Baseline → candidate (medians; established only outside both drift brackets)"
    );
    lines.push("");
    lines.push(
      "| Suite | Column | Baseline | Candidate | Delta | Established |"
    );
    lines.push("|---|---|---:|---:|---:|---|");

    for (const suite of suites) {
      const b = spreads.get(`${suite} baseline`);
      const c = spreads.get(`${suite} candidate`);
      if (!b || !c) continue;

      for (const column of ["elapsed", "summed"] as const) {
        const bs = b[column];
        const cs = c[column];
        const delta = cs.median - bs.median;
        lines.push(
          `| \`${suite}\` | ${column} | ${fmt(bs.median)} s | ${fmt(cs.median)} s | ${delta >= 0 ? "+" : ""}${fmt(delta)} s (${pct(delta / bs.median)}) | ${established(bs, cs) ? "yes" : "**no — inside drift**"} |`
        );
      }
      const added = [...c.identities].filter((id) => !b.identities.has(id));
      const removed = [...b.identities].filter((id) => !c.identities.has(id));
      lines.push(
        `| \`${suite}\` | identities | ${b.identities.size} | ${c.identities.size} | +${added.length} / −${removed.length} | — |`
      );
    }
    lines.push("");

    lines.push(
      "#### Paired alternation (candidate ÷ baseline, same sequence number)"
    );
    lines.push("");
    lines.push(
      "| Suite | Column | Pairs | Ratio min / median / max | Every pair improved |"
    );
    lines.push("|---|---|---:|---|---|");

    for (const suite of suites) {
      const b = counted.filter(
        (r) => r.run.suite === suite && r.run.revision === "baseline"
      );
      const c = counted.filter(
        (r) => r.run.suite === suite && r.run.revision === "candidate"
      );
      const pairs = b
        .map((bm) => [bm, c.find((cm) => cm.run.sequence === bm.run.sequence)] as const)
        .filter(
          (pair): pair is readonly [RunMetrics, RunMetrics] =>
            pair[1] !== undefined
        );
      if (pairs.length === 0) continue;

      for (const column of ["elapsedS", "summedS"] as const) {
        const ratios = pairs.map(([bm, cm]) => cm[column] / bm[column]);
        const s = spread(ratios);
        lines.push(
          `| \`${suite}\` | ${column === "elapsedS" ? "elapsed" : "summed"} | ${pairs.length} | ${s.min.toFixed(3)} / ${s.median.toFixed(3)} / ${s.max.toFixed(3)} | ${s.max < 1 ? "yes" : "**no**"} |`
        );
      }
    }
    lines.push("");

    lines.push("#### Identity changes");
    lines.push("");
    for (const suite of suites) {
      const b = spreads.get(`${suite} baseline`);
      const c = spreads.get(`${suite} candidate`);
      if (!b || !c) continue;
      const added = [...c.identities].filter((id) => !b.identities.has(id)).sort();
      const removed = [...b.identities].filter((id) => !c.identities.has(id)).sort();
      lines.push(`\`${suite}\` — added ${added.length}, removed ${removed.length}`);
      lines.push("");
      for (const id of added) lines.push(`- added: \`${id}\``);
      for (const id of removed) lines.push(`- removed: \`${id}\``);
      lines.push("");
    }
  }

  if (cohorts.length > 0) {
    lines.push(
      "### Changed cohorts (from the alternating suite runs; medians of summed duration)"
    );
    lines.push("");
    lines.push(
      "| Cohort | Baseline tests | Baseline summed (min / median / max) | Candidate tests | Candidate summed (min / median / max) | Delta (median) | Established |"
    );
    lines.push("|---|---:|---|---:|---|---:|---|");

    for (const cohort of cohorts) {
      const perRun = counted.map((m) => {
        let tests = 0;
        let summed = 0;
        for (const inv of m.invocations) {
          for (const r of inv.results) {
            if (matchesCohort(cohort, r.binaryId, r.name)) {
              tests += 1;
              summed += r.durationS;
            }
          }
        }
        return { m, tests, summed };
      });

      const suitesWithCohort = new Set(
        perRun.filter((s) => s.tests > 0).map((s) => s.m.run.suite)
      );
      const per = new Map<
        string,
        { tests: number[]; summed: number[] }
      >();

      for (const { m, tests, summed } of perRun) {
        if (!suitesWithCohort.has(m.run.suite)) continue;
        const entry = per.get(m.run.revision) ?? { tests: [], summed: [] };
        entry.tests.push(tests);
        entry.summed.push(summed);
        per.set(m.run.revision, entry);
      }

      const b = per.get("baseline");
      const c = per.get("candidate");
      const bt = b ? [...new Set(b.tests)].join("/") : "—";
      const ct = c ? [...new Set(c.tests)].join("/") : "—";
      const bs = b ? spread(b.summed) : undefined;
      const cs = c ? spread(c.summed) : undefined;
      const delta = bs && cs ? cs.median - bs.median : NaN;

      lines.push(
        `| \`${cohort.name}\` | ${bt} | ${bs ? `${fmt(bs.min)} / ${fmt(bs.median)} / ${fmt(bs.max)} s` : "—"} | ${ct} | ${cs ? `${fmt(cs.min)} / ${fmt(cs.median)} / ${fmt(cs.max)} s` : "—"} | ${Number.isFinite(delta) ? `${delta >= 0 ? "+" : ""}${fmt(delta)} s (${pct(delta / (bs as Spread).median)})` : "—"} | ${bs && cs ? (established(bs, cs) ? "yes" : "no — inside drift") : "—"} |`
      );
    }
    lines.push("");
  }

  if (targets.length > 0) {
    lines.push(
      "### Changed timing / concurrency contracts (every candidate execution, all roles)"
    );
    lines.push("");
    lines.push(
      "| Identity | Contract | Executions | Min | Median | Max | Non-passing | Retries |"
    );
    lines.push("|---|---|---:|---:|---:|---:|---:|---:|");

    const candidateRuns = runs.filter((r) => r.run.revision === "candidate");
    for (const target of targets) {
      const perIdentity = new Map<
        string,
        { durations: number[]; bad: number; retries: number }
      >();

      for (const m of candidateRuns) {
        for (const inv of m.invocations) {
          for (const r of inv.results) {
            if (!matchesTarget(target, r.binaryId, r.name)) continue;
            const id = `${r.binaryId} ${r.name}`;
            const entry = perIdentity.get(id) ?? { durations: [], bad: 0, retries: 0 };
            entry.durations.push(r.durationS);
            entry.retries += r.attempt - 1;
            if (r.status !== "PASS") entry.bad += 1;
            perIdentity.set(id, entry);
          }
        }
      }

      if (perIdentity.size === 0) {
        violations.push(
          `[target-unmatched] ${target.binary} ${target.name ?? target.prefix ?? "*"}: no candidate execution`
        );
        lines.push(
          `| \`${target.binary} ${target.name ?? target.prefix ?? "*"}\` | ${target.contract} | **0** | — | — | — | — | — |`
        );
        continue;
      }

      for (const [id, entry] of [...perIdentity.entries()].sort()) {
        if (entry.durations.length < 10) {
          violations.push(
            `[target-short] ${id}: ${entry.durations.length} executions, ten required`
          );
        }
        const s = spread(entry.durations);
        lines.push(
          `| \`${id}\` | ${target.contract} | ${s.n} | ${fmt(s.min, 3)} s | ${fmt(s.median, 3)} s | ${fmt(s.max, 3)} s | ${entry.bad} | ${entry.retries} |`
        );
      }
    }
    lines.push("");
  }

  if (violations.length > 0) {
    lines.push("### Violations");
    lines.push("");
    for (const v of violations) lines.push(`- ${v}`);
    lines.push("");
  }

  return { runs, violations, markdown: lines.join("\n") };
}
