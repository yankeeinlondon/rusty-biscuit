//! Recovery commands of the operator CLI: `backup`, `verify-backup`,
//! `restore`, `recover-jobs`, and `maintenance`.
//!
//! ```text
//! operator backup        --endpoint URL --token-file PATH --env ENV --destination DIR [--config PATH]
//! operator verify-backup --backup DIR/<backup_id>
//! operator restore       --backup DIR/<backup_id> --source-env ENV --dest-env ENV --dest-token-file PATH
//!                        [--dest-endpoint URL --replay STATE ...] [--report PATH] [--label L] [--config PATH]
//! operator recover-jobs  --endpoint URL --token-file PATH
//! operator maintenance   --endpoint URL --token-file PATH --mode open|quiesced [--reason TEXT] [--boundary-id ID]
//! ```
//!
//! Every input is checked before the first mutation. Secrets never travel
//! through argv, stdout, or logs: the operator token is read from an
//! owner-only file, a restore's new operator token is written to one, and the
//! new cursor key reaches Wrangler only through stdin. A backup holds
//! credential digests, never bearer secrets (the database holds none).
//!
//! Restore loads the D1 export in the order tables → rows → indexes →
//! triggers, whatever order the export used, so no trigger fires on a restored
//! row (an `observations` insert would otherwise append a second `accepted`
//! event). Its own repairs run after the evidence triggers exist and before
//! the maintenance triggers do, so they write their change events normally
//! while the restored store stays quiesced until the restore reopens it.

import { createHash, randomBytes, randomUUID } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import type { DatabaseSync } from "node:sqlite";
import {
  BACKUP_FORMAT_VERSION,
  MANIFEST_FILE,
  parseBackupManifest,
  type BackupManifest,
  type BackupObject,
  type CredentialRecord,
  type GrantRecord,
  type MissingObject,
  type PendingUpload,
  type RepositoryCounters,
  type RetentionJobRecord,
  type WindowRevocation,
} from "../src/backup-manifest.ts";
import { LIMITS } from "../src/contracts/api.ts";
import { nodeReportReader } from "../src/contracts/node-report-reader.ts";
import { ApiError } from "../src/http.ts";
import { FORMATS } from "../src/requests.ts";
import { mintToken, randomId, secretDigest } from "../src/tokens.ts";
import { checkEndpoint, loadState, resume, resumeCommand, type IngestIo, type ResumeState } from "./ingest.ts";
import {
  BOOTSTRAP_ENVIRONMENT,
  bootstrapRecord,
  bootstrapSql,
  CliError,
  missingEnvironment,
  operatorPost,
  readTokenFile,
  stripJsonComments,
  WRANGLER_ENVIRONMENTS,
  writeSecretFile,
  type OperatorIo,
} from "./operator.ts";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WRANGLER = join(PACKAGE_ROOT, "node_modules", "wrangler", "bin", "wrangler.js");
const DEFAULT_CONFIG = join(PACKAGE_ROOT, "wrangler.jsonc");

/** Complete backups kept in a destination; older ones are removed after each backup. */
export const BACKUPS_KEPT = 7;
export const DATABASE_FILE = "database.sql";
const DEFAULT_RESTORE_LABEL = "restored-operator";
const REPORT_FORMAT_VERSION = 1;

/** The operator IO plus the two ingestion hooks a replay needs; both default to the real ones. */
export interface RecoveryIo extends OperatorIo {
  readonly sleep?: (ms: number) => Promise<void>;
  readonly random?: () => number;
}

// ---- Wrangler -----------------------------------------------------------------

/** The D1 database and R2 bucket an environment of `wrangler.jsonc` binds. */
export interface WranglerResources {
  readonly environment: string;
  readonly d1_database_name: string;
  readonly d1_database_id: string;
  readonly r2_bucket: string;
}

export function wranglerResources(configText: string, environment: string): WranglerResources {
  let config: { env?: Record<string, { d1_databases?: { binding?: unknown; database_name?: unknown; database_id?: unknown }[]; r2_buckets?: { binding?: unknown; bucket_name?: unknown }[] }> };
  try {
    config = JSON.parse(stripJsonComments(configText)) as typeof config;
  } catch {
    throw new CliError(2, "the Wrangler configuration is not valid JSONC");
  }
  const section = config.env?.[environment];
  const database = section?.d1_databases?.find((entry) => entry.binding === "EVIDENCE_DB");
  const bucket = section?.r2_buckets?.find((entry) => entry.binding === "EVIDENCE_BUCKET");
  const text = (value: unknown) => (typeof value === "string" && value.length > 0 ? value : null);
  const name = text(database?.database_name);
  const id = text(database?.database_id);
  const bucketName = text(bucket?.bucket_name);
  if (name === null || id === null || bucketName === null) {
    throw new CliError(2, `the Wrangler configuration binds no complete EVIDENCE_DB and EVIDENCE_BUCKET in environment ${environment}`);
  }
  return { environment, d1_database_name: name, d1_database_id: id, r2_bucket: bucketName };
}

function checkEnvironmentName(flag: string, environment: string): string {
  if (!(WRANGLER_ENVIRONMENTS as readonly string[]).includes(environment)) {
    throw new CliError(2, `${flag} must be one of ${WRANGLER_ENVIRONMENTS.join(", ")}`);
  }
  return environment;
}

function requireCloudflare(io: OperatorIo, command: string): void {
  const missing = missingEnvironment(io.env, BOOTSTRAP_ENVIRONMENT);
  if (missing.length > 0) {
    throw new CliError(2, `${command} runs Wrangler against Cloudflare; set ${missing.join(" and ")} (no interactive login is attempted). Nothing was changed.`);
  }
}

