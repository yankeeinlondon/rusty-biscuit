#!/usr/bin/env bun
/**
 * Monitor the current branch's CI run and delegate repairs until one complete
 * run passes. Production mode exits successfully only after both the workflow
 * and its ci-verdict job succeed with no bad job conclusions.
 *
 * Usage:
 *   .claudine/scripts/ci-watchdog.ts
 *   .claudine/scripts/ci-watchdog.ts --dry-run
 *   .claudine/scripts/ci-watchdog.ts --poll-seconds 60
 */

import { spawnSync } from "node:child_process";
import {
    existsSync,
    mkdirSync,
    readFileSync,
    rmSync,
    writeFileSync,
} from "node:fs";
import { join } from "node:path";

type CommandResult = {
    status: number;
    stdout: string;
    stderr: string;
};

type WorkflowRun = {
    databaseId: number;
    status: string;
    conclusion: string;
    headSha: string;
    event: string;
    createdAt: string;
    updatedAt: string;
    url: string;
};

type WorkflowJob = {
    id: number;
    name: string;
    status: string;
    conclusion: string | null;
    html_url: string;
};

type JobsPage = {
    total_count: number;
    jobs: WorkflowJob[];
};

const BAD_CONCLUSIONS = new Set([
    "action_required",
    "cancelled",
    "failure",
    "stale",
    "startup_failure",
    "timed_out",
]);

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const help = args.includes("--help") || args.includes("-h");

function option(name: string): string | undefined {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : undefined;
}

function positiveInteger(value: string | undefined, fallback: number): number {
    if (value === undefined) return fallback;
    const parsed = Number(value);
    if (!Number.isSafeInteger(parsed) || parsed <= 0) {
        throw new Error(`Expected a positive integer, received: ${value}`);
    }
    return parsed;
}

const pollSeconds = positiveInteger(
    option("--poll-seconds") ?? process.env.CI_WATCHDOG_POLL_SECONDS,
    30 * 60,
);
const apiRetries = positiveInteger(process.env.CI_WATCHDOG_API_RETRIES, 5);
const apiRetrySeconds = positiveInteger(
    process.env.CI_WATCHDOG_API_RETRY_SECONDS,
    10,
);
const ghTimeoutMs =
    positiveInteger(process.env.CI_WATCHDOG_GH_TIMEOUT_SECONDS, 60) * 1000;

if (help) {
    console.log(`Usage: ci-watchdog.ts [--dry-run] [--poll-seconds N]

Production mode polls every 30 minutes by default, delegates a repair cycle
when a CI failure appears, and exits successfully only after a complete green
run. --dry-run inspects once without canceling CI, invoking an agent, or
sleeping.`);
    process.exit(0);
}

function log(message: string): void {
    console.log(`[${new Date().toISOString()}] ${message}`);
}

function run(
    command: string,
    commandArgs: string[],
    options: {
        cwd?: string;
        inherit?: boolean;
        env?: NodeJS.ProcessEnv;
        timeoutMs?: number;
    } = {},
): CommandResult {
    const result = spawnSync(command, commandArgs, {
        cwd: options.cwd,
        encoding: "utf8",
        env: options.env ?? process.env,
        maxBuffer: 64 * 1024 * 1024,
        stdio: options.inherit ? "inherit" : "pipe",
        timeout: options.timeoutMs,
    });
    if (result.error) throw result.error;
    return {
        status: result.status ?? 1,
        stdout: result.stdout ?? "",
        stderr: result.stderr ?? "",
    };
}

function requireSuccess(
    command: string,
    commandArgs: string[],
    cwd?: string,
): string {
    const result = run(command, commandArgs, { cwd });
    if (result.status !== 0) {
        const detail = result.stderr.trim() || result.stdout.trim();
        throw new Error(
            `${command} ${commandArgs.join(" ")} failed: ${detail}`,
        );
    }
    return result.stdout.trim();
}

function parseJson<T>(value: string, label: string): T {
    try {
        return JSON.parse(value) as T;
    } catch (error) {
        throw new Error(`Invalid JSON from ${label}: ${String(error)}`);
    }
}

const repoRoot = requireSuccess("git", ["rev-parse", "--show-toplevel"]);
const branch = requireSuccess("git", ["branch", "--show-current"], repoRoot);
if (!branch) throw new Error("CI watchdog requires a named Git branch");

