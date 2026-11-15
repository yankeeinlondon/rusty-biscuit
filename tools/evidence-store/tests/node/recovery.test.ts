// The recovery commands in process, against a fake Wrangler over real local
// state (node:sqlite databases built from `migrations/`, a directory per R2
// bucket) and a fake of the operator API's maintenance and audit routes over
// the same source database. Replay goes to the ingestion fake service.
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { DatabaseSync } from "node:sqlite";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ingest } from "../../cli/ingest.ts";
import { run } from "../../cli/operator.ts";
import { classifyStatement, planExport, splitSql, type RecoveryIo, type RecoveryReport } from "../../cli/recovery.ts";
import { parseBackupManifest, type BackupManifest } from "../../src/backup-manifest.ts";
import { createNodeReportReader } from "../../src/contracts/node-report-reader.ts";
import { ApiError } from "../../src/http.ts";
import { secretDigest } from "../../src/tokens.ts";
import { FakeService, PRODUCER_TOKEN, rigWorkspace } from "../fixtures/ingest/fake-service.ts";
import {
  exportSql,
  fail,
  FakeWrangler,
  fakeOperatorApi,
  migratedDatabase,
  OPERATOR_TOKEN,
  REPO_A,
  seedSource,
  sha256,
  type ApiCall,
  type Seeded,
} from "../fixtures/recovery/fake-wrangler.ts";

const ENVIRONMENTS = {
  development: { database: "evidence-store-development", databaseId: "dev-database-id", bucket: "evidence-store-development" },
  "restore-test": { database: "evidence-store-restore-test", databaseId: "restore-database-id", bucket: "evidence-store-restore-test" },
};
const SOURCE_BUCKET = ENVIRONMENTS.development.bucket;
const DEST_BUCKET = ENVIRONMENTS["restore-test"].bucket;
const ENDPOINT = "https://evidence.test";
const RESTORED = "https://restored.test";

let dir: string;
let wrangler: FakeWrangler;
let source: DatabaseSync;
let seeded: Seeded;
let apiCalls: ApiCall[];
let restoredService: FakeService;
let clock: number;
let out: string[];
let err: string[];

const path = (...parts: string[]) => join(dir, ...parts);
const config = () => path("wrangler.jsonc");
const backups = () => path("backups");
const dest = () => wrangler.database("restore-test");

async function setup(extraObservations: string[] = []): Promise<void> {
  wrangler = new FakeWrangler(path("cloud"), ENVIRONMENTS);
  source = migratedDatabase();
  wrangler.databases.set(ENVIRONMENTS.development.database, source);
  seeded = await seedSource(source, (key, bytes) => wrangler.putObject(SOURCE_BUCKET, key, bytes), extraObservations);
  writeFileSync(config(), wrangler.config());
}

