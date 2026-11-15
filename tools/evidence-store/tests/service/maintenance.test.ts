// Persisted maintenance mode through the Worker's public `fetch`, against
// local D1 and R2 built from `migrations/`: the backup boundary, the 503 gate
// on every mutation route, the database trigger behind it, and what still
// works while quiesced.
import { env } from "cloudflare:workers";
import { afterEach, describe, expect, it } from "vitest";
import { ingestCollectionBatch } from "../../src/db/ingest.ts";
import type { Env } from "../../src/worker.ts";
import {
  collection,
  db,
  expectError,
  hex,
  ingestReport,
  newRepository,
  observation,
  payload,
  putPart,
  rejection,
  reportEvent,
  reportUpload,
  stage,
} from "./helpers.ts";
import { forceOpen, forceQuiesce, OPERATOR_ID, post, recoveryHarness, storeContents } from "./recovery-helpers.ts";

afterEach(forceOpen);

async function auditActions(sinceSeq: number): Promise<string[]> {
  const { results } = await db().prepare("SELECT action FROM audit_log WHERE seq > ?1 ORDER BY seq").bind(sinceSeq).all<{ action: string }>();
  return results.map((row) => row.action);
}

async function lastAuditSeq(): Promise<number> {
  return (await db().prepare("SELECT COALESCE(MAX(seq), 0) AS seq FROM audit_log").first<{ seq: number }>())!.seq;
}

async function counters(repositoryId: string) {
  return db()
    .prepare(
      `SELECT repository_id, generation, retention_epoch, evidence_revision, observation_event_watermark, location_event_watermark
       FROM repositories WHERE repository_id = ?1`,
    )
    .bind(repositoryId)
    .first();
}

describe("POST /v1/operator/maintenance", () => {
  it("quiesces once per boundary, answers the boundary read after the quiesce, and reopens", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const before = await lastAuditSeq();
    const boundary = hex(32);

    const quiesced = await post(request, "/v1/operator/maintenance", { mode: "quiesced", reason: "nightly backup", boundary_id: boundary }, "operator");
    expect(quiesced.status).toBe(200);
    const record = (await quiesced.json()) as { maintenance: Record<string, unknown>; changed: boolean; repositories: Record<string, unknown>[] };
    expect(record.maintenance).toEqual({ mode: "quiesced", reason: "nightly backup", since_ms: request.clock, boundary_id: boundary });
    expect(record.changed).toBe(true);
    expect(record.repositories).toContainEqual(await counters(repo));

    request.clock += 1_000;
    const again = await post(request, "/v1/operator/maintenance", { mode: "quiesced", reason: "retry", boundary_id: boundary }, "operator");
    expect(again.status).toBe(200);
    expect(((await again.json()) as typeof record).maintenance).toEqual(record.maintenance);

    const other = await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: hex(32) }, "operator");
    await expectError(other, 409, "identity_conflict", "/boundary_id");
    const wrongOpen = await post(request, "/v1/operator/maintenance", { mode: "open", boundary_id: hex(32) }, "operator");
    await expectError(wrongOpen, 409, "identity_conflict", "/boundary_id");

    const read = await request("/v1/operator/maintenance", { as: "operator" });
    expect(read.status).toBe(200);
    expect(((await read.json()) as typeof record).maintenance.mode).toBe("quiesced");

    const opened = await post(request, "/v1/operator/maintenance", { mode: "open", boundary_id: boundary }, "operator");
    expect(opened.status).toBe(200);
    expect(((await opened.json()) as typeof record).maintenance).toEqual({ mode: "open", reason: null, since_ms: null, boundary_id: null });
    const reopened = await post(request, "/v1/operator/maintenance", { mode: "open" }, "operator");
    expect(((await reopened.json()) as typeof record).changed).toBe(false);

    expect(await auditActions(before)).toEqual(["maintenance.quiesce", "maintenance.resume"]);
  });

  it("applies the default reason, refuses a non-operator, and validates the request strictly", async () => {
    const request = recoveryHarness([]);
    const quiesced = await post(request, "/v1/operator/maintenance", { mode: "quiesced" }, "operator");
    expect(((await quiesced.json()) as { maintenance: { reason: string; boundary_id: null } }).maintenance).toMatchObject({ reason: "operator maintenance", boundary_id: null });
    // Both without a boundary: the same quiesce.
    expect((await post(request, "/v1/operator/maintenance", { mode: "quiesced" }, "operator")).status).toBe(200);
    await forceOpen();

    await expectError(await post(request, "/v1/operator/maintenance", { mode: "quiesced" }, "producer"), 403, "denied");
    await expectError(await post(request, "/v1/operator/maintenance", { mode: "paused" }, "operator"), 400, "invalid_input", "/mode");
    await expectError(await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: null }, "operator"), 400, "invalid_input", "/boundary_id");
    await expectError(await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: "XYZ" }, "operator"), 400, "invalid_input", "/boundary_id");
    await expectError(await post(request, "/v1/operator/maintenance", { mode: "quiesced", extra: 1 }, "operator"), 400, "invalid_input", "/extra");
    await expectError(await post(request, "/v1/operator/maintenance", '{"mode":"open","mode":"open"}', "operator"), 400, "invalid_input");
    expect((await readMode()).maintenance_mode).toBe("open");
  });

  it("puts every write acknowledged before the quiesce inside the boundary", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const acknowledged: { ids: string[]; revision: number }[] = [];
    for (let index = 0; index < 3; index += 1) {
      const fixture = await ingestReport(request, repo, payload(100 + index, index));
      const revision = (await counters(repo)) as { evidence_revision: number };
      acknowledged.push({ ids: [fixture.observationId], revision: revision.evidence_revision });
    }
    const response = await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: hex(32) }, "operator");
    const { repositories } = (await response.json()) as { repositories: { repository_id: string; evidence_revision: number; observation_event_watermark: number }[] };
    const boundary = repositories.find((entry) => entry.repository_id === repo)!;
    for (const ack of acknowledged) expect(ack.revision).toBeLessThanOrEqual(boundary.evidence_revision);
    const stored = await db().prepare("SELECT observation_id FROM observations WHERE repository_id = ?1").bind(repo).all<{ observation_id: string }>();
    expect(stored.results.map((row) => row.observation_id).sort()).toEqual(acknowledged.flatMap((ack) => ack.ids).sort());
    const lastEvent = await db().prepare("SELECT MAX(seq) AS seq FROM observation_events WHERE repository_id = ?1").bind(repo).first<{ seq: number }>();
    expect(boundary.observation_event_watermark).toBe(lastEvent!.seq);
  });
});

