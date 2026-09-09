import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";

import type { Command, CommandIo } from "../command.ts";
import { EXIT, UsageError } from "../errors.ts";
import { parseArgv, requireString, optionalString, hasFlag } from "../args.ts";
import { loadConfig } from "../config.ts";
import { parseLog, readLog } from "../nextest-log.ts";
import {
  readManifest,
  buildReport,
  type CohortSpec,
  type TargetSpec,
} from "./index.ts";
import { run as runMeasurement } from "./runner.ts";

const USAGE = `usage: test-audit measure report --config <cfg> --manifest <runs.jsonl> [--cohorts <json>] [--targets <json>] [--markdown] [--json]
       test-audit measure parse <log>...
       test-audit measure run --plan <plan.json> --out <dir>`;

export const measureCommand: Command = {
  name: "measure",
  summary: "local alternating-run measurement report and runner",
  usage: USAGE,
  run(argv: string[], io: CommandIo): number {
    const parsed = parseArgv(argv, ["markdown", "json"]);
    const subcommand = parsed.positional[0];

    if (subcommand === "report") {
      return runReport(parsed.positional.slice(1), parsed.flags, io);
    }

    if (subcommand === "parse") {
      return runParse(parsed.positional.slice(1), io);
    }

    if (subcommand === "run") {
      return runRunner(parsed.positional.slice(1), parsed.flags, io);
    }

    throw new UsageError(`unknown subcommand: ${subcommand ?? "(none)"}\n${USAGE}`);
  },
};

function runReport(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length > 0) {
    throw new UsageError(`unexpected positional arguments\n${USAGE}`);
  }

  const parsed = { positional, flags };
  const configPath = requireString(parsed, "config", USAGE);
  const manifestPath = requireString(parsed, "manifest", USAGE);
  const cohortsPath = optionalString(parsed, "cohorts");
  const targetsPath = optionalString(parsed, "targets");

  if (!existsSync(manifestPath) || !statSync(manifestPath).isFile()) {
    throw new UsageError(`not a readable file: ${manifestPath}`);
  }

  loadConfig(configPath);

  const manifest = readManifest(manifestPath);
  const cohorts: CohortSpec[] = cohortsPath
    ? (JSON.parse(readFileSync(cohortsPath, "utf8")) as { cohorts: CohortSpec[] }).cohorts
    : [];
  const targets: TargetSpec[] = targetsPath
    ? (JSON.parse(readFileSync(targetsPath, "utf8")) as { targets: TargetSpec[] }).targets
    : [];

  const report = buildReport(
    manifest,
    cohorts,
    targets,
    dirname(resolve(manifestPath))
  );

  if (hasFlag(parsed, "json")) {
    io.out(
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
        2
      )
    );
  } else {
    io.out(report.markdown);
  }

  io.err(`GATE EXIT=${report.violations.length === 0 ? EXIT.clean : EXIT.violations}`);
  return report.violations.length === 0 ? EXIT.clean : EXIT.violations;
}

function runParse(positional: string[], io: CommandIo): number {
  if (positional.length === 0) {
    throw new UsageError(`no log files provided\n${USAGE}`);
  }

  for (const path of positional) {
    if (!existsSync(path) || !statSync(path).isFile()) {
      throw new UsageError(`not a readable file: ${path}`);
    }
  }

  for (const path of positional) {
    const invocations = parseLog(readLog(path), path);
    for (const inv of invocations) {
      const summed = inv.results.reduce((acc, r) => acc + r.durationS, 0);
      const bad = inv.results.filter((r) => r.status !== "PASS").length;
      io.out(
        `${path}\telapsed=${inv.summary.elapsedS}\tsummed=${summed.toFixed(2)}\trun=${inv.summary.run}\tpassed=${inv.summary.passed}\tfailed=${inv.summary.failed}\tskipped=${inv.summary.skipped}\tnon_passing_lines=${bad}\tslow=${inv.slowMarks}`
      );
    }
  }

  return EXIT.clean;
}

function runRunner(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length > 0) {
    throw new UsageError(`unexpected positional arguments\n${USAGE}`);
  }

  const parsed = { positional, flags };
  const planPath = requireString(parsed, "plan", USAGE);
  const outDir = requireString(parsed, "out", USAGE);

  if (!existsSync(planPath) || !statSync(planPath).isFile()) {
    throw new UsageError(`not a readable file: ${planPath}`);
  }

  return runMeasurement(planPath, outDir);
}
