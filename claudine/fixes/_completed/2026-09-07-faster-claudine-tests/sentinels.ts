#!/usr/bin/env -S npx tsx
/**
 * Work counters and sentinel effects for Phase 8 (fix 2026-09-07-faster-claudine-tests).
 *
 * Every eliminated-work claim Phases 5–7 made is checked here by counting the
 * work, not by timing it. The counters are lldb breakpoint hit counts on the
 * exact function each claim says is no longer called, taken on the same test
 * binaries the suite runs, at both revisions:
 *
 * - `capture_file_resolution_context` / `GitRepo::discover` /
 *   `resolve_repo_root` while the four redirected lib modules run in one
 *   process (`--test-threads=1`);
 * - `run_scan` — the `OnceLock` initializer behind `scan_production_sources`
 *   — once per `error_guards` identity in its own process, which is how
 *   nextest runs it, summed across identities;
 * - a `git` shim on `PATH` that logs every parent-side `git` invocation with
 *   its working directory while `context_command` runs, so the launch origin
 *   of each fixture repository is a recorded path rather than an inference.
 *
 * The output is `measurement/sentinels/*.txt` (raw lldb / shim transcripts)
 * plus `measurement/sentinels/summary.tsv`. Exit 1 when a counter could not
 * be taken (lldb failed to launch, a test process did not exit 0, a binary
 * could not be resolved); a missing counter is a missing proof.
 *
 *   npx tsx sentinels.ts --baseline /tmp/rb-baseline-9fc5151a0 \
 *       --candidate /path/to/worktree --out measurement/sentinels
 */
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

interface Revision {
  name: string;
  root: string;
}

const LIB_MODULES = [
  "composition::schema::",
  "composition::sequence::preflight::",
  "composition::sequence::task::",
  "linking::paths::",
];

const LIB_SYMBOLS = ["capture_file_resolution_context", "GitRepo.*::discover", "resolve_repo_root"];

/**
 * The source file that defines each counted function. A regex breakpoint on a
 * bare name can also resolve a same-named function in another crate
 * (darkmatter carries its own `capture_file_resolution_context`), so only the
 * entry location in the defining file is counted; every other entry location
 * is still listed in the record.
 */
const DEFINING_FILE: Record<string, string> = {
  capture_file_resolution_context: "resolve.rs",
  discover: "types.rs",
  resolve_repo_root: "paths.rs",
  run_scan: "source_scan.rs",
};

let failures = 0;
const rows: string[] = ["sentinel\trevision\tsubject\tcounter\tvalue\tnote"];

function fail(message: string): void {
  failures += 1;
  process.stderr.write(`FAIL: ${message}\n`);
}

/**
 * The package selection `just test` hands nextest (`_test_local_all`), so the
 * artifacts resolved here are the ones the measured suite ran — a narrower
 * selection unifies features differently and can name a different binary.
 */
const SUITE_PACKAGES = ["claudine-catalog-types", "claudine", "claudine-contract", "claudine-cli", "claudine-gen"];

/** Resolve a test executable without rebuilding anything the suite did not build. */
function testExecutable(root: string, targetName: string, targetKind: string): string | undefined {
  const args = SUITE_PACKAGES.flatMap((p) => ["-p", p]);
  const out = spawnSync("cargo", ["test", "--no-run", "--message-format=json", ...args], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
    env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
  });
  if (out.status !== 0) {
    fail(`cargo test --no-run at ${root}: exit ${out.status}\n${out.stderr.slice(-2000)}`);
    return undefined;
  }
  const compiled = (out.stderr.match(/^\s*Compiling /gm) ?? []).length;
  if (compiled > 0) {
    fail(`cargo test --no-run at ${root} compiled ${compiled} crate(s); the resolved binary is not the one the suite ran`);
  }
  for (const line of out.stdout.split("\n")) {
    if (!line.startsWith("{")) continue;
    const message = JSON.parse(line);
    if (
      message.reason === "compiler-artifact" &&
      typeof message.executable === "string" &&
      message.target?.name === targetName &&
      Array.isArray(message.target?.kind) &&
      message.target.kind.includes(targetKind)
    ) {
      return message.executable;
    }
  }
  fail(`no executable reported for ${targetName} (${targetKind}) at ${root}`);
  return undefined;
}

