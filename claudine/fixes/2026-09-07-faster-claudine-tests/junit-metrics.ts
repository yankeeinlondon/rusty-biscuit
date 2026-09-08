#!/usr/bin/env -S npx tsx
/**
 * Baseline/candidate gate over the CI JUnit staging trees.
 *
 * Forked from the predecessor fix's `junit-metrics.ts`
 * (`claudine/fixes/_completed/2026-08-01-cli-slow-tests/junit-metrics.ts`),
 * which read one hard-coded artifact path per environment with an
 * attribute-order-sensitive regex and always exited 0. This fork reads the
 * staging tree the `just` recipes actually upload and **fails** the process on
 * any of the six rejection classes the plan names.
 *
 * Coupled to `just/devops.just` `_stage_junit`: each environment directory is
 * one uploaded `target/nextest/ci-reports` tree, holding `<tier>/<package>.xml`
 * per nextest invocation plus an appended `manifest.jsonl` whose records carry
 * `{tier, package, xml, exit_code, environment, duration_s, report_present}`.
 * A change to that record shape breaks this gate.
 *
 * The three cost columns are kept separate because they answer different
 * questions and move independently:
 *
 * - **build/setup** — `manifest.duration_s` minus the run's own wall time;
 *   `duration_s` brackets the whole nextest invocation, compilation included.
 * - **runner elapsed** — `<testsuites time>`, nextest's wall time for the run.
 * - **summed duration** — Σ `<testcase time>`, which exceeds elapsed under
 *   parallelism and is the only column comparable across runner core counts.
 *
 * Usage:
 *   gh run download <run-id> -R yankeeinlondon/rusty-biscuit \
 *     -n junit-claudine-cli-L1-<env> -D <dir>/<env>      # once per leg
 *   npx tsx junit-metrics.ts <dir> [--label "run <id>"] [--expect <json>]
 *                                  [--markdown] [--json]
 *
 * Exit codes: 0 all expectations met; 1 one or more violations; 2 usage error.
 */
import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Expectations
// ---------------------------------------------------------------------------

/** The four configured legs, in the order the spec's tables list them. */
export const CI_ENVIRONMENTS = [
  "ubuntu-latest",
  "macos-latest",
  "windows-latest",
  "wsl2-ubuntu",
] as const;

/**
 * What a complete artifact set must contain.
 *
 * `requiredTests` are identities (`<suite>::<case>`) whose *absence* is a
 * failure rather than a smaller number. Without them a leg that silently
 * stopped listing a binary reads as an improvement — the exact defect the
 * predecessor's script could not detect.
 */
export interface Expectations {
  environments: string[];
  /** `<tier>/<package>` cells every environment must carry. */
  cells: { tier: string; package: string }[];
  requiredTests: string[];
  /** Legs named here are reported as pending instead of missing. */
  pendingEnvironments: string[];
  /**
   * Turn a `TIMEOUT_TESTS` miss into a violation rather than a table cell.
   *
   * Off by default because the floors below were ratified under the
   * predecessor's clock; a candidate set that re-derives them switches this on
   * so a miss cannot pass as a printed remark.
   */
  enforceTimeoutFloors: boolean;
}

/**
 * The nine timeout-shaped tests and the semantic floor each pays, in seconds.
 *
 * Carried over from the predecessor because the same runs close its deferred
 * acceptance criterion 4. `budget` is the wall-clock or stream-silence budget
 * the test waits out; `tick` is the watchdog interval it may additionally wait.
 * The bound is `budget + tick + 1 s` natively and `+ 2 s` on WSL2.
 *
 * These floors were ratified under the pre-fix first-event grace. The
 * 2026-08-31 startup-stall fix replaced that with a spawn-fallback silence
 * clock, so a miss here is re-derived rather than treated as a regression.
 */