const hostHome = process.env.CI_WATCHDOG_USER_HOME ?? "/Users/ken";
const runtimeEnv: NodeJS.ProcessEnv = {
    ...process.env,
    GH_CONFIG_DIR:
        process.env.CI_WATCHDOG_GH_CONFIG_DIR ??
        join(hostHome, ".config", "gh"),
    GIT_TERMINAL_PROMPT: "0",
    GNUPGHOME: process.env.CI_WATCHDOG_GNUPGHOME ?? join(hostHome, ".gnupg"),
};

function gh(commandArgs: string[]): CommandResult {
    return run("gh", commandArgs, {
        cwd: repoRoot,
        env: runtimeEnv,
        timeoutMs: ghTimeoutMs,
    });
}

function ghJson<T>(commandArgs: string[]): T {
    const result = gh(commandArgs);
    if (result.status !== 0) {
        throw new Error(result.stderr.trim() || result.stdout.trim());
    }
    return parseJson<T>(result.stdout, `gh ${commandArgs.join(" ")}`);
}

let repo = "";

const runtimeDir = join(repoRoot, ".claudine", "tmp", "ci-watchdog");
const lockDir = join(repoRoot, ".claudine", "tmp", "ci-watchdog.lock");
let ownsLock = false;

function acquireLock(): void {
    mkdirSync(runtimeDir, { recursive: true });
    try {
        mkdirSync(lockDir);
    } catch {
        const pidFile = join(lockDir, "pid");
        const pid = existsSync(pidFile)
            ? Number(readFileSync(pidFile, "utf8").trim())
            : Number.NaN;
        if (Number.isInteger(pid)) {
            try {
                process.kill(pid, 0);
                throw new Error(`CI watchdog is already running as PID ${pid}`);
            } catch (error) {
                if (
                    error instanceof Error &&
                    error.message.includes("already running")
                ) {
                    throw error;
                }
            }
        }
        rmSync(lockDir, { recursive: true, force: true });
        mkdirSync(lockDir);
    }
    writeFileSync(join(lockDir, "pid"), `${process.pid}\n`);
    ownsLock = true;
}

function releaseLock(): void {
    if (!ownsLock) return;
    rmSync(lockDir, { recursive: true, force: true });
    ownsLock = false;
}

function stop(signal: NodeJS.Signals): never {
    log(`Received ${signal}; stopping without claiming CI success.`);
    releaseLock();
    process.exit(signal === "SIGINT" ? 130 : 143);
}

process.on("SIGINT", () => stop("SIGINT"));
process.on("SIGTERM", () => stop("SIGTERM"));
process.on("exit", releaseLock);

function sleep(seconds: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}

async function ghApi<T>(endpoint: string): Promise<T> {
    let lastError = "unknown GitHub API error";
    for (let attempt = 1; attempt <= apiRetries; attempt += 1) {
        const result = gh(["api", endpoint]);
        if (result.status === 0) {
            return parseJson<T>(result.stdout, endpoint);
        }
        lastError = result.stderr.trim() || result.stdout.trim();
        log(`GitHub API attempt ${attempt}/${apiRetries} failed: ${lastError}`);
        if (attempt < apiRetries) await sleep(apiRetrySeconds);
    }
    throw new Error(lastError);
}

function remoteHead(): string {
    const result = run(
        "git",
        ["ls-remote", "--heads", "origin", `refs/heads/${branch}`],
        { cwd: repoRoot, env: runtimeEnv, timeoutMs: ghTimeoutMs },
    );
    if (result.status !== 0) {
        throw new Error(result.stderr.trim() || result.stdout.trim());
    }
    const output = result.stdout.trim();
    const sha = output.split(/\s+/)[0];
    if (!/^[0-9a-f]{40}$/.test(sha)) {
        throw new Error(`Unable to resolve origin/${branch}`);
    }
    return sha;
}

function currentRun(sha: string): WorkflowRun | undefined {
    const runs = ghJson<WorkflowRun[]>([
        "run",
        "list",
        "--repo",
        repo,
        "--workflow",
        "ci.yml",
        "--branch",
        branch,
        "--limit",
        "20",
        "--json",
        "databaseId,status,conclusion,headSha,event,createdAt,updatedAt,url",
    ]);
    return runs.find((run) => run.headSha === sha);
}