beforeEach(async () => {
  dir = mkdtempSync(join(tmpdir(), "evidence-recovery-"));
  mkdirSync(backups());
  writeFileSync(path("operator.token"), `${OPERATOR_TOKEN}\n`);
  writeFileSync(path("producer.token"), `${PRODUCER_TOKEN}\n`);
  apiCalls = [];
  restoredService = new FakeService();
  clock = 1_800_000_000_000;
  out = [];
  err = [];
  await setup();
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

function io(fetchOverride?: typeof fetch): RecoveryIo {
  const operatorApi = fakeOperatorApi(() => source, () => clock, apiCalls, seeded.operatorId);
  return {
    env: { CLOUDFLARE_API_TOKEN: "scoped", CLOUDFLARE_ACCOUNT_ID: "account", USERNAME: "tester" },
    fetch:
      fetchOverride ??
      ((async (input: string | URL, init?: RequestInit) =>
        String(input).startsWith(RESTORED) ? restoredService.fetch(input, init) : operatorApi(input, init)) as typeof fetch),
    spawn: (command, args, options) => (command === "icacls" ? { status: 0, stdout: "", stderr: "" } : wrangler.spawn(command, args, options)),
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => (clock += 1_000),
    sleep: async () => undefined,
    random: () => 0.5,
  };
}

const backupArgs = () => ["backup", "--endpoint", ENDPOINT, "--token-file", path("operator.token"), "--env", "development", "--destination", backups(), "--config", config()];

async function takeBackup(): Promise<string> {
  const before = new Set(readdirSync(backups()));
  expect(await run(backupArgs(), io()), err.join("")).toBe(0);
  const created = readdirSync(backups()).filter((name) => !before.has(name));
  expect(created).toHaveLength(1);
  out.length = 0;
  return path("backups", created[0]!);
}

const manifestOf = (backup: string): BackupManifest => parseBackupManifest(readFileSync(join(backup, "manifest.json")));

function restoreArgs(backup: string, extra: string[] = []): string[] {
  return ["restore", "--backup", backup, "--source-env", "development", "--dest-env", "restore-test", "--dest-token-file", path("restored.token"), "--config", config(), ...extra];
}

const rows = (db: DatabaseSync, sql: string) => db.prepare(sql).all() as Record<string, unknown>[];
const destCalls = () => wrangler.calls.filter((call) => call.args.includes("restore-test"));

describe("backup", () => {
  it("quiesces, exports, always reopens, copies referenced objects, and writes a strict manifest last", async () => {
    const backup = await takeBackup();
    const manifest = manifestOf(backup);

    expect(apiCalls.map((call) => `${call.path} ${(call.body as { mode?: string } | undefined)?.mode ?? ""}`.trim())).toEqual([
      "/v1/operator/maintenance quiesced",
      "/v1/operator/audit/query",
      "/v1/operator/maintenance open",
    ]);
    expect(apiCalls[0]!.body).toMatchObject({ boundary_id: manifest.backup_id });
    expect(rows(source, "SELECT maintenance_mode FROM service_state")).toEqual([{ maintenance_mode: "open" }]);

    expect(manifest.boundary.boundary_id).toBe(manifest.backup_id);
    const migrations = readdirSync(new URL("../../migrations/", import.meta.url)).filter((name) => name.endsWith(".sql"));
    expect(manifest.migration).toBe(Math.max(...migrations.map((name) => Number(name.slice(0, 4)))));
    expect(manifest.source).toEqual({ environment: "development", d1_database_name: ENVIRONMENTS.development.database, d1_database_id: "dev-database-id", r2_bucket: SOURCE_BUCKET });
    expect(manifest.repositories).toEqual(
      rows(source, "SELECT repository_id, generation, retention_epoch, evidence_revision, observation_event_watermark, location_event_watermark FROM repositories ORDER BY repository_id").map((row) => ({ ...row })),
    );
    // Objects are found through the exported uploads, not by listing the bucket.
    expect(manifest.objects).toEqual([
      { upload_id: seeded.presentUpload.upload_id, repository_id: REPO_A, key: seeded.presentUpload.key, file: `objects/${seeded.presentUpload.upload_id}`, sha256: seeded.presentUpload.sha256, bytes: seeded.presentUpload.bytes.byteLength },
    ]);
    expect(readFileSync(join(backup, "objects", seeded.presentUpload.upload_id))).toEqual(Buffer.from(seeded.presentUpload.bytes));
    expect(manifest.missing_objects).toMatchObject([{ upload_id: seeded.absentUpload.upload_id, key: seeded.absentUpload.key, reason: "absent" }]);
    expect(manifest.pending_uploads.map((upload) => upload.upload_id)).toEqual([seeded.openUpload]);
    expect(manifest.retention_jobs).toMatchObject([{ job_id: seeded.jobId, state: "scheduled" }]);
    expect(manifest.credentials.map((credential) => [credential.credential_id, credential.kind, credential.revoked_at_ms !== undefined])).toEqual([
      [seeded.operatorId, "operator", false],
      [seeded.hostId, "host", false],
      [seeded.revokedHostId, "host", true],
    ]);
    expect(manifest.credentials[0]!.secret_sha256).toBe(await secretDigest("o".repeat(43)));
    expect(manifest.grants).toHaveLength(3);
    // The database export is the one taken while quiesced.
    expect(readFileSync(join(backup, "database.sql"), "utf8")).toContain("'quiesced'");
    const backupText = readdirSync(backup, { recursive: true })
      .map(String)
      .filter((name) => statSync(join(backup, name)).isFile())
      .map((name) => readFileSync(join(backup, name), "utf8"))
      .join("");
    expect(backupText).not.toContain(OPERATOR_TOKEN);
    expect(wrangler.calls.every((call) => !call.args.join(" ").includes(OPERATOR_TOKEN))).toBe(true);

    expect(await run(["verify-backup", "--backup", backup], io())).toBe(0);
  });

  it("records revocations made inside the quiesced window", async () => {
    const quiesceThenRevoke: typeof fetch = async (input, init) => {
      const answer = await fakeOperatorApi(() => source, () => clock, apiCalls, seeded.operatorId)(input, init);
      if (String(input).endsWith("/maintenance") && String(init?.body).includes("quiesced")) {
        source.prepare("UPDATE credentials SET revoked_at_ms = ?, revocation_reason = 'leak' WHERE credential_id = ?").run(clock + 1, seeded.hostId);
        source
          .prepare("INSERT INTO audit_log(at_ms, action, actor, actor_credential_id, target_credential_id, request_id) VALUES (?, 'credential.revoke', ?, ?, ?, 'r')")
          .run(clock + 1, JSON.stringify({ kind: "credential", credential_id: seeded.operatorId }), seeded.operatorId, seeded.hostId);
      }
      return answer;
    };
    expect(await run(backupArgs(), io(quiesceThenRevoke)), err.join("")).toBe(0);
    const manifest = manifestOf(path("backups", readdirSync(backups())[0]!));
    expect(manifest.window_revocations).toEqual([{ credential_id: seeded.hostId, at_ms: expect.any(Number) }]);
  });

  it("reopens the store even when the export fails, and leaves no partial backup", async () => {
    wrangler.faults.push((call) => (call.args[1] === "export" ? fail("D1 export failed: internal error") : undefined));
    expect(await run(backupArgs(), io())).toBe(1);
    expect(err.join("")).toContain("d1 export");
    expect(apiCalls.map((call) => (call.body as { mode?: string } | undefined)?.mode)).toEqual(["quiesced", "open"]);
    expect(rows(source, "SELECT maintenance_mode FROM service_state")).toEqual([{ maintenance_mode: "open" }]);
    expect(readdirSync(backups())).toEqual([]);
  });

  it("names the reopen command when the store cannot be reopened", async () => {
    const operatorApi = fakeOperatorApi(() => source, () => clock, apiCalls, seeded.operatorId);
    const refuseOpen: typeof fetch = async (input, init) =>
      String(init?.body).includes('"open"') ? new Response("{}", { status: 503 }) : operatorApi(input, init);
    expect(await run(backupArgs(), io(refuseOpen))).toBe(1);
    expect(err.join("")).toMatch(/still quiesced for boundary [0-9a-f]{32}[\s\S]*just operator maintenance .* --mode open --boundary-id [0-9a-f]{32}/);
    expect(readdirSync(backups())).toEqual([]);
  });

  it("does not reopen a quiesce it does not own", async () => {
    source.exec("UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = 'other', maintenance_since_ms = 1, maintenance_boundary_id = 'ffffffffffffffffffffffffffffffff' WHERE id = 1");
    expect(await run(backupArgs(), io())).toBe(1);
    // It reopens only by its own boundary, which the other quiesce refuses.
    expect(apiCalls.map((call) => (call.body as { mode?: string } | undefined)?.mode)).toEqual(["quiesced", "open"]);
    expect(rows(source, "SELECT maintenance_boundary_id FROM service_state")).toEqual([{ maintenance_boundary_id: "ffffffffffffffffffffffffffffffff" }]);
  });

  it("reopens by its own boundary when the quiesce answer is lost", async () => {
    const operatorApi = fakeOperatorApi(() => source, () => clock, apiCalls, seeded.operatorId);
    const loseQuiesceAnswer: typeof fetch = async (input, init) => {
      const answer = await operatorApi(input, init);
      if (String(init?.body).includes('"quiesced"')) throw new TypeError("fetch failed");
      return answer;
    };
    expect(await run(backupArgs(), io(loseQuiesceAnswer))).toBe(1);
    expect(apiCalls.map((call) => (call.body as { mode?: string } | undefined)?.mode)).toEqual(["quiesced", "open"]);
    expect(rows(source, "SELECT maintenance_mode FROM service_state")).toEqual([{ maintenance_mode: "open" }]);
  });

  it("keeps the seven newest complete backups and never touches an incomplete or invalid one", async () => {
    mkdirSync(path("backups", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"));
    mkdirSync(path("backups", "dddddddddddddddddddddddddddddddd"));
    writeFileSync(path("backups", "dddddddddddddddddddddddddddddddd", "manifest.json"), "{}");
    const made: string[] = [];
    for (let index = 0; index < 9; index += 1) made.push(await takeBackup().then((backup) => backup.slice(-32)));
    const remaining = readdirSync(backups()).sort();
    expect(remaining).toEqual([...made.slice(2), "dddddddddddddddddddddddddddddddd", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"].sort());
  });

  it("checks its inputs before contacting anything", async () => {
    const noCloudflare = { ...io(), env: {} };
    expect(await run(backupArgs(), noCloudflare)).toBe(2);
    expect(await run([...backupArgs().slice(0, -4), "--destination", path("absent"), "--config", config()], io())).toBe(2);
    expect(await run(backupArgs().map((arg) => (arg === "development" ? "production" : arg)), io())).toBe(2);
    expect(await run(backupArgs().map((arg) => (arg === ENDPOINT ? "http://evidence.test" : arg)), io())).toBe(2);
    expect(apiCalls).toEqual([]);
    expect(wrangler.calls).toEqual([]);
  });
});

describe("restore", () => {
  it("round-trips a backup into isolated resources: identical evidence, rotated generations and keys, repaired gaps", async () => {
    const backup = await takeBackup();
    const manifest = manifestOf(backup);
    expect(await run(restoreArgs(backup, ["--report", path("report.json")]), io()), err.join("")).toBe(0);
    const report = JSON.parse(readFileSync(path("report.json"), "utf8")) as RecoveryReport;
    const target = dest();

    // Evidence is identical, with no event duplicated by a restored row.
    for (const table of ["observations", "collections", "collection_members", "observation_events", "uploads", "credential_grants", "retention_jobs", "retention_certificates", "d1_migrations"]) {
      const order = table === "collections" || table === "collection_members" || table === "credential_grants" ? "rowid" : "1";
      const expected = rows(source, `SELECT * FROM ${table} ORDER BY ${order}`).map((row) => (table === "uploads" && row["state"] === "open" ? { ...row, state: "expired" } : row));
      expect(rows(target, `SELECT * FROM ${table} ORDER BY ${order}`), table).toEqual(expected);
    }
    // Locations: the source's rows, then one `unavailable` event for the object the backup lacks.
    const sourceLocations = rows(source, "SELECT * FROM report_locations ORDER BY seq");
    const restoredLocations = rows(target, "SELECT * FROM report_locations ORDER BY seq");
    expect(restoredLocations.slice(0, sourceLocations.length)).toEqual(sourceLocations);
    expect(restoredLocations.slice(sourceLocations.length)).toMatchObject([
      { state: "unavailable", observation_id: seeded.absentUpload.observation_id, locator_json: JSON.stringify({ kind: "object", upload_id: seeded.absentUpload.upload_id }) },
    ]);
    const sourceEvents = rows(source, "SELECT * FROM location_events ORDER BY seq");
    const restoredEvents = rows(target, "SELECT * FROM location_events ORDER BY seq");
    expect(restoredEvents.slice(0, sourceEvents.length)).toEqual(sourceEvents);
    expect(restoredEvents.slice(sourceEvents.length)).toMatchObject([{ kind: "unavailable" }]);

    // Generations rotate; counters move only by the appended event.
    for (const repository of manifest.repositories) {
      const restored = rows(target, `SELECT * FROM repositories WHERE repository_id = '${repository.repository_id}'`)[0]!;
      const rotated = report.generations.find((entry) => entry.repository_id === repository.repository_id)!;
      expect(rotated).toMatchObject({ from: repository.generation, to: restored["generation"] });
      expect(restored["generation"]).not.toBe(repository.generation);
      const appended = repository.repository_id === REPO_A ? 1 : 0;
      expect(restored["evidence_revision"]).toBe(repository.evidence_revision + appended);
      expect(restored["observation_event_watermark"]).toBe(repository.observation_event_watermark);
    }

    // The restored operator credential is revoked; a new one exists whose token went only to its file.
    const token = readFileSync(path("restored.token"), "utf8").trim();
    if (process.platform !== "win32") expect(statSync(path("restored.token")).mode & 0o777).toBe(0o600);
    const credentials = rows(target, "SELECT credential_id, kind, secret_sha256, revoked_at_ms FROM credentials ORDER BY issued_at_ms, credential_id");
    expect(credentials.find((row) => row["credential_id"] === seeded.operatorId)!["revoked_at_ms"]).not.toBeNull();
    expect(credentials.find((row) => row["credential_id"] === seeded.hostId)!["revoked_at_ms"]).toBeNull();
    const fresh = credentials.find((row) => row["credential_id"] === report.operator.credential_id)!;
    expect(fresh).toMatchObject({ kind: "operator", revoked_at_ms: null, secret_sha256: await secretDigest(token.split(".")[1]!) });
    expect(report.operator.revoked).toEqual([seeded.operatorId]);
    const actions = rows(target, "SELECT action FROM audit_log ORDER BY seq").map((row) => row["action"]);
    expect(actions.slice(-5)).toEqual(["operator.bootstrap", "credential.revoke", "restore.generation", "restore.generation", "maintenance.resume"]);

    // The cursor key rotated through stdin only; neither secret reached argv.
    const key = wrangler.secrets.get("restore-test:CURSOR_HMAC_KEY");
    expect(key).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(wrangler.calls.find((call) => call.args[0] === "secret")).toEqual({ args: ["secret", "put", "CURSOR_HMAC_KEY", "--env", "restore-test", "--config", config()], input: key });
    expect(wrangler.calls.every((call) => !call.args.join(" ").includes(token) && !call.args.join(" ").includes(key!))).toBe(true);
    expect(out.join("") + err.join("")).not.toContain(token);

    // Objects are back and intact; the open upload is expired; the store is open and guarded again.
    expect(wrangler.objectKeys(DEST_BUCKET)).toEqual([seeded.presentUpload.key]);
    expect(sha256(wrangler.getObject(DEST_BUCKET, seeded.presentUpload.key)!)).toBe(seeded.presentUpload.sha256);
    expect(rows(target, "SELECT maintenance_mode FROM service_state")).toEqual([{ maintenance_mode: "open" }]);
    expect(rows(target, "SELECT COUNT(*) AS n FROM sqlite_master WHERE type IN ('trigger', 'index')")).toEqual(rows(source, "SELECT COUNT(*) AS n FROM sqlite_master WHERE type IN ('trigger', 'index')"));
    target.exec("UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = 't', maintenance_since_ms = 1 WHERE id = 1");
    expect(() => target.exec("UPDATE uploads SET state = 'expired' WHERE 0")).not.toThrow();
    expect(() => target.prepare("INSERT INTO repositories(repository_id, slug, generation, registered_at_ms) VALUES ('5', 's/t', 'g', 1)").run()).toThrow(/maintenance_mode/);

    expect(report).toMatchObject({
      backup_id: manifest.backup_id,
      boundary: { boundary_id: manifest.backup_id, repositories: manifest.repositories },
      objects_restored: 1,
      missing_objects: [{ upload_id: seeded.absentUpload.upload_id }],
      pending_uploads_expired: [seeded.openUpload],
      cursor_key_rotated: true,
      acknowledged: { recovered: [], replayed: [], unrecoverable: [] },
    });
    expect(report.post_boundary_loss).toMatch(/^unknown/);
  });

  it("reorders the export: run in its own order it would fire triggers on restored rows", async () => {
    const naive = (await import("node:sqlite")).DatabaseSync;
    const db = new naive(":memory:");
    expect(() => db.exec(exportSql(source))).toThrow(/UNIQUE constraint failed|maintenance_mode/);
    const plan = planExport(exportSql(source));
    expect(plan.triggers.length + plan.maintenanceTriggers.length).toBe(rows(source, "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'trigger'")[0]!["n"]);
    expect(plan.maintenanceTriggers.every((trigger) => trigger.includes("service_state"))).toBe(true);
  });

  it("re-applies revocations recorded in the backup window", async () => {
    const backup = await takeBackup();
    const manifest = JSON.parse(readFileSync(join(backup, "manifest.json"), "utf8")) as Record<string, unknown>;
    manifest["window_revocations"] = [{ credential_id: seeded.hostId, at_ms: 5 }];
    writeFileSync(join(backup, "manifest.json"), JSON.stringify(manifest));
    expect(await run(restoreArgs(backup), io()), err.join("")).toBe(0);
    expect(rows(dest(), `SELECT revocation_reason FROM credentials WHERE credential_id = '${seeded.hostId}'`)[0]!["revocation_reason"]).toMatch(/backup window/);
  });

  it("classifies acknowledged IDs as recovered, replayed, or unrecoverable, and replays with the original IDs", async () => {
    // B was acknowledged before the boundary; A and C after it.
    const workspace = (name: string, bytes: number) => {
      mkdirSync(path(name));
      return rigWorkspace(path(name), bytes);
    };
    const a = workspace("a", 300);
    const b = workspace("b", 400);
    const c = workspace("c", 500);
    rmSync(path("cloud"), { recursive: true, force: true });
    await setup([b.observationId]);
    const backup = await takeBackup();

    const live = new FakeService();
    const ingestIo = {
      fetch: live.fetch,
      sleep: async () => undefined,
      random: () => 0.5,
      now: () => clock,
      stdout: () => undefined,
      stderr: () => undefined,
      readReport: createNodeReportReader({ home: () => dir }),
    };
    const submit = (w: typeof a, name: string, proofs: boolean) =>
      ingest({ input: w.input, endpoint: ENDPOINT, repo: REPO_A, tokenFile: path("producer.token"), state: path(name, "state.json"), ...(proofs ? { proofDir: w.proofDir } : {}) }, ingestIo);
    await submit(a, "a", true);
    await submit(b, "b", false);
    await submit(c, "c", false);
    rmSync(path("c", "state.json.collection.json"));

    const args = restoreArgs(backup, ["--dest-endpoint", RESTORED, "--replay", path("a", "state.json"), "--replay", path("b", "state.json"), "--replay", path("c", "state.json")]);
    expect(await run(args, io()), err.join("")).toBe(0);
    const report = JSON.parse(out.join("")) as RecoveryReport;
    const ids = (list: readonly { observation_id: string }[]) => list.map((entry) => entry.observation_id);
    expect(ids(report.acknowledged.recovered)).toEqual([b.observationId]);
    expect(ids(report.acknowledged.replayed)).toEqual([a.observationId]);
    expect(ids(report.acknowledged.unrecoverable)).toEqual([c.observationId]);
    expect(report.acknowledged.unrecoverable[0]!.reason).toMatch(/no intact retained collection/);
    expect(report.post_boundary_loss).toMatch(/^known only for the 3 replay input/);

    // The replay resubmitted the exact retained collection, with its original IDs, to the restored endpoint.
    expect([...restoredService.observations]).toEqual([a.observationId]);
    const submitted = restoredService.requests.find((request) => request.path.endsWith("/collections"))!;
    expect(new TextDecoder().decode(submitted.body)).toContain(a.canonical);
    const replay = report.replays.find((entry) => entry.state === path("a", "state.json"))!;
    expect(replay.replay_state).toMatch(/state\.json\.restore-[0-9a-f]{32}\.json$/);
    expect(JSON.parse(readFileSync(replay.replay_state!, "utf8"))).toMatchObject({ endpoint: RESTORED, ingest: { state: "acknowledged", accepted: [a.observationId] } });
    expect(replay.finalize_command).toContain("just evidence-finalize");
    // The original state is untouched.
    expect(JSON.parse(readFileSync(path("a", "state.json"), "utf8"))).toMatchObject({ endpoint: ENDPOINT });
  });
});

describe("restore refuses before any mutation", () => {
  it("refuses the live source as destination, by environment or by any shared resource", async () => {
    const backup = await takeBackup();
    wrangler.calls.length = 0;
    expect(await run(restoreArgs(backup).map((arg) => (arg === "restore-test" ? "development" : arg)), io())).toBe(2);
    const shared = JSON.parse(readFileSync(config(), "utf8").replace(/^\/\/.*\n/, "")) as { env: Record<string, { r2_buckets: { bucket_name: string }[] }> };
    shared.env["restore-test"]!.r2_buckets[0]!.bucket_name = SOURCE_BUCKET;
    writeFileSync(path("shared.jsonc"), JSON.stringify(shared));
    expect(await run(restoreArgs(backup).map((arg) => (arg === config() ? path("shared.jsonc") : arg)), io())).toBe(2);
    expect(err.join("")).toContain("r2_bucket");
    const sharedDatabase = JSON.parse(readFileSync(config(), "utf8").replace(/^\/\/.*\n/, "")) as { env: Record<string, { d1_databases: { database_id: string }[] }> };
    sharedDatabase.env["restore-test"]!.d1_databases[0]!.database_id = sharedDatabase.env["development"]!.d1_databases[0]!.database_id;
    writeFileSync(path("shared-database.jsonc"), JSON.stringify(sharedDatabase));
    expect(await run(restoreArgs(backup).map((arg) => (arg === config() ? path("shared-database.jsonc") : arg)), io())).toBe(2);
    expect(err.join("")).toContain("d1_database_id");
    expect(wrangler.calls).toEqual([]);
    expect(existsSync(path("restored.token"))).toBe(false);
  });

  it("refuses a tampered backup: verify-backup fails and restore executes nothing", async () => {
    const backup = await takeBackup();
    const object = join(backup, "objects", seeded.presentUpload.upload_id);
    const bytes = readFileSync(object);
    bytes[0] = bytes[0]! ^ 1;
    writeFileSync(object, bytes);
    wrangler.calls.length = 0;

    expect(await run(["verify-backup", "--backup", backup], io())).toBe(1);
    expect(JSON.parse(out.join(""))).toMatchObject({ ok: false, problems: [{ file: `objects/${seeded.presentUpload.upload_id}`, problem: "digest" }] });
    expect(await run(restoreArgs(backup), io())).toBe(1);
    expect(err.join("")).toContain("failed verification, so nothing was restored");
    expect(wrangler.calls).toEqual([]);
    expect(existsSync(path("restored.token"))).toBe(false);

    rmSync(join(backup, "database.sql"));
    expect(await run(["verify-backup", "--backup", backup], io())).toBe(1);
  });

  it("refuses a non-empty destination database", async () => {
    const backup = await takeBackup();
    dest().exec("CREATE TABLE leftovers(x)");
    wrangler.calls.length = 0;
    expect(await run(restoreArgs(backup), io())).toBe(1);
    expect(err.join("")).toContain("is not empty");
    expect(destCalls().map((call) => call.args.slice(0, 2).join(" "))).toEqual(["d1 execute"]);
    expect(destCalls()[0]!.args).toContain("--json");
    expect(wrangler.objectKeys(DEST_BUCKET)).toEqual([]);
    expect(existsSync(path("restored.token"))).toBe(false);
  });

  it("refuses an existing token file, replay without an endpoint, and an invalid replay state", async () => {
    const backup = await takeBackup();
    wrangler.calls.length = 0;
    writeFileSync(path("restored.token"), "keep\n");
    expect(await run(restoreArgs(backup), io())).toBe(2);
    expect(readFileSync(path("restored.token"), "utf8")).toBe("keep\n");
    rmSync(path("restored.token"));
    expect(await run(restoreArgs(backup, ["--replay", path("missing.json")]), io())).toBe(2);
    writeFileSync(path("bad-state.json"), '{"format_version":1,"format_version":1}');
    expect(await run(restoreArgs(backup, ["--dest-endpoint", RESTORED, "--replay", path("bad-state.json")]), io())).toBe(2);
    expect(err.join("")).toContain("duplicate");
    expect(wrangler.calls).toEqual([]);
  });

  it("removes the new token file when loading the database fails", async () => {
    const backup = await takeBackup();
    wrangler.faults.push((call) => (call.args[1] === "execute" && call.args.includes("--file") ? fail("D1_ERROR: too large") : undefined));
    expect(await run(restoreArgs(backup), io())).toBe(1);
    expect(existsSync(path("restored.token"))).toBe(false);
    expect(err.join("")).toContain("recreate the destination database");
  });
});

describe("recover-jobs and maintenance", () => {
  it("call the operator routes and print their answers", async () => {
    expect(await run(["recover-jobs", "--endpoint", ENDPOINT, "--token-file", path("operator.token")], io())).toBe(0);
    expect(JSON.parse(out.join(""))).toEqual({ steps: [], released_locks: 0, expired_uploads: 0 });
    out.length = 0;
    expect(await run(["maintenance", "--endpoint", ENDPOINT, "--token-file", path("operator.token"), "--mode", "quiesced", "--reason", "drill"], io())).toBe(0);
    expect(JSON.parse(out.join(""))).toMatchObject({ maintenance: { mode: "quiesced", reason: "drill" } });
    expect(await run(["maintenance", "--endpoint", ENDPOINT, "--token-file", path("operator.token"), "--mode", "paused"], io())).toBe(2);
    expect(await run(["maintenance", "--endpoint", ENDPOINT, "--token-file", path("operator.token"), "--mode", "open", "--boundary-id", "nope"], io())).toBe(2);
    expect(apiCalls.map((call) => call.path)).toEqual(["/v1/operator/jobs/recover", "/v1/operator/maintenance"]);
  });
});

describe("SQL export handling", () => {
  it("splits on statement ends only, keeping quoted semicolons and trigger bodies whole", () => {
    const sql = `PRAGMA defer_foreign_keys=TRUE;
CREATE TABLE "t" (a TEXT, "b;c" INTEGER);
CREATE TRIGGER g AFTER INSERT ON t BEGIN UPDATE t SET a = CASE WHEN a = 'x;' THEN 'y' END; INSERT INTO t VALUES('it''s; fine', 1); END;
-- a comment; with a semicolon
INSERT INTO "t" ("a","b;c") VALUES('line one;
line two',2);
DELETE FROM sqlite_sequence;
CREATE UNIQUE INDEX i ON t(a);`;
    const statements = splitSql(sql);
    expect(statements.map(classifyStatement)).toEqual(["pragma", "table", "trigger", "data", "data", "index"]);
    expect(statements[2]).toMatch(/END$/);
    expect(statements[3]).toContain("line one;\nline two");
  });

  it("refuses a statement a D1 export does not produce", () => {
    expect(() => planExport("CREATE TABLE t(a);\nDROP TABLE t;")).toThrow(/does not load: DROP TABLE/);
    expect(() => planExport("BEGIN TRANSACTION;")).toThrow(/does not load/);
  });
});

// ---- Manifest robustness: each shared validator's shapes once, plus the loader's wiring ----

const FIXTURE = readFileSync(new URL("../fixtures/recovery/backup/0123456789abcdef0123456789abcdef/manifest.json", import.meta.url), "utf8");

function mutated(edit: (manifest: Record<string, any>) => void): Uint8Array {
  const manifest = JSON.parse(FIXTURE) as Record<string, any>;
  edit(manifest);
  return new TextEncoder().encode(JSON.stringify(manifest));
}

function locationOf(bytes: Uint8Array): string | null {
  try {
    parseBackupManifest(bytes);
  } catch (error) {
    if (error instanceof ApiError) return error.location;
    throw error;
  }
  return null;
}

describe("backup manifest", () => {
  it("accepts the frozen fixture, with empty lists valid", () => {
    expect(parseBackupManifest(new TextEncoder().encode(FIXTURE)).backup_id).toBe("0123456789abcdef0123456789abcdef");
    expect(locationOf(mutated((m) => Object.assign(m, { objects: [], missing_objects: [], pending_uploads: [], retention_jobs: [], credentials: [], grants: [], window_revocations: [], repositories: [] })))).toBeNull();
  });

  it.each([
    ["string absent", (m: Record<string, any>) => delete m["backup_id"], "/backup_id"],
    ["string null", (m: Record<string, any>) => (m["backup_id"] = null), "/backup_id"],
    ["string wrong type", (m: Record<string, any>) => (m["backup_id"] = 7), "/backup_id"],
    ["string empty", (m: Record<string, any>) => (m["backup_id"] = ""), "/backup_id"],
    ["string bad format", (m: Record<string, any>) => (m["backup_id"] = "XYZ"), "/backup_id"],
    ["enum outside", (m: Record<string, any>) => (m.source.environment = "production"), "/source/environment"],
    ["integer absent", (m: Record<string, any>) => delete m["created_at_ms"], "/created_at_ms"],
    ["integer null", (m: Record<string, any>) => (m["created_at_ms"] = null), "/created_at_ms"],
    ["integer string", (m: Record<string, any>) => (m["created_at_ms"] = "1"), "/created_at_ms"],
    ["integer negative", (m: Record<string, any>) => (m["created_at_ms"] = -1), "/created_at_ms"],
    ["integer fractional", (m: Record<string, any>) => (m["created_at_ms"] = 1.5), "/created_at_ms"],
    ["object absent", (m: Record<string, any>) => delete m["boundary"], "/boundary"],
    ["object null", (m: Record<string, any>) => (m["boundary"] = null), "/boundary"],
    ["object array", (m: Record<string, any>) => (m["boundary"] = []), "/boundary"],
    ["object empty", (m: Record<string, any>) => (m["boundary"] = {}), "/boundary/boundary_id"],
    ["object unknown key", (m: Record<string, any>) => (m.boundary.extra = 1), "/boundary/extra"],
    ["list absent", (m: Record<string, any>) => delete m["objects"], "/objects"],
    ["list null", (m: Record<string, any>) => (m["objects"] = null), "/objects"],
    ["list scalar", (m: Record<string, any>) => (m["objects"] = "objects/a"), "/objects"],
    ["list wrong element", (m: Record<string, any>) => (m["objects"] = ["objects/a"]), "/objects/0"],
    ["list element invalid", (m: Record<string, any>) => (m.objects[0].sha256 = "x"), "/objects/0/sha256"],
    ["list element unknown key", (m: Record<string, any>) => (m.grants[0].scope = "all"), "/grants/0/scope"],
    ["optional null", (m: Record<string, any>) => (m.credentials[1].revoked_at_ms = null), "/credentials/1/revoked_at_ms"],
    ["unknown root key", (m: Record<string, any>) => (m["secrets"] = []), "/secrets"],
    ["version", (m: Record<string, any>) => (m["format_version"] = 2), "/format_version"],
    ["version float", (m: Record<string, any>) => (m["format_version"] = 1.0001), "/format_version"],
    ["file escapes the backup", (m: Record<string, any>) => (m.database.file = "../database.sql"), "/database/file"],
    ["file absolute", (m: Record<string, any>) => (m.objects[0].file = "/etc/passwd"), "/objects/0/file"],
    ["boundary of another backup", (m: Record<string, any>) => (m.boundary.boundary_id = "f".repeat(32)), "/boundary/boundary_id"],
  ])("refuses %s at its location", (_name, edit, location) => {
    expect(locationOf(mutated(edit as (m: Record<string, any>) => void))).toBe(location);
  });

  it("goes through the shared strict loader (duplicate key, trailing content)", async () => {
    const backup = path("backups", "0123456789abcdef0123456789abcdef");
    mkdirSync(backup);
    writeFileSync(join(backup, "manifest.json"), FIXTURE.replace('"format_version": 1,', '"format_version": 1, "format_version": 1,'));
    expect(await run(["verify-backup", "--backup", backup], io())).toBe(2);
    expect(err.join("")).toMatch(/invalid at "\/format_version": duplicate-key/);
    writeFileSync(join(backup, "manifest.json"), `${FIXTURE}{}`);
    expect(await run(["verify-backup", "--backup", backup], io())).toBe(2);
    expect(err.join("")).toContain("trailing-content");
  });
});
