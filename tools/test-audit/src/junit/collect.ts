/**
 * Gate logic: collect evidence from a staging tree and validate against
 * expectations. Returns measurements, violations, and exclusions.
 */
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import type { EnvironmentConfig, JunitExpectations } from "../config.ts";
import { MalformedInput, type Violation } from "../errors.ts";
import { parseJunit, type Report, type TestCase } from "./parse.ts";
import { parseManifest, type ManifestRecord } from "./manifest.ts";

export type ViolationKind =
  | "missing-artifact"
  | "malformed-report"
  | "missing-test"
  | "duplicate-identity"
  | "invalid-duration"
  | "failed-run"
  | "timeout-floor-miss"
  | "stale-exclusion";

export interface JunitViolation extends Violation<ViolationKind> {
  environment: string;
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
  retries: number;
}

export interface PlatformExclusion {
  environment: string;
  test: string;
}

export interface CollectionResult {
  label: string;
  measurements: CellMeasurement[];
  pending: string[];
  violations: JunitViolation[];
  /** Declared exclusions whose test was indeed absent from that leg. */
  exclusions: PlatformExclusion[];
  /** Per environment, every case keyed by identity — for downstream phases. */
  casesByEnvironment: Map<string, TestCase[]>;
}

export interface CollectionInput {
  dir: string;
  label: string;
  environments: EnvironmentConfig[];
  expectations: JunitExpectations;
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
function timeoutAllowance(kind: "native" | "wsl"): number {
  return kind === "wsl" ? 2 : 1;
}

function environmentDirectories(dir: string): string[] {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw new MalformedInput(`no such artifact directory: ${dir}`);
  }
  return readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
}

