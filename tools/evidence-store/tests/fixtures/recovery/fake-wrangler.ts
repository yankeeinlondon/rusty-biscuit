// A fake of the Wrangler commands the recovery CLI runs (`d1 export`,
// `d1 execute --file/--command`, `r2 object get/put`, `secret put`) over real
// local state: `node:sqlite` databases built from the package's own
// `migrations/*.sql` (with Wrangler's `d1_migrations` table) and an R2 bucket
// per environment in a temporary directory. Plus a fake of the operator API's
// maintenance and audit routes over the same source database, and a seed that
// writes rows the way the Worker does.
//
// The fake export deliberately emits every trigger before the rows, so a
// restore that ran the export in its own order would fire triggers on
// restored rows; the restore must reorder it.
import { createHash, randomUUID } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { fileURLToPath } from "node:url";
import type { SpawnResult } from "../../../cli/operator.ts";
import { secretDigest } from "../../../src/tokens.ts";

const MIGRATIONS = fileURLToPath(new URL("../../../migrations/", import.meta.url));

export const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");

/** A database as `wrangler d1 migrations apply` leaves it. */
export function migratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(":memory:");
  db.exec("CREATE TABLE d1_migrations(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE, applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL)");
  for (const name of readdirSync(MIGRATIONS).filter((file) => file.endsWith(".sql")).sort()) {
    db.exec(readFileSync(join(MIGRATIONS, name), "utf8"));
    db.prepare("INSERT INTO d1_migrations(name, applied_at) VALUES (?, '2026-11-01 00:00:00')").run(name);
  }
  return db;
}

function literal(value: unknown): string {
  if (value === null) return "NULL";
  if (typeof value === "number" || typeof value === "bigint") return String(value);
  if (value instanceof Uint8Array) return `X'${Buffer.from(value).toString("hex")}'`;
  return `'${String(value).replaceAll("'", "''")}'`;
}

/** SQL in Wrangler's export style, except that triggers come before rows (see the module comment). */
export function exportSql(db: DatabaseSync): string {
  const objects = db
    .prepare("SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY rowid")
    .all() as { type: string; name: string; sql: string }[];
  const lines = ["PRAGMA defer_foreign_keys=TRUE;"];
  const tables = objects.filter((object) => object.type === "table");
  for (const table of tables) lines.push(`${table.sql};`);
  for (const trigger of objects.filter((object) => object.type === "trigger")) lines.push(`${trigger.sql};`);
  for (const table of tables) {
    const columns = (db.prepare(`PRAGMA table_xinfo("${table.name}")`).all() as { name: string; hidden: number }[])
      .filter((column) => column.hidden === 0)
      .map((column) => column.name);
    const quoted = columns.map((column) => `"${column}"`).join(",");
    for (const row of db.prepare(`SELECT ${quoted} FROM "${table.name}" ORDER BY rowid`).all() as Record<string, unknown>[]) {
      lines.push(`INSERT INTO "${table.name}" (${quoted}) VALUES(${columns.map((column) => literal(row[column])).join(",")});`);
    }
  }
  lines.push("DELETE FROM sqlite_sequence;");
  for (const row of db.prepare("SELECT name, seq FROM sqlite_sequence").all() as { name: string; seq: number }[]) {
    lines.push(`INSERT INTO "sqlite_sequence" VALUES(${literal(row.name)},${row.seq});`);
  }
  for (const index of objects.filter((object) => object.type === "index")) lines.push(`${index.sql};`);
  return `${lines.join("\n")}\n`;
}

export interface FakeEnvironment {
  readonly database: string;
  readonly databaseId: string;
  readonly bucket: string;
}

export interface WranglerCall {
  readonly args: readonly string[];
  readonly input?: string;
}

const VALUE_FLAGS = new Set(["--env", "--output", "--file", "--command", "--config"]);
const BOOLEAN_FLAGS = new Set(["--remote", "--yes", "--json"]);

export class FakeWrangler {
  readonly databases = new Map<string, DatabaseSync>();
  readonly secrets = new Map<string, string>();
  readonly calls: WranglerCall[] = [];
  /** Return a result to fail a call before it acts; checked in order. */
  readonly faults: ((call: WranglerCall) => SpawnResult | undefined)[] = [];

  constructor(
    readonly root: string,
    readonly environments: Record<string, FakeEnvironment>,
  ) {}

