// Budget admission (`src/capacity.ts`): an upload reserves its projected R2
// storage and write operations before it is admitted, a refusal past the 80%
// threshold writes nothing and never touches existing evidence or
// ingestion, and closed or pruned uploads release their storage. Its own
// file, because the external baseline is store-wide.
import { env } from "cloudflare:workers";
import { afterEach, describe, expect, it, vi } from "vitest";
import { capacityPeriod, capacityReport, refusalThreshold } from "../../src/capacity.ts";
import { serviceConfig } from "../../src/config.ts";
import { runDaily } from "../../src/daily.ts";
import { cleanupStaging } from "../../src/uploads.ts";
import {
  db,
  expectError,
  finalize,
  harness,
  ingestReport,
  newRepository,
  PART,
  payload,
  postJson,
  reportUpload,
  stage,
  type Harness,
  type IngestedFixture,
} from "./helpers.ts";

const quotas = serviceConfig(env).quotas.capacity;
const uploadsPath = (repo: string) => `/v1/repos/${repo}/uploads`;

async function setBaseline(resource: string, nowMs: number, used: number): Promise<void> {
  await db()
    .prepare(
      `INSERT INTO capacity_baseline(resource, period, external_used, measured_at_ms) VALUES (?1, ?2, ?3, ?4)
       ON CONFLICT(resource, period) DO UPDATE SET external_used = excluded.external_used`,
    )
    .bind(resource, capacityPeriod(nowMs), used, nowMs)
    .run();
}

async function reservations(repo: string): Promise<{ resource: string; amount: number; state: string }[]> {
  const { results } = await db()
    .prepare("SELECT resource, amount, state FROM quota_reservations WHERE repository_id = ?1 ORDER BY resource, state")
    .bind(repo)
    .all<{ resource: string; amount: number; state: string }>();
  return results;
}

async function setup(content: Uint8Array): Promise<{ repo: string; request: Harness; fixture: IngestedFixture }> {
  const repo = await newRepository();
  const request = harness([repo]);
  return { repo, request, fixture: await ingestReport(request, repo, content) };
}

/** Live bytes already reserved or committed across the store. */
async function liveBytes(): Promise<number> {
  return (await db()
    .prepare("SELECT COALESCE(SUM(amount), 0) AS n FROM quota_reservations WHERE resource = 'r2_bytes' AND state IN ('reserved', 'committed')")
    .first<{ n: number }>())!.n;
}

afterEach(async () => {
  await db().prepare("DELETE FROM capacity_baseline").run();
});