function readConfig(path: string): string {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    throw new CliError(2, `could not read the Wrangler configuration ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
}

/** Runs the package's pinned Wrangler with Node directly (no shell, so nothing needs quoting on any OS). */
function wrangler(io: OperatorIo, config: string, args: readonly string[], input?: string) {
  const argv = [WRANGLER, ...args, "--config", config];
  return input === undefined ? io.spawn(process.execPath, argv) : io.spawn(process.execPath, argv, { input });
}

function wranglerFailure(what: string, result: { status: number | null; stdout: string; stderr: string }): CliError {
  return new CliError(1, `wrangler ${what} failed (exit ${result.status})\n${`${result.stdout}${result.stderr}`.slice(-2000)}`);
}

/** The rows of a `d1 execute --json` single-statement query. */
function queryRows(io: OperatorIo, config: string, database: string, environment: string, sql: string): Record<string, unknown>[] {
  const result = wrangler(io, config, ["d1", "execute", database, "--env", environment, "--remote", "--json", "--command", sql]);
  if (result.status !== 0) throw wranglerFailure(`d1 execute ${database}`, result);
  const start = result.stdout.indexOf("[");
  let parsed: unknown;
  try {
    parsed = JSON.parse(result.stdout.slice(start));
  } catch {
    throw new CliError(1, `wrangler d1 execute ${database} answered output that is not JSON`);
  }
  const rows = Array.isArray(parsed) ? (parsed[0] as { results?: unknown } | undefined)?.results : undefined;
  if (start < 0 || !Array.isArray(rows)) throw new CliError(1, `wrangler d1 execute ${database} answered JSON without results`);
  return rows as Record<string, unknown>[];
}

// ---- SQL export handling -------------------------------------------------------------

/**
 * Splits SQL text into statements (without their terminating `;`). Quoted
 * strings, quoted identifiers, and comments may contain `;`; a
 * `CREATE TRIGGER` ends only at the `;` after the `END` that closes its
 * `BEGIN` (a `CASE … END` inside the body is counted, too).
 */
export function splitSql(text: string): string[] {
  const statements: string[] = [];
  let start = 0;
  let depth = 0;
  let sawBegin = false;
  let index = 0;
  const flush = (end: number) => {
    const statement = text.slice(start, end).trim();
    if (stripComments(statement).length > 0) statements.push(statement);
    start = end + 1;
    depth = 0;
    sawBegin = false;
  };
  while (index < text.length) {
    const char = text[index]!;
    if (char === "'" || char === '"' || char === "`") {
      const close = text.indexOf(char, index + 1);
      // A doubled quote inside a quoted run reopens it; the loop handles that as two runs.
      index = close === -1 ? text.length : close + 1;
    } else if (char === "[") {
      const close = text.indexOf("]", index + 1);
      index = close === -1 ? text.length : close + 1;
    } else if (char === "-" && text[index + 1] === "-") {
      const close = text.indexOf("\n", index);
      index = close === -1 ? text.length : close + 1;
    } else if (char === "/" && text[index + 1] === "*") {
      const close = text.indexOf("*/", index + 2);
      index = close === -1 ? text.length : close + 2;
    } else if (/[A-Za-z_]/.test(char)) {
      let end = index + 1;
      while (end < text.length && /[A-Za-z0-9_$]/.test(text[end]!)) end += 1;
      const word = text.slice(index, end).toUpperCase();
      if (word === "BEGIN" || word === "CASE") {
        depth += 1;
        if (word === "BEGIN") sawBegin = true;
      } else if (word === "END") {
        depth -= 1;
      }
      index = end;
    } else if (char === ";") {
      const isTrigger = /^CREATE\s+(?:TEMP\s+|TEMPORARY\s+)?TRIGGER\b/i.test(stripComments(text.slice(start, index)));
      if (!isTrigger || (sawBegin && depth <= 0)) flush(index);
      index += 1;
    } else {
      index += 1;
    }
  }
  flush(text.length);
  return statements;
}

function stripComments(statement: string): string {
  return statement.replace(/^(?:\s+|--[^\n]*(?:\n|$)|\/\*[\s\S]*?\*\/)*/, "");
}

export type StatementKind = "pragma" | "table" | "data" | "index" | "trigger" | "view";