  /** The `wrangler.jsonc` text these environments correspond to. */
  config(): string {
    const env = Object.fromEntries(
      Object.entries(this.environments).map(([name, e]) => [
        name,
        {
          // Comments as in the real file, to exercise the JSONC reader.
          d1_databases: [{ binding: "EVIDENCE_DB", database_name: e.database, database_id: e.databaseId, migrations_dir: "migrations" }],
          r2_buckets: [{ binding: "EVIDENCE_BUCKET", bucket_name: e.bucket }],
        },
      ]),
    );
    return `// fake wrangler.jsonc\n${JSON.stringify({ name: "evidence-store", env }, null, 2)}\n`;
  }

  database(environment: string): DatabaseSync {
    const name = this.environments[environment]!.database;
    let db = this.databases.get(name);
    if (db === undefined) {
      db = new DatabaseSync(":memory:");
      this.databases.set(name, db);
    }
    return db;
  }

  private objectPath(bucket: string, key: string): string {
    return join(this.root, "r2", bucket, encodeURIComponent(key));
  }

  putObject(bucket: string, key: string, bytes: Uint8Array): void {
    mkdirSync(join(this.root, "r2", bucket), { recursive: true });
    writeFileSync(this.objectPath(bucket, key), bytes);
  }

  getObject(bucket: string, key: string): Buffer | null {
    const path = this.objectPath(bucket, key);
    return existsSync(path) ? readFileSync(path) : null;
  }

  objectKeys(bucket: string): string[] {
    const dir = join(this.root, "r2", bucket);
    return existsSync(dir) ? readdirSync(dir).map(decodeURIComponent).sort() : [];
  }

  /** `OperatorIo.spawn`. */
  readonly spawn = (command: string, args: readonly string[], options?: { readonly input?: string }): SpawnResult => {
    const call: WranglerCall = options?.input === undefined ? { args: args.slice(1) } : { args: args.slice(1), input: options.input };
    this.calls.push(call);
    if (command !== process.execPath || !args[0]?.endsWith(join("wrangler", "bin", "wrangler.js"))) return fail(`not a Wrangler invocation: ${command}`);
    for (const fault of this.faults) {
      const result = fault(call);
      if (result !== undefined) return result;
    }
    const positional: string[] = [];
    const flags = new Map<string, string | true>();
    for (let index = 0; index < call.args.length; index += 1) {
      const arg = call.args[index]!;
      if (VALUE_FLAGS.has(arg)) flags.set(arg, call.args[++index] ?? "");
      else if (BOOLEAN_FLAGS.has(arg)) flags.set(arg, true);
      else if (arg.startsWith("--")) return fail(`unknown flag ${arg}`);
      else positional.push(arg);
    }
    if (typeof flags.get("--config") !== "string") return fail("no --config");
    const environmentName = flags.get("--env");
    const environment = typeof environmentName === "string" ? this.environments[environmentName] : undefined;
    if (environment === undefined) return fail(`unknown --env ${String(environmentName)}`);
    const [group, action, target] = positional;
    if (group === "secret" && action === "put" && target !== undefined) {
      if (options?.input === undefined || options.input.length === 0) return fail("secret put reads the value from stdin");
      this.secrets.set(`${environmentName}:${target}`, options.input);
      return ok("Success! Uploaded secret");
    }
    if (flags.get("--remote") !== true) return fail("this fake only models --remote");
    if (group === "d1") {
      if (target !== environment.database) return fail(`database ${target} is not bound in ${String(environmentName)}`);
      const db = this.database(environmentName as string);
      if (action === "export") {
        const output = flags.get("--output");
        if (typeof output !== "string") return fail("export needs --output");
        writeFileSync(output, exportSql(db));
        return ok("exported");
      }
      if (action === "execute") {
        const file = flags.get("--file");
        const sql = typeof file === "string" ? readFileSync(file, "utf8") : flags.get("--command");
        if (typeof sql !== "string") return fail("execute needs --file or --command");
        if (flags.get("--json") === true) {
          try {
            return ok(JSON.stringify([{ results: db.prepare(sql).all(), success: true, meta: {} }]));
          } catch (error) {
            return fail((error as Error).message);
          }
        }
        if (flags.get("--yes") !== true) return fail("execute without --yes would prompt");
        // D1 applies one execution atomically.
        db.exec("BEGIN");
        try {
          db.exec(sql);
          db.exec("COMMIT");
        } catch (error) {
          db.exec("ROLLBACK");
          return fail(`D1_ERROR: ${(error as Error).message}`);
        }
        return ok("executed");
      }
    }
    if (group === "r2" && action === "object" && positional.length === 4) {
      const [, , verb, path] = positional as [string, string, string, string];
      const slash = path.indexOf("/");
      const bucket = path.slice(0, slash);
      const key = path.slice(slash + 1);
      if (bucket !== environment.bucket) return fail(`bucket ${bucket} is not bound in ${String(environmentName)}`);
      const file = flags.get("--file");
      if (typeof file !== "string") return fail("r2 object needs --file");
      if (verb === "get") {
        const bytes = this.getObject(bucket, key);
        if (bytes === null) return fail(`The specified key does not exist.`);
        writeFileSync(file, bytes);
        return ok("downloaded");
      }
      if (verb === "put") {
        this.putObject(bucket, key, readFileSync(file));
        return ok("uploaded");
      }
    }
    return fail(`unsupported: ${positional.join(" ")}`);
  };
}

