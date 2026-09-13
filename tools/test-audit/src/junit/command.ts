/**
 * JUnit gate command: validate staging trees against expectations, compare
 * baseline and candidate sets.
 *
 * ## Legacy expectations mapping
 *
 * The `--expect` flag accepts both the new config-derived shape and the legacy
 * Claudine shape from `claudine/fixes/.../baseline/expectations.json`:
 *
 * - Legacy `environments` (string[]) → per-leg environment declarations
 * - Legacy `cells` ([{tier, package}]) → shared cells for all environments
 * - Legacy `pendingEnvironments` (string[]) → marks legs `pending: true`
 * - Legacy `requiredTests`, `platformExclusions`, `enforceTimeoutFloors` →
 *   `config.junit.*`
 * - Legacy hard-coded `TIMEOUT_TESTS` constant → `config.junit.timeoutFloors`
 *
 * The new shape is a partial config override; when both are present, new shape
 * wins.
 */
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import type { Command, CommandIo } from "../command.ts";
import { loadConfig, activeEnvironments, type EnvironmentConfig, type JunitExpectations } from "../config.ts";
import { hasFlag, optionalString, parseArgv, requireString } from "../args.ts";
import { EXIT, MalformedInput, UsageError } from "../errors.ts";
import { versionBanner } from "../version.ts";
import { collect, type CollectionInput } from "./collect.ts";
import { compare } from "./compare.ts";
import {
  renderComparison,
  renderExclusions,
  renderJson,
  renderNote,
  renderTable,
  renderTimeoutFloors,
  renderViolations,
} from "./render.ts";

const USAGE = `usage: test-audit junit <staging-dir> --config <audit.config.json>
                        [--expect <json>] [--label <text>]
                        [--baseline <dir> [--baseline-expect <json>]]
                        [--markdown] [--json]`;

interface LegacyExpectations {
  environments?: string[];
  cells?: Array<{ tier: string; package: string }>;
  requiredTests?: string[];
  pendingEnvironments?: string[];
  platformExclusions?: Record<string, string[]>;
  enforceTimeoutFloors?: boolean;
  /** Legacy hard-coded timeout floors from the original script. */
  timeoutFloors?: Record<string, { budget: number; tick: number }>;
}

interface MergedExpectations {
  environments: EnvironmentConfig[];
  junitExpectations: JunitExpectations;
}

/**
 * Merge config-derived expectations with a legacy `--expect` override.
 *
 * Legacy shape: `{environments, cells, requiredTests, pendingEnvironments,
 * platformExclusions, enforceTimeoutFloors}` where `cells` apply to every
 * environment and `pendingEnvironments` marks legs pending.
 *
 * New shape: partial override of `{environments, junit}`.
 */
function mergeExpectations(
  configEnvironments: EnvironmentConfig[],
  configJunit: JunitExpectations,
  expectPath: string | undefined,
): MergedExpectations {
  if (!expectPath) {
    return { environments: configEnvironments, junitExpectations: configJunit };
  }

  if (!existsSync(expectPath)) {
    throw new UsageError(`--expect file not found: ${expectPath}`);
  }

  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(expectPath, "utf8"));
  } catch (error) {
    throw new MalformedInput(`--expect is not valid JSON: ${(error as Error).message}`);
  }

  const parsed = raw as Partial<LegacyExpectations>;

  // Detect legacy shape by presence of `cells` or `pendingEnvironments`
  const isLegacy = "cells" in parsed || "pendingEnvironments" in parsed;

  if (isLegacy) {
    const legacyEnvs = parsed.environments ?? [];
    const cells = parsed.cells ?? [];
    const pending = new Set(parsed.pendingEnvironments ?? []);

    const environments: EnvironmentConfig[] = legacyEnvs.map((name) => {
      const existing = configEnvironments.find((e) => e.name === name);
      return {
        name,
        kind: name === "wsl2-ubuntu" ? "wsl" : "native",
        os: name.startsWith("ubuntu")
          ? "linux"
          : name.startsWith("macos")
            ? "macos"
            : name.startsWith("windows")
              ? "windows"
              : "linux",
        cells,
        pending: pending.has(name),
        ...(existing ? { workflow: existing.workflow, reason: existing.reason } : {}),
      };
    });

    const junitExpectations: JunitExpectations = {
      requiredTests: parsed.requiredTests ?? [],
      platformExclusions: parsed.platformExclusions ?? {},
      timeoutFloors: parsed.timeoutFloors ?? {},
      enforceTimeoutFloors: parsed.enforceTimeoutFloors ?? false,
    };

    return { environments, junitExpectations };
  }

  // New shape: partial override (cast to unknown first to avoid type incompatibility)
  const override = parsed as unknown as {
    environments?: EnvironmentConfig[];
    junit?: Partial<JunitExpectations>;
  };
  return {
    environments: override.environments ?? configEnvironments,
    junitExpectations: {
      ...configJunit,
      ...override.junit,
    },
  };
}

