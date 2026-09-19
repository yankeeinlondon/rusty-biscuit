import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";

import type { Command, CommandIo } from "../command.ts";
import { EXIT, UsageError } from "../errors.ts";
import { parseFamilyFile } from "../reconcile/families.ts";
import { parseArgv, requireString, optionalString, hasFlag, flagAll } from "../args.ts";
import { loadConfig } from "../config.ts";
import { aggregate, renderAggregateTable } from "./aggregate.ts";
import {
  parseAndCheckLogs,
  attribute,
  attributeByBinary,
  renderAttributionTable,
  renderBinaryTable,
  deriveBudgets,
  renderBudgetTable,
  type BudgetInput,
} from "./index.ts";

const USAGE = `usage: test-audit attribute <run.log|report.xml>... --config <cfg> [--families <json>] [--by-binary] [--markdown] [--json]
       test-audit attribute aggregate <run-dir>... --config <cfg> [--families <json>] [--out <json>] [--headroom <ratio>] [--json]
       test-audit attribute budgets --config <cfg> --runs <manifest.json> [--headroom <ratio>] [--markdown]`;

export const attributeCommand: Command = {
  name: "attribute",
  summary: "attribute nextest run cost to families",
  usage: USAGE,
  run(argv: string[], io: CommandIo): number {
    const parsed = parseArgv(argv, ["markdown", "json", "by-binary"]);
    const subcommand = parsed.positional[0];

    if (subcommand === "budgets") {
      return runBudgets(parsed.positional.slice(1), parsed.flags, io);
    }
    if (subcommand === "aggregate") {
      return runAggregate(parsed.positional.slice(1), parsed.flags, io);
    }

    const logPaths = parsed.positional;
    if (logPaths.length === 0) {
      throw new UsageError(`no log files provided\n${USAGE}`);
    }

    const configPath = requireString(parsed, "config", USAGE);
    const config = loadConfig(configPath);
    const familiesPath = optionalString(parsed, "families") ?? config.familiesPath;

    for (const path of [familiesPath, ...logPaths]) {
      if (!existsSync(path) || !statSync(path).isFile()) {
        throw new UsageError(`not a readable file: ${path}`);
      }
    }

    const families = parseFamilyFile(readFileSync(familiesPath, "utf8")).families;
    const { invocations, violations } = parseAndCheckLogs(logPaths);
    const attribution = attribute(invocations, families);
    violations.push(...attribution.violations);
    const byBinary = hasFlag(parsed, "by-binary") ? attributeByBinary(invocations) : undefined;

    if (hasFlag(parsed, "json")) {
      io.out(
        JSON.stringify(
          {
            families: attribution.families,
            ...(byBinary === undefined ? {} : { binaries: byBinary }),
            summed: attribution.summed,
            elapsed: attribution.elapsed,
            count: attribution.count,
            violations,
          },
          null,
          2
        )
      );
    } else {
      io.out(renderAttributionTable(attribution));
      if (byBinary !== undefined) {
        io.out("");
        io.out(renderBinaryTable(byBinary));
      }
    }

    if (violations.length > 0) {
      io.err(`${violations.length} violation(s):`);
      for (const v of violations) io.err(`  [${v.kind}] ${v.detail}`);
      io.err(`GATE EXIT=${EXIT.violations}`);
      return EXIT.violations;
    }
    io.out(`GATE EXIT=${EXIT.clean}`);
    return EXIT.clean;
  },
};

/**
 * `attribute aggregate` — join stored CI staging trees to families.
 *
 * Writes the same JSON `attribute budgets --runs` reads, so a clean
 * aggregation followed by a refused derivation is the expected end state until
 * three green runs per leg exist.
 */
function runAggregate(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length === 0) {
    throw new UsageError(`no run directories provided\n${USAGE}`);
  }

  const parsed = { positional, flags };
  const configPath = requireString(parsed, "config", USAGE);
  const config = loadConfig(configPath);
  const familiesPath = optionalString(parsed, "families") ?? config.familiesPath;
  const headroom = optionalNonNegative(parsed, "headroom");

  if (!existsSync(familiesPath) || !statSync(familiesPath).isFile()) {
    throw new UsageError(`not a readable file: ${familiesPath}`);
  }

  const families = parseFamilyFile(readFileSync(familiesPath, "utf8")).families;
  const result = aggregate({
    runDirs: positional,
    environments: config.environments,
    families,
    ...(headroom === undefined ? {} : { headroom }),
  });

  const outPath = optionalString(parsed, "out");
  const serialized = JSON.stringify(result.budgetInput, null, 2);
  if (outPath !== undefined) writeFileSync(outPath, `${serialized}\n`, "utf8");

  if (hasFlag(parsed, "json")) {
    io.out(serialized);
  } else {
    io.out(renderAggregateTable(result));
    if (outPath !== undefined) {
      io.out("");
      io.out(`wrote ${outPath}`);
    }
  }

  if (result.violations.length > 0) {
    io.err(`${result.violations.length} violation(s):`);
    for (const v of result.violations) io.err(`  [${v.kind}] ${v.detail}`);
    io.err(`GATE EXIT=${EXIT.violations}`);
    return EXIT.violations;
  }
  io.out(`GATE EXIT=${EXIT.clean}`);
  return EXIT.clean;
}

function optionalNonNegative(
  parsed: { positional: string[]; flags: Map<string, string | true> },
  name: string
): number | undefined {
  const raw = optionalString(parsed, name);
  if (raw === undefined) return undefined;
  const value = Number(raw);
  if (!Number.isFinite(value) || value < 0) {
    throw new UsageError(`--${name} must be a finite number >= 0, got ${raw}`);
  }
  return value;
}

function runBudgets(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length > 0) {
    throw new UsageError(`unexpected positional arguments\n${USAGE}`);
  }

  const parsed = { positional, flags };
  const configPath = requireString(parsed, "config", USAGE);
  const runsPath = requireString(parsed, "runs", USAGE);
  const headroomStr = optionalString(parsed, "headroom");
  const headroom = headroomStr === undefined ? undefined : Number(headroomStr);

  if (headroom !== undefined && (!Number.isFinite(headroom) || headroom < 0)) {
    throw new UsageError(`--headroom must be a finite number >= 0, got ${headroomStr}`);
  }

  if (!existsSync(runsPath) || !statSync(runsPath).isFile()) {
    throw new UsageError(`not a readable file: ${runsPath}`);
  }

  loadConfig(configPath);

  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(runsPath, "utf8"));
  } catch (error) {
    throw new UsageError(`${runsPath}: not JSON: ${(error as Error).message}`);
  }

  const input = raw as BudgetInput;
  if (headroom !== undefined) input.headroom = headroom;

  const { budgets, violations } = deriveBudgets(input);

  if (hasFlag(parsed, "markdown") || !hasFlag(parsed, "json")) {
    if (budgets.length > 0) {
      io.out(renderBudgetTable(budgets));
    }
  }

  if (hasFlag(parsed, "json")) {
    io.out(JSON.stringify({ budgets, violations }, null, 2));
  }

  if (violations.length > 0) {
    io.err(`${violations.length} violation(s):`);
    for (const v of violations) io.err(`  [${v.kind}] ${v.detail}`);
    io.err(`GATE EXIT=${EXIT.violations}`);
    return EXIT.violations;
  }
  io.out(`GATE EXIT=${EXIT.clean}`);
  return EXIT.clean;
}
