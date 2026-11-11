//! Operator CLI: bootstrap an operator credential, then manage repositories
//! and host credentials through the operator API.
//!
//! ```text
//! operator bootstrap --env <development|restore-test> --label L --token-file PATH [--expires-in-days N]
//! operator register  --endpoint URL --token-file PATH --slug owner/name
//! operator issue     --endpoint URL --token-file PATH --grant FILE
//! operator revoke    --endpoint URL --token-file PATH --credential ID --reason TEXT
//! ```
//!
//! Tokens travel only through owner-only files: a token is never printed,
//! never placed in argv, and never quoted in an error. `bootstrap` writes the
//! operator credential's digest to D1 through the package's pinned Wrangler
//! (scoped `CLOUDFLARE_API_TOKEN`, never an interactive login).
//!
//! Exit codes: 0 success; 1 the operation failed (API error, invalid grant
//! file, Wrangler or file failure); 2 usage error or missing environment.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { LIMITS } from "../src/contracts/api.ts";
import { credentialRequestBody, parseCredentialGrant } from "../src/credential-request.ts";
import { ApiError } from "../src/http.ts";
import { FORMATS } from "../src/requests.ts";
import { mintToken, secretDigest } from "../src/tokens.ts";

export const BOOTSTRAP_ENVIRONMENT = ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"] as const;
export const WRANGLER_ENVIRONMENTS = ["development", "restore-test"] as const;

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DAY_MS = 86_400_000;
const SLUG = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const REQUEST_TIMEOUT_MS = 60_000;