function ok(stdout: string): SpawnResult {
  return { status: 0, stdout, stderr: "" };
}

export function fail(stderr: string): SpawnResult {
  return { status: 1, stdout: "", stderr };
}

// ---- The operator API's maintenance and audit routes over the source database ----

export const OPERATOR_TOKEN = `ebt_${"0".repeat(31)}1.${"o".repeat(43)}`;

export interface ApiCall {
  readonly method: string;
  readonly path: string;
  readonly body: unknown;
}

/** Answers `/v1/operator/maintenance` and `/v1/operator/audit/query` as the Worker does, over `db`. */
export function fakeOperatorApi(db: () => DatabaseSync, now: () => number, calls: ApiCall[], operatorId: string) {
  const counters = () =>
    db()
      .prepare(
        `SELECT repository_id, generation, retention_epoch, evidence_revision, observation_event_watermark, location_event_watermark
         FROM repositories ORDER BY repository_id`,
      )
      .all();
  const state = () => {
    const row = db().prepare("SELECT maintenance_mode, maintenance_reason, maintenance_since_ms, maintenance_boundary_id FROM service_state WHERE id = 1").get() as Record<string, unknown>;
    return { mode: row["maintenance_mode"], reason: row["maintenance_reason"], since_ms: row["maintenance_since_ms"], boundary_id: row["maintenance_boundary_id"] };
  };
  const audit = (action: string, detail: unknown) =>
    db()
      .prepare(`INSERT INTO audit_log(at_ms, action, actor, actor_credential_id, request_id, detail_json) VALUES (?, ?, ?, ?, ?, ?)`)
      .run(now(), action, JSON.stringify({ kind: "credential", credential_id: operatorId }), operatorId, randomUUID(), JSON.stringify(detail));
  const json = (status: number, body: unknown) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
  const error = (status: number, code: string) => json(status, { error: { code, message: code, location: null, retryable: status === 503, retry_after_ms: null }, request_id: "r" });
  return (async (input: string | URL, init?: RequestInit) => {
    const url = new URL(String(input));
    const body = init?.body === undefined ? undefined : (JSON.parse(String(init.body)) as Record<string, unknown>);
    calls.push({ method: init?.method ?? "GET", path: url.pathname, body });
    if (new Headers(init?.headers).get("authorization") !== `Bearer ${OPERATOR_TOKEN}`) return error(401, "unauthenticated");
    if (url.pathname === "/v1/operator/maintenance") {
      const current = state();
      const boundary = (body?.["boundary_id"] as string | undefined) ?? null;
      if (body?.["mode"] === "quiesced") {
        if (current.mode === "open") {
          db()
            .prepare("UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = ?, maintenance_since_ms = ?, maintenance_boundary_id = ? WHERE id = 1")
            .run(String(body["reason"] ?? "operator maintenance"), now(), boundary);
          audit("maintenance.quiesce", { boundary_id: boundary });
        } else if (current.boundary_id !== boundary) {
          return error(409, "identity_conflict");
        }
      } else {
        if (current.mode === "quiesced" && boundary !== null && current.boundary_id !== boundary) return error(409, "identity_conflict");
        if (current.mode === "quiesced") {
          audit("maintenance.resume", { boundary_id: current.boundary_id });
          db().exec("UPDATE service_state SET maintenance_mode = 'open', maintenance_reason = NULL, maintenance_since_ms = NULL, maintenance_boundary_id = NULL WHERE id = 1");
        }
      }
      return json(200, { maintenance: state(), changed: true, repositories: counters() });
    }
    if (url.pathname === "/v1/operator/audit/query") {
      const items = db()
        .prepare("SELECT seq, at_ms, action, actor, target_credential_id, request_id FROM audit_log WHERE action = ? AND at_ms >= ? ORDER BY seq")
        .all(String(body?.["action"]), Number(body?.["since_ms"] ?? 0)) as Record<string, unknown>[];
      return json(200, {
        items: items.map((row) => ({ sequence: row["seq"], at_ms: row["at_ms"], action: row["action"], actor: JSON.parse(String(row["actor"])), target_credential_id: row["target_credential_id"], request_id: row["request_id"] })),
        has_more: false,
        next_cursor: null,
      });
    }
    if (url.pathname === "/v1/operator/jobs/recover") return json(200, { steps: [], released_locks: 0, expired_uploads: 0 });
    return error(404, "not_found");
  }) as typeof fetch;
}