interface LldbRun {
  transcript: string;
  /** Hits at the function's own entry locations, keyed by the symbol's base name. */
  hits: Map<string, number>;
  /** The entry locations behind each count, `file:line = hits`, for the record. */
  locations: Map<string, string[]>;
  exitStatus: number | undefined;
}

/** libtest colours its summary when lldb hands it a terminal. */
function stripAnsi(text: string): string {
  return text.replace(/\x1b\[[0-9;]*m/g, "").replace(/\x1b\(B/g, "");
}

/**
 * Read an lldb transcript: process exit status plus, per breakpoint, the hit
 * counts at the *entry* locations of the named function. A regex breakpoint
 * also lands on the closures and generic wrappers the function instantiates
 * (`map<…, capture_file_resolution_context::{closure_env#1}>`), which fire
 * once per call each and would inflate an aggregate; the entry location is the
 * one whose `where` starts with the bare function name.
 */
export function parseLldbTranscript(transcript: string, symbols: string[]): Omit<LldbRun, "transcript"> {
  const clean = stripAnsi(transcript);
  const hits = new Map<string, number>();
  const locations = new Map<string, string[]>();
  let exitStatus: number | undefined;
  const exitMatch = /Process \d+ exited with status = (\d+)/.exec(clean);
  if (exitMatch) exitStatus = Number(exitMatch[1]);
  const baseNames = symbols.map((s) => s.replace(/^.*::/, "").replace(/[^A-Za-z0-9_]/g, ""));
  for (const [index, symbol] of symbols.entries()) {
    const base = baseNames[index];
    let total = 0;
    const where: string[] = [];
    const re = /^\s+\d+\.\d+: where = [^`]*`([A-Za-z0-9_]+) \+ \d+ at ([^:\s]+):(\d+):\d+, .*hit count = (\d+)/gm;
    let m: RegExpExecArray | null;
    while ((m = re.exec(clean)) !== null) {
      if (m[1] !== base) continue;
      const counted = DEFINING_FILE[base] === undefined || m[2] === DEFINING_FILE[base];
      if (counted) total += Number(m[4]);
      where.push(`${m[2]}:${m[3]} = ${m[4]}${counted ? "" : " (other crate, not counted)"}`);
    }
    hits.set(symbol, total);
    locations.set(symbol, where);
  }
  return { hits, locations, exitStatus };
}

/**
 * Launch `binary` under lldb with an auto-continuing breakpoint per symbol
 * regex, and read the entry-location hit counts after the process exits. A
 * transcript already at `transcriptPath` is re-read instead of re-run, so a
 * parser fix never needs the debugger again.
 */
function lldbCount(
  binary: string,
  symbols: string[],
  processArgs: string[],
  cwd: string,
  env: NodeJS.ProcessEnv,
  transcriptPath: string,
): LldbRun {
  let transcript: string;
  if (!FRESH && existsSync(transcriptPath)) {
    transcript = readFileSync(transcriptPath, "utf8");
  } else {
    const commands: string[] = [];
    for (const symbol of symbols) {
      commands.push("-o", `breakpoint set -r '${symbol}' --auto-continue true`);
    }
    commands.push("-o", `process launch -- ${processArgs.join(" ")}`);
    commands.push("-o", "breakpoint list");
    const out = spawnSync("xcrun", ["lldb", "--batch", ...commands, "--", binary], {
      cwd,
      env,
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    });
    transcript = `${out.stdout}\n--- stderr ---\n${out.stderr}`;
    writeFileSync(transcriptPath, transcript);
  }
  return { transcript, ...parseLldbTranscript(transcript, symbols) };
}

let FRESH = false;

function listTests(binary: string): string[] {
  return execFileSync(binary, ["--list", "--format", "terse"], { encoding: "utf8" })
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.endsWith(": test"))
    .map((line) => line.slice(0, -": test".length));
}

function main(argv: string[]): number {
  let baseline = "";
  let candidate = "";
  let outDir = "";
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--baseline") baseline = argv[++i];
    else if (argv[i] === "--candidate") candidate = argv[++i];
    else if (argv[i] === "--out") outDir = argv[++i];
    else if (argv[i] === "--fresh") FRESH = true;
  }
  if (!baseline || !candidate || !outDir) {
    process.stderr.write("usage: sentinels.ts --baseline <root> --candidate <root> --out <dir>\n");
    return 2;
  }
  outDir = resolve(outDir);
  mkdirSync(outDir, { recursive: true });
  const revisions: Revision[] = [
    { name: "baseline", root: resolve(baseline) },
    { name: "candidate", root: resolve(candidate) },
  ];
  const quietEnv = { ...process.env, GIT_TERMINAL_PROMPT: "0", NO_COLOR: "1" };

  // --- S1: ambient repository discovery inside the redirected lib modules ---
  for (const rev of revisions) {
    const bin = testExecutable(rev.root, "claudine", "lib");
    if (!bin) continue;
    for (const module of LIB_MODULES) {
      const slug = module.replace(/::/g, "-").replace(/-$/, "");
      const run = lldbCount(
        bin,
        LIB_SYMBOLS,
        [module, "--test-threads=1"],
        join(rev.root, "claudine", "lib"),
        quietEnv,
        join(outDir, `s1-${rev.name}-${slug}.txt`),
      );
      const passed = /test result: ok\. (\d+) passed; 0 failed/.exec(stripAnsi(run.transcript));
      if (run.exitStatus !== 0 || !passed) {
        fail(`S1 ${rev.name} ${module}: process exit ${run.exitStatus ?? "unknown"}, tests ${passed ? passed[1] : "not reported"}`);
      }
      for (const symbol of LIB_SYMBOLS) {
        const value = run.hits.get(symbol) ?? 0;
        const where = run.locations.get(symbol) ?? [];
        if (where.length === 0) fail(`S1 ${rev.name} ${module}: no entry location resolved for ${symbol}`);
        rows.push(`S1 ambient discovery\t${rev.name}\t${module} (${passed ? passed[1] : "?"} tests, one process)\t${symbol}\t${value}\t${where.join("; ")}`);
      }
    }
  }

  // --- S2: one production scan per error_guards process ----------------------
  for (const rev of revisions) {
    const bin = testExecutable(rev.root, "error_guards", "test");
    if (!bin) continue;
    const identities = listTests(bin);
    let scans = 0;
    const perIdentity: string[] = [];
    for (const identity of identities) {
      const run = lldbCount(
        bin,
        ["run_scan"],
        ["--exact", identity, "--test-threads=1"],
        join(rev.root, "claudine", "cli"),
        quietEnv,
        join(outDir, `s2-${rev.name}-${identity}.txt`),
      );
      const hits = run.hits.get("run_scan") ?? 0;
      const where = run.locations.get("run_scan") ?? [];
      if (run.exitStatus !== 0 || where.length === 0) {
        fail(`S2 ${rev.name} ${identity}: exit ${run.exitStatus ?? "unknown"}, entry locations ${where.length}`);
      }
      scans += hits;
      perIdentity.push(`${identity}\t${hits}\t${run.exitStatus ?? "?"}\t${where.join("; ")}`);
    }
    writeFileSync(join(outDir, `s2-${rev.name}-per-identity.tsv`), `identity\trun_scan_hits\texit\tentry_locations\n${perIdentity.join("\n")}\n`);
    rows.push(`S2 error_guards scans\t${rev.name}\t${identities.length} identities, one process each\trun_scan hits summed over processes\t${scans}\t`);
    rows.push(`S2 error_guards identities\t${rev.name}\terror_guards\tidentities\t${identities.length}\t`);
  }

  // --- S3: where context_command's repositories are built ------------------
  const shimDir = join(outDir, "git-shim");
  mkdirSync(shimDir, { recursive: true });
  const shim = join(shimDir, "git");
  writeFileSync(
    shim,
    `#!/bin/bash\n# Phase 8 sentinel: log every parent-side git invocation, then run the real one.\nprintf '%s\\t%s\\t%s\\n' "$$" "$PWD" "$*" >> "\${RB_GIT_SHIM_LOG:?}"\nexec /usr/bin/git "$@"\n`,
  );
  chmodSync(shim, 0o755);
  for (const rev of revisions) {
    const logPath = join(outDir, `s3-${rev.name}-git-shim.tsv`);
    const recipeLog = join(outDir, `s3-${rev.name}-context_command.log`);
    let stdout: string;
    let status: number | null;
    if (!FRESH && existsSync(logPath) && existsSync(recipeLog)) {
      stdout = readFileSync(recipeLog, "utf8");
      status = /GATE EXIT=(\d+)/.exec(stdout) ? Number(/GATE EXIT=(\d+)/.exec(stdout)![1]) : 0;
    } else {
      writeFileSync(logPath, "");
      const env = { ...quietEnv, PATH: `${shimDir}:${process.env.PATH ?? ""}`, RB_GIT_SHIM_LOG: logPath };
      delete env.NEXTEST_PROFILE;
      delete env.BISCUIT_CI_ENVIRONMENT;
      const out = spawnSync("just", ["test-cli", "--test", "context_command"], {
        cwd: join(rev.root, "claudine"),
        env,
        encoding: "utf8",
        maxBuffer: 256 * 1024 * 1024,
      });
      stdout = out.stdout;
      status = out.status;
      writeFileSync(recipeLog, `${out.stdout}\n--- stderr ---\n${out.stderr}\nGATE EXIT=${out.status}\n`);
    }
    if (status !== 0) fail(`S3 ${rev.name}: just test-cli --test context_command exited ${status}`);
    const lines = readFileSync(logPath, "utf8").split("\n").filter((l) => l.length > 0);
    const inits = lines.filter((l) => /\tinit(\s|$)/.test(l));
    const realRoot = rev.root.replace(/^\/private/, "");
    const insideCheckout = inits.filter((l) => {
      const pwd = l.split("\t")[1].replace(/^\/private/, "");
      return pwd === realRoot || pwd.startsWith(`${realRoot}/`);
    });
    const summary = /(\d+) tests run: (\d+) passed/.exec(stdout);
    rows.push(`S3 context_command git init\t${rev.name}\t${summary ? summary[0] : "?"}\tgit init invocations (parent side)\t${inits.length}\t${insideCheckout.length} inside the checkout`);
    rows.push(`S3 context_command git any\t${rev.name}\tcontext_command\tall git invocations logged\t${lines.length}\t`);
    const sites = readFileSync(join(rev.root, "claudine", "cli", "tests", "context_command.rs"), "utf8");
    rows.push(`S3 context_command source\t${rev.name}\tcontext_command.rs\tcurrent_dir(repo_root()) launch sites\t${(sites.match(/repo_root\(\)/g) ?? []).length}\t`);
    rows.push(`S3 context_command source\t${rev.name}\tcontext_command.rs\trepository_fixture() call sites\t${(sites.match(/repository_fixture\(\)/g) ?? []).length}\t`);
  }

  writeFileSync(join(outDir, "summary.tsv"), rows.join("\n") + "\n");
  process.stdout.write(rows.join("\n") + "\n");
  process.stderr.write(`SENTINELS EXIT=${failures === 0 ? 0 : 1}\n`);
  return failures === 0 ? 0 : 1;
}

process.exit(main(process.argv.slice(2)));