/** The kind of one exported statement; anything a D1 export does not produce is refused. */
export function classifyStatement(statement: string): StatementKind {
  const head = stripComments(statement);
  if (/^PRAGMA\s/i.test(head)) return "pragma";
  if (/^CREATE\s+TABLE\s/i.test(head)) return "table";
  if (/^CREATE\s+(?:UNIQUE\s+)?INDEX\s/i.test(head)) return "index";
  if (/^CREATE\s+TRIGGER\s/i.test(head)) return "trigger";
  if (/^CREATE\s+VIEW\s/i.test(head)) return "view";
  if (/^INSERT\s+INTO\s/i.test(head)) return "data";
  if (/^DELETE\s+FROM\s+["`[]?sqlite_sequence["`\]]?\s*$/i.test(head)) return "data";
  throw new CliError(1, `the database export holds a statement a restore does not load: ${head.slice(0, 80)}`);
}

export interface ExportPlan {
  readonly pragmas: readonly string[];
  readonly tables: readonly string[];
  readonly data: readonly string[];
  readonly indexes: readonly string[];
  readonly views: readonly string[];
  /** Every trigger except the 0007 maintenance guards. */
  readonly triggers: readonly string[];
  /** Triggers that read `service_state` (the maintenance guards); created after the restore's repairs. */
  readonly maintenanceTriggers: readonly string[];
}

/** Sorts an export's statements by kind, keeping each kind's original order. */
export function planExport(sql: string): ExportPlan {
  const plan = { pragmas: [] as string[], tables: [] as string[], data: [] as string[], indexes: [] as string[], views: [] as string[], triggers: [] as string[], maintenanceTriggers: [] as string[] };
  for (const statement of splitSql(sql)) {
    const kind = classifyStatement(statement);
    if (kind === "pragma") plan.pragmas.push(statement);
    else if (kind === "table") plan.tables.push(statement);
    else if (kind === "data") plan.data.push(statement);
    else if (kind === "index") plan.indexes.push(statement);
    else if (kind === "view") plan.views.push(statement);
    else if (/\bservice_state\b/.test(statement)) plan.maintenanceTriggers.push(statement);
    else plan.triggers.push(statement);
  }
  return plan;
}

const joinSql = (statements: readonly string[]) => statements.map((statement) => `${statement};\n`).join("");

function sqlText(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

function sqlValue(value: string | number | null): string {
  if (value === null) return "NULL";
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) throw new Error(`not an integer: ${value}`);
    return String(value);
  }
  return sqlText(value);
}

/** Facts a backup records, read from the export itself rather than from the live store. */
export interface ExportFacts {
  readonly migration: number;
  readonly repositories: RepositoryCounters[];
  readonly finalized: { upload_id: string; repository_id: string; key: string; sha256: string; bytes: number }[];
  readonly pending: PendingUpload[];
  readonly jobs: RetentionJobRecord[];
  readonly credentials: CredentialRecord[];
  readonly grants: GrantRecord[];
}

async function openSqlite(): Promise<typeof DatabaseSync> {
  try {
    return (await import("node:sqlite")).DatabaseSync;
  } catch {
    throw new CliError(1, "this command needs Node's built-in node:sqlite (Node 22.13 or later)");
  }
}

/** Loads the export's tables and rows (no triggers) into memory and reads what the manifest records. */
export async function inspectExport(sql: string): Promise<ExportFacts> {
  const Database = await openSqlite();
  const plan = planExport(sql);
  const db = new Database(":memory:", { enableForeignKeyConstraints: false });
  try {
    try {
      db.exec(joinSql([...plan.tables, ...plan.data]));
    } catch (error) {
      throw new CliError(1, `the database export does not load: ${(error as Error).message}`);
    }
    const all = <T>(query: string) => db.prepare(query).all() as unknown as T[];
    let migrations: { name: string }[];
    try {
      migrations = all<{ name: string }>("SELECT name FROM d1_migrations ORDER BY id");
    } catch {
      throw new CliError(1, "the database export has no d1_migrations table, so its schema version is unknown");
    }
    const numbers = migrations.map((row) => Number(/^([0-9]+)_/.exec(row.name)?.[1] ?? Number.NaN));
    if (numbers.length === 0 || numbers.some((number) => !Number.isSafeInteger(number))) {
      throw new CliError(1, "the database export records no numbered migrations");
    }
    const credentials = all<CredentialRecord & { revoked_at_ms: number | null }>(
      "SELECT credential_id, kind, secret_sha256, expires_at_ms, revoked_at_ms FROM credentials ORDER BY credential_id",
    ).map(({ revoked_at_ms, ...record }) => (revoked_at_ms === null ? record : { ...record, revoked_at_ms }));
    return {
      migration: Math.max(...numbers),
      repositories: all<RepositoryCounters>(
        `SELECT repository_id, generation, retention_epoch, evidence_revision, observation_event_watermark, location_event_watermark
         FROM repositories ORDER BY repository_id`,
      ).map(normalizeCounters),
      finalized: all("SELECT upload_id, repository_id, staging_key AS key, sha256, bytes FROM uploads WHERE state = 'finalized' ORDER BY upload_id"),
      pending: all(
        `SELECT upload_id, repository_id, observation_id, reference_kind, bytes, created_at_ms, expires_at_ms
         FROM uploads WHERE state = 'open' ORDER BY upload_id`,
      ),
      jobs: all(
        `SELECT job_id, repository_id, state, generation, retention_epoch, evidence_revision, updated_at_ms
         FROM retention_jobs ORDER BY job_id`,
      ),
      credentials,
      grants: all("SELECT credential_id, repository_id, role FROM credential_grants ORDER BY credential_id, repository_id, role"),
    };
  } finally {
    db.close();
  }
}

function normalizeCounters(row: RepositoryCounters): RepositoryCounters {
  return {
    repository_id: row.repository_id,
    generation: row.generation,
    retention_epoch: row.retention_epoch,
    evidence_revision: row.evidence_revision,
    observation_event_watermark: row.observation_event_watermark,
    location_event_watermark: row.location_event_watermark,
  };
}

// ---- Files ------------------------------------------------------------------------------

function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function digestFile(path: string): { sha256: string; bytes: number } | null {
  if (!existsSync(path)) return null;
  const bytes = readFileSync(path);
  return { sha256: sha256Hex(bytes), bytes: bytes.byteLength };
}

/** Writes a sibling temporary file and renames it over `path`, so readers see the old or the new file whole. */
function atomicWrite(path: string, text: string): void {
  const temporary = join(dirname(path), `.${basename(path)}.${process.pid}.tmp`);
  try {
    writeFileSync(temporary, text, { mode: 0o600 });
    renameSync(temporary, path);
  } catch (error) {
    rmSync(temporary, { force: true });
    throw new CliError(1, `could not write ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
}

/** Reads and strictly validates `DIR/manifest.json`; an invalid manifest is a usage error (exit 2). */
export function loadManifest(backupDir: string): BackupManifest {
  const path = join(backupDir, MANIFEST_FILE);
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(path);
  } catch (error) {
    throw new CliError(2, `could not read the backup manifest ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  try {
    return parseBackupManifest(bytes);
  } catch (error) {
    if (!(error instanceof ApiError)) throw error;
    throw new CliError(2, `the backup manifest ${path} is invalid at "${error.location ?? ""}": ${error.message}`);
  }
}

// ---- verify-backup -----------------------------------------------------------------------

export interface BackupProblem {
  readonly file: string;
  readonly problem: "missing" | "size" | "digest";
}

export interface VerifyReport {
  readonly backup_id: string;
  readonly ok: boolean;
  readonly checked_files: number;
  readonly problems: BackupProblem[];
  /** Objects already missing when the backup was taken; known gaps, not verification failures. */
  readonly missing_objects: readonly MissingObject[];
}

/** Checks the manifest, the database export, and every object's size and SHA-256. */
export function verifyBackupDirectory(backupDir: string): { manifest: BackupManifest; report: VerifyReport } {
  const manifest = loadManifest(backupDir);
  const problems: BackupProblem[] = [];
  if (manifest.backup_id !== basename(resolve(backupDir))) {
    throw new CliError(2, `the backup manifest names backup ${manifest.backup_id}, but it lies in ${basename(resolve(backupDir))}`);
  }
  const files = [manifest.database, ...manifest.objects];
  for (const expected of files) {
    const found = digestFile(join(backupDir, ...expected.file.split("/")));
    if (found === null) problems.push({ file: expected.file, problem: "missing" });
    else if (found.bytes !== expected.bytes) problems.push({ file: expected.file, problem: "size" });
    else if (found.sha256 !== expected.sha256) problems.push({ file: expected.file, problem: "digest" });
  }
  return {
    manifest,
    report: { backup_id: manifest.backup_id, ok: problems.length === 0, checked_files: files.length, problems, missing_objects: manifest.missing_objects },
  };
}

export async function verifyBackup(options: { readonly backup: string }, io: OperatorIo): Promise<VerifyReport> {
  const { report } = verifyBackupDirectory(resolve(options.backup));
  io.stdout(`${JSON.stringify(report, null, 2)}\n`);
  if (!report.ok) {
    throw new CliError(1, `backup ${report.backup_id} failed verification: ${report.problems.map((p) => `${p.file} (${p.problem})`).join(", ")}`);
  }
  return report;
}

// ---- backup -----------------------------------------------------------------------------

export interface BackupOptions {
  readonly endpoint: string;
  readonly tokenFile: string;
  readonly environment: string;
  readonly destination: string;
  readonly config?: string;
}

export interface BackupSummary {
  readonly backup_id: string;
  readonly path: string;
  readonly boundary: BackupManifest["boundary"];
  readonly objects: number;
  readonly missing_objects: readonly MissingObject[];
  readonly pending_uploads: number;
  readonly removed_backups: readonly string[];
}

interface MaintenanceAnswer {
  readonly mode: string;
  readonly since_ms: number | null;
  readonly boundary_id: string | null;
  readonly repositories: RepositoryCounters[];
}

function readMaintenanceAnswer(body: unknown, label: string): MaintenanceAnswer {
  const record = body as { maintenance?: { mode?: unknown; since_ms?: unknown; boundary_id?: unknown }; repositories?: unknown };
  const state = record.maintenance;
  if (state === undefined || typeof state.mode !== "string" || !Array.isArray(record.repositories)) {
    throw new CliError(1, `${label} answered with an unexpected body`);
  }
  return {
    mode: state.mode,
    since_ms: Number.isSafeInteger(state.since_ms) ? (state.since_ms as number) : null,
    boundary_id: typeof state.boundary_id === "string" ? state.boundary_id : null,
    repositories: (record.repositories as RepositoryCounters[]).map(normalizeCounters),
  };
}

async function setMaintenanceMode(
  io: OperatorIo,
  endpoint: string,
  token: string,
  request: { mode: "open" | "quiesced"; reason?: string; boundary_id?: string },
): Promise<MaintenanceAnswer> {
  const result = await operatorPost(io, endpoint, token, "/v1/operator/maintenance", JSON.stringify(request));
  return readMaintenanceAnswer(result.body, "POST /v1/operator/maintenance");
}

/** `credential.revoke` audit records at or after `sinceMs`: revocations inside the quiesced window. */
async function windowRevocations(io: OperatorIo, endpoint: string, token: string, sinceMs: number): Promise<WindowRevocation[]> {
  const found: WindowRevocation[] = [];
  let cursor: string | undefined;
  do {
    const query = { action: "credential.revoke", since_ms: sinceMs, page_size: LIMITS.page_max_records, ...(cursor === undefined ? {} : { cursor }) };
    const { body } = await operatorPost(io, endpoint, token, "/v1/operator/audit/query", JSON.stringify(query));
    const page = body as { items?: unknown; has_more?: unknown; next_cursor?: unknown };
    if (!Array.isArray(page.items) || typeof page.has_more !== "boolean") throw new CliError(1, "POST /v1/operator/audit/query answered with an unexpected body");
    for (const item of page.items as { target_credential_id?: unknown; at_ms?: unknown }[]) {
      if (typeof item.target_credential_id !== "string" || !FORMATS.serverId.test(item.target_credential_id) || !Number.isSafeInteger(item.at_ms)) {
        throw new CliError(1, "POST /v1/operator/audit/query answered a revocation without a credential and time");
      }
      found.push({ credential_id: item.target_credential_id, at_ms: item.at_ms as number });
    }
    cursor = page.has_more && typeof page.next_cursor === "string" ? page.next_cursor : undefined;
  } while (cursor !== undefined);
  return found;
}

function maintenanceCommand(endpoint: string, boundaryId: string): string {
  return `just operator maintenance --endpoint ${endpoint} --token-file <operator token path> --mode open --boundary-id ${boundaryId}`;
}

/**
 * Quiesces the store for a fresh boundary, exports D1 through Wrangler,
 * reopens the store (always, even when the export failed), then copies every
 * object a finalized upload references and digest-checks it. The manifest is
 * written last; a backup directory without one is incomplete and is removed
 * on failure. Keeps the `BACKUPS_KEPT` newest complete backups.
 */
export async function backup(options: BackupOptions, io: OperatorIo): Promise<BackupSummary> {
  requireCloudflare(io, "backup");
  const endpoint = checkEndpoint(options.endpoint);
  const environment = checkEnvironmentName("--env", options.environment);
  const config = resolve(options.config ?? DEFAULT_CONFIG);
  const resources = wranglerResources(readConfig(config), environment);
  const destination = resolve(options.destination);
  if (!existsSync(destination) || !statSync(destination).isDirectory()) throw new CliError(2, `--destination ${destination} is not an existing directory`);
  const token = readTokenFile(options.tokenFile);

  const backupId = randomId();
  const directory = join(destination, backupId);
  mkdirSync(join(directory, "objects"), { recursive: true });
  try {
    let quiesced: MaintenanceAnswer;
    try {
      quiesced = await setMaintenanceMode(io, endpoint, token, { mode: "quiesced", reason: `backup ${backupId}`, boundary_id: backupId });
    } catch (error) {
      // The quiesce may have applied with its answer lost. Reopening by this
      // backup's own boundary undoes only that; anyone else's quiesce answers 409 and stays.
      await setMaintenanceMode(io, endpoint, token, { mode: "open", boundary_id: backupId }).catch(() => undefined);
      throw error;
    }
    if (quiesced.mode !== "quiesced" || quiesced.boundary_id !== backupId || quiesced.since_ms === null) {
      throw new CliError(1, "the store did not confirm the quiesce for this backup's boundary");
    }
    let revocations: WindowRevocation[] = [];
    let failure: unknown = null;
    try {
      const exported = wrangler(io, config, ["d1", "export", resources.d1_database_name, "--env", environment, "--remote", "--output", join(directory, DATABASE_FILE)]);
      if (exported.status !== 0) throw wranglerFailure(`d1 export ${resources.d1_database_name}`, exported);
      revocations = await windowRevocations(io, endpoint, token, quiesced.since_ms);
    } catch (error) {
      failure = error;
    } finally {
      try {
        await setMaintenanceMode(io, endpoint, token, { mode: "open", reason: `backup ${backupId} finished`, boundary_id: backupId });
      } catch (error) {
        const cause = failure instanceof Error ? `${failure.message}\n` : "";
        throw new CliError(1, `${cause}could not reopen the store, which is still quiesced for boundary ${backupId}: ${(error as Error).message}\nreopen it with: ${maintenanceCommand(endpoint, backupId)}`);
      }
    }
    if (failure !== null) throw failure;
    const resumedAt = io.now();

    const sql = readFileSync(join(directory, DATABASE_FILE), "utf8");
    const facts = await inspectExport(sql);
    if (JSON.stringify(facts.repositories) !== JSON.stringify(quiesced.repositories)) {
      throw new CliError(1, "the database export does not match the repository counters read at the boundary; the quiesce did not hold");
    }

    const objects: BackupObject[] = [];
    const missing: MissingObject[] = [];
    for (const upload of facts.finalized) {
      const file = `objects/${upload.upload_id}`;
      const target = join(directory, "objects", upload.upload_id);
      const got = wrangler(io, config, ["r2", "object", "get", `${resources.r2_bucket}/${upload.key}`, "--env", environment, "--remote", "--file", target]);
      if (got.status !== 0) {
        if (!/does not exist|NoSuchKey|not found/i.test(`${got.stdout}${got.stderr}`)) throw wranglerFailure(`r2 object get ${upload.key}`, got);
        rmSync(target, { force: true });
        missing.push({ ...upload, reason: "absent" });
        continue;
      }
      const found = digestFile(target);
      if (found === null || found.sha256 !== upload.sha256 || found.bytes !== upload.bytes) {
        rmSync(target, { force: true });
        missing.push({ ...upload, reason: found === null ? "absent" : "digest-mismatch" });
        continue;
      }
      objects.push({ ...upload, file, sha256: found.sha256, bytes: found.bytes });
    }

    const database = digestFile(join(directory, DATABASE_FILE))!;
    const manifest: BackupManifest = {
      format_version: BACKUP_FORMAT_VERSION,
      backup_id: backupId,
      created_at_ms: io.now(),
      boundary: { boundary_id: backupId, quiesced_at_ms: quiesced.since_ms, resumed_at_ms: resumedAt },
      source: { environment: environment as BackupManifest["source"]["environment"], d1_database_name: resources.d1_database_name, d1_database_id: resources.d1_database_id, r2_bucket: resources.r2_bucket },
      migration: facts.migration,
      repositories: facts.repositories,
      database: { file: DATABASE_FILE, ...database },
      objects,
      missing_objects: missing,
      pending_uploads: facts.pending,
      retention_jobs: facts.jobs,
      credentials: facts.credentials,
      grants: facts.grants,
      window_revocations: revocations,
    };
    atomicWrite(join(directory, MANIFEST_FILE), `${JSON.stringify(manifest, null, 2)}\n`);

    const removed = rotateBackups(destination, io);
    const summary: BackupSummary = {
      backup_id: backupId,
      path: directory,
      boundary: manifest.boundary,
      objects: objects.length,
      missing_objects: missing,
      pending_uploads: facts.pending.length,
      removed_backups: removed,
    };
    io.stdout(`${JSON.stringify(summary, null, 2)}\n`);
    if (missing.length > 0) io.stderr(`operator: ${missing.length} referenced object(s) were missing or corrupt in the live bucket; they are listed in the manifest\n`);
    return summary;
  } catch (error) {
    rmSync(directory, { recursive: true, force: true });
    throw error;
  }
}

/** Removes complete backups beyond the `BACKUPS_KEPT` newest; a directory with a missing or invalid manifest is never touched. */
export function rotateBackups(destination: string, io: Pick<OperatorIo, "stderr">): string[] {
  const complete: { id: string; created: number }[] = [];
  for (const name of readdirSync(destination)) {
    if (!FORMATS.serverId.test(name) || !existsSync(join(destination, name, MANIFEST_FILE))) continue;
    try {
      const manifest = loadManifest(join(destination, name));
      if (manifest.backup_id === name) complete.push({ id: name, created: manifest.created_at_ms });
    } catch (error) {
      if (!(error instanceof CliError)) throw error;
      io.stderr(`operator: left ${join(destination, name)} in place: ${error.message}\n`);
    }
  }
  complete.sort((a, b) => b.created - a.created || (a.id < b.id ? -1 : 1));
  const removed = complete.slice(BACKUPS_KEPT).map((entry) => entry.id);
  for (const id of removed) rmSync(join(destination, id), { recursive: true, force: true });
  return removed;
}

// ---- restore ------------------------------------------------------------------------------

export interface RestoreOptions {
  readonly backup: string;
  readonly sourceEnvironment: string;
  readonly destinationEnvironment: string;
  readonly destinationTokenFile: string;
  readonly destinationEndpoint?: string;
  readonly replay?: readonly string[];
  readonly report?: string;
  readonly label?: string;
  readonly config?: string;
}

export interface AcknowledgedId {
  readonly repository_id: string;
  readonly observation_id: string;
  /** The resume state that recorded the acknowledgment. */
  readonly state: string;
}

export interface RecoveryReport {
  readonly format_version: 1;
  readonly restore_id: string;
  readonly backup_id: string;
  readonly source: BackupManifest["source"];
  readonly destination: WranglerResources;
  readonly boundary: BackupManifest["boundary"] & { readonly repositories: readonly RepositoryCounters[] };
  readonly generations: readonly { repository_id: string; from: string; to: string }[];
  readonly operator: { readonly credential_id: string; readonly token_file: string; readonly revoked: readonly string[] };
  readonly cursor_key_rotated: true;
  readonly revocations_reapplied: readonly string[];
  readonly objects_restored: number;
  readonly missing_objects: readonly MissingObject[];
  readonly pending_uploads_expired: readonly string[];
  readonly acknowledged: {
    readonly recovered: readonly AcknowledgedId[];
    readonly replayed: readonly AcknowledgedId[];
    readonly unrecoverable: readonly (AcknowledgedId & { readonly reason: string })[];
  };
  readonly replays: readonly { state: string; replay_state: string | null; outcome: string; uploads_not_replayed: number; finalize_command: string | null }[];
  readonly unacknowledged_states: readonly string[];
  /** Never "none": the backup alone cannot show what was acknowledged after the boundary. */
  readonly post_boundary_loss: string;
}

function auditSql(input: {
  at: number;
  action: string;
  actorId: string;
  requestId: string;
  repositoryId?: string;
  targetId?: string;
  detail: unknown;
  when?: string;
}): string {
  const values = [
    sqlValue(input.at),
    sqlText(input.action),
    sqlText(JSON.stringify({ kind: "credential", credential_id: input.actorId })),
    sqlText(input.actorId),
    sqlValue(input.repositoryId ?? null),
    sqlValue(input.targetId ?? null),
    sqlText(input.requestId),
    sqlText(JSON.stringify(input.detail)),
  ].join(", ");
  const columns = "at_ms, action, actor, actor_credential_id, repository_id, target_credential_id, request_id, detail_json";
  return input.when === undefined
    ? `INSERT INTO audit_log(${columns}) VALUES (${values})`
    : `INSERT INTO audit_log(${columns}) SELECT ${values} WHERE ${input.when}`;
}

/** Revokes `credentialId` once, auditing only when the revocation applies. */
function revokeSql(credentialId: string, reason: string, at: number, actorId: string, requestId: string, detail: unknown): string[] {
  const unrevoked = `EXISTS (SELECT 1 FROM credentials WHERE credential_id = ${sqlText(credentialId)} AND revoked_at_ms IS NULL)`;
  return [
    auditSql({ at, action: "credential.revoke", actorId, requestId, targetId: credentialId, detail, when: unrevoked }),
    `UPDATE credentials SET revoked_at_ms = ${at}, revocation_reason = ${sqlText(reason)} WHERE credential_id = ${sqlText(credentialId)} AND revoked_at_ms IS NULL`,
  ];
}

interface RepairInput {
  readonly manifest: BackupManifest;
  readonly restoreId: string;
  readonly nowMs: number;
  readonly operatorSql: string;
  readonly operatorId: string;
  readonly generations: readonly { repository_id: string; from: string; to: string }[];
}

/** The restore's repairs, run while the restored store is quiesced for the restore itself. */
export function repairSql(input: RepairInput): string[] {
  const { manifest, restoreId, nowMs: at, operatorId } = input;
  const requestId = `restore-${restoreId}`;
  const reason = `restore ${restoreId} of backup ${manifest.backup_id}`;
  const statements = [
    `UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = ${sqlText(reason)}, maintenance_since_ms = ${at}, maintenance_boundary_id = ${sqlText(restoreId)} WHERE id = 1`,
    ...splitSql(input.operatorSql),
  ];
  for (const credential of manifest.credentials) {
    if (credential.kind !== "operator" || credential.revoked_at_ms !== undefined) continue;
    statements.push(...revokeSql(credential.credential_id, `operator credentials rotated by ${reason}`, at, operatorId, requestId, { restore_id: restoreId }));
  }
  for (const revocation of manifest.window_revocations) {
    statements.push(
      ...revokeSql(revocation.credential_id, `re-applied by ${reason}: revoked during the backup window`, at, operatorId, requestId, {
        restore_id: restoreId,
        revoked_at_ms: revocation.at_ms,
      }),
    );
  }
  for (const generation of input.generations) {
    statements.push(
      auditSql({ at, action: "restore.generation", actorId: operatorId, requestId, repositoryId: generation.repository_id, detail: { ...generation, backup_id: manifest.backup_id, restore_id: restoreId } }),
      `UPDATE repositories SET generation = ${sqlText(generation.to)} WHERE repository_id = ${sqlText(generation.repository_id)}`,
    );
  }
  const checkedBy = sqlText(JSON.stringify({ kind: "credential", credential_id: operatorId }));
  for (const object of manifest.missing_objects) {
    // Only an object whose newest object event still says `verified` advertises bytes; append `unavailable` to it.
    statements.push(
      `INSERT INTO report_locations(repository_id, event_id, observation_id, reference_kind, reference_digest, report_path,
         location_key, location_kind, locator_json, state, verification_method, checked_at_ms, valid_until_ms, checked_by)
       SELECT l.repository_id, ${sqlText(randomUUID())}, l.observation_id, l.reference_kind, l.reference_digest, l.report_path,
         l.location_key, l.location_kind, l.locator_json, 'unavailable', NULL, ${at}, NULL, ${checkedBy}
       FROM report_locations l
       WHERE l.seq = (SELECT MAX(seq) FROM report_locations
                      WHERE repository_id = ${sqlText(object.repository_id)} AND location_kind = 'object'
                        AND json_extract(locator_json, '$.upload_id') = ${sqlText(object.upload_id)})
         AND l.state = 'verified'`,
    );
  }
  statements.push("UPDATE uploads SET state = 'expired' WHERE state = 'open'");
  return statements;
}

function reopenSql(restoreId: string, backupId: string, at: number, operatorId: string): string {
  return [
    auditSql({ at, action: "maintenance.resume", actorId: operatorId, requestId: `restore-${restoreId}`, detail: { restore_id: restoreId, backup_id: backupId } }),
    `UPDATE service_state SET maintenance_mode = 'open', maintenance_reason = NULL, maintenance_since_ms = NULL, maintenance_boundary_id = NULL
     WHERE id = 1 AND maintenance_boundary_id = ${sqlText(restoreId)}`,
  ].join("; ");
}

/** Refuses a destination that is, or shares a resource with, the live source or the backup's recorded source. */
export function checkIsolation(source: WranglerResources, destination: WranglerResources, recorded: BackupManifest["source"]): void {
  if (source.environment === destination.environment) throw new CliError(2, "--dest-env must differ from --source-env; a restore never writes to its live source");
  for (const [label, other] of [
    ["the source environment", source],
    ["the backup's recorded source", recorded],
  ] as const) {
    for (const field of ["d1_database_name", "d1_database_id", "r2_bucket"] as const) {
      if (other[field] === destination[field]) {
        throw new CliError(2, `the destination ${field} ${destination[field]} is also ${label}'s; restore only into isolated resources`);
      }
    }
  }
}