async function jobsFor(runId: number): Promise<WorkflowJob[]> {
    const first = await ghApi<JobsPage>(
        `repos/${repo}/actions/runs/${runId}/jobs?filter=latest&per_page=100&page=1`,
    );
    const jobs = [...first.jobs];
    const pages = Math.ceil(first.total_count / 100);
    const remaining: Array<Promise<JobsPage>> = [];
    for (let page = 2; page <= pages; page += 1) {
        remaining.push(
            ghApi<JobsPage>(
                `repos/${repo}/actions/runs/${runId}/jobs?filter=latest&per_page=100&page=${page}`,
            ),
        );
    }
    for (const result of await Promise.all(remaining)) {
        jobs.push(...result.jobs);
    }
    return jobs;
}

function badJobs(jobs: WorkflowJob[]): WorkflowJob[] {
    return jobs.filter(
        (job) => job.conclusion !== null && BAD_CONCLUSIONS.has(job.conclusion),
    );
}

function repairTargets(jobs: WorkflowJob[]): WorkflowJob[] {
    const actionable = badJobs(jobs).filter(
        (job) => job.conclusion !== "cancelled",
    );
    const concrete = actionable.filter((job) => job.name !== "ci-verdict");
    return concrete.length > 0 ? concrete : actionable;
}

function repairPrompt(
    runInfo: WorkflowRun,
    failedJobs: WorkflowJob[],
    sha: string,
): string {
    const failures = failedJobs.length
        ? failedJobs
              .map(
                  (job) => `- ${job.name}: ${job.conclusion} (${job.html_url})`,
              )
              .join("\n")
        : `- Workflow conclusion: ${runInfo.conclusion || runInfo.status}`;

    return `You are the unattended repair agent for rusty-biscuit CI.

The user explicitly authorizes you to diagnose, fix, locally test, create a
verified OpenPGP-signed commit, and push every fix that is ready. Do not ask the
user questions. Continue until this repair cycle has either pushed a tested fix
or restarted a full run after proving the failure was transient infrastructure.

CI run: ${runInfo.databaseId}
Run URL: ${runInfo.url}
Branch: ${branch}
Remote head when detected: ${sha}
Detected bad jobs:
${failures}

Required workflow:

1. Use the github/gh-fix-ci skill and gh Actions logs to establish the exact
   root cause. The watchdog cancels an active run after detecting a bad cell, so
   canceled downstream cells are not independent defects.
2. Inspect and preserve any existing worktree changes from an earlier repair
   attempt. Make the smallest long-term fix and review affected comments/docs.
3. Before pushing, determine every affected package area and downstream scope.
   Use canonical nextest-based recipes: just test, just test-l2 where applicable,
   and just lint. Do not use cargo test and do not run cargo fmt.
4. Validate the fix without GitHub on all available local environments:
   - macOS host
   - build-linux (Linux SSH alias)
   - build-win (WSL SSH alias)
   - build-win-native (native Windows SSH alias)
   Every unattended SSH invocation must pass -o BatchMode=yes.
   If build-win is unavailable, use build-win-native to diagnose/recover its WSL
   service and retry. L2/browser tests must never steal terminal/browser focus.
   Re-run L1 after L2 fixes to prove there are no regressions.
5. Do not push or rerun CI until all applicable local environments are green.
6. For a code/configuration fix, inspect the final diff, stage only intended
   files, and commit as Ken Snyder <ken@ken.net>. Signing is authorized and
   non-interactive on this host. Use the standard keyring at
   /Users/ken/.gnupg, verify with
   env GNUPGHOME=/Users/ken/.gnupg git verify-commit HEAD, then push the current
   branch and confirm the remote SHA matches.
7. If and only if logs prove a transient runner/infrastructure failure with no
   repository fix, first complete the same applicable local verification, then
   rerun the entire workflow (not only failed jobs).

Do not stop at diagnosis, a local edit, or a local commit: the repair cycle is
complete only after a tested fix is pushed or a justified full rerun starts.`;
}

