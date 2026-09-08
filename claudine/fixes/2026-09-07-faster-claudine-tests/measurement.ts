#!/usr/bin/env -S npx tsx
/**
 * Phase 8 local-measurement gate (fix 2026-09-07-faster-claudine-tests).
 *
 * Reads the nextest console logs the area recipes emit (`just test`,
 * `just test-rendezvous`, `just _test_l2 …`), joined to the run manifest the
 * alternating runner writes (`measurement/runs.jsonl`), and produces the
 * tables Phase 8 owes:
 *
 * - one row per run with the three costs kept apart — build/setup
 *   (`wall − elapsed`), runner elapsed (nextest's `Summary [ … ]`), and summed
 *   test duration (Σ per-test times) — beside identities, counts, failures,
 *   skips, timeouts, leaks, retries and slow marks;
 * - per-revision spread, and the drift bracket a cross-revision delta has to
 *   clear before it counts as established;
 * - the changed cohorts, extracted from those same suite reports rather than
 *   from isolated runs;
 * - the ten-execution table for every changed timing/concurrency contract.
 *
 * It is a gate, not a formatter: it exits 1 when a log is truncated, a run's
 * result lines disagree with its own summary, a revision's identity set is
 * not stable across its runs, or any run recorded a non-passing result — a
 * table of speedups must never be built over lost coverage.
 *
 * Usage:
 *   npx tsx measurement.ts report --manifest measurement/runs.jsonl \
 *       --cohorts measurement/cohorts.json --targets measurement/targets.json \
 *       [--markdown] [--json]
 *   npx tsx measurement.ts parse <log>...        # per-invocation summary
 *
 * Exit codes: 0 clean; 1 one or more violations; 2 usage error.
 */