export function collect(input: CollectionInput): CollectionResult {
  const { dir, label, environments, expectations } = input;
  const violations: JunitViolation[] = [];
  const measurements: CellMeasurement[] = [];
  const casesByEnvironment = new Map<string, TestCase[]>();
  const pending: string[] = [];
  const exclusions: PlatformExclusion[] = [];

  const present = new Set(environmentDirectories(dir));
  const envMap = new Map(environments.map((e) => [e.name, e]));
  const known = environments.filter((env) => present.has(env.name));
  const extra = Array.from(present).filter((name) => !envMap.has(name));

  for (const env of environments) {
    if (present.has(env.name)) continue;
    if (env.pending) {
      pending.push(env.name);
      continue;
    }
    violations.push({
      environment: env.name,
      kind: "missing-artifact",
      detail: "no artifact directory; a leg is pending only when declared pending",
    });
  }

  for (const envName of [...known.map((e) => e.name), ...extra]) {
    const env = envMap.get(envName);
    const root = join(dir, envName);
    const manifestPath = join(root, "manifest.jsonl");
    const cases: TestCase[] = [];
    casesByEnvironment.set(envName, cases);

    let records: ManifestRecord[] = [];
    if (!existsSync(manifestPath)) {
      violations.push({
        environment: envName,
        kind: "missing-artifact",
        detail: "manifest.jsonl absent; the tree is not a `_stage_junit` staging directory",
      });
    } else {
      try {
        records = parseManifest(readFileSync(manifestPath, "utf8"), `${envName}/manifest.jsonl`);
      } catch (error) {
        violations.push({ environment: envName, kind: "malformed-report", detail: (error as Error).message });
      }
    }

    const cells = env?.cells ?? [];
    for (const cell of cells) {
      const record = records.find((r) => r.tier === cell.tier && r.package === cell.package);
      const relative = `${cell.tier}/${cell.package}.xml`;
      if (!record) {
        violations.push({
          environment: envName,
          kind: "missing-artifact",
          detail: `no manifest record for ${relative}`,
        });
        continue;
      }
      if (record.exit_code !== 0) {
        violations.push({
          environment: envName,
          kind: "failed-run",
          detail: `${relative} exited ${record.exit_code}`,
        });
      }
      if (!record.report_present) {
        violations.push({
          environment: envName,
          kind: "missing-artifact",
          detail: `${relative} recorded report_present=false (nextest emitted no XML)`,
        });
        continue;
      }
      const xmlPath = join(root, record.xml);
      if (!existsSync(xmlPath)) {
        violations.push({
          environment: envName,
          kind: "missing-artifact",
          detail: `${record.xml} named by the manifest is absent from the artifact`,
        });
        continue;
      }

      let report: Report;
      try {
        report = parseJunit(readFileSync(xmlPath, "utf8"), `${envName}/${record.xml}`);
      } catch (error) {
        const kind: ViolationKind = (error as Error).message.includes("duration")
          ? "invalid-duration"
          : "malformed-report";
        violations.push({ environment: envName, kind, detail: (error as Error).message });
        continue;
      }

      const seen = new Set<string>();
      for (const testCase of report.cases) {
        if (seen.has(testCase.identity)) {
          violations.push({
            environment: envName,
            kind: "duplicate-identity",
            detail: `${record.xml} lists ${testCase.identity} more than once`,
          });
        }
        seen.add(testCase.identity);
      }
      cases.push(...report.cases);

      const failures = report.cases.filter((c) => c.outcome === "failed" || c.outcome === "errored");
      for (const failure of failures) {
        violations.push({
          environment: envName,
          kind: "failed-run",
          detail: `${failure.identity} ${failure.outcome}`,
        });
      }
      if (report.declaredFailures + report.declaredErrors > failures.length) {
        violations.push({
          environment: envName,
          kind: "failed-run",
          detail:
            `${record.xml} declares ${report.declaredFailures} failures / ` +
            `${report.declaredErrors} errors but enumerates ${failures.length}`,
        });
      }
      if (report.declaredTests !== report.cases.length) {
        violations.push({
          environment: envName,
          kind: "malformed-report",
          detail: `${record.xml} declares ${report.declaredTests} tests but enumerates ${report.cases.length}`,
        });
      }

      const summedSeconds = report.cases.reduce((total, c) => total + c.seconds, 0);
      const rawBuild = record.duration_s - report.wallSeconds;
      if (rawBuild < -BUILD_ROUNDING_TOLERANCE) {
        violations.push({
          environment: envName,
          kind: "invalid-duration",
          detail:
            `${record.xml}: manifest duration_s ${record.duration_s} s is below the ` +
            `run's own elapsed ${report.wallSeconds.toFixed(1)} s`,
        });
      }

      const retryCount = report.cases.reduce((total, c) => total + (c.retries ?? 0), 0);

      measurements.push({
        environment: envName,
        tier: cell.tier,
        package: cell.package,
        buildSeconds: Math.max(0, rawBuild),
        runnerSeconds: report.wallSeconds,
        summedSeconds,
        tests: report.cases.length,
        failures: failures.length,
        skips: report.cases.filter((c) => c.outcome === "skipped").length,
        retries: retryCount,
      });
    }

    const identities = new Set(cases.map((c) => c.identity));
    const names = new Set(cases.map((c) => c.name));
    const ran = (test: string) => (test.includes("::") ? identities.has(test) : names.has(test));
    const excluded = new Set(expectations.platformExclusions[envName] ?? []);
    for (const required of expectations.requiredTests) {
      if (ran(required) || excluded.has(required)) continue;
      violations.push({
        environment: envName,
        kind: "missing-test",
        detail: `required test ${required} was not executed`,
      });
    }
    for (const test of Array.from(excluded)) {
      if (ran(test)) {
        violations.push({
          environment: envName,
          kind: "stale-exclusion",
          detail: `${test} is declared excluded on ${envName} but was executed there`,
        });
      } else {
        exclusions.push({ environment: envName, test });
      }
    }

    if (expectations.enforceTimeoutFloors && env) {
      const allowance = timeoutAllowance(env.kind);
      for (const [name, floor] of Object.entries(expectations.timeoutFloors)) {
        const found = cases.find((c) => c.name === name);
        if (!found) continue;
        const bound = floor.budget + floor.tick + allowance;
        if (found.seconds > bound) {
          violations.push({
            environment: envName,
            kind: "timeout-floor-miss",
            detail: `${name} took ${found.seconds.toFixed(1)} s against a ${bound.toFixed(1)} s bound`,
          });
        }
      }
    }
  }

  return { label, measurements, pending, violations, exclusions, casesByEnvironment };
}
