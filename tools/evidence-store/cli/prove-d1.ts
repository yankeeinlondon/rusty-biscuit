//! Phase 1 D1 feasibility checkpoint against a real, disposable D1 database.
//!
//! Creates a throwaway D1 database and Worker with the package's pinned
//! Wrangler, applies `spike/d1-guard/schema.sql`, runs the shared scenarios
//! (including 20 concurrent conflicting pairs) over HTTPS, samples Worker CPU
//! at the declared bound, writes a redacted result, and deletes both
//! resources. Never logs in interactively: missing credentials end the run.
//!
//! Exit codes: 0 every D1 check passed; 1 a D1 correctness check failed or the
//! run could not complete; 2 required credentials are missing.

import { spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cpuSample, DEPLOYED_OPTIONS, runScenarios, type SpikeCheck, type SpikePost } from "../spike/d1-guard/scenarios.ts";

export const REQUIRED_ENVIRONMENT = ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"] as const;

/** The names of required variables that are unset or empty, in declaration order. */
export function missingEnvironment(environment: NodeJS.ProcessEnv): string[] {
  return REQUIRED_ENVIRONMENT.filter((name) => !environment[name]);
}

/** Wrangler prints the new database's binding snippet; take its `database_id`. */
export function parseDatabaseId(output: string): string | null {
  return /"?database_id"?\s*[:=]\s*"([0-9a-f-]{36})"/.exec(output)?.[1] ?? null;
}

/** The first workers.dev URL Wrangler reports after a deploy. */
export function parseWorkerUrl(output: string): string | null {
  return /https:\/\/[A-Za-z0-9.-]+\.workers\.dev/.exec(output)?.[0] ?? null;
}

export interface CpuBound {
  observations: number;
  request_bytes: number;
  passed: boolean;
}

export interface ProveResult {
  ok: boolean;
  ran_at: string;
  wrangler: string;
  d1_checks: SpikeCheck[];
  /** Largest passing power-of-two collection size at the request bound; null when none passed. */
  cpu_bounds: CpuBound[];
  advertised_collection_observations: number | null;
  failure: string | null;
}

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function wrangler(args: string[], options: { cwd: string; input?: string }): string {
  const executable = join(PACKAGE_ROOT, "node_modules", ".bin", process.platform === "win32" ? "wrangler.cmd" : "wrangler");
  const result = spawnSync(executable, args, {
    cwd: options.cwd,
    input: options.input ?? "",
    encoding: "utf8",
    shell: process.platform === "win32",
    env: { ...process.env, CI: "true", WRANGLER_SEND_METRICS: "false" },
    timeout: 300_000,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (result.status !== 0) {
    throw new Error(`wrangler ${args.slice(0, 3).join(" ")} failed (exit ${result.status}): ${output.slice(-2000)}`);
  }
  return output;
}

function httpPost(base: string, token: string): SpikePost {
  return async (path, body) => {
    const response = await fetch(`${base}${path}`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: typeof body === "string" ? body : JSON.stringify(body),
      signal: AbortSignal.timeout(60_000),
    });
    const text = await response.text();
    let parsed: unknown = text;
    try {
      parsed = JSON.parse(text);
    } catch {
      // Platform errors (CPU exceeded, 1101) arrive as HTML; keep a short excerpt.
      parsed = text.slice(0, 200);
    }
    return { status: response.status, body: parsed };
  };
}

async function waitUntilReady(post: SpikePost): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const response = await post("/state", { repository_id: "readiness-probe" }).catch(() => null);
    if (response !== null && response.status === 404) return;
    await new Promise((done) => setTimeout(done, 2_000));
  }
  throw new Error("the deployed spike Worker never became ready (60 s)");
}

/** Halve the collection size from the declared maximum until a CPU sample passes. */
async function sampleCpu(post: SpikePost): Promise<CpuBound[]> {
  const bounds: CpuBound[] = [];
  for (let observations = DEPLOYED_OPTIONS.cpuObservations; observations >= 1; observations /= 2) {
    const passed = (await cpuSample(post, observations, DEPLOYED_OPTIONS.cpuRequestBytes)).ok;
    bounds.push({ observations, request_bytes: DEPLOYED_OPTIONS.cpuRequestBytes, passed });
    if (passed) break;
  }
  return bounds;
}