describe("budget admission", () => {
  it("reserves storage and class A operations when an upload opens and commits them on finalize", async () => {
    const content = payload(PART + 10, 3);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    expect(await reservations(repo)).toEqual([
      { resource: "r2_bytes", amount: PART + 10, state: "reserved" },
      { resource: "r2_class_a", amount: 3, state: "reserved" },
    ]);
    // A repeated request reuses the upload and reserves nothing more.
    expect((await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer")).status).toBe(200);
    expect((await finalize(request, repo, uploadId)).status).toBe(200);
    expect(await reservations(repo)).toEqual([
      { resource: "r2_bytes", amount: PART + 10, state: "committed" },
      { resource: "r2_class_a", amount: 3, state: "committed" },
    ]);
  });

  it("refuses past the storage threshold with 429, writes nothing, and leaves evidence and ingestion alone", async () => {
    const content = payload(500, 4);
    const { repo, request, fixture } = await setup(content);
    const threshold = refusalThreshold(quotas, "r2_bytes");
    await setBaseline("r2_bytes", request.clock, threshold - (await liveBytes()) - 499);

    const refused = await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer"), 429, "quota_exhausted", null);
    expect({ retryable: refused.error.retryable, retry_after_ms: refused.error.retry_after_ms }).toEqual({ retryable: true, retry_after_ms: 3_600_000 });
    expect(await reservations(repo)).toEqual([]);
    expect((await db().prepare("SELECT COUNT(*) AS n FROM uploads WHERE repository_id = ?1").bind(repo).first<{ n: number }>())!.n).toBe(0);
    // The observation stays readable, and ingestion (including of failures) is never admission-controlled.
    expect((await request(`/v1/repos/${repo}/observations/${fixture.observationId}`)).status).toBe(200);
    const other = await ingestReport(request, repo, payload(600, 5));
    expect(other.observationId).not.toBe(fixture.observationId);

    // Exactly at the threshold is admitted.
    await setBaseline("r2_bytes", request.clock, threshold - (await liveBytes()) - 500);
    expect((await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer")).status).toBe(201);
  });

  it("refuses past the monthly class A threshold and counts only the current month", async () => {
    const content = payload(PART * 2, 6);
    const { repo, request, fixture } = await setup(content);
    const used = (await db()
      .prepare("SELECT COALESCE(SUM(amount), 0) AS n FROM quota_reservations WHERE resource = 'r2_class_a' AND period = ?1 AND state <> 'released'")
      .bind(capacityPeriod(request.clock))
      .first<{ n: number }>())!.n;
    await setBaseline("r2_class_a", request.clock, refusalThreshold(quotas, "r2_class_a") - used - 2);
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer"), 429, "quota_exhausted");
    // Next month starts a new window.
    request.clock += 32 * 86_400_000;
    expect((await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer")).status).toBe(201);
  });

  it("releases the storage of rejected and expired uploads, which admits waiting work again", async () => {
    const content = payload(400, 7);
    const { repo, request, fixture } = await setup(content);
    const threshold = refusalThreshold(quotas, "r2_bytes");
    // Room for exactly one 400-byte upload.
    await setBaseline("r2_bytes", request.clock, threshold - (await liveBytes()) - 400);
    const rejected = await stage(request, repo, reportUpload(fixture), payload(400, 8));
    const second = await ingestReport(request, repo, payload(400, 9));
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(second), "producer"), 429, "quota_exhausted");

    await expectError(await finalize(request, repo, rejected), 400, "invalid_input");
    expect((await reservations(repo)).find((row) => row.resource === "r2_bytes")?.state).toBe("released");
    const opened = await postJson(request, uploadsPath(repo), reportUpload(second), "producer");
    expect(opened.status).toBe(201);

    // The open upload expires; the daily cleanup releases its storage.
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer"), 429, "quota_exhausted");
    await cleanupStaging(env, request.clock + 25 * 3_600_000);
    expect((await reservations(repo)).filter((row) => row.resource === "r2_bytes").map((row) => row.state)).toEqual(["released", "released"]);
    expect((await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer")).status).toBe(201);
  });

  it("reports usage against each allowance and logs only numbers at the alert ratio", async () => {
    const nowMs = 1_800_000_000_000;
    await setBaseline("r2_class_b", nowMs, Math.ceil(quotas.r2_class_b_per_month * 0.75));
    const report = await capacityReport(db(), quotas, nowMs);
    const classB = report.resources.find((usage) => usage.resource === "r2_class_b")!;
    expect({ alert: classB.alert, refusing: classB.refusing, allowance: classB.allowance }).toEqual({
      alert: true,
      refusing: false,
      allowance: quotas.r2_class_b_per_month,
    });

    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    try {
      const daily = await runDaily(env, nowMs);
      expect(daily.capacity?.resources.find((usage) => usage.resource === "r2_class_b")?.alert).toBe(true);
      expect(warn).toHaveBeenCalledTimes(1);
      const logged = JSON.parse(warn.mock.calls[0]![0] as string) as Record<string, unknown>;
      expect(Object.keys(logged).sort()).toEqual(["allowance", "event", "period", "ratio", "refusing", "resource", "used"]);
      expect(logged).toMatchObject({ event: "evidence_store.capacity_alert", resource: "r2_class_b", allowance: quotas.r2_class_b_per_month });
    } finally {
      warn.mockRestore();
    }
  });
});
