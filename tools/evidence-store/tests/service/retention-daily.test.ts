// The daily scheduled run end to end: the `scheduled` handler advances
// retention jobs, and a later run expires 30-day-old `pruned` tombstones and
// advances both change-feed replay horizons. Its own file, because tombstone
// expiry is store-wide and must not see other tests' tombstones.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { runDaily } from "../../src/daily.ts";
import { createWorker } from "../../src/worker.ts";
import { db, expectError, postJson } from "./helpers.ts";
import { DAY, jobState, pruneScenario } from "./retention-fixtures.ts";

describe("runDaily", () => {
  it("runs retention jobs from the scheduled handler, then expires old tombstones and advances both horizons", async () => {
    const s = await pruneScenario();
    const worker = createWorker({
      authenticate: async () => {
        throw new Error("the scheduled run authenticates nobody");
      },
    });
    await worker.scheduled!({ scheduledTime: s.runAt, cron: "0 3 * * *", noRetry() {} } as ScheduledController, env, {} as ExecutionContext);
    expect(await jobState(s.jobId)).toBe("completed");

    const horizons = async () =>
      (await db()
        .prepare("SELECT observation_event_horizon AS observations, location_event_horizon AS locations FROM repositories WHERE repository_id = ?1")
        .bind(s.repo)
        .first())!;
    const tombstone = async (table: string) =>
      (await db().prepare(`SELECT MAX(seq) AS seq FROM ${table} WHERE repository_id = ?1 AND kind = 'pruned'`).bind(s.repo).first<{ seq: number }>())!.seq;
    const observationTombstone = await tombstone("observation_events");
    const locationTombstone = await tombstone("location_events");

    // Tombstones are kept for 30 days.
    const kept = await runDaily(env, s.runAt + 29 * DAY);
    expect(kept.tombstones).toEqual({ observation_events_deleted: 0, location_events_deleted: 0 });
    expect(await horizons()).toEqual({ observations: 0, locations: 0 });

    const report = await runDaily(env, s.runAt + 31 * DAY);
    expect(report.skipped_for_maintenance).toBe(false);
    // The pruned observation's accepted event and tombstone; each pruned location row's event and tombstone.
    expect(report.tombstones).toEqual({ observation_events_deleted: 2, location_events_deleted: 8 });
    expect(await horizons()).toEqual({ observations: observationTombstone, locations: locationTombstone });
    const events = await db()
      .prepare("SELECT observation_id FROM observation_events WHERE repository_id = ?1 ORDER BY seq")
      .bind(s.repo)
      .all<{ observation_id: string }>();
    // The retained observations' events stay; nothing of the pruned one remains.
    expect(events.results.map((row) => row.observation_id).sort()).toEqual([s.newer.id, s.extra.id].sort());

    for (const stream of ["observations", "locations"]) {
      await expectError(await postJson(s.request, `/v1/repos/${s.repo}/changes/query`, { stream, after_sequence: 0 }), 409, "cursor_expired", "/after_sequence");
    }
    const resumed = await postJson(s.request, `/v1/repos/${s.repo}/changes/query`, { stream: "observations", after_sequence: observationTombstone });
    expect(resumed.status).toBe(200);

    // A later run never moves a horizon back.
    await runDaily(env, s.runAt + 60 * DAY);
    expect(await horizons()).toEqual({ observations: observationTombstone, locations: locationTombstone });
    expect(report.capacity?.resources.map((usage) => usage.resource)).toEqual(["r2_bytes", "r2_class_a", "r2_class_b"]);
  });
});