export const TIMEOUT_TESTS: Record<string, { budget: number; tick: number }> = {
  watchdog_subagent_hang_terminates_and_names_stuck_ids: { budget: 1, tick: 0.2 },
  watchdog_stream_idle_timeout_after_tool_call_hang: { budget: 1, tick: 0.2 },
  watchdog_wall_clock_timeout_terminates_active_stream: { budget: 1, tick: 0.2 },
  watchdog_opencode_post_fanout_silence_does_not_kill_prematurely: { budget: 1, tick: 0.2 },
  compose_non_harness_respects_cli_timeout: { budget: 1, tick: 0.2 },
  inline_compose_non_harness_respects_cli_step_timeout: { budget: 1, tick: 0.2 },
  sequence_per_step_step_timeout_override: { budget: 0.5, tick: 0.1 },
  opencode_stderr_rate_limit_before_stdout_forces_early_termination: { budget: 0, tick: 0.2 },
  opencode_stderr_stream_error_cap_1_17_8_forces_early_termination: { budget: 0, tick: 0.2 },
};

export const DEFAULT_EXPECTATIONS: Expectations = {
  environments: [...CI_ENVIRONMENTS],
  cells: [{ tier: "L1", package: "claudine-cli" }],
  requiredTests: [],
  pendingEnvironments: [],
  enforceTimeoutFloors: false,
};

// ---------------------------------------------------------------------------
// JUnit parsing
// ---------------------------------------------------------------------------

export type Outcome = "passed" | "failed" | "errored" | "skipped";

export interface TestCase {
  /** `<suite>::<case>`, matching `scripts/ci-rollup.rs`'s identity rule. */
  identity: string;
  binary: string;
  name: string;
  seconds: number;
  outcome: Outcome;
}

export interface Report {
  /** `<testsuites time>` — nextest's wall time for the whole run. */
  wallSeconds: number;
  declaredTests: number;
  declaredFailures: number;
  declaredErrors: number;
  cases: TestCase[];
}

export class MalformedReport extends Error {}

/** A report that is well-formed but carries a duration nothing can use. */
export class InvalidDuration extends MalformedReport {}

/** Text containers whose contents are never markup for our purposes. */
const TEXT_ELEMENTS = new Set(["system-out", "system-err"]);

function unescapeXml(value: string): string {
  return value.replace(/&(#x?[0-9A-Fa-f]+|[a-zA-Z]+);/g, (whole, entity: string) => {
    if (entity.startsWith("#x") || entity.startsWith("#X")) {
      return String.fromCodePoint(Number.parseInt(entity.slice(2), 16));
    }
    if (entity.startsWith("#")) {
      return String.fromCodePoint(Number.parseInt(entity.slice(1), 10));
    }
    const named: Record<string, string> = {
      amp: "&", lt: "<", gt: ">", quot: '"', apos: "'",
    };
    return named[entity] ?? whole;
  });
}

interface Tag {
  name: string;
  attributes: Record<string, string>;
  closing: boolean;
  selfClosing: boolean;
}

/**
 * Attribute-order-independent tag scan.
 *
 * The predecessor matched `name=… classname=… time=…` in that fixed order with
 * one regex, so a `<testcase>` written with any other ordering — or as a
 * self-closing element — was silently dropped from every total.
 */