function ingestIoFor(io: RecoveryIo): IngestIo {
  return {
    fetch: io.fetch,
    sleep: io.sleep ?? ((ms) => new Promise((done) => setTimeout(done, ms))),
    random: io.random ?? Math.random,
    now: io.now,
    // The recovery report owns stdout; the ingestion library's progress goes to stderr.
    stdout: io.stderr,
    stderr: io.stderr,
    readReport: nodeReportReader,
  };
}

function acknowledgedIds(state: ResumeState): string[] {
  return [...(state.ingest.accepted ?? []), ...(state.ingest.already_present ?? [])];
}

function retainedIntact(state: ResumeState): boolean {
  const input = digestFile(state.input.file);
  const collection = digestFile(state.submission.file);
  return input?.sha256 === state.input.sha256 && collection?.sha256 === state.submission.collection_id;
}

/** A fresh resume state for the restored endpoint that resubmits the retained collection with its original IDs. */
function replayState(statePath: string, state: ResumeState, replayPath: string, endpoint: string, io: OperatorIo): { skippedUploads: number } {
  const input = `${replayPath}.input.json`;
  const collection = `${replayPath}.collection.json`;
  copyFileSync(state.input.file, input);
  copyFileSync(state.submission.file, collection);
  const uploads = state.uploads.filter((upload) => digestFile(upload.file)?.sha256 === upload.reference_digest);
  const replay: ResumeState = {
    ...state,
    endpoint,
    input: { ...state.input, file: input },
    submission: { ...state.submission, file: collection },
    ingest: { state: "pending" },
    uploads: uploads.map(({ upload_id: _upload, ...upload }) => ({ ...upload, parts_acknowledged: 0, state: "pending" })),
    updated_at_ms: io.now(),
  };
  atomicWrite(replayPath, `${JSON.stringify(replay, null, 2)}\n`);
  io.stderr(`operator: replaying ${statePath} through ${replayPath}\n`);
  return { skippedUploads: state.uploads.length - uploads.length };
}

