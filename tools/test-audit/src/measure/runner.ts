/**
 * Alternating-run driver for local measurement.
 *
 * Executes a plan's phases in order (warm-up, alternating, load), never
 * concurrently, and writes run manifest + gzipped logs. Records failing runs
 * exactly like passing ones (the gate is in the report).
 */

import { spawnSync, execFileSync } from "node:child_process";
import {
  appendFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { gzipSync } from "node:zlib";
import { join, resolve } from "node:path";
import { loadavg } from "node:os";

import { UsageError } from "../errors.ts";

interface Plan {
  revisions: Record<string, { root: string }>;
  suites: Record<
    string,
    { cwd: string; command: string[]; env?: Record<string, string> }
  >;
  phases: {
    role: "warmup" | "alternating" | "load";
    revisions: string[];
    suites: string[];
    rounds: number;
  }[];
  /** Environment variable prefixes to capture in provenance. */
  environmentPrefixes?: string[];
}

function sh(cmd: string, args: string[], cwd?: string): string {
  try {
    return execFileSync(cmd, args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    })
      .trim();
  } catch {
    return "<unavailable>";
  }
}

function loadavgString(): string {
  const avg = loadavg();
  return `{ ${avg[0]?.toFixed(2) ?? "—"} ${avg[1]?.toFixed(2) ?? "—"} ${avg[2]?.toFixed(2) ?? "—"} }`;
}

function cpuIdle(): string {
  if (process.platform !== "darwin") {
    return `<unavailable on ${process.platform}>`;
  }
  const out = sh("/usr/bin/top", ["-l", "2", "-n", "0", "-s", "1"]);
  const lines = out.split("\n").filter((l) => l.startsWith("CPU usage"));
  return lines.at(-1) ?? "<unavailable>";
}

function sysctlValue(key: string): string {
  if (process.platform !== "darwin") {
    return `<unavailable on ${process.platform}>`;
  }
  return sh("sysctl", ["-n", key]);
}

function slug(text: string): string {
  return text
    .replace(/[^a-z0-9]+/gi, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase();
}

export function run(planPath: string, outDir: string): number {
  const plan: Plan = JSON.parse(readFileSync(planPath, "utf8"));
  outDir = resolve(outDir);
  mkdirSync(join(outDir, "runs"), { recursive: true });

  const prefixes = plan.environmentPrefixes ?? [
    "NEXTEST_",
    "BISCUIT_",
    "CARGO_",
    "RUSTC_WRAPPER",
    "CLAUDINE_",
    "PLAYA_",
    "MODEL",
    "NO_COLOR",
    "FORCE_COLOR",
    "TERM",
  ];

  const environment: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (
      prefixes.some(
        (p) => key === p || (p.endsWith("_") && key.startsWith(p))
      )
    ) {
      environment[key] = value ?? "";
    }
  }

  const provenance = {
    started: new Date().toISOString(),
    host: {
      platform: process.platform,
      arch: process.arch,
      uname: sh("uname", ["-a"]),
      cpu:
        process.platform === "darwin"
          ? sysctlValue("machdep.cpu.brand_string")
          : "<platform-specific>",
      cores:
        process.platform === "darwin"
          ? sysctlValue("hw.ncpu")
          : "<platform-specific>",
      memoryBytes:
        process.platform === "darwin"
          ? sysctlValue("hw.memsize")
          : "<platform-specific>",
      os:
        process.platform === "darwin"
          ? sh("sw_vers", []).replace(/\s+/g, " ")
          : "<platform-specific>",
    },
    toolchain: {
      rustc: sh("rustc", ["--version"]),
      cargo: sh("cargo", ["--version"]),
      nextest: sh("cargo", ["nextest", "--version"]).split("\n")[0],
      just: sh("just", ["--version"]),
      node: process.version,
    },
    environment,
    profile:
      "default (NEXTEST_PROFILE unset; BISCUIT_CI_ENVIRONMENT unset)",
    revisions: Object.fromEntries(
      Object.entries(plan.revisions).map(([name, { root }]) => [
        name,
        {
          root,
          sha: sh("git", ["rev-parse", "HEAD"], root),
          branch: sh("git", ["rev-parse", "--abbrev-ref", "HEAD"], root),
          dirty: sh("git", ["status", "--short"], root)
            .split("\n")
            .filter(
              (line) =>
                line.trim().length > 0 && !/claudine\/fixes\//.test(line)
            ),
          targetDirPresent: existsSync(join(root, "target", "debug")),
        },
      ])
    ),
    plan,
  };

  for (const [name, rev] of Object.entries(provenance.revisions)) {
    if (!rev.targetDirPresent) {
      process.stderr.write(
        `refusing to start: ${name} has no warm target directory at ${rev.root}/target/debug\n`
      );
      return 1;
    }
  }

  writeFileSync(
    join(outDir, "provenance.json"),
    JSON.stringify(provenance, null, 2) + "\n"
  );

  const sequence = new Map<string, number>();
  const manifest = join(outDir, "runs.jsonl");

  for (const phase of plan.phases) {
    for (let round = 1; round <= phase.rounds; round += 1) {
      for (const suiteName of phase.suites) {
        for (const revision of phase.revisions) {
          const suite = plan.suites[suiteName];
          if (!suite) {
            throw new UsageError(`plan references unknown suite: ${suiteName}`);
          }
          const revisionSpec = plan.revisions[revision];
          if (!revisionSpec) {
            throw new UsageError(
              `plan references unknown revision: ${revision}`
            );
          }
          const root = revisionSpec.root;
          const key = `${suiteName}|${revision}`;
          const seq = (sequence.get(key) ?? 0) + 1;
          sequence.set(key, seq);
          const logName = `${slug(suiteName)}-${revision}-${seq}.log.gz`;
          const env: Record<string, string | undefined> = {
            ...process.env,
            ...(suite.env ?? {}),
            GIT_TERMINAL_PROMPT: "0",
          };
          delete env["NEXTEST_PROFILE"];
          delete env["BISCUIT_CI_ENVIRONMENT"];
          delete env["BISCUIT_TEST_FILTER"];

          const loadBefore = loadavgString();
          const idleBefore = cpuIdle();
          const started = new Date().toISOString();
          process.stderr.write(
            `[${started}] ${phase.role} ${suiteName} @ ${revision} #${seq} (load ${loadBefore})\n`
          );

          const t0 = performance.now();
          const result = spawnSync(suite.command[0]!, suite.command.slice(1), {
            cwd: join(root, suite.cwd),
            env,
            encoding: "buffer",
            maxBuffer: 1024 * 1024 * 1024,
            stdio: ["ignore", "pipe", "pipe"],
          });
          const wallS = (performance.now() - t0) / 1000;

          const combined = Buffer.concat([
            result.stdout ?? Buffer.alloc(0),
            Buffer.from("\n--- stderr ---\n"),
            result.stderr ?? Buffer.alloc(0),
          ]);
          writeFileSync(join(outDir, "runs", logName), gzipSync(combined));

          const record = {
            suite: suiteName,
            revision,
            sha: provenance.revisions[revision]!.sha,
            sequence: seq,
            role: phase.role,
            started,
            wallS: Number(wallS.toFixed(2)),
            exitCode: result.status ?? -1,
            signal: result.signal ?? null,
            log: `runs/${logName}`,
            loadavgBefore: loadBefore,
            loadavgAfter: loadavgString(),
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
