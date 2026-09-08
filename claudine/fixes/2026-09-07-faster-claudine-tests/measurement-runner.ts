#!/usr/bin/env -S npx tsx
/**
 * Alternating-run driver for Phase 8 (fix 2026-09-07-faster-claudine-tests).
 *
 * Executes the plan's phases in order — warm-up, then `baseline, candidate,
 * baseline, candidate, …` per suite, then the candidate-only load rounds —
 * one recipe at a time, never concurrently, and writes:
 *
 * - `<out>/provenance.json` — host, toolchain, and per-revision source state
 *   (`git rev-parse HEAD`, dirty files outside the fix directory) captured
 *   once before the first run;
 * - `<out>/runs.jsonl` — one record per run: suite, revision, sequence, role,
 *   wall time, exit code, load average before/after, CPU idle before, and the
 *   path of the gzipped recipe log;
 * - `<out>/runs/<suite>-<revision>-<seq>.log.gz` — the recipe's combined
 *   stdout/stderr, verbatim.
 *
 * It is an evidence generator, not a gate: it records a failing run exactly
 * like a passing one (the gate is `measurement.ts`). It refuses to start when
 * the target directories are missing, because a cold build inside a timed run
 * would land in the build/setup column and be mistaken for suite cost.
 *
 *   npx tsx measurement-runner.ts --plan measurement/plan.json --out measurement
 */
import { spawnSync, execFileSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { join, resolve } from "node:path";

interface Plan {
  revisions: Record<string, { root: string }>;
  suites: Record<string, { cwd: string; command: string[]; env?: Record<string, string> }>;
  phases: { role: "warmup" | "alternating" | "load"; revisions: string[]; suites: string[]; rounds: number }[];
}

function sh(cmd: string, args: string[], cwd?: string): string {
  try {
    return execFileSync(cmd, args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return "<unavailable>";
  }
}

function loadavg(): string {
  return sh("sysctl", ["-n", "vm.loadavg"]);
}

function cpuIdle(): string {
  // The second `top` sample is the first true delta; the first is a lifetime average.
  const out = sh("/usr/bin/top", ["-l", "2", "-n", "0", "-s", "1"]);
  const lines = out.split("\n").filter((l) => l.startsWith("CPU usage"));
  return lines.at(-1) ?? "<unavailable>";
}

function slug(text: string): string {
  return text.replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "").toLowerCase();
}

function main(argv: string[]): number {
  let planPath = "";
  let outDir = "";
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--plan") planPath = argv[++i];
    else if (argv[i] === "--out") outDir = argv[++i];
  }
  if (!planPath || !outDir) {
    process.stderr.write("usage: measurement-runner.ts --plan <plan.json> --out <dir>\n");
    return 2;
  }
  const plan: Plan = JSON.parse(readFileSync(planPath, "utf8"));
  outDir = resolve(outDir);
  mkdirSync(join(outDir, "runs"), { recursive: true });

  const environment: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (/^(NEXTEST_|BISCUIT_|CARGO_|RUSTC_WRAPPER|CLAUDINE_|PLAYA_|MODEL$|NO_COLOR|FORCE_COLOR|TERM$)/.test(key)) {
      environment[key] = value ?? "";
    }
  }
  const provenance = {
    started: new Date().toISOString(),
    host: {
      uname: sh("uname", ["-a"]),
      cpu: sh("sysctl", ["-n", "machdep.cpu.brand_string"]),
      cores: sh("sysctl", ["-n", "hw.ncpu"]),
      memoryBytes: sh("sysctl", ["-n", "hw.memsize"]),
      os: sh("sw_vers", []).replace(/\s+/g, " "),
    },
    toolchain: {
      rustc: sh("rustc", ["--version"]),
      cargo: sh("cargo", ["--version"]),
      nextest: sh("cargo", ["nextest", "--version"]).split("\n")[0],
      just: sh("just", ["--version"]),
      node: process.version,
    },
    environment,
    profile: "default (NEXTEST_PROFILE unset; BISCUIT_CI_ENVIRONMENT unset)",
    revisions: Object.fromEntries(
      Object.entries(plan.revisions).map(([name, { root }]) => [
        name,
        {
          root,
          sha: sh("git", ["rev-parse", "HEAD"], root),
          branch: sh("git", ["rev-parse", "--abbrev-ref", "HEAD"], root),
          dirty: sh("git", ["status", "--short"], root)
            .split("\n")
            .filter((line) => line.trim().length > 0 && !/claudine\/fixes\//.test(line)),
          targetDirPresent: existsSync(join(root, "target", "debug")),
        },
      ]),
    ),
    plan,
  };
  for (const [name, rev] of Object.entries(provenance.revisions)) {
    if (!rev.targetDirPresent) {
      process.stderr.write(`refusing to start: ${name} has no warm target directory at ${rev.root}/target/debug\n`);
      return 1;
    }
  }
  writeFileSync(join(outDir, "provenance.json"), JSON.stringify(provenance, null, 2) + "\n");

  const sequence = new Map<string, number>();
  const manifest = join(outDir, "runs.jsonl");
  for (const phase of plan.phases) {
    for (let round = 1; round <= phase.rounds; round += 1) {
      for (const suiteName of phase.suites) {
        for (const revision of phase.revisions) {
          const suite = plan.suites[suiteName];
          const root = plan.revisions[revision].root;
          const key = `${suiteName}|${revision}`;
          const seq = (sequence.get(key) ?? 0) + 1;
          sequence.set(key, seq);
          const logName = `${slug(suiteName)}-${revision}-${seq}.log.gz`;
          const env = { ...process.env, ...(suite.env ?? {}), GIT_TERMINAL_PROMPT: "0" };
          delete env.NEXTEST_PROFILE;
          delete env.BISCUIT_CI_ENVIRONMENT;
          delete env.BISCUIT_TEST_FILTER;
          const loadBefore = loadavg();
          const idleBefore = cpuIdle();
          const started = new Date().toISOString();
          process.stderr.write(`[${started}] ${phase.role} ${suiteName} @ ${revision} #${seq} (load ${loadBefore})\n`);
          const t0 = performance.now();
          const result = spawnSync(suite.command[0], suite.command.slice(1), {
            cwd: join(root, suite.cwd),
            env,
            encoding: "buffer",
            maxBuffer: 1024 * 1024 * 1024,
            stdio: ["ignore", "pipe", "pipe"],
          });
          const wallS = (performance.now() - t0) / 1000;
          const combined = Buffer.concat([result.stdout ?? Buffer.alloc(0), Buffer.from("\n--- stderr ---\n"), result.stderr ?? Buffer.alloc(0)]);
          writeFileSync(join(outDir, "runs", logName), gzipSync(combined));
          const record = {
            suite: suiteName,
            revision,
            sha: provenance.revisions[revision].sha,
            sequence: seq,
            role: phase.role,
            started,
            wallS: Number(wallS.toFixed(2)),
            exitCode: result.status ?? -1,
            signal: result.signal ?? null,
            log: `runs/${logName}`,
            loadavgBefore: loadBefore,
            loadavgAfter: loadavg(),
            cpuIdleBefore: idleBefore,
            cwd: join(root, suite.cwd),
            command: suite.command,
          };
          appendFileSync(manifest, JSON.stringify(record) + "\n");
          process.stderr.write(`    exit ${record.exitCode} in ${record.wallS}s\n`);
        }
      }
    }
  }
  process.stderr.write("done\n");
  return 0;
}

process.exit(main(process.argv.slice(2)));
