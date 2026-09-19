/**
 * `fetch`: assemble one CI run's JUnit staging trees, one directory per
 * environment, in the layout `_stage_junit` uploads (`<tier>/<package>.xml`
 * beside an appended `manifest.jsonl`). Each GitHub artifact carries exactly
 * one (package, tier, environment) cell, so per-cell manifests are
 * concatenated per environment; `status-*` artifacts land under `status/`.
 *
 * The cells to fetch are the configuration's environment cells. A cell whose
 * artifact does not exist is recorded in `missing.jsonl` and reported; the
 * `junit` gate then decides whether that absence is a violation or a pending
 * leg. Requires the `gh` CLI with repository access.
 */
import { spawnSync } from "node:child_process";
import { appendFileSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import { hasFlag, optionalString, parseArgv, requireString } from "../args.ts";
import type { Command, CommandIo } from "../command.ts";
import { loadConfig, type ResolvedConfig } from "../config.ts";
import { EXIT, UsageError } from "../errors.ts";
import { TOOL_VERSION } from "../version.ts";

const USAGE = `usage: test-audit fetch --config <audit.config.json> --run <run-id> [--repo <owner/name>] [--out <dir>] [--only <env>[,<env>...]] [--include-pending]

Downloads junit-<package>-<tier>-<environment> and status-* artifacts for every
cell the configuration declares and assembles <out>/<environment>/ staging
trees. <out> defaults to <evidenceDir>/baseline/<run-id>.`;

export interface FetchSummary {
  toolVersion: string;
  run: string;
  repo: string;
  fetchedAt: string;
  environments: Record<string, { cells: number; reports: number; missing: string[] }>;
}

function artifactName(kind: "junit" | "status", pkg: string, tier: string, environment: string): string {
  return `${kind}-${pkg}-${tier}-${environment}`;
}

function download(repo: string, run: string, name: string, dest: string): boolean {
  const result = spawnSync("gh", ["run", "download", run, "-R", repo, "-n", name, "-D", dest], {
    encoding: "utf8",
    env: { ...process.env, GH_PROMPT_DISABLED: "1", GH_NO_UPDATE_NOTIFIER: "1" },
  });
  return result.status === 0;
}

export function assembleCell(artifactDir: string, out: string, tier: string, pkg: string): number {
  let reports = 0;
  const xml = join(artifactDir, tier, `${pkg}.xml`);
  if (existsSync(xml)) {
    mkdirSync(join(out, tier), { recursive: true });
    copyFileSync(xml, join(out, tier, `${pkg}.xml`));
    reports += 1;
  }
  for (const entry of readdirSync(artifactDir)) {
    if (entry.endsWith(".jsonl")) appendFileSync(join(out, entry), readFileSync(join(artifactDir, entry), "utf8"));
  }
  return reports;
}

export function fetchRun(config: ResolvedConfig, run: string, repo: string, out: string, only: string[] | undefined, includePending: boolean, io: CommandIo): number {
  const environments = config.environments.filter((e) => (includePending || !e.pending) && (!only || only.includes(e.name)));
  if (environments.length === 0) throw new UsageError("no environments selected");
  mkdirSync(out, { recursive: true });
  const scratch = mkdtempSync(join(tmpdir(), "test-audit-fetch-"));
  const summary: FetchSummary = { toolVersion: TOOL_VERSION, run, repo, fetchedAt: new Date().toISOString(), environments: {} };
  let missingTotal = 0;
  try {
    for (const environment of environments) {
      const envOut = join(out, environment.name);
      rmSync(envOut, { recursive: true, force: true });
      mkdirSync(envOut, { recursive: true });
      writeFileSync(join(envOut, "manifest.jsonl"), "");
      const record = { cells: environment.cells.length, reports: 0, missing: [] as string[] };
      for (const cell of environment.cells) {
        for (const kind of ["junit", "status"] as const) {
          const name = artifactName(kind, cell.package, cell.tier, environment.name);
          const dest = join(scratch, name);
          if (!download(repo, run, name, dest)) {
            record.missing.push(name);
            appendFileSync(join(envOut, "missing.jsonl"), `${JSON.stringify({ tier: cell.tier, package: cell.package, environment: environment.name, artifact: name })}\n`);
            continue;
          }
          if (kind === "junit") {
            record.reports += assembleCell(dest, envOut, cell.tier, cell.package);
          } else {
            mkdirSync(join(envOut, "status"), { recursive: true });
            for (const entry of readdirSync(dest)) {
              if (entry.endsWith(".json")) copyFileSync(join(dest, entry), join(envOut, "status", `${cell.package}-${cell.tier}.json`));
            }
          }
        }
      }
      missingTotal += record.missing.length;
      summary.environments[environment.name] = record;
      io.err(`${environment.name}: ${record.reports} report(s) for ${record.cells} cell(s)${record.missing.length > 0 ? `, missing ${record.missing.join(", ")}` : ""}`);
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  writeFileSync(join(out, "fetch.json"), `${JSON.stringify(summary, null, 2)}\n`);
  io.out(`assembled run ${run} from ${repo} → ${out}${missingTotal > 0 ? ` (${missingTotal} artifact(s) missing)` : ""}`);
  return missingTotal === 0 ? EXIT.clean : EXIT.violations;
}

export const fetchCommand: Command = {
  name: "fetch",
  summary: "assemble a CI run's JUnit staging trees per environment via gh",
  usage: USAGE,
  run(argv, io) {
    const parsed = parseArgv(argv, ["include-pending"]);
    const config = loadConfig(requireString(parsed, "config", USAGE));
    const run = requireString(parsed, "run", USAGE);
    const repo = optionalString(parsed, "repo") ?? "yankeeinlondon/rusty-biscuit";
    const out = optionalString(parsed, "out") ?? join(config.evidenceRoot, "baseline", run);
    const only = optionalString(parsed, "only");
    return fetchRun(config, run, repo, out, only ? only.split(",").map((s) => s.trim()).filter(Boolean) : undefined, hasFlag(parsed, "include-pending"), io);
  },
};