function* tags(xml: string, source: string): Generator<Tag> {
  let index = 0;
  while (index < xml.length) {
    const open = xml.indexOf("<", index);
    if (open === -1) return;
    if (xml.startsWith("<!--", open)) {
      const end = xml.indexOf("-->", open);
      if (end === -1) throw new MalformedReport(`${source}: unterminated comment`);
      index = end + 3;
      continue;
    }
    if (xml.startsWith("<?", open) || xml.startsWith("<!", open)) {
      const end = xml.indexOf(">", open);
      if (end === -1) throw new MalformedReport(`${source}: unterminated declaration`);
      index = end + 1;
      continue;
    }

    // Scan to the tag's own `>`, skipping any that sits inside a quoted value.
    let cursor = open + 1;
    let quote: string | null = null;
    let close = -1;
    while (cursor < xml.length) {
      const char = xml[cursor];
      if (quote) {
        if (char === quote) quote = null;
      } else if (char === '"' || char === "'") {
        quote = char;
      } else if (char === ">") {
        close = cursor;
        break;
      }
      cursor += 1;
    }
    if (close === -1) {
      throw new MalformedReport(
        `${source}: unterminated tag at byte ${open}${quote ? " (unterminated attribute value)" : ""}`,
      );
    }

    let body = xml.slice(open + 1, close).trim();
    const closing = body.startsWith("/");
    if (closing) body = body.slice(1).trim();
    const selfClosing = body.endsWith("/");
    if (selfClosing) body = body.slice(0, -1).trim();

    const nameMatch = /^[^\s/>]+/.exec(body);
    if (!nameMatch) throw new MalformedReport(`${source}: nameless tag at byte ${open}`);
    const name = nameMatch[0].replace(/^[^:]*:/, "");

    const attributes: Record<string, string> = {};
    const attributeRe = /([^\s=/>]+)\s*=\s*("([^"]*)"|'([^']*)')/g;
    attributeRe.lastIndex = nameMatch[0].length;
    let match: RegExpExecArray | null;
    while ((match = attributeRe.exec(body)) !== null) {
      attributes[match[1]] = unescapeXml(match[3] ?? match[4] ?? "");
    }

    yield { name, attributes, closing, selfClosing };
    index = close + 1;
  }
}

function requiredNumber(raw: string | undefined, what: string, source: string): number {
  if (raw === undefined) throw new InvalidDuration(`${source}: ${what} has no value`);
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new InvalidDuration(`${source}: ${what} is not a number: ${raw}`);
  if (value < 0) throw new InvalidDuration(`${source}: ${what} is negative: ${raw}`);
  return value;
}

/**
 * Parse one nextest JUnit document.
 *
 * ## Errors
 *
 * `MalformedReport` when the document is not well-formed, is not a JUnit
 * report (no `<testsuites>` root), leaves a tag unclosed, or carries a missing,
 * non-numeric or negative `time`. An unparseable duration is an error rather
 * than a zero: a silently-zeroed case makes a slow test look free.
 */
