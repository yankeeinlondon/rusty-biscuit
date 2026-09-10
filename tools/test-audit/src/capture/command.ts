/**
 * `capture`: run `cargo nextest list --message-format json` once per declared
 * selection and store the listing beside its stderr and a `captures.json`
 * record of revision, toolchain, platform, and the exact command. A listing
 * is the runner's own statement of what exists under that selection; the
 * reconciler treats it as the authority over source scans.
 *
 * Each selection is captured as `<enumeration>/<label>.json` plus
 * `<label>.err`. A non-zero cargo exit keeps the partial files (the stderr is
 * the diagnosis) and is reported as a failure rather than hidden.
 */
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { hasFlag, optionalString, parseArgv, requireString } from "../args.ts";
import type { Command, CommandIo } from "../command.ts";
import { loadConfig, type ResolvedConfig, type SelectionConfig } from "../config.ts";
import { EXIT, UsageError } from "../errors.ts";
import { TOOL_VERSION } from "../version.ts";

const USAGE = `usage: test-audit capture --config <audit.config.json> [--only <label>[,<label>...]] [--note <text>] [--dry-run]

Runs \`cargo nextest list --message-format json\` from the repository root for
every selection the configuration declares (or only the named labels) and
writes <enumeration>/<label>.json, <label>.err, and captures.json.

--note records one line of provenance in captures.json — why this listing was
taken where it was, which \`revision\` and \`dirty\` alone cannot say.`;

export interface CaptureRecord {
  label: string;
  packages: string[];
  features: string[];
  env?: Record<string, string>;
  command: string;
  exitCode: number;
  seconds: number;
  /** Test identities listed, counted from the capture; `null` when the run failed. */
  identities: number | null;
}

export interface CapturesManifest {
  toolVersion: string;
  revision: string;
  dirty: string[];
  /** Operator-supplied provenance for this listing; absent when none was given. */
  note?: string;
  toolchain: string;
  nextest: string;
  platform: string;
  capturedAt: string;
  captures: CaptureRecord[];
}

function probe(cmd: string, args: string[], cwd: string): string {
  try {
    return execFileSync(cmd, args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return "<unavailable>";
  }
}

export function selectionCommand(selection: SelectionConfig): string[] {
  const args = ["nextest", "list", "--message-format", "json"];
  for (const pkg of selection.packages) args.push("-p", pkg);
  if (selection.features.length > 0) args.push("--features", selection.features.join(","));
  return args;
}

/** Count `test-cases` entries across every suite in a nextest listing. */
export function countIdentities(listingJson: string): number {
  const listing = JSON.parse(listingJson) as {
    "test-count"?: number;
    "rust-suites"?: Record<string, { testcases?: Record<string, unknown> }>;
  };
  if (typeof listing["test-count"] === "number") return listing["test-count"];
  let count = 0;
  for (const suite of Object.values(listing["rust-suites"] ?? {})) {
    count += Object.keys(suite.testcases ?? {}).length;
  }
  return count;
}

/**
 * A note describes the revision it was taken at, so a partial re-capture of
 * that same revision inherits it rather than silently dropping the provenance.
 * A different revision drops it: the reason no longer applies, and a stale
 * explanation of a dirty tree is worse than none.
 */
export function noteFor(
  note: string | undefined,
  previous: Pick<CapturesManifest, "revision" | "note"> | undefined,
  revision: string,
): string | undefined {
  if (note !== undefined) return note;
  return previous?.revision === revision ? previous.note : undefined;
}

export function captureSelections(
  config: ResolvedConfig,
  labels: string[] | undefined,
  io: CommandIo,
  dryRun: boolean,
  note?: string,
): number {
  const selections = labels
    ? config.selections.filter((s) => labels.includes(s.label))
    : config.selections;
  if (labels) {
    const unknown = labels.filter((l) => !config.selections.some((s) => s.label === l));
    if (unknown.length > 0) throw new UsageError(`unknown selection label(s): ${unknown.join(", ")}`);
  }
  mkdirSync(config.enumerationDir, { recursive: true });
  const manifestPath = join(config.enumerationDir, "captures.json");
  const previous: CapturesManifest | undefined = existsSync(manifestPath)
    ? (JSON.parse(readFileSync(manifestPath, "utf8")) as CapturesManifest)
    : undefined;

  const revision = probe("git", ["rev-parse", "HEAD"], config.repoRoot);
  const dirty = probe("git", ["status", "--porcelain"], config.repoRoot)
    .split("\n")
    .filter((line) => line.length > 0);
  const carriedNote = noteFor(note, previous, revision);
  const manifest: CapturesManifest = {
    toolVersion: TOOL_VERSION,
    revision,
    dirty,
    ...(carriedNote === undefined ? {} : { note: carriedNote }),
    toolchain: probe("rustc", ["--version"], config.repoRoot),
    nextest: probe("cargo", ["nextest", "--version"], config.repoRoot).split("\n")[0] ?? "<unavailable>",
    platform: `${process.arch}-${process.platform} (${probe("uname", ["-sr"], config.repoRoot)})`,
    capturedAt: new Date().toISOString(),
    captures: previous?.revision === revision ? previous.captures.filter((c) => !selections.some((s) => s.label === c.label)) : [],
  };

  let failures = 0;
  for (const selection of selections) {
    const args = selectionCommand(selection);
    const command = `cargo ${args.join(" ")}`;
    io.err(`== ${selection.label}: ${command}`);
    if (dryRun) continue;
    const started = Date.now();
    const result = spawnSync("cargo", args, {
      cwd: config.repoRoot,
      encoding: "utf8",
      env: { ...process.env, ...(selection.env ?? {}) },
      maxBuffer: 1024 * 1024 * 512,
    });
    const seconds = Math.round((Date.now() - started) / 100) / 10;
    const exitCode = result.status ?? -1;
    writeFileSync(join(config.enumerationDir, `${selection.label}.json`), result.stdout ?? "");
    writeFileSync(join(config.enumerationDir, `${selection.label}.err`), result.stderr ?? "");
    let identities: number | null = null;
    if (exitCode === 0) {
      try {
        identities = countIdentities(result.stdout ?? "");
      } catch (error) {
        io.err(`   listing for ${selection.label} is not valid nextest JSON: ${(error as Error).message}`);
        failures += 1;
      }
    } else {
      failures += 1;
    }
    io.err(`== ${selection.label}: exit ${exitCode} in ${seconds}s${identities === null ? "" : `, ${identities} identities`}`);
    const record: CaptureRecord = {
      label: selection.label,
      packages: selection.packages,
      features: selection.features,
      command,
      exitCode,
      seconds,
      identities,
    };
    if (selection.env) record.env = selection.env;
    manifest.captures.push(record);
  }
  if (!dryRun) {
    manifest.captures.sort((a, b) => a.label.localeCompare(b.label));
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    io.out(`captured ${selections.length} selection(s) at ${revision.slice(0, 9)} → ${manifestPath}`);
  }
  return failures === 0 ? EXIT.clean : EXIT.violations;
}

export const captureCommand: Command = {
  name: "capture",
  summary: "capture `cargo nextest list` for every declared selection",
  usage: USAGE,
  run(argv, io) {
    const parsed = parseArgv(argv, ["dry-run"]);
    const config = loadConfig(requireString(parsed, "config", USAGE));
    const only = optionalString(parsed, "only");
    return captureSelections(
      config,
      only ? only.split(",").map((s) => s.trim()).filter(Boolean) : undefined,
      io,
      hasFlag(parsed, "dry-run"),
      optionalString(parsed, "note"),
    );
  },
};
