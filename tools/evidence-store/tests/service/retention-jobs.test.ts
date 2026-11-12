// The retention job runner (`advanceRetentionJobs`) and the daily run that
// schedules it: grace, invalidation, withdraw → delete objects → delete rows,
// cell locks, the retention epoch, tombstone expiry, and a crash injected at
// every D1 and R2 boundary of a prune.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { advanceRetentionJobs } from "../../src/retention.ts";
import type { Env } from "../../src/worker.ts";
import { db, expectError, harness, newRepository, payload, postJson, repositoryState } from "./helpers.ts";
import {
  ancestry,
  certificate,
  certifiedCell,
  currentSnapshot,
  DAY,
  expectNoDanglingAvailability,
  GRACE,
  headOf,
  jobPath,
  jobState,
  objectPath,
  pruneScenario,
  RIG_CELL,
  rigObservation,
  rigUpload,
  seedAt,
  submit,
  synthetic,
  type RetentionJobBody,
} from "./retention-fixtures.ts";

type RunEnv = Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET">;

async function counts(repo: string, observationId: string): Promise<Record<string, number>> {
  return (await db()
    .prepare(
      `SELECT
         (SELECT COUNT(*) FROM observations WHERE repository_id = ?1 AND observation_id = ?2) AS observations,
         (SELECT COUNT(*) FROM report_locations WHERE repository_id = ?1 AND observation_id = ?2) AS locations,
         (SELECT COUNT(*) FROM uploads WHERE repository_id = ?1 AND observation_id = ?2) AS uploads,
         (SELECT COUNT(*) FROM observation_events WHERE repository_id = ?1 AND observation_id = ?2 AND kind = 'pruned') AS observation_tombstones,
         (SELECT COUNT(*) FROM location_events WHERE repository_id = ?1 AND observation_id = ?2 AND kind = 'pruned') AS location_tombstones,
         (SELECT COUNT(*) FROM cell_locks WHERE repository_id = ?1) AS locks,
         (SELECT retention_epoch FROM repositories WHERE repository_id = ?1) AS epoch`,
    )
    .bind(repo, observationId)
    .first())!;
}

/** The R2 keys an upload owns: its final object and its staged parts. */
async function uploadKeys(uploadId: string): Promise<string[]> {
  const row = await db().prepare("SELECT staging_key FROM uploads WHERE upload_id = ?1").bind(uploadId).first<{ staging_key: string }>();
  const parts = (await env.EVIDENCE_BUCKET.list({ prefix: `staging/${uploadId}/` })).objects.map((object) => object.key);
  const final = row === null ? [] : (await env.EVIDENCE_BUCKET.head(row.staging_key)) === null ? [] : [row.staging_key];
  return [...final, ...parts];
}

