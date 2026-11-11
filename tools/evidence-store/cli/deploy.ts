//! Deploy the Worker to one Wrangler environment and record what was deployed.
//!
//! ```text
//! deploy --env <development|restore-test> [--manifest /abs/path.json]
//! ```
//!
//! Applies the numbered D1 migrations, then deploys the Worker, both through
//! the package's pinned Wrangler with a scoped `CLOUDFLARE_API_TOKEN` (never
//! an interactive login). The resulting manifest names the environment, its
//! resources, the migration files by digest, and the Worker version; it never
//! carries a token, a secret, or the account ID.
//!
//! The resources themselves are provisioned once, by hand, before the first
//! deploy (see `PROVISIONING`); a placeholder `database_id` in
//! `wrangler.jsonc` stops the command before anything runs.
//!
//! Exit codes: 0 deployed; 1 a Wrangler step or file failed; 2 usage error,
//! missing credentials, or an unprovisioned environment.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { BOOTSTRAP_ENVIRONMENT, CliError, defaultIo, missingEnvironment, stripJsonComments, WRANGLER_ENVIRONMENTS, type OperatorIo } from "./operator.ts";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PLACEHOLDER_DATABASE_ID = "00000000-0000-0000-0000-000000000000";
const VERSION_ID = /Version ID:\s*([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/i;
const HTTPS_URL = /https:\/\/[A-Za-z0-9.-]+(?:\/[^\s]*)?/;

export interface DeployManifest {
  readonly manifest_version: 1;
  readonly environment: string;
  readonly worker_name: string;
  readonly d1: { readonly name: string; readonly id: string };
  readonly r2_bucket: string;
  readonly migrations: readonly { readonly file: string; readonly sha256: string }[];
  /** Null when Wrangler's output names no version. */
  readonly worker_version_id: string | null;
  /** The first HTTPS URL Wrangler reports for the deploy; null when it reports none. */
  readonly endpoint: string | null;
  readonly wrangler_version: string;
  readonly deployed_at: string;
}

/** The deploy targets `environment` declares in `wrangler.jsonc`. */
export interface EnvironmentResources {
  readonly workerName: string;
  readonly databaseName: string;
  readonly databaseId: string;
  readonly bucketName: string;
}

export function environmentResources(wranglerConfig: string, environment: string): EnvironmentResources {
  const config = JSON.parse(stripJsonComments(wranglerConfig)) as {
    env?: Record<
      string,
      {
        name?: string;
        d1_databases?: { binding: string; database_name: string; database_id: string }[];
        r2_buckets?: { binding: string; bucket_name: string }[];
      }
    >;
  };
  const section = config.env?.[environment];
  const database = section?.d1_databases?.find((entry) => entry.binding === "EVIDENCE_DB");
  const bucket = section?.r2_buckets?.find((entry) => entry.binding === "EVIDENCE_BUCKET");
  if (section?.name === undefined || database === undefined || bucket === undefined) {
    throw new CliError(1, `wrangler.jsonc does not declare a Worker name, EVIDENCE_DB, and EVIDENCE_BUCKET in environment ${environment}`);
  }
  return { workerName: section.name, databaseName: database.database_name, databaseId: database.database_id, bucketName: bucket.bucket_name };
}

/** The one-time steps that replace the placeholder, as an operator would type them. */
export function provisioningSteps(environment: string, resources: EnvironmentResources): string {
  return (
    `one-time provisioning for ${environment} (run from tools/evidence-store with the same credentials):\n` +
    `  1. pnpm exec wrangler d1 create ${resources.databaseName}\n` +
    `  2. pnpm exec wrangler r2 bucket create ${resources.bucketName}\n` +
    `  3. set env.${environment}.d1_databases[EVIDENCE_DB].database_id in wrangler.jsonc to the ID step 1 printed, and commit it\n` +
    `  4. feed a fresh random key on stdin (never argv): <generate a key> | pnpm exec wrangler secret put CURSOR_HMAC_KEY --env ${environment}\n`
  );
}

/** Every `migrations/*.sql` file in apply order, with the SHA-256 of its bytes. */
export function migrationDigests(directory: string): { file: string; sha256: string }[] {
  return readdirSync(directory)
    .filter((name) => name.endsWith(".sql"))
    .sort()
    .map((file) => ({ file, sha256: createHash("sha256").update(readFileSync(join(directory, file))).digest("hex") }));
}

export function parseVersionId(output: string): string | null {
  return VERSION_ID.exec(output)?.[1]?.toLowerCase() ?? null;
}

export function parseEndpoint(output: string): string | null {
  return HTTPS_URL.exec(output)?.[0] ?? null;
}

/** Wrangler's output with the credential values blanked, for an error message. */
function redact(output: string, env: NodeJS.ProcessEnv): string {
  let text = output;
  for (const name of BOOTSTRAP_ENVIRONMENT) {
    const value = env[name];
    if (value) text = text.split(value).join(`<${name}>`);
  }
  return text.slice(-2000);
}

export interface DeployOptions {
  readonly environment: string;
  /** Absolute path; the manifest goes to stdout when absent. */
  readonly manifest?: string;
}

export async function deploy(options: DeployOptions, io: OperatorIo, packageRoot: string = PACKAGE_ROOT): Promise<DeployManifest> {
  const missing = missingEnvironment(io.env, BOOTSTRAP_ENVIRONMENT);
  if (missing.length > 0) {
    throw new CliError(
      2,
      `deploy needs a scoped Cloudflare API token; set ${missing.join(" and ")} (no interactive login is attempted). Nothing was deployed.`,
    );
  }
  if (!(WRANGLER_ENVIRONMENTS as readonly string[]).includes(options.environment)) {
    throw new CliError(2, `--env must be one of ${WRANGLER_ENVIRONMENTS.join(", ")}`);
  }
  if (options.manifest !== undefined) {
    if (!isAbsolute(options.manifest)) throw new CliError(2, "--manifest must be an absolute path");
    if (!existsSync(dirname(options.manifest))) throw new CliError(2, `the directory for --manifest ${options.manifest} does not exist`);
  }
  const config = join(packageRoot, "wrangler.jsonc");
  const resources = environmentResources(readFileSync(config, "utf8"), options.environment);
  if (resources.databaseId === PLACEHOLDER_DATABASE_ID) {
    throw new CliError(
      2,
      `wrangler.jsonc still holds the placeholder database_id for ${options.environment}, so there is nothing to deploy to. Nothing was run.\n` +
        provisioningSteps(options.environment, resources),
    );
  }
  const migrations = migrationDigests(join(packageRoot, "migrations"));
  const packageJson = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")) as { devDependencies?: Record<string, string> };
  const wranglerVersion = packageJson.devDependencies?.["wrangler"];
  if (wranglerVersion === undefined) throw new CliError(1, "package.json pins no wrangler devDependency");

  const wrangler = join(packageRoot, "node_modules", "wrangler", "bin", "wrangler.js");
  const steps: { label: string; args: string[] }[] = [
    {
      label: "wrangler d1 migrations apply",
      args: [wrangler, "d1", "migrations", "apply", resources.databaseName, "--remote", "--env", options.environment, "--config", config],
    },
    { label: "wrangler deploy", args: [wrangler, "deploy", "--env", options.environment, "--config", config] },
  ];
  let deployOutput = "";
  for (const [index, step] of steps.entries()) {
    io.stderr(`deploy: ${step.label} (${options.environment})\n`);
    // Node runs Wrangler's entry directly: no shell, so nothing needs quoting on any OS.
    const result = io.spawn(process.execPath, step.args);
    if (result.status !== 0) {
      const consequence = index === 0 ? "the Worker was not deployed" : "the migrations were applied but the Worker was not deployed";
      throw new CliError(1, `${step.label} failed (exit ${result.status}); ${consequence}.\n${redact(`${result.stdout}${result.stderr}`, io.env)}`);
    }
    deployOutput = `${result.stdout}${result.stderr}`;
  }

  const manifest: DeployManifest = {
    manifest_version: 1,
    environment: options.environment,
    worker_name: resources.workerName,
    d1: { name: resources.databaseName, id: resources.databaseId },
    r2_bucket: resources.bucketName,
    migrations,
    worker_version_id: parseVersionId(deployOutput),
    endpoint: parseEndpoint(deployOutput),
    wrangler_version: wranglerVersion,
    deployed_at: new Date(io.now()).toISOString(),
  };
  const text = `${JSON.stringify(manifest, null, 2)}\n`;
  if (options.manifest === undefined) {
    io.stdout(text);
  } else {
    try {
      writeFileSync(options.manifest, text);
    } catch (error) {
      throw new CliError(1, `deployed, but could not write the manifest ${options.manifest} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
    }
    io.stderr(`deploy: manifest written to ${options.manifest}\n`);
  }
  return manifest;
}

const USAGE = "usage:\n  deploy --env <development|restore-test> [--manifest /abs/path.json]\n";

/** Runs the command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: OperatorIo = defaultIo(), packageRoot: string = PACKAGE_ROOT): Promise<number> {
  try {
    let values: { env?: string | undefined; manifest?: string | undefined };
    try {
      values = parseArgs({
        args: [...argv],
        options: { env: { type: "string" }, manifest: { type: "string" } },
        strict: true,
        allowPositionals: false,
      }).values;
    } catch (error) {
      throw new CliError(2, (error as Error).message);
    }
    if (values.env === undefined) throw new CliError(2, "missing --env");
    await deploy(values.manifest === undefined ? { environment: values.env } : { environment: values.env, manifest: values.manifest }, io, packageRoot);
    return 0;
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`deploy: ${error.message}\n`);
    if (error.exitCode === 2 && /^(missing --|Unknown option|Option )/.test(error.message)) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