// ---- Seed: rows as the Worker writes them ----

export const REPO_A = "873451209";
export const REPO_B = "412000001";
const HEAD = "a".repeat(40);
const TREE = "b".repeat(40);

export function observationPayload(id: string, packageName = "biscuit-hash"): string {
  return `{"cell":{"gate":"L1","gate_input_identity":"0123abcd","outcome":"pass","package":"${packageName}"},"completion":"complete","observation_id":"${id}"}`;
}

export interface Seeded {
  readonly operatorId: string;
  readonly hostId: string;
  readonly revokedHostId: string;
  readonly observations: string[];
  /** Finalized, its object present in the bucket. */
  readonly presentUpload: { upload_id: string; key: string; bytes: Uint8Array; sha256: string; observation_id: string };
  /** Finalized, its object deliberately absent from the bucket. */
  readonly absentUpload: { upload_id: string; key: string; observation_id: string };
  /** Still open at the boundary. */
  readonly openUpload: string;
  readonly jobId: string;
}

/** Writes a small but complete store: two repositories, observations with events, uploads, credentials, a retention job. */
export async function seedSource(db: DatabaseSync, putObject: (key: string, bytes: Uint8Array) => void, extraObservations: readonly string[] = []): Promise<Seeded> {
  const at = 1_790_000_000_000;
  const operatorId = "0".repeat(31) + "1";
  const hostId = "c".repeat(32);
  const revokedHostId = "d".repeat(32);
  const insertRepository = db.prepare("INSERT INTO repositories(repository_id, owner_id, slug, generation, registered_at_ms) VALUES (?, '4417321', ?, ?, ?)");
  insertRepository.run(REPO_A, "owner/a", "1".repeat(32), at);
  insertRepository.run(REPO_B, "owner/b", "2".repeat(32), at);

  const ids = [...["e1", "e2", "e3"].map((tag) => sha256(tag).slice(0, 64)), ...extraObservations];
  const collectionId = sha256("collection");
  db.prepare(
    `INSERT INTO collections(repository_id, collection_id, environment, head, tree, observation_count, source_version, accepted_principal, accepted_request_id, received_at_ms)
     VALUES (?, ?, 'ubuntu-latest', ?, ?, ?, 3, ?, 'req-seed', ?)`,
  ).run(REPO_A, collectionId, HEAD, TREE, ids.length, JSON.stringify({ kind: "credential", credential_id: hostId }), at);
  ids.forEach((id, position) => {
    db.prepare(
      `INSERT INTO observations(repository_id, observation_id, environment, head, tree, payload_json, source_version, accepted_principal, accepted_request_id, accepted_collection_id, received_at_ms)
       VALUES (?, ?, 'ubuntu-latest', ?, ?, ?, 3, ?, 'req-seed', ?, ?)`,
    ).run(REPO_A, id, HEAD, TREE, observationPayload(id), JSON.stringify({ kind: "credential", credential_id: hostId }), collectionId, at);
    db.prepare("INSERT INTO collection_members(repository_id, collection_id, position, observation_id) VALUES (?, ?, ?, ?)").run(REPO_A, collectionId, position, id);
  });
  // A second repository with one observation, so sequences interleave across repositories.
  const other = sha256("other").slice(0, 64);
  db.prepare(
    `INSERT INTO observations(repository_id, observation_id, environment, head, tree, payload_json, source_version, accepted_principal, accepted_request_id, accepted_collection_id, received_at_ms)
     VALUES (?, ?, 'macos-latest', ?, ?, ?, 2, 'p', 'req-b', ?, ?)`,
  ).run(REPO_B, other, HEAD, TREE, observationPayload(other, "sniff"), sha256("collection-b"), at);

  const presentBytes = new TextEncoder().encode("<testsuite name='biscuit'>it's here; really</testsuite>\n");
  const absentBytes = new TextEncoder().encode("never copied");
  const upload = (uploadId: string, observationId: string, bytes: Uint8Array, state: "open" | "finalized") => {
    const digest = sha256(bytes);
    const key = `objects/${REPO_A}/${observationId}/report/${sha256(digest)}/${uploadId}`;
    db.prepare(
      `INSERT INTO uploads(upload_id, repository_id, observation_id, reference_kind, reference_digest, path, bytes, part_count, state, staging_key, sha256, created_by, created_at_ms, expires_at_ms, finalized_at_ms)
       VALUES (?, ?, ?, 'report', ?, 'target/nextest/junit.xml', ?, ?, ?, ?, ?, 'p', ?, ?, ?)`,
    ).run(uploadId, REPO_A, observationId, digest, bytes.byteLength, Math.ceil(bytes.byteLength / 1048576), state, key, state === "finalized" ? digest : null, at, at + 86_400_000, state === "finalized" ? at + 5 : null);
    if (state === "finalized") {
      const locator = JSON.stringify({ kind: "object", upload_id: uploadId });
      db.prepare(
        `INSERT INTO report_locations(repository_id, event_id, observation_id, reference_kind, reference_digest, report_path, location_key, location_kind, locator_json, state, verification_method, checked_at_ms, valid_until_ms, checked_by)
         VALUES (?, ?, ?, 'report', ?, 'target/nextest/junit.xml', ?, 'object', ?, 'verified', 'bytes', ?, ?, ?)`,
      ).run(REPO_A, randomUUID(), observationId, digest, sha256(locator), locator, at + 5, at + 5 + 86_400_000, JSON.stringify({ kind: "credential", credential_id: "f".repeat(32) }));
    }
    return { key, digest };
  };
  const presentId = "a1".repeat(16);
  const absentId = "a2".repeat(16);
  const openId = "a3".repeat(16);
  const present = upload(presentId, ids[0]!, presentBytes, "finalized");
  const absent = upload(absentId, ids[1]!, absentBytes, "finalized");
  upload(openId, ids[2]!, new Uint8Array(10), "open");
  putObject(present.key, presentBytes);

  const credential = db.prepare(
    `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms, repository_ids_json, roles_json)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  );
  credential.run(operatorId, "operator", "ops", await secretDigest("o".repeat(43)), at, at + 1, at + 2, "[]", "[]");
  credential.run(hostId, "host", "rig", await secretDigest("h".repeat(43)), at, at + 1, at + 2, JSON.stringify([REPO_A]), JSON.stringify(["producer", "reader"]));
  credential.run(revokedHostId, "host", "old-rig", await secretDigest("r".repeat(43)), at, at + 1, at + 2, JSON.stringify([REPO_A]), JSON.stringify(["reader"]));
  db.prepare("INSERT INTO credential_grants(credential_id, repository_id, role) VALUES (?, ?, 'producer'), (?, ?, 'reader'), (?, ?, 'reader')").run(hostId, REPO_A, hostId, REPO_A, revokedHostId, REPO_A);
  db.prepare("UPDATE credentials SET revoked_at_ms = ?, revocation_reason = 'retired' WHERE credential_id = ?").run(at + 3, revokedHostId);

  const certificateId = randomUUID();
  const jobId = "9".repeat(32);
  db.prepare(
    `INSERT INTO retention_certificates(repository_id, certificate_id, certificate_json, certificate_sha256, generation, retention_epoch, evidence_revision, verifier_policy_digest, submitted_by, submitted_request_id, submitted_at_ms)
     VALUES (?, ?, '{}', ?, ?, 0, 0, ?, 'retention-verifier:test', 'req', ?)`,
  ).run(REPO_A, certificateId, sha256("cert"), "1".repeat(32), sha256("policy"), at);
  db.prepare(
    `INSERT INTO retention_jobs(job_id, repository_id, certificate_id, state, generation, retention_epoch, evidence_revision, not_before_ms, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, 'scheduled', ?, 0, 0, ?, ?, ?)`,
  ).run(jobId, REPO_A, certificateId, "1".repeat(32), at, at, at);

  return {
    operatorId,
    hostId,
    revokedHostId,
    observations: ids,
    presentUpload: { upload_id: presentId, key: present.key, bytes: presentBytes, sha256: present.digest, observation_id: ids[0]! },
    absentUpload: { upload_id: absentId, key: absent.key, observation_id: ids[1]! },
    openUpload: openId,
    jobId,
  };
}