function cancelRun(runInfo: WorkflowRun): void {
    if (runInfo.status === "completed") return;
    if (dryRun) {
        log(`[dry-run] Would cancel active CI run ${runInfo.databaseId}`);
        return;
    }
    const result = gh([
        "run",
        "cancel",
        String(runInfo.databaseId),
        "--repo",
        repo,
    ]);
    if (result.status === 0) {
        log(
            `Canceled active CI run ${runInfo.databaseId} to stop wasted jobs.`,
        );
    } else {
        log(
            `Unable to cancel run ${runInfo.databaseId}: ${result.stderr.trim() || result.stdout.trim()}`,
        );
    }
}

function invokeRepairAgent(
    runInfo: WorkflowRun,
    failedJobs: WorkflowJob[],
    sha: string,
): boolean {
    if (dryRun) {
        log(
            `[dry-run] Would invoke repair agent for run ${runInfo.databaseId}`,
        );
        return false;
    }
    const prompt = repairPrompt(runInfo, failedJobs, sha);
    log(`Starting unattended repair agent for run ${runInfo.databaseId}.`);
    const result = run(
        "claudine",
        [
            "codex",
            "--repo",
            "--silent",
            "--operation",
            "ci-watchdog",
            "-y",
            "--timeout",
            "6h",
            "exec",
            prompt,
        ],
        { cwd: repoRoot, env: runtimeEnv, inherit: true },
    );
    if (result.status !== 0) {
        log(
            `Repair agent exited with status ${result.status}; it will be retried.`,
        );
        return false;
    }
    log("Repair agent finished; checking for a new or rerun workflow.");
    return true;
}

type Inspection = "failure" | "missing" | "pending" | "success";

async function inspectOnce(): Promise<Inspection> {
    const sha = remoteHead();
    const runInfo = currentRun(sha);
    if (!runInfo) {
        log(
            `No ci.yml run exists yet for origin/${branch} at ${sha.slice(0, 10)}.`,
        );
        return "missing";
    }

    log(
        `Run ${runInfo.databaseId} for ${sha.slice(0, 10)} is ${runInfo.status}` +
            `${runInfo.conclusion ? `/${runInfo.conclusion}` : ""}.`,
    );

    const jobs = await jobsFor(runInfo.databaseId);
    const failedJobs = badJobs(jobs);
    if (failedJobs.length > 0) {
        const targets = repairTargets(jobs);
        const cancelled = failedJobs.filter(
            (job) => job.conclusion === "cancelled",
        ).length;
        log(
            `Detected ${targets.length} actionable failure(s) and ${cancelled} canceled job(s)` +
                `${targets.length > 0 ? `: ${targets.map((job) => `${job.name}=${job.conclusion}`).join(", ")}` : "."}`,
        );
        cancelRun(runInfo);
        invokeRepairAgent(runInfo, targets, sha);
        return "failure";
    }

    if (runInfo.status !== "completed") {
        log(`No errors reported; checking again in ${pollSeconds} seconds.`);
        return "pending";
    }

    const verdict = jobs.find((job) => job.name === "ci-verdict");
    if (runInfo.conclusion === "success" && verdict?.conclusion === "success") {
        log(
            `SUCCESS: full CI run ${runInfo.databaseId} completed with ci-verdict green and zero bad jobs.`,
        );
        return "success";
    }

    log(
        `Completed run is not fully green: workflow=${runInfo.conclusion || "none"}, ` +
            `ci-verdict=${verdict?.conclusion ?? "missing"}.`,
    );
    invokeRepairAgent(runInfo, failedJobs, sha);
    return "failure";
}

async function main(): Promise<void> {
    if (!dryRun) acquireLock();
    while (true) {
        try {
            if (!repo) {
                const auth = gh(["auth", "status"]);
                if (auth.status !== 0) {
                    throw new Error(
                        `gh authentication is unavailable: ${auth.stderr.trim()}`,
                    );
                }
                repo = ghJson<{ nameWithOwner: string }>([
                    "repo",
                    "view",
                    "--json",
                    "nameWithOwner",
                ]).nameWithOwner;
            }
            const result = await inspectOnce();
            if (result === "success") {
                releaseLock();
                process.exit(0);
            }
            if (dryRun) {
                process.exit(result === "failure" ? 1 : 2);
            }
        } catch (error) {
            log(`Watchdog check failed: ${String(error)}`);
            if (dryRun) process.exit(2);
        }
        await sleep(pollSeconds);
    }
}

main().catch((error) => {
    log(`Fatal startup error: ${String(error)}`);
    releaseLock();
    process.exit(2);
});
