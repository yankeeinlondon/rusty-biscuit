/**
 * Bounded compatibility adapter for nextest console output.
 *
 * Supported format: nextest 0.9.x console output. Expected lines:
 * - `Starting N tests across M binaries (K skipped)`
 * - `PASS [ 0.123s] (12/34) pkg::bin test::name`
 * - `TRY n` retry prefix before status
 * - `Summary [ 12.345s] N tests run: M passed, ...`
 * - Accepts gzip-compressed `.log.gz` files
 *
 * Structured JUnit reports (the `junit` command) are the preferred input; this
 * adapter exists for existing console captures that cannot be regenerated.
 */

import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";

import { MalformedInput } from "./errors.ts";

export const TERMINAL_STATUSES = [
  "PASS",
  "FAIL",
  "LEAK",
  "LEAK-FAIL",
  "TIMEOUT",
  "SIGSEGV",
  "ABORT",
  "SIGABRT",
] as const;

export type TerminalStatus = (typeof TERMINAL_STATUSES)[number];

export interface TestResult {
  binaryId: string;
  pkg: string;
  name: string;
  status: TerminalStatus;
  /** Duration in seconds. */
  durationS: number;
  /** Attempt number; 1 unless the test was retried. */
  attempt: number;
}

export interface InvocationSummary {
  /** `N tests run` from the summary line. */
  run: number;
  passed: number;
  failed: number;
  timedOut: number;
  skipped: number;
  slow: number;
  leaky: number;
  flaky: number;
  /** Runner elapsed as nextest measured it, in seconds. */
  elapsedS: number;
}

export interface Invocation {
  /** Number of tests nextest declared when starting. */
  declared?: { tests: number; binaries: number; skipped: number };
  results: TestResult[];
  summary: InvocationSummary;
  /** Count of `SLOW [> ...]` informational notices. */
  slowMarks: number;
}

const ANSI = /\x1b\[[0-9;]*m/g;

function stripAnsi(text: string): string {
  return text.replace(ANSI, "");
}

const STARTING =
  /^\s*Starting\s+(\d+)\s+tests?\s+across\s+(\d+)\s+binar(?:y|ies)(?:\s+\((\d+)\s+tests?\s+skipped\))?/;
const RESULT =
  /^\s*(?:TRY\s+(\d+)\s+)?(PASS|FAIL|TIMEOUT|LEAK-FAIL|LEAK|ABORT|SIGSEGV|SIGABRT)\s+\[\s*([\d.]+)s\]\s+\(\s*(\d+)\/(\d+)\)\s+(\S+)\s+(\S+)\s*$/;
const SUMMARY = /^\s*Summary\s+\[\s*([\d.]+)s\]\s+(\d+)\s+tests?\s+run:\s+(.*)$/;
const SLOW = /^\s*SLOW\s+\[>/;

function count(re: RegExp, text: string): number {
  const m = re.exec(text);
  return m ? Number(m[1]) : 0;
}

/** `claudine-cli::context_command` → `claudine-cli`; `claudine` → `claudine`. */
export function packageOf(binaryId: string): string {
  const marker = binaryId.indexOf("::");
  return marker === -1 ? binaryId : binaryId.slice(0, marker);
}

/**
 * Parse nextest console log into invocations.
 *
 * A log may hold several invocations (e.g. `just test-rendezvous` runs three
 * packages). Each `Summary` line closes an invocation. Result lines before the
 * first `Starting` line belong to the first invocation (some recipes trim the
 * banner). Result lines after the last summary are a failure recap and are
 * ignored.
 *
 * ## Errors
 *
 * Throws `MalformedInput` when the log contains result lines but no summary
 * (truncated capture), or when a duration is not a valid number.
 */
export function parseLog(text: string, label: string): Invocation[] {
  const lines = stripAnsi(text).split(/\r?\n/);
  const invocations: Invocation[] = [];
  let current: Invocation | null = null;
  const pending = new Map<string, number>();

  for (const line of lines) {
    const starting = STARTING.exec(line);
    if (starting) {
      if (current) {
        throw new MalformedInput(
          `${label}: 'Starting' without a preceding 'Summary' at line: ${line}`
        );
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

    if (SLOW.test(line)) {
      if (current) current.slowMarks += 1;
      continue;
    }

    const result = RESULT.exec(line);
    if (result) {
      if (!current) {
        current = {
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
      }
      const tryIndex = result[1];
      const status = result[2]!;
      const duration = result[3]!;
      const binary = result[6]!;
      const name = result[7]!;
      const identity = `${binary} ${name}`;
      if (tryIndex !== undefined) {
        pending.set(identity, Number(tryIndex));
        continue;
      }
      const durationS = Number(duration);
      if (!Number.isFinite(durationS) || durationS < 0) {
        throw new MalformedInput(
          `${label}: invalid duration for ${binary} ${name}: ${duration}`
        );
      }
      current.results.push({
        binaryId: binary,
        pkg: packageOf(binary),
        name,
        status: status as TerminalStatus,
        durationS,
        attempt: (pending.get(identity) ?? 0) + 1,
      });
      pending.delete(identity);
      continue;
    }

    const summary = SUMMARY.exec(line);
    if (summary) {
      if (!current) {
        throw new MalformedInput(
          `${label}: 'Summary' without result lines at line: ${line}`
        );
      }
      const tail = summary[3]!;
      const elapsedS = Number(summary[1]!);
      if (!Number.isFinite(elapsedS) || elapsedS < 0) {
        throw new MalformedInput(
          `${label}: invalid summary elapsed: ${summary[1]!}`
        );
      }
      current.summary = {
        elapsedS,
        run: Number(summary[2]!),
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
    }
  }

  if (invocations.length === 0) {
    throw new MalformedInput(
      `${label}: no nextest summary line; the log is truncated or is not a run log`
    );
  }
  if (current !== null) {
    throw new MalformedInput(
      `${label}: truncated — result lines without a closing 'Summary'`
    );
  }
  return invocations;
}

export function readLog(path: string): string {
  const bytes = readFileSync(path);
  return path.endsWith(".gz")
    ? gunzipSync(bytes).toString("utf8")
    : bytes.toString("utf8");
}