async function readMode() {
  return (await db().prepare("SELECT maintenance_mode FROM service_state WHERE id = 1").first<{ maintenance_mode: string }>())!;
}

describe("while quiesced", () => {
  it("answers 503 unavailable to every mutation route and writes nothing to D1 or R2", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const content = payload(2048, 7);
    const fixture = await ingestReport(request, repo, content);
    const stagedId = await stage(request, repo, reportUpload(fixture), content);
    const openUpload = (await (await post(request, `/v1/repos/${repo}/uploads`, reportUpload(await ingestReport(request, repo, payload(10, 3))), "producer")).json()) as { upload_id: string };

    await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: hex(32) }, "operator");
    const before = await storeContents();
    const auditBefore = await lastAuditSeq();

    const refused: [string, Response][] = [
      ["ingestCollection", await post(request, `/v1/repos/${repo}/collections`, { source_version: 3, collection: {} }, "producer")],
      ["appendLocation", await post(request, `/v1/repos/${repo}/locations`, reportEvent(fixture), "verifier")],
      ["createUpload", await post(request, `/v1/repos/${repo}/uploads`, reportUpload(fixture), "producer")],
      ["putUploadPart", await putPart(request, repo, openUpload.upload_id, 0, payload(10, 3))],
      ["finalizeUpload", await request(`/v1/repos/${repo}/uploads/${stagedId}/finalize`, { method: "POST", as: "verifier" })],
      ["registerRepository", await post(request, "/v1/operator/repositories", { repository_id: "77", owner_id: "1", slug: "o/r" }, "operator")],
      ["issueCredential", await post(request, "/v1/operator/credentials", { label: "x", repository_ids: [repo], roles: ["reader"] }, "operator")],
      ["recoverJobs", await post(request, "/v1/operator/jobs/recover", {}, "operator")],
    ];
    for (const [operation, response] of refused) {
      expect(response.status, operation).toBe(503);
      expect(response.headers.get("retry-after"), operation).toBe("60");
      const body = (await response.json()) as { error: { code: string; retryable: boolean; retry_after_ms: number } };
      expect(body.error, operation).toMatchObject({ code: "unavailable", retryable: true, retry_after_ms: 60_000 });
    }
    expect(await storeContents()).toEqual(before);
    expect(await auditActions(auditBefore)).toEqual([]);
  });

  it("still serves reads and the audit trail, and still audits a denial", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const fixture = await ingestReport(request, repo, payload(32, 1));
    await forceQuiesce();
    const before = await lastAuditSeq();

    expect((await request(`/v1/repos/${repo}/snapshot`)).status).toBe(200);
    expect((await request(`/v1/repos/${repo}/observations/${fixture.observationId}`)).status).toBe(200);
    expect((await request(`/v1/repos/${repo}/collections/${fixture.collectionId}`)).status).toBe(200);
    expect((await post(request, `/v1/repos/${repo}/changes/query`, { stream: "observations" }, "reader")).status).toBe(200);
    expect((await post(request, "/v1/operator/audit/query", {}, "operator")).status).toBe(200);

    // The reader may not ingest: refused for its role, and the refusal is audited despite the quiesce.
    await expectError(await post(request, `/v1/repos/${repo}/collections`, { source_version: 3, collection: {} }, "reader"), 403, "denied");
    expect(await auditActions(before)).toEqual(["credential.denied"]);
  });

  it("revokes a credential (revocation outranks the boundary)", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const issued = (await (await post(request, "/v1/operator/credentials", { label: "leaky", repository_ids: [repo], roles: ["reader"] }, "operator")).json()) as {
      credential: { credential_id: string };
    };
    await forceQuiesce();
    const revoked = await post(request, `/v1/operator/credentials/${issued.credential.credential_id}/revoke`, { reason: "leaked" }, "operator");
    expect(revoked.status).toBe(200);
    expect(((await revoked.json()) as { revocation_reason: string }).revocation_reason).toBe("leaked");
  });

  it("refuses a write that races the quiesce in the database itself, as 503, leaving nothing", async () => {
    const repo = await newRepository();
    const real = env as unknown as Env;
    // The route check sees `open`; the quiesce lands before the batch commits.
    const racing: D1Database = new Proxy(real.EVIDENCE_DB, {
      get(target, property) {
        if (property === "batch") {
          return async (statements: D1PreparedStatement[]) => {
            await forceQuiesce();
            return target.batch(statements);
          };
        }
        const value = Reflect.get(target, property) as unknown;
        return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
      },
    });
    const request = recoveryHarness([repo], () => ({ ...real, EVIDENCE_DB: racing }));
    const withoutState = async () => {
      const { rows, keys } = await storeContents();
      delete rows["service_state"];
      return { rows, keys };
    };
    const before = await withoutState();
    const response = await ingestReport(request, repo, payload(48, 5)).catch((error: Error) => error);
    expect(String(response)).toContain("answered 503");
    expect(String(response)).toContain("the evidence store is in maintenance");
    expect(await withoutState()).toEqual(before);

    // And directly at the database layer, with no route check at all.
    const message = await rejection(ingestCollectionBatch(db(), collection(repo, [observation()])));
    expect(message).toContain("maintenance_mode");
    const stored = await db().prepare("SELECT COUNT(*) AS n FROM observations WHERE repository_id = ?1").bind(repo).first<{ n: number }>();
    expect(stored!.n).toBe(0);
  });

  it("restores writes when reopened", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const boundary = hex(32);
    await post(request, "/v1/operator/maintenance", { mode: "quiesced", boundary_id: boundary }, "operator");
    expect((await post(request, `/v1/repos/${repo}/uploads`, {}, "producer")).status).toBe(503);
    await post(request, "/v1/operator/maintenance", { mode: "open", boundary_id: boundary }, "operator");
    const fixture = await ingestReport(request, repo, payload(16, 2));
    expect(fixture.observationId).toMatch(/^[0-9a-f]+$/);
    const audit = await db().prepare("SELECT actor_credential_id FROM audit_log WHERE action = 'maintenance.resume' ORDER BY seq DESC LIMIT 1").first<{ actor_credential_id: string }>();
    expect(audit!.actor_credential_id).toBe(OPERATOR_ID);
  });
});
