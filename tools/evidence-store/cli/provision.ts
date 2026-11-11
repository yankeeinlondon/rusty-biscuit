//! Create one Wrangler environment's Cloudflare resources and record them.
//!
//! ```text
//! provision [--env development|restore-test]
//! ```
//!
//! Brings the environment's D1 database, R2 bucket, `database_id` in
//! `wrangler.jsonc`, and `CURSOR_HMAC_KEY` secret into existence through the
//! package's pinned Wrangler with a scoped `CLOUDFLARE_API_TOKEN`. Every step
//! first looks for what it would create and reuses it, so a rerun after a
//! partial failure finishes the job and a rerun after success changes nothing.
//! An existing cursor key is never replaced: rotating it would invalidate every
//! outstanding paging cursor.
//!
//! The `SERVICE_CONFIG` allowlists are a trust decision and are never filled
//! in here; an empty one is reported as the remaining step.
//!
//! Exit codes: 0 provisioned; 1 a Wrangler step or file failed, or
//! `wrangler.jsonc` names a different database than Cloudflare holds; 2 usage
//! error or missing credentials.

import { randomBytes } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { environmentResources, PLACEHOLDER_DATABASE_ID, redact } from "./deploy.ts";
import { BOOTSTRAP_ENVIRONMENT, CliError, DEFAULT_ENVIRONMENT, defaultIo, missingEnvironment, stripJsonComments, WRANGLER_ENVIRONMENTS, type OperatorIo, type SpawnResult } from "./operator.ts";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CURSOR_SECRET = "CURSOR_HMAC_KEY";

/** What each step found or did, for the summary and for tests. */
export interface ProvisionReport {
  readonly environment: string;
  readonly database: { readonly name: string; readonly id: string; readonly created: boolean };
  readonly bucket: { readonly name: string; readonly created: boolean };
  /** False when `wrangler.jsonc` already held the database's ID. */
  readonly config_updated: boolean;
  readonly cursor_secret_created: boolean;
  /** The `SERVICE_CONFIG` allowlists still empty, by field name. */
  readonly empty_allowlists: readonly string[];
}

/**
 * `text` with the `database_id` of the D1 entry named `databaseName` set to
 * `id`, comments and layout untouched. Throws when the entry is absent.
 */
export function withDatabaseId(text: string, databaseName: string, id: string): string {
  const anchor = text.indexOf(`"database_name": "${databaseName}"`);
  if (anchor < 0) throw new CliError(1, `wrangler.jsonc has no D1 entry named ${databaseName}`);
  const field = /"database_id":\s*"[^"]*"/g;
  field.lastIndex = anchor;
  const match = field.exec(text);
  if (match === null) throw new CliError(1, `wrangler.jsonc gives the D1 entry ${databaseName} no database_id`);
  return `${text.slice(0, match.index)}"database_id": "${id}"${text.slice(match.index + match[0].length)}`;
}

/** The empty `repositories` / `oidc.trusted_workflows` allowlists in an environment's `SERVICE_CONFIG`. */
export function emptyAllowlists(wranglerConfig: string, environment: string): string[] {
  const config = JSON.parse(stripJsonComments(wranglerConfig)) as { env?: Record<string, { vars?: { SERVICE_CONFIG?: string } }> };
  const raw = config.env?.[environment]?.vars?.SERVICE_CONFIG;
  if (raw === undefined) return ["repositories", "oidc.trusted_workflows"];
  const service = JSON.parse(raw) as { repositories?: unknown[]; oidc?: { trusted_workflows?: unknown[] } };
  const empty: string[] = [];
  if ((service.repositories ?? []).length === 0) empty.push("repositories");
  if ((service.oidc?.trusted_workflows ?? []).length === 0) empty.push("oidc.trusted_workflows");
  return empty;
}

function parseJson<T>(label: string, result: SpawnResult, env: NodeJS.ProcessEnv): T {
  try {
    return JSON.parse(result.stdout) as T;
  } catch {
    throw new CliError(1, `${label} did not print JSON.\n${redact(`${result.stdout}${result.stderr}`, env)}`);
  }
}