export interface SpawnResult {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

/** Everything the commands touch outside the file system, so tests run them in process. */
export interface OperatorIo {
  readonly env: NodeJS.ProcessEnv;
  readonly fetch: typeof fetch;
  readonly spawn: (command: string, args: readonly string[]) => SpawnResult;
  readonly stdout: (text: string) => void;
  readonly stderr: (text: string) => void;
  readonly platform: NodeJS.Platform;
  readonly now: () => number;
}

export function defaultIo(): OperatorIo {
  return {
    env: process.env,
    fetch: globalThis.fetch,
    spawn: (command, args) => {
      const result = spawnSync(command, [...args], {
        cwd: PACKAGE_ROOT,
        encoding: "utf8",
        env: { ...process.env, CI: "true", WRANGLER_SEND_METRICS: "false" },
        timeout: 300_000,
      });
      return { status: result.status, stdout: result.stdout ?? "", stderr: result.stderr ?? "" };
    },
    stdout: (text) => process.stdout.write(text),
    stderr: (text) => process.stderr.write(text),
    platform: process.platform,
    now: Date.now,
  };
}

export class CliError extends Error {
  constructor(
    readonly exitCode: 1 | 2,
    message: string,
  ) {
    super(message);
  }
}

/** The names of required variables that are unset or empty, in declaration order. */
export function missingEnvironment(env: NodeJS.ProcessEnv, names: readonly string[]): string[] {
  return names.filter((name) => !env[name]);
}

/** The token in `path`, without one trailing newline. Its contents never reach an error. */
export function readTokenFile(path: string): string {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (error) {
    throw new CliError(1, `could not read the token file ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  const token = text.replace(/\r?\n$/, "");
  if (token.length === 0 || /\s/.test(token)) throw new CliError(1, `the token file ${path} does not hold one token`);
  return token;
}

/**
 * Creates `path` exclusively with owner-only permissions (mode 0600; on
 * Windows an `icacls` owner-only ACL applied before the secret is written),
 * then writes `secret`. Any failure removes the file.
 */
export function writeSecretFile(path: string, secret: string, io: Pick<OperatorIo, "platform" | "spawn" | "env">): void {
  try {
    writeFileSync(path, "", { flag: "wx", mode: 0o600 });
  } catch (error) {
    throw new CliError(1, `could not create ${path} (${(error as NodeJS.ErrnoException).code ?? "error"}); it must not already exist`);
  }
  try {
    if (io.platform === "win32") {
      const user = io.env["USERNAME"];
      if (!user) throw new CliError(1, "USERNAME is not set, so the secret file cannot be restricted to its owner");
      const result = io.spawn("icacls", [path, "/inheritance:r", "/grant:r", `${user}:F`]);
      if (result.status !== 0) throw new CliError(1, `icacls could not restrict ${path} to its owner (exit ${result.status})`);
    }
    writeFileSync(path, `${secret}\n`, { flag: "r+" });
  } catch (error) {
    rmSync(path, { force: true });
    throw error instanceof CliError ? error : new CliError(1, `could not write ${path}`);
  }
}

/** Removes line and block comments outside strings (Wrangler's JSONC; no trailing commas). */
export function stripJsonComments(text: string): string {
  let out = "";
  let inString = false;
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index]!;
    if (inString) {
      out += char;
      if (char === "\\") out += text[++index] ?? "";
      else if (char === '"') inString = false;
    } else if (char === "/" && text[index + 1] === "/") {
      while (index < text.length && text[index] !== "\n") index += 1;
      out += "\n";
    } else if (char === "/" && text[index + 1] === "*") {
      const end = text.indexOf("*/", index + 2);
      index = end === -1 ? text.length : end + 1;
    } else {
      if (char === '"') inString = true;
      out += char;
    }
  }
  return out;
}

/** The D1 `database_name` bound as `EVIDENCE_DB` in `environment` of `wrangler.jsonc`. */
export function databaseName(wranglerConfig: string, environment: string): string {
  const config = JSON.parse(stripJsonComments(wranglerConfig)) as {
    env?: Record<string, { d1_databases?: { binding: string; database_name: string }[] }>;
  };
  const name = config.env?.[environment]?.d1_databases?.find((database) => database.binding === "EVIDENCE_DB")?.database_name;
  if (name === undefined) throw new CliError(1, `wrangler.jsonc binds no EVIDENCE_DB database in environment ${environment}`);
  return name;
}

function sqlText(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

export interface BootstrapRecord {
  readonly credential_id: string;
  readonly secret_sha256: string;
  readonly label: string;
  readonly issued_at_ms: number;
  readonly rotate_at_ms: number;
  readonly expires_at_ms: number;
  readonly request_id: string;
}

export function bootstrapRecord(input: {
  credentialId: string;
  digest: string;
  label: string;
  nowMs: number;
  expiresInDays: number;
  requestId: string;
}): BootstrapRecord {
  const expires = input.nowMs + input.expiresInDays * DAY_MS;
  return {
    credential_id: input.credentialId,
    secret_sha256: input.digest,
    label: input.label,
    issued_at_ms: input.nowMs,
    rotate_at_ms: Math.min(input.nowMs + LIMITS.credential_rotation_day * DAY_MS, expires),
    expires_at_ms: expires,
    request_id: input.requestId,
  };
}

/**
 * The bootstrap statements: the operator `credentials` row (digest only) and
 * its `operator.bootstrap` audit row, whose actor is the new credential
 * itself (there is no prior principal).
 */
export function bootstrapSql(record: BootstrapRecord): string {
  const id = sqlText(record.credential_id);
  const actor = sqlText(JSON.stringify({ kind: "credential", credential_id: record.credential_id }));
  const detail = sqlText(JSON.stringify({ label: record.label, expires_at_ms: record.expires_at_ms }));
  return (
    `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms) ` +
    `VALUES (${id}, 'operator', ${sqlText(record.label)}, ${sqlText(record.secret_sha256)}, ` +
    `${record.issued_at_ms}, ${record.rotate_at_ms}, ${record.expires_at_ms}); ` +
    `INSERT INTO audit_log(at_ms, action, actor, actor_credential_id, repository_id, target_credential_id, request_id, detail_json) ` +
    `VALUES (${record.issued_at_ms}, 'operator.bootstrap', ${actor}, ${id}, NULL, ${id}, ${sqlText(record.request_id)}, ${detail});`
  );
}

function checkLabel(label: string): string {
  if (label.length < 1 || label.length > 128) throw new CliError(2, "--label must be 1 to 128 characters");
  return label;
}

function checkDays(text: string | undefined): number {
  if (text === undefined) return LIMITS.credential_expiry_days;
  const days = /^[1-9][0-9]*$/.test(text) ? Number(text) : Number.NaN;
  if (!(days >= 1 && days <= LIMITS.credential_expiry_days)) {
    throw new CliError(2, `--expires-in-days must be an integer from 1 to ${LIMITS.credential_expiry_days}`);
  }
  return days;
}

export interface BootstrapOptions {
  readonly environment: string;
  readonly label: string;
  readonly tokenFile: string;
  readonly expiresInDays?: string;
}

export async function bootstrap(options: BootstrapOptions, io: OperatorIo): Promise<void> {
  const missing = missingEnvironment(io.env, BOOTSTRAP_ENVIRONMENT);
  if (missing.length > 0) {
    throw new CliError(
      2,
      `bootstrap needs a scoped Cloudflare API token; set ${missing.join(" and ")} (no interactive login is attempted). Nothing was written.`,
    );
  }
  if (!(WRANGLER_ENVIRONMENTS as readonly string[]).includes(options.environment)) {
    throw new CliError(2, `--env must be one of ${WRANGLER_ENVIRONMENTS.join(", ")}`);
  }
  const label = checkLabel(options.label);
  const days = checkDays(options.expiresInDays);
  const config = join(PACKAGE_ROOT, "wrangler.jsonc");
  const database = databaseName(readFileSync(config, "utf8"), options.environment);

  const minted = mintToken();
  const record = bootstrapRecord({
    credentialId: minted.credentialId,
    digest: await secretDigest(minted.secret),
    label,
    nowMs: io.now(),
    expiresInDays: days,
    requestId: `bootstrap-${crypto.randomUUID()}`,
  });
  writeSecretFile(options.tokenFile, minted.token, io);
  const wrangler = join(PACKAGE_ROOT, "node_modules", "wrangler", "bin", "wrangler.js");
  // Node runs Wrangler's entry directly: no shell, so the SQL needs no quoting on any OS.
  const result = io.spawn(process.execPath, [
    wrangler,
    "d1",
    "execute",
    database,
    "--env",
    options.environment,
    "--remote",
    "--yes",
    "--config",
    config,
    "--command",
    bootstrapSql(record),
  ]);
  if (result.status !== 0) {
    rmSync(options.tokenFile, { force: true });
    const output = `${result.stdout}${result.stderr}`.slice(-2000);
    throw new CliError(1, `wrangler d1 execute failed (exit ${result.status}); no operator credential was created and ${options.tokenFile} was removed.\n${output}`);
  }
  const { secret_sha256: _digest, request_id: _request, ...publicRecord } = record;
  io.stdout(`${JSON.stringify({ ...publicRecord, kind: "operator" }, null, 2)}\n`);
  io.stderr(`operator: bootstrapped operator credential ${record.credential_id}; token written to ${options.tokenFile}\n`);
}

interface ErrorEnvelope {
  error?: { code?: unknown; message?: unknown; location?: unknown };
  request_id?: unknown;
}

/** POSTs `body` to the operator API and answers the parsed success body. */
async function operatorPost(io: OperatorIo, endpoint: string, token: string, path: string, body: string): Promise<{ status: number; body: unknown }> {
  const url = `${endpoint.replace(/\/+$/, "")}${path}`;
  let response: Response;
  try {
    response = await io.fetch(url, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body,
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch (error) {
    throw new CliError(1, `request to ${url} failed: ${error instanceof Error ? error.message : "network error"}`);
  }
  const text = await response.text();
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new CliError(1, `${url} answered ${response.status} with a body that is not JSON`);
  }
  if (!response.ok) {
    const envelope = parsed as ErrorEnvelope;
    const field = (value: unknown) => (value === undefined || value === null ? "-" : String(value));
    throw new CliError(
      1,
      `${url} answered ${response.status}\n` +
        `  code: ${field(envelope.error?.code)}\n  message: ${field(envelope.error?.message)}\n` +
        `  location: ${field(envelope.error?.location)}\n  request_id: ${field(envelope.request_id)}`,
    );
  }
  return { status: response.status, body: parsed };
}

export interface RegisterOptions {
  readonly endpoint: string;
  readonly tokenFile: string;
  readonly slug: string;
}

/** Registers by the stable IDs GitHub's repository API reports, never by the slug alone. */
export async function register(options: RegisterOptions, io: OperatorIo): Promise<void> {
  if (!SLUG.test(options.slug)) throw new CliError(2, "--slug must be owner/name");
  const github = io.env["GITHUB_TOKEN"] || io.env["GH_TOKEN"];
  if (!github) throw new CliError(2, "register reads the repository's stable IDs from GitHub; set GITHUB_TOKEN or GH_TOKEN");
  const token = readTokenFile(options.tokenFile);
  const url = `https://api.github.com/repos/${options.slug}`;
  let response: Response;
  try {
    response = await io.fetch(url, {
      headers: { authorization: `Bearer ${github}`, accept: "application/vnd.github+json", "user-agent": "evidence-store-operator" },
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch (error) {
    throw new CliError(1, `GitHub lookup of ${options.slug} failed: ${error instanceof Error ? error.message : "network error"}`);
  }
  if (!response.ok) throw new CliError(1, `GitHub lookup of ${options.slug} answered ${response.status}`);
  const repository = (await response.json()) as { id?: unknown; full_name?: unknown; owner?: { id?: unknown } };
  const id = repository.id;
  const ownerId = repository.owner?.id;
  if (!Number.isSafeInteger(id) || !Number.isSafeInteger(ownerId) || (id as number) < 1 || (ownerId as number) < 1) {
    throw new CliError(1, `GitHub lookup of ${options.slug} did not return numeric repository and owner IDs`);
  }
  const slug = typeof repository.full_name === "string" && SLUG.test(repository.full_name) ? repository.full_name : options.slug;
  const result = await operatorPost(
    io,
    options.endpoint,
    token,
    "/v1/operator/repositories",
    JSON.stringify({ repository_id: String(id), owner_id: String(ownerId), slug }),
  );
  io.stdout(`${JSON.stringify(result.body, null, 2)}\n`);
}

export interface IssueOptions {
  readonly endpoint: string;
  readonly tokenFile: string;
  readonly grant: string;
}

export async function issue(options: IssueOptions, io: OperatorIo): Promise<void> {
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(options.grant);
  } catch (error) {
    throw new CliError(1, `could not read the grant file ${options.grant} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  let grant;
  try {
    grant = parseCredentialGrant(bytes);
  } catch (error) {
    if (!(error instanceof ApiError)) throw error;
    throw new CliError(1, `the grant file ${options.grant} is invalid at "${error.location ?? ""}": ${error.message}`);
  }
  const secretFile = resolve(grant.secret_file);
  if (existsSync(secretFile)) throw new CliError(1, `secret_file ${secretFile} already exists; it is never overwritten`);
  const token = readTokenFile(options.tokenFile);
  const result = await operatorPost(io, options.endpoint, token, "/v1/operator/credentials", credentialRequestBody(grant));
  const issued = result.body as { credential?: { credential_id?: unknown }; secret?: unknown };
  if (typeof issued.secret !== "string" || issued.credential === undefined) {
    throw new CliError(1, "the service answered without a credential and secret");
  }
  try {
    writeSecretFile(secretFile, issued.secret, io);
  } catch (error) {
    const id = String(issued.credential.credential_id);
    throw new CliError(1, `${(error as Error).message}; credential ${id} was issued but its secret could not be saved, so revoke it`);
  }
  io.stdout(`${JSON.stringify(issued.credential, null, 2)}\n`);
  io.stderr(`operator: secret written to ${secretFile}\n`);
}

export interface RevokeOptions {
  readonly endpoint: string;
  readonly tokenFile: string;
  readonly credential: string;
  readonly reason: string;
}

export async function revoke(options: RevokeOptions, io: OperatorIo): Promise<void> {
  if (!FORMATS.serverId.test(options.credential)) throw new CliError(2, "--credential must be a 32-character hex credential ID");
  if (options.reason.length < 1 || options.reason.length > 512) throw new CliError(2, "--reason must be 1 to 512 characters");
  const token = readTokenFile(options.tokenFile);
  const result = await operatorPost(
    io,
    options.endpoint,
    token,
    `/v1/operator/credentials/${options.credential}/revoke`,
    JSON.stringify({ reason: options.reason }),
  );
  io.stdout(`${JSON.stringify(result.body, null, 2)}\n`);
}

const USAGE = `usage:
  operator bootstrap --env <development|restore-test> --label L --token-file PATH [--expires-in-days N]
  operator register  --endpoint URL --token-file PATH --slug owner/name
  operator issue     --endpoint URL --token-file PATH --grant FILE
  operator revoke    --endpoint URL --token-file PATH --credential ID --reason TEXT
`;

function required(values: Record<string, string | undefined>, names: readonly string[]): void {
  const absent = names.filter((name) => values[name] === undefined);
  if (absent.length > 0) throw new CliError(2, `missing ${absent.map((name) => `--${name}`).join(", ")}`);
}

function options(argv: readonly string[], names: readonly string[]): Record<string, string | undefined> {
  try {
    const { values } = parseArgs({
      args: [...argv],
      options: Object.fromEntries(names.map((name) => [name, { type: "string" as const }])),
      strict: true,
      allowPositionals: false,
    });
    return values as Record<string, string | undefined>;
  } catch (error) {
    throw new CliError(2, (error as Error).message);
  }
}

/** Runs one command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: OperatorIo = defaultIo()): Promise<number> {
  const [command, ...rest] = argv;
  try {
    switch (command) {
      case "bootstrap": {
        const v = options(rest, ["env", "label", "token-file", "expires-in-days"]);
        required(v, ["env", "label", "token-file"]);
        await bootstrap(
          v["expires-in-days"] === undefined
            ? { environment: v["env"]!, label: v["label"]!, tokenFile: v["token-file"]! }
            : { environment: v["env"]!, label: v["label"]!, tokenFile: v["token-file"]!, expiresInDays: v["expires-in-days"] },
          io,
        );
        return 0;
      }
      case "register": {
        const v = options(rest, ["endpoint", "token-file", "slug"]);
        required(v, ["endpoint", "token-file", "slug"]);
        await register({ endpoint: v["endpoint"]!, tokenFile: v["token-file"]!, slug: v["slug"]! }, io);
        return 0;
      }
      case "issue": {
        const v = options(rest, ["endpoint", "token-file", "grant"]);
        required(v, ["endpoint", "token-file", "grant"]);
        await issue({ endpoint: v["endpoint"]!, tokenFile: v["token-file"]!, grant: v["grant"]! }, io);
        return 0;
      }
      case "revoke": {
        const v = options(rest, ["endpoint", "token-file", "credential", "reason"]);
        required(v, ["endpoint", "token-file", "credential", "reason"]);
        await revoke({ endpoint: v["endpoint"]!, tokenFile: v["token-file"]!, credential: v["credential"]!, reason: v["reason"]! }, io);
        return 0;
      }
      default:
        io.stderr(USAGE);
        return 2;
    }
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`operator: ${error.message}\n`);
    if (error.exitCode === 2) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
