import { existsSync, readFileSync, statSync } from "node:fs";

import type { Command, CommandIo } from "../command.ts";
import { EXIT, UsageError, MalformedInput } from "../errors.ts";
import { parseArgv, requireString, hasFlag } from "../args.ts";
import { loadConfig } from "../config.ts";
import { TOOL_VERSION } from "../version.ts";
import {
  EvidenceFileSchema,
  validateEvidence,
  compareEvidence,
  renderComparisonTable,
  type EvidenceFile,
} from "./index.ts";

const USAGE = `usage: test-audit counters validate <evidence.json> --config <cfg> [--json]
       test-audit counters compare <baseline.json> <candidate.json> --config <cfg> [--markdown] [--json]`;

export const countersCommand: Command = {
  name: "counters",
  summary: "validate and compare work-count evidence",
  usage: USAGE,
  run(argv: string[], io: CommandIo): number {
    const parsed = parseArgv(argv, ["markdown", "json"]);
    const subcommand = parsed.positional[0];

    if (subcommand === "validate") {
      return runValidate(parsed.positional.slice(1), parsed.flags, io);
    }

    if (subcommand === "compare") {
      return runCompare(parsed.positional.slice(1), parsed.flags, io);
    }

    throw new UsageError(
      `unknown subcommand: ${subcommand ?? "(none)"}\n${USAGE}`
    );
  },
};

function runValidate(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length !== 1) {
    throw new UsageError(`expected exactly one evidence file\n${USAGE}`);
  }

  const evidencePath = positional[0]!;
  const parsed = { positional, flags };
  const configPath = requireString(parsed, "config", USAGE);

  if (!existsSync(evidencePath) || !statSync(evidencePath).isFile()) {
    throw new UsageError(`not a readable file: ${evidencePath}`);
  }

  const config = loadConfig(configPath);
  if (!config.counters) {
    throw new UsageError(`config does not define counters section`);
  }

  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(evidencePath, "utf8"));
  } catch (error) {
    throw new MalformedInput(
      `${evidencePath}: not JSON: ${(error as Error).message}`
    );
  }

  const parsed_evidence = EvidenceFileSchema.safeParse(raw);
  if (!parsed_evidence.success) {
    const lines = parsed_evidence.error.issues.map(
      (issue) =>
        `  ${issue.path.join(".") || "<root>"}: ${issue.message}`
    );
    throw new MalformedInput(
      `${evidencePath}: schema violations:\n${lines.join("\n")}`
    );
  }

  const evidence = parsed_evidence.data;
  const violations = validateEvidence(evidence, config.counters);

  if (hasFlag(parsed, "json")) {
    io.out(
      JSON.stringify(
        {
          toolVersion: TOOL_VERSION,
          violations,
          valid: violations.length === 0,
        },
        null,
        2
      )
    );
  } else {
    if (violations.length === 0) {
      io.out(`${evidencePath}: valid (${evidence.readings.length} readings)`);
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
}

function runCompare(
  positional: string[],
  flags: Map<string, string | true>,
  io: CommandIo
): number {
  if (positional.length !== 2) {
    throw new UsageError(
      `expected exactly two evidence files (baseline and candidate)\n${USAGE}`
    );
  }

  const baselinePath = positional[0]!;
  const candidatePath = positional[1]!;
  const parsed = { positional, flags };
  const configPath = requireString(parsed, "config", USAGE);

  for (const path of [baselinePath, candidatePath]) {
    if (!existsSync(path) || !statSync(path).isFile()) {
      throw new UsageError(`not a readable file: ${path}`);
    }
  }

  const config = loadConfig(configPath);
  if (!config.counters) {
    throw new UsageError(`config does not define counters section`);
  }

  function loadEvidence(path: string): EvidenceFile {
    let raw: unknown;
    try {
      raw = JSON.parse(readFileSync(path, "utf8"));
    } catch (error) {
      throw new MalformedInput(
        `${path}: not JSON: ${(error as Error).message}`
      );
    }

    const parsed_evidence = EvidenceFileSchema.safeParse(raw);
    if (!parsed_evidence.success) {
      const lines = parsed_evidence.error.issues.map(
        (issue) =>
          `  ${issue.path.join(".") || "<root>"}: ${issue.message}`
      );
      throw new MalformedInput(
        `${path}: schema violations:\n${lines.join("\n")}`
      );
    }

    return parsed_evidence.data;
  }

  const baseline = loadEvidence(baselinePath);
  const candidate = loadEvidence(candidatePath);

  const baselineViolations = validateEvidence(baseline, config.counters);
  const candidateViolations = validateEvidence(candidate, config.counters);

  if (baselineViolations.length > 0 || candidateViolations.length > 0) {
    io.err("evidence file validation failed:");
    if (baselineViolations.length > 0) {
      io.err(`${baselinePath}: ${baselineViolations.length} violation(s):`);
      for (const v of baselineViolations) io.err(`  [${v.kind}] ${v.detail}`);
    }
    if (candidateViolations.length > 0) {
      io.err(`${candidatePath}: ${candidateViolations.length} violation(s):`);
      for (const v of candidateViolations)
        io.err(`  [${v.kind}] ${v.detail}`);
    }
    io.err(`GATE EXIT=${EXIT.violations}`);
    return EXIT.violations;
  }

  const result = compareEvidence(baseline, candidate, config.counters);

  if (hasFlag(parsed, "json")) {
    io.out(
      JSON.stringify(
        {
          toolVersion: TOOL_VERSION,
          comparisons: result.comparisons.map((c) => ({
            signal: c.signal,
            baseline: c.baseline.value,
            candidate: c.candidate.value,
            delta: c.delta,
            ratio: c.ratio,
            environment: c.baseline.environment,
          })),
          pendingSignals: result.pendingSignals,
          violations: result.violations,
        },
        null,
        2
      )
    );
  } else {
    io.out(renderComparisonTable(result));
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