async function run(argv: string[], io: CommandIo): Promise<number> {
  const parsed = parseArgv(argv, ["markdown", "json"]);

  if (parsed.positional.length === 0) {
    throw new UsageError(`no staging directory given\n${USAGE}`);
  }
  if (parsed.positional.length > 1) {
    throw new UsageError(`unexpected argument ${parsed.positional[1]}\n${USAGE}`);
  }

  const dir = resolve(parsed.positional[0]!);
  const configPath = requireString(parsed, "config", USAGE);
  const label = optionalString(parsed, "label") ?? dir;
  const expectPath = optionalString(parsed, "expect");
  const baselineDir = optionalString(parsed, "baseline");
  const baselineExpectPath = optionalString(parsed, "baseline-expect");
  const markdown = hasFlag(parsed, "markdown");
  const json = hasFlag(parsed, "json");

  if (baselineExpectPath && !baselineDir) {
    throw new UsageError(`--baseline-expect requires --baseline\n${USAGE}`);
  }

  const config = loadConfig(configPath);
  const configEnvs = config.environments;
  const configJunit = config.junit;

  const { environments, junitExpectations } = mergeExpectations(configEnvs, configJunit, expectPath);

  const candidateInput: CollectionInput = {
    dir,
    label,
    environments: activeEnvironments({ ...config, environments }),
    expectations: junitExpectations,
  };

  let candidate: ReturnType<typeof collect>;
  let baseline: ReturnType<typeof collect> | undefined;
  let comparisons: ReturnType<typeof compare> | undefined;

  try {
    candidate = collect(candidateInput);

    if (baselineDir) {
      const baselineResolved = resolve(baselineDir);
      const baselineExpect = baselineExpectPath ?? expectPath;
      const { environments: baselineEnvs, junitExpectations: baselineJunit } = mergeExpectations(
        configEnvs,
        configJunit,
        baselineExpect,
      );
      baseline = collect({
        dir: baselineResolved,
        label: `baseline ${baselineDir}`,
        environments: activeEnvironments({ ...config, environments: baselineEnvs }),
        expectations: baselineJunit,
      });
      comparisons = compare(baseline, candidate);
    }
  } catch (error) {
    if (error instanceof MalformedInput || error instanceof UsageError) {
      io.err((error as Error).message);
      return error instanceof UsageError ? EXIT.usage : EXIT.violations;
    }
    throw error;
  }

  const envKinds = new Map(environments.map((e) => [e.name, e.kind]));

  if (json) {
    io.out(renderJson(candidate, baseline, comparisons));
  } else {
    io.out(versionBanner());
    io.out("");
    io.out(renderTable(candidate));
    if (!markdown) {
      const timeoutTable = renderTimeoutFloors(candidate, junitExpectations, envKinds);
      if (timeoutTable) io.out(timeoutTable);
    }
    const exclusionTable = renderExclusions(candidate.exclusions);
    if (exclusionTable) io.out(exclusionTable);
    if (baseline && comparisons) {
      io.out("");
      io.out(renderTable(baseline));
      io.out(renderComparison(comparisons));
    }
    io.out(renderNote());
  }

  let exit: number = EXIT.clean;
  for (const result of baseline ? [baseline, candidate] : [candidate]) {
    const problems = renderViolations(result);
    if (problems) {
      io.err(problems);
      exit = EXIT.violations;
    }
  }

  return exit;
}

export const junitCommand: Command = {
  name: "junit",
  summary: "gate JUnit staging trees, compare baseline and candidate",
  usage: USAGE,
  run,
};