import { readFileSync, existsSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { dirname, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Log parsing
// ---------------------------------------------------------------------------

export type Status =
  | "PASS"
  | "FAIL"
  | "TIMEOUT"
  | "LEAK"
  | "LEAK-FAIL"
  | "ABORT"
  | "SIGSEGV"
  | "SIGABRT";

export interface TestResult {
  binary: string;
  name: string;
  status: Status;
  durationS: number;
  /** Number of `TRY n` lines that preceded the final line (0 without retries). */
  retries: number;
}

export interface Invocation {
  /** `Starting N tests across M binaries (K tests skipped)`. */
  declared: { tests: number; binaries: number; skipped: number };
  results: TestResult[];
  /** From the `Summary` line. */
  summary: {
    elapsedS: number;
    run: number;
    passed: number;
    failed: number;
    timedOut: number;
    skipped: number;
    slow: number;
    leaky: number;
    flaky: number;
  };
  slowMarks: number;
}

const RESULT_LINE =
  /^\s*(?:TRY\s+(\d+)\s+)?(PASS|FAIL|TIMEOUT|LEAK-FAIL|LEAK|ABORT|SIGSEGV|SIGABRT)\s+\[\s*([\d.]+)s\]\s+\(\s*(\d+)\/(\d+)\)\s+(\S+)\s+(\S+)\s*$/;
const STARTING_LINE =
  /^\s*Starting\s+(\d+)\s+tests?\s+across\s+(\d+)\s+binar(?:y|ies)(?:\s+\((\d+)\s+tests?\s+skipped\))?/;
const SUMMARY_LINE = /^\s*Summary\s+\[\s*([\d.]+)s\]\s+(\d+)\s+tests?\s+run:\s+(.*)$/;
const SLOW_LINE = /^\s*SLOW\s+\[>/;

function count(re: RegExp, text: string): number {
  const m = re.exec(text);
  return m ? Number(m[1]) : 0;
}

/**
 * Parse one recipe log into its nextest invocations.
 *
 * A log may hold several (`just test-rendezvous` runs three packages). Result
 * lines after a `Summary` and before the next `Starting` are nextest's failure
 * recap and are ignored, so a failed test is counted once.
 */
export function parseLog(text: string, label = "<log>"): Invocation[] {
  const invocations: Invocation[] = [];
  let current: Invocation | null = null;
  const pending = new Map<string, number>(); // identity -> retries seen
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.replace(/\x1b\[[0-9;]*m/g, "");
    const starting = STARTING_LINE.exec(line);
    if (starting) {
      if (current) {
        throw new Error(`${label}: 'Starting' without a preceding 'Summary'`);
      }
      current = {
        declared: {
          tests: Number(starting[1]),
          binaries: Number(starting[2]),
          skipped: Number(starting[3] ?? 0),
        },
        results: [],
        summary: {
          elapsedS: 0,
          run: 0,
          passed: 0,
          failed: 0,
          timedOut: 0,
          skipped: 0,
          slow: 0,
          leaky: 0,
          flaky: 0,
        },
        slowMarks: 0,
      };
      pending.clear();
      continue;
    }
    if (!current) continue;
    const summary = SUMMARY_LINE.exec(line);
    if (summary) {
      const tail = summary[3];
      current.summary = {
        elapsedS: Number(summary[1]),
        run: Number(summary[2]),
        passed: count(/(\d+)\s+passed/, tail),
        failed: count(/(\d+)\s+failed/, tail),
        timedOut: count(/(\d+)\s+timed out/, tail),
        skipped: count(/(\d+)\s+skipped/, tail),
        slow: count(/(\d+)\s+slow/, tail),
        leaky: count(/(\d+)\s+leaky/, tail),
        flaky: count(/(\d+)\s+flaky/, tail),
      };
      invocations.push(current);
      current = null;
      continue;
    }
    if (SLOW_LINE.test(line)) {
      current.slowMarks += 1;
      continue;
    }
    const result = RESULT_LINE.exec(line);
    if (!result) continue;
    const [, tryIndex, status, duration, , , binary, name] = result;
    const identity = `${binary} ${name}`;
    if (tryIndex !== undefined) {
      // A `TRY n` line is a retried attempt; the final attempt has no prefix.
      pending.set(identity, Number(tryIndex));
      continue;
    }
    current.results.push({
      binary,
      name,
      status: status as Status,
      durationS: Number(duration),
      retries: pending.get(identity) ?? 0,
    });
  }
  if (current) {
    throw new Error(`${label}: truncated — 'Starting' without a 'Summary'`);
  }
  if (invocations.length === 0) {
    throw new Error(`${label}: no nextest invocation found`);
  }
  return invocations;
}

export function readLog(path: string): string {
  const bytes = readFileSync(path);
  return path.endsWith(".gz") ? gunzipSync(bytes).toString("utf8") : bytes.toString("utf8");
}

// ---------------------------------------------------------------------------
// Manifest, cohorts, targets
// ---------------------------------------------------------------------------

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
  /** Whole binaries, e.g. `claudine-cli::context_command`. */
  binaries?: string[];
  /** Every binary whose id starts with one of these, e.g. `claudine-cli::`. */
  binaryPrefixes?: string[];
  /** Name prefixes within a binary, e.g. lib modules. */
  prefixes?: { binary: string; prefix: string }[];
}

export interface TargetSpec {
  binary: string;
  /** Exact test name; when absent, `suffix`/`prefix` narrow, else the whole binary. */
  name?: string;
  prefix?: string;
  /** Matches the leaf of a lib unit test whose module path is not repeated here. */
  suffix?: string;
  contract: string;
}

export function readManifest(path: string): RunRecord[] {
  return readFileSync(path, "utf8")
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .map((line, index) => {
      const record = JSON.parse(line) as RunRecord;
      for (const key of ["suite", "revision", "sha", "sequence", "role", "log", "wallS", "exitCode"]) {
        if (!(key in record)) {
          throw new Error(`manifest line ${index + 1}: missing '${key}'`);
        }
      }
      return record;
    });
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

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
        `[count-mismatch] ${run.suite} ${run.revision}#${run.sequence}: ${inv.results.length} result lines against '${inv.summary.run} tests run'`,
      );
    }
    if (inv.declared.tests !== inv.summary.run) {
      violations.push(
        `[count-mismatch] ${run.suite} ${run.revision}#${run.sequence}: 'Starting ${inv.declared.tests}' against '${inv.summary.run} tests run'`,
      );
    }
    for (const result of inv.results) {
      const identity = `${result.binary} ${result.name}`;
      if (identities.has(identity)) {
        violations.push(`[duplicate-identity] ${run.suite} ${run.revision}#${run.sequence}: ${identity}`);
      }
      identities.add(identity);
      durations.set(identity, result.durationS);
      summedS += result.durationS;
      retries += result.retries;
      if (result.status === "LEAK") leaks += 1;
      if (result.status === "LEAK-FAIL") leakFails += 1;
      if (result.status !== "PASS" && result.status !== "LEAK") {
        violations.push(
          `[non-passing] ${run.suite} ${run.revision}#${run.sequence}: ${result.status} ${identity} (${result.durationS}s)`,
        );
      }
    }
  }
  if (run.exitCode !== 0) {
    violations.push(`[exit-code] ${run.suite} ${run.revision}#${run.sequence}: recipe exited ${run.exitCode}`);
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
  return sorted.length % 2 === 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

export interface Spread {
  n: number;
  min: number;
  median: number;
  max: number;
  /** `(max − min) / median`, the same-code drift floor. */
  driftRatio: number;
}

export function spread(values: number[]): Spread {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const med = median(values);
  return { n: values.length, min, median: med, max, driftRatio: med === 0 ? 0 : (max - min) / med };
}

/**
 * A cross-revision delta is established only when it clears both revisions'
 * own same-code spread; otherwise it is reported as inside the drift bracket.
 */
export function established(baseline: Spread, candidate: Spread): boolean {
  const delta = Math.abs(baseline.median - candidate.median);
  const floor = Math.max(baseline.max - baseline.min, candidate.max - candidate.min);
  return delta > floor;
}

export function matchesCohort(spec: CohortSpec, binary: string, name: string): boolean {
  if (spec.binaries?.includes(binary)) return true;
  if ((spec.binaryPrefixes ?? []).some((p) => binary.startsWith(p))) return true;
  return (spec.prefixes ?? []).some((p) => p.binary === binary && name.startsWith(p.prefix));
}

export function matchesTarget(spec: TargetSpec, binary: string, name: string): boolean {
  if (spec.binary !== binary) return false;
  if (spec.name !== undefined) return spec.name === name;
  if (spec.prefix !== undefined) return name.startsWith(spec.prefix);
  if (spec.suffix !== undefined) return name.endsWith(spec.suffix);
  return true;
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

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
  manifestDir: string,
): Report {
  const runs: RunMetrics[] = [];
  const violations: string[] = [];
  for (const run of manifest) {
    const logPath = resolve(manifestDir, run.log);
    if (!existsSync(logPath)) {
      violations.push(`[missing-log] ${run.suite} ${run.revision}#${run.sequence}: ${run.log}`);
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

  // --- per-run table --------------------------------------------------------
  lines.push("### Runs");
  lines.push("");
  lines.push(
    "| Suite | Revision | Seq | Role | Wall | Build/setup | Runner elapsed | Summed | Tests | Passed | Failed | Timed out | Skipped | Leaks | Retries | Slow | Load before |",
  );
  lines.push("|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|");
  for (const m of runs) {
    lines.push(
      `| \`${m.run.suite}\` | ${m.run.revision} | ${m.run.sequence} | ${m.run.role} | ${fmt(m.run.wallS, 1)} s | ${fmt(m.buildSetupS, 1)} s | ${fmt(m.elapsedS)} s | ${fmt(m.summedS)} s | ${m.tests} | ${m.passed} | ${m.failed} | ${m.timedOut} | ${m.skipped} | ${m.leaks + m.leakFails} | ${m.retries} | ${m.slowMarks} | ${m.run.loadavgBefore ?? "—"} |`,
    );
  }
  lines.push("");

  // --- per-revision spread and drift bracket --------------------------------
  const counted = runs.filter((r) => r.run.role === "alternating");
  lines.push("### Per-revision spread (alternating runs only)");
  lines.push("");
  lines.push(
    "| Suite | Revision | Runs | Elapsed min / median / max | Elapsed drift | Summed min / median / max | Summed drift | Identities |",
  );
  lines.push("|---|---|---:|---|---:|---|---:|---:|");
  const spreads = new Map<string, { elapsed: Spread; summed: Spread; identities: Set<string> }>();
  for (const suite of suites) {
    for (const revision of revisions) {
      const group = counted.filter((r) => r.run.suite === suite && r.run.revision === revision);
      if (group.length === 0) continue;
      const elapsed = spread(group.map((r) => r.elapsedS));
      const summed = spread(group.map((r) => r.summedS));
      const first = group[0].identities;
      for (const other of group.slice(1)) {
        const missing = [...first].filter((id) => !other.identities.has(id));
        const extra = [...other.identities].filter((id) => !first.has(id));
        if (missing.length > 0 || extra.length > 0) {
          violations.push(
            `[unstable-identities] ${suite} ${revision}: run #${other.run.sequence} differs from #${group[0].run.sequence} by −${missing.length}/+${extra.length}`,
          );
        }
      }
      spreads.set(`${suite} ${revision}`, { elapsed, summed, identities: first });
      lines.push(
        `| \`${suite}\` | ${revision} | ${group.length} | ${fmt(elapsed.min)} / ${fmt(elapsed.median)} / ${fmt(elapsed.max)} s | ${pct(elapsed.driftRatio)} | ${fmt(summed.min)} / ${fmt(summed.median)} / ${fmt(summed.max)} s | ${pct(summed.driftRatio)} | ${first.size} |`,
      );
    }
  }
  lines.push("");

  if (revisions.includes("baseline") && revisions.includes("candidate")) {
    lines.push("### Baseline → candidate (medians; established only outside both drift brackets)");
    lines.push("");
    lines.push("| Suite | Column | Baseline | Candidate | Delta | Established |");
    lines.push("|---|---|---:|---:|---:|---|");
    for (const suite of suites) {
      const b = spreads.get(`${suite} baseline`);
      const c = spreads.get(`${suite} candidate`);
      if (!b || !c) continue;
      for (const column of ["elapsed", "summed"] as const) {
        const bs = b[column];
        const cs = c[column];
        const delta = cs.median - bs.median;
        lines.push(
          `| \`${suite}\` | ${column} | ${fmt(bs.median)} s | ${fmt(cs.median)} s | ${delta >= 0 ? "+" : ""}${fmt(delta)} s (${pct(delta / bs.median)}) | ${established(bs, cs) ? "yes" : "**no — inside drift**"} |`,
        );
      }
      const added = [...c.identities].filter((id) => !b.identities.has(id));
      const removed = [...b.identities].filter((id) => !c.identities.has(id));
      lines.push(
        `| \`${suite}\` | identities | ${b.identities.size} | ${c.identities.size} | +${added.length} / −${removed.length} | — |`,
      );
    }
    lines.push("");
    // Paired alternation: the protocol alternates so that each baseline run
    // has a candidate neighbour taken under the same host state. The ratio
    // per pair is insensitive to slow drift that the whole-series bracket
    // above cannot separate from the effect.
    lines.push("#### Paired alternation (candidate ÷ baseline, same sequence number)");
    lines.push("");
    lines.push("| Suite | Column | Pairs | Ratio min / median / max | Every pair improved |");
    lines.push("|---|---|---:|---|---|");
    for (const suite of suites) {
      const b = counted.filter((r) => r.run.suite === suite && r.run.revision === "baseline");
      const c = counted.filter((r) => r.run.suite === suite && r.run.revision === "candidate");
      const pairs = b
        .map((bm) => [bm, c.find((cm) => cm.run.sequence === bm.run.sequence)] as const)
        .filter((pair): pair is readonly [RunMetrics, RunMetrics] => pair[1] !== undefined);
      if (pairs.length === 0) continue;
      for (const column of ["elapsedS", "summedS"] as const) {
        const ratios = pairs.map(([bm, cm]) => cm[column] / bm[column]);
        const s = spread(ratios);
        lines.push(
          `| \`${suite}\` | ${column === "elapsedS" ? "elapsed" : "summed"} | ${pairs.length} | ${s.min.toFixed(3)} / ${s.median.toFixed(3)} / ${s.max.toFixed(3)} | ${s.max < 1 ? "yes" : "**no**"} |`,
        );
      }
    }
    lines.push("");
    lines.push("#### Identity changes");
    lines.push("");
    for (const suite of suites) {
      const b = spreads.get(`${suite} baseline`);
      const c = spreads.get(`${suite} candidate`);
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

  // --- changed cohorts, from the same reports -------------------------------
  if (cohorts.length > 0) {
    lines.push("### Changed cohorts (from the alternating suite runs; medians of summed duration)");
    lines.push("");
    lines.push(
      "| Cohort | Baseline tests | Baseline summed (min / median / max) | Candidate tests | Candidate summed (min / median / max) | Delta (median) | Established |",
    );
    lines.push("|---|---:|---|---:|---|---:|---|");
    for (const cohort of cohorts) {
      // A cohort lives in the suite(s) that run it; a run of an unrelated
      // suite contributes no sample rather than a zero.
      const perRun = counted.map((m) => {
        let tests = 0;
        let summed = 0;
        for (const inv of m.invocations) {
          for (const r of inv.results) {
            if (matchesCohort(cohort, r.binary, r.name)) {
              tests += 1;
              summed += r.durationS;
            }
          }
        }
        return { m, tests, summed };
      });
      const suitesWithCohort = new Set(perRun.filter((s) => s.tests > 0).map((s) => s.m.run.suite));
      const per = new Map<string, { tests: number[]; summed: number[] }>();
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
        `| \`${cohort.name}\` | ${bt} | ${bs ? `${fmt(bs.min)} / ${fmt(bs.median)} / ${fmt(bs.max)} s` : "—"} | ${ct} | ${cs ? `${fmt(cs.min)} / ${fmt(cs.median)} / ${fmt(cs.max)} s` : "—"} | ${Number.isFinite(delta) ? `${delta >= 0 ? "+" : ""}${fmt(delta)} s (${pct(delta / (bs as Spread).median)})` : "—"} | ${bs && cs ? (established(bs, cs) ? "yes" : "no — inside drift") : "—"} |`,
      );
    }
    lines.push("");
  }

  // --- ten-execution targets ------------------------------------------------
  if (targets.length > 0) {
    lines.push("### Changed timing / concurrency contracts (every candidate execution, all roles)");
    lines.push("");
    lines.push("| Identity | Contract | Executions | Min | Median | Max | Non-passing | Retries |");
    lines.push("|---|---|---:|---:|---:|---:|---:|---:|");
    const candidateRuns = runs.filter((r) => r.run.revision === "candidate");
    for (const target of targets) {
      const perIdentity = new Map<string, { durations: number[]; bad: number; retries: number }>();
      for (const m of candidateRuns) {
        for (const inv of m.invocations) {
          for (const r of inv.results) {
            if (!matchesTarget(target, r.binary, r.name)) continue;
            const id = `${r.binary} ${r.name}`;
            const entry = perIdentity.get(id) ?? { durations: [], bad: 0, retries: 0 };
            entry.durations.push(r.durationS);
            entry.retries += r.retries;
            if (r.status !== "PASS") entry.bad += 1;
            perIdentity.set(id, entry);
          }
        }
      }
      if (perIdentity.size === 0) {
        violations.push(`[target-unmatched] ${target.binary} ${target.name ?? target.prefix ?? "*"}: no candidate execution`);
        lines.push(`| \`${target.binary} ${target.name ?? target.prefix ?? "*"}\` | ${target.contract} | **0** | — | — | — | — | — |`);
        continue;
      }
      for (const [id, entry] of [...perIdentity.entries()].sort()) {
        if (entry.durations.length < 10) {
          violations.push(`[target-short] ${id}: ${entry.durations.length} executions, ten required`);
        }
        const s = spread(entry.durations);
        lines.push(
          `| \`${id}\` | ${target.contract} | ${s.n} | ${fmt(s.min, 3)} s | ${fmt(s.median, 3)} s | ${fmt(s.max, 3)} s | ${entry.bad} | ${entry.retries} |`,
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

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function usage(): never {
  process.stderr.write(
    "usage: measurement.ts report --manifest <runs.jsonl> [--cohorts <json>] [--targets <json>] [--json] [--allow-violations]\n" +
      "       measurement.ts parse <log>...\n",
  );
  process.exit(2);
}

function main(argv: string[]): number {
  const [command, ...rest] = argv;
  if (command === "parse") {
    if (rest.length === 0) usage();
    for (const path of rest) {
      const invocations = parseLog(readLog(path), path);
      for (const inv of invocations) {
        const summed = inv.results.reduce((acc, r) => acc + r.durationS, 0);
        const bad = inv.results.filter((r) => r.status !== "PASS").length;
        process.stdout.write(
          `${path}\telapsed=${inv.summary.elapsedS}\tsummed=${summed.toFixed(2)}\trun=${inv.summary.run}\tpassed=${inv.summary.passed}\tfailed=${inv.summary.failed}\tskipped=${inv.summary.skipped}\tnon_passing_lines=${bad}\tslow=${inv.slowMarks}\n`,
        );
      }
    }
    return 0;
  }
  if (command !== "report") usage();
  let manifestPath: string | undefined;
  let cohortsPath: string | undefined;
  let targetsPath: string | undefined;
  let json = false;
  let allowViolations = false;
  for (let i = 0; i < rest.length; i += 1) {
    const arg = rest[i];
    if (arg === "--manifest") manifestPath = rest[++i];
    else if (arg === "--cohorts") cohortsPath = rest[++i];
    else if (arg === "--targets") targetsPath = rest[++i];
    else if (arg === "--json") json = true;
    else if (arg === "--allow-violations") allowViolations = true;
    else if (arg === "--markdown") {
      /* default */
    } else usage();
  }
  if (!manifestPath) usage();
  const manifest = readManifest(manifestPath);
  const cohorts: CohortSpec[] = cohortsPath ? JSON.parse(readFileSync(cohortsPath, "utf8")).cohorts : [];
  const targets: TargetSpec[] = targetsPath ? JSON.parse(readFileSync(targetsPath, "utf8")).targets : [];
  const report = buildReport(manifest, cohorts, targets, dirname(resolve(manifestPath)));
  if (json) {
    process.stdout.write(
      JSON.stringify(
        {
          runs: report.runs.map((m) => ({
            suite: m.run.suite,
            revision: m.run.revision,
            sequence: m.run.sequence,
            role: m.run.role,
            wallS: m.run.wallS,
            buildSetupS: m.buildSetupS,
            elapsedS: m.elapsedS,
            summedS: m.summedS,
            tests: m.tests,
            passed: m.passed,
            failed: m.failed,
            timedOut: m.timedOut,
            skipped: m.skipped,
            leaks: m.leaks + m.leakFails,
            retries: m.retries,
            slowMarks: m.slowMarks,
            durations: Object.fromEntries(m.durations),
          })),
          violations: report.violations,
        },
        null,
        2,
      ) + "\n",
    );
  } else {
    process.stdout.write(report.markdown + "\n");
  }
  process.stderr.write(`GATE EXIT=${report.violations.length === 0 || allowViolations ? 0 : 1}\n`);
  return report.violations.length === 0 || allowViolations ? 0 : 1;
}

const invokedDirectly =
  typeof process.argv[1] === "string" && /measurement\.ts$/.test(process.argv[1]);
if (invokedDirectly) {
  process.exit(main(process.argv.slice(2)));
}