export function parseJunit(xml: string, source: string): Report {
  let sawRoot = false;
  let wallSeconds: number | null = null;
  let declaredTests = 0;
  let declaredFailures = 0;
  let declaredErrors = 0;
  let suiteName = "";
  let openCase: TestCase | null = null;
  const cases: TestCase[] = [];
  const stack: string[] = [];
  /** Depth at which a `<system-out>`/`<system-err>` body began, if inside one. */
  let textDepth: number | null = null;

  for (const tag of tags(xml, source)) {
    if (tag.closing) {
      const expected = stack.pop();
      if (expected !== tag.name) {
        throw new MalformedReport(
          `${source}: </${tag.name}> closes <${expected ?? "nothing"}>`,
        );
      }
      if (textDepth !== null && stack.length < textDepth) textDepth = null;
      if (tag.name === "testcase") {
        if (openCase) cases.push(openCase);
        openCase = null;
      }
      continue;
    }

    // A `<failure>` quoted inside captured stdout is prose, not an outcome.
    const insideText = textDepth !== null;
    if (!insideText) {
      switch (tag.name) {
        case "testsuites": {
          sawRoot = true;
          wallSeconds = requiredNumber(tag.attributes.time, "<testsuites time>", source);
          declaredTests = requiredNumber(tag.attributes.tests ?? "0", "<testsuites tests>", source);
          declaredFailures = requiredNumber(tag.attributes.failures ?? "0", "<testsuites failures>", source);
          declaredErrors = requiredNumber(tag.attributes.errors ?? "0", "<testsuites errors>", source);
          break;
        }
        case "testsuite":
          suiteName = tag.attributes.name ?? "";
          break;
        case "testcase": {
          if (!sawRoot) throw new MalformedReport(`${source}: <testcase> outside <testsuites>`);
          const name = tag.attributes.name ?? "";
          const classname = tag.attributes.classname ?? "";
          const owner = suiteName || classname;
          const built: TestCase = {
            identity: owner ? `${owner}::${name}` : name,
            binary: owner,
            name,
            seconds: requiredNumber(tag.attributes.time, `<testcase ${name} time>`, source),
            outcome: "passed",
          };
          if (tag.selfClosing) cases.push(built);
          else openCase = built;
          break;
        }
        case "failure":
          if (openCase) openCase.outcome = "failed";
          break;
        case "error":
          if (openCase) openCase.outcome = "errored";
          break;
        case "skipped":
          if (openCase) openCase.outcome = "skipped";
          break;
        default:
          break;
      }
    }

    if (!tag.selfClosing) {
      if (textDepth === null && TEXT_ELEMENTS.has(tag.name)) textDepth = stack.length;
      stack.push(tag.name);
    }
  }

  if (stack.length > 0) {
    throw new MalformedReport(`${source}: unclosed <${stack[stack.length - 1]}>`);
  }
  if (!sawRoot) {
    throw new MalformedReport(`${source}: not a JUnit report (no <testsuites> root)`);
  }
  return { wallSeconds: wallSeconds ?? 0, declaredTests, declaredFailures, declaredErrors, cases };
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

export interface ManifestRecord {
  tier: string;
  package: string;
  xml: string;
  exit_code: number;
  environment: string;
  duration_s: number;
  report_present: boolean;
}

export class MalformedManifest extends Error {}

export function parseManifest(text: string, source: string): ManifestRecord[] {
  const records: ManifestRecord[] = [];
  text.split("\n").forEach((line, index) => {
    if (line.trim() === "") return;
    let value: unknown;
    try {
      value = JSON.parse(line);
    } catch (error) {
      throw new MalformedManifest(`${source}:${index + 1}: not JSON — ${(error as Error).message}`);
    }
    const record = value as Partial<ManifestRecord>;
    for (const key of ["tier", "package", "xml", "environment"] as const) {
      if (typeof record[key] !== "string") {
        throw new MalformedManifest(`${source}:${index + 1}: '${key}' is not a string`);
      }
    }
    for (const key of ["exit_code", "duration_s"] as const) {
      if (typeof record[key] !== "number" || !Number.isFinite(record[key])) {
        throw new MalformedManifest(`${source}:${index + 1}: '${key}' is not a finite number`);
      }
    }
    if (typeof record.report_present !== "boolean") {
      throw new MalformedManifest(`${source}:${index + 1}: 'report_present' is not a boolean`);
    }
    if ((record.duration_s as number) < 0) {
      throw new MalformedManifest(`${source}:${index + 1}: 'duration_s' is negative`);
    }
    records.push(record as ManifestRecord);
  });
  return records;
}

// ---------------------------------------------------------------------------
// Gate
// ---------------------------------------------------------------------------

export type ViolationKind =
  | "missing-artifact"
  | "malformed-report"
  | "missing-test"
  | "duplicate-identity"
  | "invalid-duration"
  | "failed-run"
  | "timeout-floor-miss";

export interface Violation {
  environment: string;
  kind: ViolationKind;
  detail: string;
}

export interface CellMeasurement {
  environment: string;
  tier: string;
  package: string;
  /** `duration_s` minus runner elapsed; the invocation's compile/setup share. */
  buildSeconds: number;
  runnerSeconds: number;
  summedSeconds: number;
  tests: number;
  failures: number;
  skips: number;
}

export interface Baseline {
  label: string;
  measurements: CellMeasurement[];
  pending: string[];
  violations: Violation[];
  /** Per environment, every case keyed by identity — for downstream phases. */
  casesByEnvironment: Map<string, TestCase[]>;
}

/**
 * `duration_s` is whole seconds while runner elapsed is fractional, so their
 * difference can be slightly negative on a fast cell without anything being
 * wrong. Beyond one second apart the two are measuring different runs.
 */
const BUILD_ROUNDING_TOLERANCE = 1;

/**
 * The WSL2 leg reads its workspace across the Windows filesystem boundary, so
 * the predecessor granted it one extra second on every timeout floor.
 */
function timeoutAllowance(environment: string): number {
  return environment === "wsl2-ubuntu" ? 2 : 1;
}

function environmentDirectories(dir: string): string[] {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw new Error(`no such artifact directory: ${dir}`);
  }
  return readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
}