export async function provision(environment: string, io: OperatorIo, packageRoot: string = PACKAGE_ROOT): Promise<ProvisionReport> {
  const missing = missingEnvironment(io.env, BOOTSTRAP_ENVIRONMENT);
  if (missing.length > 0) {
    throw new CliError(2, `provision needs a scoped Cloudflare API token; set ${missing.join(" and ")} (no interactive login is attempted). Nothing was created.`);
  }
  if (!(WRANGLER_ENVIRONMENTS as readonly string[]).includes(environment)) {
    throw new CliError(2, `--env must be one of ${WRANGLER_ENVIRONMENTS.join(", ")}`);
  }
  const configPath = join(packageRoot, "wrangler.jsonc");
  const configText = readFileSync(configPath, "utf8");
  const resources = environmentResources(configText, environment);
  const wrangler = join(packageRoot, "node_modules", "wrangler", "bin", "wrangler.js");
  // Node runs Wrangler's entry directly: no shell, so nothing needs quoting on any OS.
  const step = (label: string, args: string[], input?: string): SpawnResult => {
    io.stderr(`provision: ${label} (${environment})\n`);
    return io.spawn(process.execPath, [wrangler, ...args], input === undefined ? undefined : { input });
  };
  const must = (label: string, result: SpawnResult): SpawnResult => {
    if (result.status !== 0) throw new CliError(1, `${label} failed (exit ${result.status}).\n${redact(`${result.stdout}${result.stderr}`, io.env)}`);
    return result;
  };
  const listDatabases = (): { name: string; uuid: string }[] =>
    parseJson("wrangler d1 list", must("wrangler d1 list", step("wrangler d1 list", ["d1", "list", "--json", "--config", configPath])), io.env);

  const existingId = listDatabases().find((db) => db.name === resources.databaseName)?.uuid;
  const databaseCreated = existingId === undefined;
  let databaseId: string;
  if (existingId === undefined) {
    must("wrangler d1 create", step("wrangler d1 create", ["d1", "create", resources.databaseName, "--config", configPath]));
    // Read the ID back rather than scraping the create command's prose.
    const createdId = listDatabases().find((db) => db.name === resources.databaseName)?.uuid;
    if (createdId === undefined) throw new CliError(1, `wrangler d1 create reported success, but ${resources.databaseName} is not listed`);
    databaseId = createdId;
  } else {
    databaseId = existingId;
  }

  // `info` fails for a missing bucket; any other cause resurfaces from `create`.
  const bucketCreated = step("wrangler r2 bucket info", ["r2", "bucket", "info", resources.bucketName, "--json", "--config", configPath]).status !== 0;
  if (bucketCreated) must("wrangler r2 bucket create", step("wrangler r2 bucket create", ["r2", "bucket", "create", resources.bucketName, "--config", configPath]));

  let configUpdated = false;
  if (resources.databaseId !== databaseId) {
    if (resources.databaseId !== PLACEHOLDER_DATABASE_ID) {
      throw new CliError(
        1,
        `wrangler.jsonc names database_id ${resources.databaseId} for ${environment}, but Cloudflare's ${resources.databaseName} is ${databaseId}. ` +
          "Nothing in wrangler.jsonc was changed; correct it by hand once you know which is right.",
      );
    }
    writeFileSync(configPath, withDatabaseId(configText, resources.databaseName, databaseId));
    io.stderr(`provision: wrote database_id ${databaseId} for ${environment} into wrangler.jsonc\n`);
    configUpdated = true;
  }

  // Listing fails while the Worker does not exist yet; then there is no secret either.
  const listed = step("wrangler secret list", ["secret", "list", "--env", environment, "--format", "json", "--config", configPath]);
  const secrets = listed.status === 0 ? parseJson<{ name: string }[]>("wrangler secret list", listed, io.env) : [];
  const secretCreated = !secrets.some((secret) => secret.name === CURSOR_SECRET);
  if (secretCreated) {
    // On stdin, never argv. Before the first deploy Wrangler creates a stub Worker to hold it.
    const key = randomBytes(32).toString("base64url");
    must("wrangler secret put", step(`wrangler secret put ${CURSOR_SECRET}`, ["secret", "put", CURSOR_SECRET, "--env", environment, "--config", configPath], key));
  }

  return {
    environment,
    database: { name: resources.databaseName, id: databaseId, created: databaseCreated },
    bucket: { name: resources.bucketName, created: bucketCreated },
    config_updated: configUpdated,
    cursor_secret_created: secretCreated,
    empty_allowlists: emptyAllowlists(readFileSync(configPath, "utf8"), environment),
  };
}

function summary(report: ProvisionReport): string {
  const state = (created: boolean) => (created ? "created" : "already existed");
  const changed = report.database.created || report.bucket.created || report.config_updated || report.cursor_secret_created;
  const lines = [
    changed ? `provisioned ${report.environment}:` : `${report.environment} is already provisioned; nothing was changed:`,
    `  D1 database   ${report.database.name} (${report.database.id}) ${state(report.database.created)}`,
    `  R2 bucket     ${report.bucket.name} ${state(report.bucket.created)}`,
    `  wrangler.jsonc ${report.config_updated ? "database_id written; commit it" : "already held the database_id"}`,
    `  ${CURSOR_SECRET} ${state(report.cursor_secret_created)}`,
    "next:",
  ];
  if (report.empty_allowlists.length > 0) {
    lines.push(
      `  - fill SERVICE_CONFIG ${report.empty_allowlists.join(" and ")} for ${report.environment} in tools/evidence-store/wrangler.jsonc (an empty list denies everything)`,
    );
  }
  const envFlag = report.environment === DEFAULT_ENVIRONMENT ? "" : ` --env ${report.environment}`;
  lines.push(`  - just evidence-store deploy${envFlag} --manifest /abs/deploy-${report.environment}.json`);
  return `${lines.join("\n")}\n`;
}

const USAGE = "usage:\n  provision [--env development|restore-test]\n";

/** Runs the command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: OperatorIo = defaultIo(), packageRoot: string = PACKAGE_ROOT): Promise<number> {
  try {
    let values: { env?: string | undefined };
    try {
      values = parseArgs({ args: [...argv], options: { env: { type: "string" } }, strict: true, allowPositionals: false }).values;
    } catch (error) {
      throw new CliError(2, (error as Error).message);
    }
    io.stdout(summary(await provision(values.env ?? DEFAULT_ENVIRONMENT, io, packageRoot)));
    return 0;
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`provision: ${error.message}\n`);
    if (error.exitCode === 2 && /^(missing --|Unknown option|Option )/.test(error.message)) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