export async function proveD1(): Promise<ProveResult> {
  const workDir = mkdtempSync(join(tmpdir(), "evidence-d1-spike-"));
  const name = `evidence-spike-${randomBytes(4).toString("hex")}`;
  const token = randomBytes(32).toString("base64url");
  const result: ProveResult = {
    ok: false,
    ran_at: new Date().toISOString(),
    wrangler: "",
    d1_checks: [],
    cpu_bounds: [],
    advertised_collection_observations: null,
    failure: null,
  };
  let databaseCreated = false;
  let workerDeployed = false;
  try {
    result.wrangler = wrangler(["--version"], { cwd: workDir }).trim();
    const created = wrangler(["d1", "create", name], { cwd: workDir });
    databaseCreated = true;
    const databaseId = parseDatabaseId(created);
    if (databaseId === null) throw new Error("could not read database_id from `wrangler d1 create`");
    const config = join(workDir, "wrangler.json");
    writeFileSync(
      config,
      JSON.stringify({
        name,
        main: join(PACKAGE_ROOT, "spike", "d1-guard", "worker.ts"),
        compatibility_date: "2026-11-01",
        compatibility_flags: ["nodejs_compat"],
        workers_dev: true,
        d1_databases: [{ binding: "SPIKE_DB", database_name: name, database_id: databaseId }],
      }),
    );
    wrangler(["d1", "execute", name, "--remote", "--yes", "--config", config, "--file", join(PACKAGE_ROOT, "spike", "d1-guard", "schema.sql")], { cwd: workDir });
    const secrets = join(workDir, "secrets.json");
    writeFileSync(secrets, JSON.stringify({ SPIKE_TOKEN: token }), { mode: 0o600 });
    const deployed = wrangler(["deploy", "--config", config, "--secrets-file", secrets], { cwd: workDir });
    workerDeployed = true;
    rmSync(secrets, { force: true });
    const url = parseWorkerUrl(deployed);
    if (url === null) throw new Error("could not read the workers.dev URL from `wrangler deploy`");
    const post = httpPost(url, token);
    await waitUntilReady(post);
    result.d1_checks = await runScenarios(post, { ...DEPLOYED_OPTIONS, cpuObservations: 0 });
    result.cpu_bounds = await sampleCpu(post);
    result.advertised_collection_observations = result.cpu_bounds.find((bound) => bound.passed)?.observations ?? null;
    const failedD1 = result.d1_checks.filter((check) => !check.ok);
    result.ok = failedD1.length === 0;
    if (!result.ok) result.failure = `D1 correctness failed: ${failedD1.map((check) => check.name).join("; ")}`;
  } catch (error) {
    result.failure = error instanceof Error ? error.message.replaceAll(token, "<redacted>") : String(error);
  } finally {
    if (workerDeployed) {
      try {
        wrangler(["delete", "--name", name, "--force"], { cwd: workDir });
      } catch (error) {
        result.failure = `${result.failure ?? ""} cleanup: worker ${name} was not deleted: ${String(error)}`.trim();
      }
    }
    if (databaseCreated) {
      try {
        wrangler(["d1", "delete", name, "--skip-confirmation"], { cwd: workDir });
      } catch (error) {
        result.failure = `${result.failure ?? ""} cleanup: database ${name} was not deleted: ${String(error)}`.trim();
      }
    }
    rmSync(workDir, { recursive: true, force: true });
  }
  return result;
}

async function main(): Promise<number> {
  const outIndex = process.argv.indexOf("--out");
  const out = outIndex >= 0 ? process.argv[outIndex + 1] : undefined;
  const missing = missingEnvironment(process.env);
  if (missing.length > 0) {
    process.stderr.write(
      `prove-d1: the D1 feasibility checkpoint needs a scoped Cloudflare API token; ` +
        `set ${missing.join(" and ")} (no interactive login is attempted). No feasibility result was recorded.\n`,
    );
    return 2;
  }
  const result = await proveD1();
  const text = `${JSON.stringify(result, null, 2)}\n`;
  if (out) writeFileSync(out, text);
  else process.stdout.write(text);
  process.stderr.write(result.ok ? "prove-d1: every D1 check passed\n" : `prove-d1: FAILED — ${result.failure}\n`);
  return result.ok ? 0 : 1;
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main();
}