export function collect(dir: string, label: string, expectations: Expectations): Baseline {
  const violations: Violation[] = [];
  const measurements: CellMeasurement[] = [];
  const casesByEnvironment = new Map<string, TestCase[]>();
  const pending: string[] = [];

  const present = new Set(environmentDirectories(dir));
  const known = expectations.environments.filter((env) => present.has(env));
  const extra = [...present].filter((env) => !expectations.environments.includes(env));

  for (const env of expectations.environments) {
    if (present.has(env)) continue;
    if (expectations.pendingEnvironments.includes(env)) {
      pending.push(env);
      continue;
    }
    violations.push({
      environment: env,
      kind: "missing-artifact",
      detail: "no artifact directory; a leg is pending only when declared pending",
    });
  }

  for (const env of [...known, ...extra]) {
    const root = join(dir, env);
    const manifestPath = join(root, "manifest.jsonl");
    const cases: TestCase[] = [];
    casesByEnvironment.set(env, cases);

    let records: ManifestRecord[] = [];
    if (!existsSync(manifestPath)) {
      violations.push({
        environment: env,
        kind: "missing-artifact",
        detail: "manifest.jsonl absent; the tree is not a `_stage_junit` staging directory",
      });
    } else {
      try {
        records = parseManifest(readFileSync(manifestPath, "utf8"), `${env}/manifest.jsonl`);
      } catch (error) {
        violations.push({ environment: env, kind: "malformed-report", detail: (error as Error).message });
      }
    }

    for (const cell of expectations.cells) {
      const record = records.find((r) => r.tier === cell.tier && r.package === cell.package);
      const relative = `${cell.tier}/${cell.package}.xml`;
      if (!record) {
        violations.push({
          environment: env,
          kind: "missing-artifact",
          detail: `no manifest record for ${relative}`,
        });
        continue;
      }
      if (record.exit_code !== 0) {
        violations.push({
          environment: env,
          kind: "failed-run",
          detail: `${relative} exited ${record.exit_code}`,
        });
      }
      if (!record.report_present) {
        violations.push({
          environment: env,
          kind: "missing-artifact",
          detail: `${relative} recorded report_present=false (nextest emitted no XML)`,
        });
        continue;
      }
      const xmlPath = join(root, record.xml);
      if (!existsSync(xmlPath)) {
        violations.push({
          environment: env,
          kind: "missing-artifact",
          detail: `${record.xml} named by the manifest is absent from the artifact`,
        });
        continue;
      }

      let report: Report;
      try {
        report = parseJunit(readFileSync(xmlPath, "utf8"), `${env}/${record.xml}`);
      } catch (error) {
        const kind: ViolationKind =
          error instanceof InvalidDuration ? "invalid-duration" : "malformed-report";
        violations.push({ environment: env, kind, detail: (error as Error).message });
        continue;
      }

      const seen = new Set<string>();
      for (const testCase of report.cases) {
        if (seen.has(testCase.identity)) {
          violations.push({
            environment: env,
            kind: "duplicate-identity",
            detail: `${record.xml} lists ${testCase.identity} more than once`,
          });
        }
        seen.add(testCase.identity);
      }
      cases.push(...report.cases);

      const failures = report.cases.filter(
        (c) => c.outcome === "failed" || c.outcome === "errored",
      );
      for (const failure of failures) {
        violations.push({
          environment: env,
          kind: "failed-run",
          detail: `${failure.identity} ${failure.outcome}`,
        });
      }
      // The declared totals are checked independently of the enumerated cases:
      // a report that under-lists its own failures is itself the defect.
      if (report.declaredFailures + report.declaredErrors > failures.length) {
        violations.push({
          environment: env,
          kind: "failed-run",
          detail:
            `${record.xml} declares ${report.declaredFailures} failures / ` +
            `${report.declaredErrors} errors but enumerates ${failures.length}`,
        });
      }
      if (report.declaredTests !== report.cases.length) {
        violations.push({
          environment: env,
          kind: "malformed-report",
          detail:
            `${record.xml} declares ${report.declaredTests} tests but enumerates ` +
            `${report.cases.length}`,
        });
      }

      const summedSeconds = report.cases.reduce((total, c) => total + c.seconds, 0);
      const rawBuild = record.duration_s - report.wallSeconds;
      if (rawBuild < -BUILD_ROUNDING_TOLERANCE) {
        violations.push({
          environment: env,
          kind: "invalid-duration",
          detail:
            `${record.xml}: manifest duration_s ${record.duration_s} s is below the ` +
            `run's own elapsed ${report.wallSeconds.toFixed(1)} s`,
        });
      }
      measurements.push({
        environment: env,
        tier: cell.tier,
        package: cell.package,
        buildSeconds: Math.max(0, rawBuild),
        runnerSeconds: report.wallSeconds,
        summedSeconds,
        tests: report.cases.length,
        failures: failures.length,
        skips: report.cases.filter((c) => c.outcome === "skipped").length,
      });
    }

    const identities = new Set(cases.map((c) => c.identity));
    const names = new Set(cases.map((c) => c.name));
    for (const required of expectations.requiredTests) {
      const found = required.includes("::") ? identities.has(required) : names.has(required);
      if (!found) {
        violations.push({
          environment: env,
          kind: "missing-test",
          detail: `required test ${required} was not executed`,
        });
      }
    }

    if (expectations.enforceTimeoutFloors) {
      const allowance = timeoutAllowance(env);
      for (const [name, floor] of Object.entries(TIMEOUT_TESTS)) {
        const found = cases.find((c) => c.name === name);
        if (!found) continue; // absence is `requiredTests`' job, not this one
        const bound = floor.budget + floor.tick + allowance;
        if (found.seconds > bound) {
          violations.push({
            environment: env,
            kind: "timeout-floor-miss",
            detail: `${name} took ${found.seconds.toFixed(1)} s against a ${bound.toFixed(1)} s bound`,
          });
        }
      }
    }
  }

  return { label, measurements, pending, violations, casesByEnvironment };
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

const fixed = (n: number) => n.toFixed(1);

export function renderTable(baseline: Baseline): string {
  const lines: string[] = [];
  lines.push(`### ${baseline.label}`, "");
  lines.push(
    "| Environment | Tier | Package | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips |",
  );
  lines.push("|---|---|---|---:|---:|---:|---:|---:|---:|");
  for (const m of baseline.measurements) {
    lines.push(
      `| \`${m.environment}\` | ${m.tier} | \`${m.package}\` | ${fixed(m.buildSeconds)} s | ` +
        `${fixed(m.runnerSeconds)} s | ${fixed(m.summedSeconds)} s | ${m.tests} | ${m.failures} | ${m.skips} |`,
    );
  }
  for (const env of baseline.pending) {
    lines.push(`| \`${env}\` | — | — | pending | pending | pending | — | — | — |`);
  }
  return lines.join("\n");
}

export function renderTimeoutFloors(baseline: Baseline): string {
  const lines: string[] = [];
  lines.push("", "### Timeout-shaped tests (predecessor floors)", "");
  lines.push("| Environment | Test | Observed | Bound | Verdict |");
  lines.push("|---|---|---:|---:|---|");
  for (const [env, cases] of baseline.casesByEnvironment) {
    const allowance = timeoutAllowance(env);
    for (const [name, floor] of Object.entries(TIMEOUT_TESTS)) {
      const found = cases.find((c) => c.name === name);
      const bound = floor.budget + floor.tick + allowance;
      if (!found) {
        lines.push(`| \`${env}\` | \`${name}\` | absent | ${fixed(bound)} s | ABSENT |`);
        continue;
      }
      const verdict = found.seconds <= bound ? "ok" : "MISS";
      lines.push(
        `| \`${env}\` | \`${name}\` | ${fixed(found.seconds)} s | ${fixed(bound)} s | ${verdict} |`,
      );
    }
  }
  return lines.join("\n");
}

export function renderViolations(baseline: Baseline): string {
  if (baseline.violations.length === 0) return "";
  const lines = ["", `${baseline.violations.length} violation(s):`];
  for (const v of baseline.violations) {
    lines.push(`  [${v.kind}] ${v.environment}: ${v.detail}`);
  }
  return lines.join("\n");
}

export function loadExpectations(path: string | undefined): Expectations {
  if (!path) return DEFAULT_EXPECTATIONS;
  const parsed = JSON.parse(readFileSync(path, "utf8")) as Partial<Expectations>;
  return {
    environments: parsed.environments ?? DEFAULT_EXPECTATIONS.environments,
    cells: parsed.cells ?? DEFAULT_EXPECTATIONS.cells,
    requiredTests: parsed.requiredTests ?? DEFAULT_EXPECTATIONS.requiredTests,
    pendingEnvironments: parsed.pendingEnvironments ?? DEFAULT_EXPECTATIONS.pendingEnvironments,
    enforceTimeoutFloors:
      parsed.enforceTimeoutFloors ?? DEFAULT_EXPECTATIONS.enforceTimeoutFloors,
  };
}

const USAGE =
  "usage: junit-metrics.ts <dir> [--label <text>] [--expect <json>] [--markdown] [--json]";

const VALUED_FLAGS = new Set(["--label", "--expect"]);
const BARE_FLAGS = new Set(["--markdown", "--json"]);

export interface ParsedArgs {
  dir?: string;
  label?: string;
  expect?: string;
  markdown: boolean;
  json: boolean;
  error?: string;
}

export function parseArgs(argv: string[]): ParsedArgs {
  const parsed: ParsedArgs = { markdown: false, json: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (VALUED_FLAGS.has(argument)) {
      const value = argv[index + 1];
      if (value === undefined || value.startsWith("--")) {
        return { ...parsed, error: `${argument} needs a value` };
      }
      if (argument === "--label") parsed.label = value;
      else parsed.expect = value;
      index += 1;
      continue;
    }
    if (BARE_FLAGS.has(argument)) {
      if (argument === "--markdown") parsed.markdown = true;
      else parsed.json = true;
      continue;
    }
    if (argument.startsWith("--")) return { ...parsed, error: `unknown flag ${argument}` };
    if (parsed.dir !== undefined) return { ...parsed, error: `unexpected argument ${argument}` };
    parsed.dir = argument;
  }
  if (parsed.dir === undefined) return { ...parsed, error: "no artifact directory given" };
  return parsed;
}

export function main(argv: string[]): number {
  const args = parseArgs(argv);
  if (args.error || !args.dir) {
    console.error(`error: ${args.error}`);
    console.error(USAGE);
    return 2;
  }
  const { dir } = args;
  const label = args.label ?? dir;
  const expectations = loadExpectations(args.expect);

  let baseline: Baseline;
  try {
    baseline = collect(resolve(dir), label, expectations);
  } catch (error) {
    console.error(`error: ${(error as Error).message}`);
    return 1;
  }

  if (args.json) {
    console.log(
      JSON.stringify(
        {
          label: baseline.label,
          measurements: baseline.measurements,
          pending: baseline.pending,
          violations: baseline.violations,
        },
        null,
        2,
      ),
    );
  } else {
    console.log(renderTable(baseline));
    if (!args.markdown) console.log(renderTimeoutFloors(baseline));
  }

  const problems = renderViolations(baseline);
  if (problems) {
    console.error(problems);
    return 1;
  }
  return 0;
}

const invokedDirectly =
  process.argv[1] !== undefined &&
  (process.argv[1].endsWith("junit-metrics.ts") || process.argv[1].endsWith("junit-metrics.js"));
if (invokedDirectly) {
  process.exit(main(process.argv.slice(2)));
}