/**
 * Restores a verified backup into isolated resources: objects first, then
 * the database (reordered so no trigger fires on a restored row) with the
 * restore's repairs, then a new cursor key, then reopens; finally replays
 * retained post-boundary ingestions and reports what was recovered.
 */
export async function restore(options: RestoreOptions, io: RecoveryIo): Promise<RecoveryReport> {
  requireCloudflare(io, "restore");
  const backupDir = resolve(options.backup);
  const sourceEnvironment = checkEnvironmentName("--source-env", options.sourceEnvironment);
  const destinationEnvironment = checkEnvironmentName("--dest-env", options.destinationEnvironment);
  if (sourceEnvironment === destinationEnvironment) throw new CliError(2, "--dest-env must differ from --source-env; a restore never writes to its live source");
  const config = resolve(options.config ?? DEFAULT_CONFIG);
  const configText = readConfig(config);
  const source = wranglerResources(configText, sourceEnvironment);
  const destination = wranglerResources(configText, destinationEnvironment);
  const manifest = loadManifest(backupDir);
  if (manifest.source.environment !== sourceEnvironment) {
    throw new CliError(2, `backup ${manifest.backup_id} was taken from ${manifest.source.environment}, not --source-env ${sourceEnvironment}`);
  }
  checkIsolation(source, destination, manifest.source);
  const tokenFile = resolve(options.destinationTokenFile);
  if (existsSync(tokenFile)) throw new CliError(2, `--dest-token-file ${tokenFile} already exists; it is never overwritten`);
  const label = options.label ?? DEFAULT_RESTORE_LABEL;
  if (label.length < 1 || label.length > 128) throw new CliError(2, "--label must be 1 to 128 characters");
  const replayPaths = (options.replay ?? []).map((path) => resolve(path));
  if (replayPaths.length > 0 && options.destinationEndpoint === undefined) throw new CliError(2, "--replay needs --dest-endpoint, the restored service's origin");
  const endpoint = options.destinationEndpoint === undefined ? null : checkEndpoint(options.destinationEndpoint);
  const states = replayPaths.map((path) => ({ path, state: loadState(path) }));
  if (options.report !== undefined && existsSync(resolve(options.report))) throw new CliError(2, `--report ${resolve(options.report)} already exists`);

  const { report: verified } = verifyBackupDirectory(backupDir);
  if (!verified.ok) {
    throw new CliError(1, `backup ${manifest.backup_id} failed verification, so nothing was restored: ${verified.problems.map((p) => `${p.file} (${p.problem})`).join(", ")}`);
  }
  const plan = planExport(readFileSync(join(backupDir, ...manifest.database.file.split("/")), "utf8"));

  const existing = queryRows(
    io,
    config,
    destination.d1_database_name,
    destinationEnvironment,
    "SELECT name FROM sqlite_master WHERE substr(name, 1, 7) != 'sqlite_' AND substr(name, 1, 4) != '_cf_' LIMIT 5",
  );
  if (existing.length > 0) {
    throw new CliError(1, `the destination database ${destination.d1_database_name} is not empty (it holds ${existing.map((row) => String(row["name"])).join(", ")}); restore only into a newly created database`);
  }

  for (const object of manifest.objects) {
    const put = wrangler(io, config, ["r2", "object", "put", `${destination.r2_bucket}/${object.key}`, "--env", destinationEnvironment, "--remote", "--file", join(backupDir, ...object.file.split("/"))]);
    if (put.status !== 0) throw wranglerFailure(`r2 object put ${object.key}`, put);
  }

  const restoreId = randomId();
  const nowMs = io.now();
  const minted = mintToken();
  const record = bootstrapRecord({
    credentialId: minted.credentialId,
    digest: await secretDigest(minted.secret),
    label,
    nowMs,
    expiresInDays: LIMITS.credential_expiry_days,
    requestId: `restore-${restoreId}`,
  });
  const generations = manifest.repositories.map((repository) => ({ repository_id: repository.repository_id, from: repository.generation, to: randomId() }));
  const pragmas = plan.pragmas.some((pragma) => /defer_foreign_keys/i.test(pragma)) ? plan.pragmas : ["PRAGMA defer_foreign_keys = TRUE", ...plan.pragmas];
  const repairs = repairSql({ manifest, restoreId, nowMs, operatorSql: bootstrapSql(record), operatorId: minted.credentialId, generations });
  const script = joinSql([...pragmas, ...plan.tables, ...plan.data, ...plan.indexes, ...plan.views, ...plan.triggers, ...repairs, ...plan.maintenanceTriggers]);

  writeSecretFile(tokenFile, minted.token, io);
  const work = mkdtempSync(join(tmpdir(), "evidence-restore-"));
  try {
    const scriptPath = join(work, "restore.sql");
    writeFileSync(scriptPath, script, { mode: 0o600 });
    const loaded = wrangler(io, config, ["d1", "execute", destination.d1_database_name, "--env", destinationEnvironment, "--remote", "--yes", "--file", scriptPath]);
    if (loaded.status !== 0) {
      rmSync(tokenFile, { force: true });
      throw new CliError(1, `${wranglerFailure(`d1 execute ${destination.d1_database_name}`, loaded).message}\n${tokenFile} was removed; drop and recreate the destination database before retrying`);
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }

  const key = randomBytes(32).toString("base64url");
  const secret = wrangler(io, config, ["secret", "put", "CURSOR_HMAC_KEY", "--env", destinationEnvironment], key);
  if (secret.status !== 0) {
    throw new CliError(1, `${wranglerFailure("secret put CURSOR_HMAC_KEY", secret).message}\nthe restored store stays quiesced; set the secret, then reopen it with the new operator token in ${tokenFile}`);
  }
  const reopened = wrangler(io, config, ["d1", "execute", destination.d1_database_name, "--env", destinationEnvironment, "--remote", "--yes", "--command", reopenSql(restoreId, manifest.backup_id, io.now(), minted.credentialId)]);
  if (reopened.status !== 0) throw wranglerFailure(`d1 execute ${destination.d1_database_name}`, reopened);

  const recovered: AcknowledgedId[] = [];
  const replayed: AcknowledgedId[] = [];
  const unrecoverable: (AcknowledgedId & { reason: string })[] = [];
  const replays: RecoveryReport["replays"][number][] = [];
  const unacknowledged: string[] = [];
  let replayFailed = false;
  for (const { path, state } of states) {
    const ids = acknowledgedIds(state);
    if (state.ingest.state !== "acknowledged") {
      unacknowledged.push(path);
      continue;
    }
    const present = new Set(
      queryRows(
        io,
        config,
        destination.d1_database_name,
        destinationEnvironment,
        `SELECT observation_id FROM observations WHERE repository_id = ${sqlText(state.repository_id)} AND observation_id IN (SELECT value FROM json_each(${sqlText(JSON.stringify(ids))}))`,
      ).map((row) => String(row["observation_id"])),
    );
    const entry = (id: string) => ({ repository_id: state.repository_id, observation_id: id, state: path });
    ids.filter((id) => present.has(id)).forEach((id) => recovered.push(entry(id)));
    const absent = ids.filter((id) => !present.has(id));
    if (absent.length === 0) {
      replays.push({ state: path, replay_state: null, outcome: "all acknowledged IDs are in the backup", uploads_not_replayed: 0, finalize_command: null });
      continue;
    }
    if (!retainedIntact(state)) {
      absent.forEach((id) => unrecoverable.push({ ...entry(id), reason: "absent from the backup and no intact retained collection beside the state" }));
      replays.push({ state: path, replay_state: null, outcome: "no retained bundle", uploads_not_replayed: 0, finalize_command: null });
      continue;
    }
    const replayPath = `${path}.restore-${restoreId}.json`;
    const { skippedUploads } = replayState(path, state, replayPath, endpoint!, io);
    try {
      await resume(replayPath, ingestIoFor(io));
    } catch (error) {
      if (!(error instanceof CliError)) throw error;
      replayFailed = true;
      absent.forEach((id) => unrecoverable.push({ ...entry(id), reason: `replay failed: ${error.message.split("\n")[0]}` }));
      replays.push({ state: path, replay_state: replayPath, outcome: `failed; retry with: ${resumeCommand(replayPath)}`, uploads_not_replayed: skippedUploads, finalize_command: null });
      continue;
    }
    const after = loadState(replayPath);
    const acknowledged = new Set(acknowledgedIds(after));
    for (const id of absent) {
      if (acknowledged.has(id)) replayed.push(entry(id));
      else unrecoverable.push({ ...entry(id), reason: "the replay was not acknowledged for this ID" });
    }
    const staged = after.uploads.some((upload) => upload.state === "staged");
    replays.push({
      state: path,
      replay_state: replayPath,
      outcome: "replayed",
      uploads_not_replayed: skippedUploads,
      finalize_command: staged ? `just evidence-finalize ${replayPath} --token-file <verifier token path>` : null,
    });
  }

  const report: RecoveryReport = {
    format_version: REPORT_FORMAT_VERSION,
    restore_id: restoreId,
    backup_id: manifest.backup_id,
    source: manifest.source,
    destination,
    boundary: { ...manifest.boundary, repositories: manifest.repositories },
    generations,
    operator: {
      credential_id: minted.credentialId,
      token_file: tokenFile,
      revoked: manifest.credentials.filter((c) => c.kind === "operator" && c.revoked_at_ms === undefined).map((c) => c.credential_id),
    },
    cursor_key_rotated: true,
    revocations_reapplied: manifest.window_revocations.map((revocation) => revocation.credential_id),
    objects_restored: manifest.objects.length,
    missing_objects: manifest.missing_objects,
    pending_uploads_expired: manifest.pending_uploads.map((upload) => upload.upload_id),
    acknowledged: { recovered, replayed, unrecoverable },
    replays,
    unacknowledged_states: unacknowledged,
    post_boundary_loss:
      states.length === 0
        ? "unknown: no replay inputs were given, and the backup alone cannot show what was acknowledged after the boundary"
        : `known only for the ${states.length} replay input(s) given (${unrecoverable.length} unrecoverable); acknowledgments recorded nowhere else after the boundary cannot be listed`,
  };
  const text = `${JSON.stringify(report, null, 2)}\n`;
  if (options.report === undefined) io.stdout(text);
  else atomicWrite(resolve(options.report), text);
  io.stderr(
    `operator: restored backup ${manifest.backup_id} into ${destinationEnvironment}: ${recovered.length} recovered, ${replayed.length} replayed, ` +
      `${unrecoverable.length} unrecoverable acknowledged ID(s); ${manifest.missing_objects.length} missing object(s); new operator token in ${tokenFile}\n`,
  );
  if (replayFailed) throw new CliError(1, "at least one replay failed; the report lists its retry command");
  return report;
}

// ---- recover-jobs and maintenance ------------------------------------------------------------

export async function recoverJobsCommand(options: { readonly endpoint: string; readonly tokenFile: string }, io: OperatorIo): Promise<unknown> {
  const endpoint = checkEndpoint(options.endpoint);
  const token = readTokenFile(options.tokenFile);
  const { body } = await operatorPost(io, endpoint, token, "/v1/operator/jobs/recover", "{}");
  io.stdout(`${JSON.stringify(body, null, 2)}\n`);
  return body;
}

export async function maintenance(
  options: { readonly endpoint: string; readonly tokenFile: string; readonly mode: string; readonly reason?: string; readonly boundaryId?: string },
  io: OperatorIo,
): Promise<unknown> {
  const endpoint = checkEndpoint(options.endpoint);
  if (options.mode !== "open" && options.mode !== "quiesced") throw new CliError(2, "--mode must be open or quiesced");
  if (options.boundaryId !== undefined && !FORMATS.serverId.test(options.boundaryId)) throw new CliError(2, "--boundary-id must be a 32-character hex ID");
  if (options.reason !== undefined && (options.reason.length < 1 || options.reason.length > 512)) throw new CliError(2, "--reason must be 1 to 512 characters");
  const token = readTokenFile(options.tokenFile);
  const request = {
    mode: options.mode,
    ...(options.reason === undefined ? {} : { reason: options.reason }),
    ...(options.boundaryId === undefined ? {} : { boundary_id: options.boundaryId }),
  };
  const { body } = await operatorPost(io, endpoint, token, "/v1/operator/maintenance", JSON.stringify(request));
  io.stdout(`${JSON.stringify(body, null, 2)}\n`);
  return body;
}

// ---- Command line ------------------------------------------------------------------------------

export type RecoveryCommand = "backup" | "verify-backup" | "restore" | "recover-jobs" | "maintenance";

function flags(argv: readonly string[], names: readonly string[], required: readonly string[], multiple: readonly string[] = []) {
  let values: Record<string, string | string[] | undefined>;
  try {
    values = parseArgs({
      args: [...argv],
      options: Object.fromEntries(names.map((name) => [name, { type: "string" as const, multiple: multiple.includes(name) }])),
      strict: true,
      allowPositionals: false,
    }).values as Record<string, string | string[] | undefined>;
  } catch (error) {
    throw new CliError(2, (error as Error).message);
  }
  const absent = required.filter((name) => values[name] === undefined);
  if (absent.length > 0) throw new CliError(2, `missing ${absent.map((name) => `--${name}`).join(", ")}`);
  const one = (name: string) => values[name] as string | undefined;
  return { one, all: (name: string) => (values[name] as string[] | undefined) ?? [] };
}

/** Runs one recovery command; throws `CliError` on failure. */
export async function runRecovery(command: RecoveryCommand, argv: readonly string[], io: RecoveryIo): Promise<void> {
  switch (command) {
    case "backup": {
      const f = flags(argv, ["endpoint", "token-file", "env", "destination", "config"], ["endpoint", "token-file", "env", "destination"]);
      const config = f.one("config");
      await backup(
        { endpoint: f.one("endpoint")!, tokenFile: f.one("token-file")!, environment: f.one("env")!, destination: f.one("destination")!, ...(config === undefined ? {} : { config }) },
        io,
      );
      return;
    }
    case "verify-backup": {
      const f = flags(argv, ["backup"], ["backup"]);
      await verifyBackup({ backup: f.one("backup")! }, io);
      return;
    }
    case "restore": {
      const f = flags(
        argv,
        ["backup", "source-env", "dest-env", "dest-token-file", "dest-endpoint", "replay", "report", "label", "config"],
        ["backup", "source-env", "dest-env", "dest-token-file"],
        ["replay"],
      );
      const optional = (key: keyof RestoreOptions, name: string) => (f.one(name) === undefined ? {} : { [key]: f.one(name) });
      await restore(
        {
          backup: f.one("backup")!,
          sourceEnvironment: f.one("source-env")!,
          destinationEnvironment: f.one("dest-env")!,
          destinationTokenFile: f.one("dest-token-file")!,
          replay: f.all("replay"),
          ...optional("destinationEndpoint", "dest-endpoint"),
          ...optional("report", "report"),
          ...optional("label", "label"),
          ...optional("config", "config"),
        },
        io,
      );
      return;
    }
    case "recover-jobs": {
      const f = flags(argv, ["endpoint", "token-file"], ["endpoint", "token-file"]);
      await recoverJobsCommand({ endpoint: f.one("endpoint")!, tokenFile: f.one("token-file")! }, io);
      return;
    }
    case "maintenance": {
      const f = flags(argv, ["endpoint", "token-file", "mode", "reason", "boundary-id"], ["endpoint", "token-file", "mode"]);
      const reason = f.one("reason");
      const boundaryId = f.one("boundary-id");
      await maintenance(
        {
          endpoint: f.one("endpoint")!,
          tokenFile: f.one("token-file")!,
          mode: f.one("mode")!,
          ...(reason === undefined ? {} : { reason }),
          ...(boundaryId === undefined ? {} : { boundaryId }),
        },
        io,
      );
      return;
    }
  }
}