describe("advanceRetentionJobs", () => {
  it("withdraws, deletes objects, then deletes rows, keeping a retained observation's identical bytes", async () => {
    const s = await pruneScenario();
    const locationRows = (await counts(s.repo, s.older.id))["locations"]!;
    expect(locationRows).toBe(2);
    expect(await uploadKeys(s.olderUpload)).toHaveLength(2);

    // Not before the grace ends.
    const early = await advanceRetentionJobs(env, s.runAt - 1);
    expect(early.steps).toEqual([]);
    expect(await jobState(s.jobId)).toBe("scheduled");

    const run = await advanceRetentionJobs(env, s.runAt);
    expect(run.steps.filter((step) => step.job_id === s.jobId).map((step) => step.to_state)).toEqual([
      "withdrawing",
      "deleting-objects",
      "deleting-rows",
      "completed",
    ]);
    expect(run.released_locks).toBe(1);
    // Both locations were withdrawn first; every row then left one tombstone.
    expect(await counts(s.repo, s.older.id)).toEqual({
      observations: 0,
      locations: 0,
      uploads: 0,
      observation_tombstones: 1,
      location_tombstones: locationRows + 2,
      locks: 0,
      epoch: s.snapshot.retention_epoch + 1,
    });
    expect(await uploadKeys(s.olderUpload)).toEqual([]);

    const job = (await (await s.request(jobPath(s.repo, s.jobId), { as: "retention-verifier" })).json()) as RetentionJobBody;
    expect({ state: job.state, progress: job.progress }).toEqual({
      state: "completed",
      progress: { locations_withdrawn: 2, objects_deleted: 1, observations_deleted: 1 },
    });

    // The retained observation's object holds the same bytes under its own key and stays fetchable.
    const kept = await s.request(objectPath(s.repo, s.newer));
    expect(kept.status).toBe(200);
    expect(new Uint8Array(await kept.arrayBuffer())).toEqual(payload(700, 41));
    await expectError(await s.request(objectPath(s.repo, s.older)), 404, "not_found");
    await expectError(await s.request(`/v1/repos/${s.repo}/observations/${s.older.id}`), 404, "not_found");
    expect((await s.request(`/v1/repos/${s.repo}/observations/${s.newer.id}`)).status).toBe(200);

    // The pruned upload's live storage is released; the retained one stays committed.
    const reservations = await db()
      .prepare(
        `SELECT q.state, u.observation_id FROM quota_reservations q LEFT JOIN uploads u ON u.upload_id = q.upload_id
         WHERE q.repository_id = ?1 AND q.resource = 'r2_bytes' ORDER BY q.state`,
      )
      .bind(s.repo)
      .all<{ state: string; observation_id: string | null }>();
    expect(reservations.results).toEqual([
      { state: "committed", observation_id: s.newer.id },
      { state: "released", observation_id: null },
    ]);
    const audit = await db()
      .prepare("SELECT detail_json FROM audit_log WHERE repository_id = ?1 AND action = 'retention.job'")
      .bind(s.repo)
      .all<{ detail_json: string }>();
    expect(audit.results.map((row) => JSON.parse(row.detail_json))).toEqual([{ job_id: s.jobId, certificate_id: job.certificate_id, state: "completed" }]);

    // A rerun is a no-op.
    expect((await advanceRetentionJobs(env, s.runAt + DAY)).steps).toEqual([]);
  });

  it("answers snapshot_pruned to a snapshot or cursor captured before the epoch advanced", async () => {
    const s = await pruneScenario();
    const history = { package: RIG_CELL.package, gate: RIG_CELL.gate, reachable_heads: [headOf("1"), headOf("2"), headOf("3")], page_size: 1 };
    const first = await postJson(s.request, `/v1/repos/${s.repo}/history/query`, history);
    const { next_cursor: cursor } = (await first.json()) as { next_cursor: string };
    expect(cursor).not.toBeNull();
    await advanceRetentionJobs(env, s.runAt);
    await expectError(
      await postJson(s.request, `/v1/repos/${s.repo}/history/query`, { ...history, snapshot: s.snapshot }),
      409,
      "snapshot_pruned",
      "/snapshot/retention_epoch",
    );
    await expectError(await postJson(s.request, `/v1/repos/${s.repo}/history/query`, { ...history, cursor }), 409, "snapshot_pruned", "/cursor");
  });

  it("measures the grace from the first certified supersession; a later certificate does not restart it", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const older = await seedAt(repo, headOf("1"));
    const newer = await seedAt(repo, headOf("2"));
    const start = request.clock;
    const cell = () => certifiedCell([older, newer], [newer], [ancestry(older, newer)]);
    const first = (await (await submit(request, repo, certificate(await currentSnapshot(request, repo), [cell()]))).json()) as RetentionJobBody;
    expect(first.not_before_ms).toBe(start + GRACE);

    // A write invalidates the first certificate; the verifier certifies again ten days later.
    await seedAt(repo, headOf("5"), synthetic({ package: "playa" }));
    request.clock = start + 10 * DAY;
    const second = (await (await submit(request, repo, certificate(await currentSnapshot(request, repo), [cell()]))).json()) as RetentionJobBody;
    expect(second.not_before_ms).toBe(start + GRACE);
    const member = await db().prepare("SELECT superseded_at_ms FROM retention_job_members WHERE job_id = ?1").bind(second.job_id).first();
    expect(member).toEqual({ superseded_at_ms: start });

    await advanceRetentionJobs(env, start + GRACE - 1);
    expect([await jobState(first.job_id), await jobState(second.job_id)]).toEqual(["scheduled", "scheduled"]);
    const run = await advanceRetentionJobs(env, start + GRACE);
    expect(run.steps[0]).toEqual({ job_id: first.job_id, repository_id: repo, from_state: "scheduled", to_state: "invalidated" });
    expect([await jobState(first.job_id), await jobState(second.job_id)]).toEqual(["invalidated", "completed"]);
    expect((await repositoryState(repo)).observations).not.toContain(older.id);
  });

  it("starts a job at submission when its grace has already ended, so a later write cannot invalidate it", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const older = await seedAt(repo, headOf("1"));
    const newer = await seedAt(repo, headOf("2"));
    const start = request.clock;
    const cell = () => certifiedCell([older, newer], [newer], [ancestry(older, newer)]);
    const first = (await (await submit(request, repo, certificate(await currentSnapshot(request, repo), [cell()]))).json()) as RetentionJobBody;
    expect(first.state).toBe("scheduled");

    // Fourteen days later, after unrelated writes, the verifier certifies again.
    await seedAt(repo, headOf("5"), synthetic({ package: "playa" }));
    request.clock = start + GRACE;
    const response = await submit(request, repo, certificate(await currentSnapshot(request, repo), [cell()]));
    expect(response.status).toBe(202);
    const second = (await response.json()) as RetentionJobBody;
    expect({ state: second.state, not_before_ms: second.not_before_ms }).toEqual({ state: "withdrawing", not_before_ms: start + GRACE });

    // Another write lands before the daily run; the started job still completes.
    await seedAt(repo, headOf("6"), synthetic({ package: "playa" }));
    await advanceRetentionJobs(env, start + GRACE + 1);
    expect([await jobState(first.job_id), await jobState(second.job_id)]).toEqual(["invalidated", "completed"]);
    const state = await repositoryState(repo);
    expect(state.observations).not.toContain(older.id);
    expect(state.observations).toContain(newer.id);
  });

  it("invalidates a scheduled job when the revision changed and deletes nothing", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const older = await seedAt(repo, headOf("1"));
    const newer = await seedAt(repo, headOf("2"));
    const job = (await (
      await submit(request, repo, certificate(await currentSnapshot(request, repo), [certifiedCell([older, newer], [newer], [ancestry(older, newer)])]))
    ).json()) as RetentionJobBody;
    await seedAt(repo, headOf("6"), synthetic({ package: "playa" }));
    const before = await repositoryState(repo);
    const run = await advanceRetentionJobs(env, request.clock + GRACE);
    expect(run.steps).toEqual([{ job_id: job.job_id, repository_id: repo, from_state: "scheduled", to_state: "invalidated" }]);
    expect(await repositoryState(repo)).toEqual(before);
    const audit = await db().prepare("SELECT detail_json FROM audit_log WHERE repository_id = ?1 AND action = 'retention.job'").bind(repo).first<{ detail_json: string }>();
    expect(JSON.parse(audit!.detail_json)).toMatchObject({ job_id: job.job_id, state: "invalidated" });
  });

  it("locks the cell during deletion: concurrent writes answer 503 and write nothing, and the lock is released on completion", async () => {
    const s = await pruneScenario();
    // scheduled → withdrawing, one withdrawal batch, then lock and → deleting-objects.
    await advanceRetentionJobs(env, s.runAt, { jobId: s.jobId, maxBatches: 3 });
    expect(await jobState(s.jobId)).toBe("deleting-objects");
    expect((await counts(s.repo, s.older.id))["locks"]).toBe(1);

    const late = await rigObservation(headOf("4"), payload(9, 43), 4);
    const before = await repositoryState(s.repo);
    const ingest = await postJson(s.request, `/v1/repos/${s.repo}/collections`, late.body, "producer");
    const refusal = await expectError(ingest, 503, "unavailable");
    expect({ retryable: refusal.error.retryable, retry_after_ms: refusal.error.retry_after_ms }).toEqual({ retryable: true, retry_after_ms: 60_000 });
    await expectError(await postJson(s.request, `/v1/repos/${s.repo}/uploads`, rigUpload(s.extra), "producer"), 503, "unavailable");
    const verify = {
      event_id: crypto.randomUUID(),
      observation_id: s.extra.id,
      reference_kind: "report",
      reference_digest: s.extra.report.sha256,
      path: s.extra.report.path,
      locator: { kind: "host-dir", host: "rig-mac", directory: "/r" },
      kind: "verified",
      verification_method: "bytes",
    };
    await expectError(await postJson(s.request, `/v1/repos/${s.repo}/locations`, verify, "verifier"), 503, "unavailable");
    expect(await repositoryState(s.repo)).toEqual(before);
    expect((await db().prepare("SELECT COUNT(*) AS n FROM uploads WHERE observation_id = ?1").bind(s.extra.id).first<{ n: number }>())!.n).toBe(0);
    // A withdrawal never makes bytes available, so it is not locked out.
    expect((await postJson(s.request, `/v1/repos/${s.repo}/locations`, { ...verify, event_id: crypto.randomUUID(), kind: "withdrawn", verification_method: undefined }, "verifier")).status).toBe(201);
    // Another cell is unaffected.
    await seedAt(s.repo, headOf("7"), synthetic({ package: "playa" }));

    const run = await advanceRetentionJobs(env, s.runAt);
    expect(run.released_locks).toBe(1);
    expect(await jobState(s.jobId)).toBe("completed");
    expect((await postJson(s.request, `/v1/repos/${s.repo}/collections`, late.body, "producer")).status).toBe(200);
    expect((await postJson(s.request, `/v1/repos/${s.repo}/uploads`, rigUpload(s.extra), "producer")).status).toBe(201);
  });

  it("does nothing while the store is quiesced", async () => {
    const s = await pruneScenario();
    await db()
      .prepare("UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = 'backup', maintenance_since_ms = 1 WHERE id = 1")
      .run();
    try {
      const run = await advanceRetentionJobs(env, s.runAt);
      expect(run).toMatchObject({ skipped_for_maintenance: true, steps: [] });
      expect(await jobState(s.jobId)).toBe("scheduled");
    } finally {
      await db().prepare("UPDATE service_state SET maintenance_mode = 'open', maintenance_reason = NULL, maintenance_since_ms = NULL WHERE id = 1").run();
    }
    await advanceRetentionJobs(env, s.runAt);
    expect(await jobState(s.jobId)).toBe("completed");
  });

  describe("crash at every boundary", () => {
    type Mode = "before" | "after";

    /**
     * A test-only environment whose D1 statements and batches and R2
     * deletions count as operations; operation `failAt` throws, either
     * before it runs or after it has taken effect (a lost response).
     */
    function faulty(failAt: number, mode: Mode): { env: RunEnv; operations: () => number } {
      let count = 0;
      const tick = async <T>(operation: () => Promise<T>): Promise<T> => {
        count += 1;
        if (count !== failAt) return operation();
        if (mode === "after") await operation();
        throw new Error(`injected failure at operation ${failAt} (${mode})`);
      };
      const unwrap = new WeakMap<object, D1PreparedStatement>();
      const wrapStatement = (statement: D1PreparedStatement): D1PreparedStatement => {
        const proxy = new Proxy(statement, {
          get(target, property) {
            if (property === "bind") return (...values: unknown[]) => wrapStatement(target.bind(...values));
            if (property === "run" || property === "all" || property === "first" || property === "raw") {
              return (...args: unknown[]) => tick(() => (target[property] as (...a: unknown[]) => Promise<unknown>).apply(target, args));
            }
            const value = Reflect.get(target, property) as unknown;
            return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
          },
        });
        unwrap.set(proxy, statement);
        return proxy;
      };
      const real = env.EVIDENCE_DB;
      const database = new Proxy(real, {
        get(target, property) {
          if (property === "prepare") return (sql: string) => wrapStatement(target.prepare(sql));
          if (property === "batch") {
            return (statements: D1PreparedStatement[]) => tick(() => target.batch(statements.map((statement) => unwrap.get(statement) ?? statement)));
          }
          const value = Reflect.get(target, property) as unknown;
          return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
        },
      });
      const bucket = new Proxy(env.EVIDENCE_BUCKET, {
        get(target, property) {
          if (property === "delete") return (keys: string | string[]) => tick(() => target.delete(keys));
          const value = Reflect.get(target, property) as unknown;
          return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
        },
      });
      return { env: { EVIDENCE_DB: database, EVIDENCE_BUCKET: bucket }, operations: () => count };
    }

    // Each test covers one range of injected failure points, so no single
    // test replays the whole matrix; the guard test proves the ranges cover
    // every operation a clean run performs.
    const RANGE = 6;
    const RANGES = 6;
    let clean: { total: number; expected: Record<string, number> } | undefined;

    async function cleanRun(): Promise<{ total: number; expected: Record<string, number> }> {
      if (clean !== undefined) return clean;
      const s = await pruneScenario();
      const counter = faulty(Number.POSITIVE_INFINITY, "before");
      await advanceRetentionJobs(counter.env, s.runAt, { jobId: s.jobId });
      expect(await jobState(s.jobId)).toBe("completed");
      clean = { total: counter.operations(), expected: await counts(s.repo, s.older.id) };
      return clean;
    }

    it("covers every operation of a clean prune with its failure-point ranges", async () => {
      const { total } = await cleanRun();
      expect(total).toBeGreaterThan(15);
      expect(total).toBeLessThanOrEqual(RANGE * RANGES);
    });

    const cases = (["before", "after"] as const).flatMap((mode) =>
      Array.from({ length: RANGES }, (_, index) => ({ mode, first: index * RANGE + 1, last: (index + 1) * RANGE })),
    );
    it.each(cases)(
      "never leaves a row advertising deleted bytes when operation $first-$last fails ($mode), and the rerun completes with each tombstone written once",
      async ({ mode, first, last }) => {
        const { total, expected } = await cleanRun();
        for (let failAt = first; failAt <= Math.min(last, total); failAt += 1) {
          const s = await pruneScenario();
          const injected = faulty(failAt, mode);
          await expect(advanceRetentionJobs(injected.env, s.runAt, { jobId: s.jobId }), `operation ${failAt} (${mode})`).rejects.toThrow("injected failure");
          await expectNoDanglingAvailability(s.repo);
          // A reader never gets "the rows say the bytes exist, but they are gone".
          expect([200, 404], `fetch after operation ${failAt} (${mode})`).toContain((await s.request(objectPath(s.repo, s.older))).status);

          await advanceRetentionJobs(env, s.runAt);
          expect(await jobState(s.jobId), `job after operation ${failAt} (${mode})`).toBe("completed");
          expect(await counts(s.repo, s.older.id), `state after operation ${failAt} (${mode})`).toEqual({
            ...expected,
            epoch: s.snapshot.retention_epoch + 1,
          });
          expect((await s.request(objectPath(s.repo, s.newer))).status).toBe(200);
          expect(await uploadKeys(s.olderUpload)).toEqual([]);
        }
      },
    );
  });
});
