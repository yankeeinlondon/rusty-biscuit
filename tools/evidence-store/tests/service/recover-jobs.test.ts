// `POST /v1/operator/jobs/recover` through the Worker's public `fetch`:
// unfinished retention jobs advance, locks of finished jobs are released,
// stale open uploads expire, and the run is audited.
import { afterEach, describe, expect, it } from "vitest";
import { db, expectError, ingestReport, newRepository, observation, payload, reportUpload, seed } from "./helpers.ts";
import { currentRetentionJob, forceOpen, forceQuiesce, post, recoveryHarness } from "./recovery-helpers.ts";

const HOUR_MS = 3_600_000;

afterEach(forceOpen);

interface RecoverReport {
  steps: { job_id: string; repository_id: string; from_state: string; to_state: string }[];
  released_locks: number;
  expired_uploads: number;
}

async function recover(request: ReturnType<typeof recoveryHarness>): Promise<RecoverReport> {
  const response = await post(request, "/v1/operator/jobs/recover", {}, "operator");
  expect(response.status, await response.clone().text()).toBe(200);
  return (await response.json()) as RecoverReport;
}

async function jobState(jobId: string): Promise<string> {
  return (await db().prepare("SELECT state FROM retention_jobs WHERE job_id = ?1").bind(jobId).first<{ state: string }>())!.state;
}

describe("POST /v1/operator/jobs/recover", () => {
  it("advances a due scheduled job whose revision is unchanged, and audits the run", async () => {
    const repo = await newRepository();
    const [member] = await seed(repo, [observation()]);
    const jobId = await currentRetentionJob(repo, "scheduled", [member!]);
    const request = recoveryHarness([repo]);

    const report = await recover(request);
    expect(report.steps.find((step) => step.job_id === jobId)).toMatchObject({ repository_id: repo, from_state: "scheduled" });
    expect(await jobState(jobId)).not.toBe("scheduled");
    const audit = await db().prepare("SELECT detail_json FROM audit_log WHERE action = 'retention.recover' ORDER BY seq DESC LIMIT 1").first<{ detail_json: string }>();
    expect(JSON.parse(audit!.detail_json)).toMatchObject({ steps: report.steps.length, released_locks: report.released_locks, expired_uploads: report.expired_uploads });
  });

  it("releases a cell lock still held by a completed job", async () => {
    const repo = await newRepository();
    const jobId = await currentRetentionJob(repo, "completed", []);
    await db()
      .prepare("INSERT INTO cell_locks(repository_id, package, environment, gate, job_id, acquired_at_ms) VALUES (?1, 'biscuit-hash', 'ubuntu-latest', 'L1', ?2, 1)")
      .bind(repo, jobId)
      .run();
    const report = await recover(recoveryHarness([repo]));
    expect(report.released_locks).toBeGreaterThanOrEqual(1);
    expect(await db().prepare("SELECT COUNT(*) AS n FROM cell_locks WHERE job_id = ?1").bind(jobId).first<{ n: number }>()).toEqual({ n: 0 });
    expect((await recover(recoveryHarness([repo]))).steps.filter((step) => step.job_id === jobId)).toEqual([]);
  });

  it("expires open uploads past their expiry", async () => {
    const repo = await newRepository();
    const request = recoveryHarness([repo]);
    const fixture = await ingestReport(request, repo, payload(20, 4));
    const created = (await (await post(request, `/v1/repos/${repo}/uploads`, reportUpload(fixture), "producer")).json()) as { upload_id: string };
    request.clock += 25 * HOUR_MS;
    const report = await recover(request);
    expect(report.expired_uploads).toBeGreaterThanOrEqual(1);
    expect(await db().prepare("SELECT state FROM uploads WHERE upload_id = ?1").bind(created.upload_id).first()).toEqual({ state: "expired" });
  });

  it("is refused while quiesced and for a non-operator", async () => {
    const request = recoveryHarness([]);
    await expectError(await post(request, "/v1/operator/jobs/recover", {}, "reader"), 403, "denied");
    await expectError(await post(request, "/v1/operator/jobs/recover", { job_id: "x" }, "operator"), 400, "invalid_input", "/job_id");
    await forceQuiesce();
    await expectError(await post(request, "/v1/operator/jobs/recover", {}, "operator"), 503, "unavailable");
  });
});
