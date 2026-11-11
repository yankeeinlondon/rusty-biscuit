// Schema-level guarantees of the numbered migrations, checked directly with
// SQL against local D1: ordering, CHECKs on generated columns, immutability,
// the retention deletion guard, release pins, and cell locks.
import { describe, expect, it } from "vitest";
import { ingestCollectionBatch } from "../../src/db/ingest.ts";
import {
  collection,
  db,
  hex,
  HEAD,
  newRepository,
  observation,
  rejection,
  repositoryState,
  retentionJob,
  TREE,
} from "./helpers.ts";

async function seed(repositoryId: string, count = 1): Promise<string[]> {
  const observations = Array.from({ length: count }, () => observation());
  const outcome = await ingestCollectionBatch(db(), collection(repositoryId, observations));
  if (outcome.kind !== "ok") throw new Error(`seed failed: ${outcome.kind}`);
  return outcome.result.accepted;
}

describe("migrations", () => {
  it("apply in numbered order and create every table", async () => {
    const applied = await db()
      .prepare("SELECT name FROM d1_migrations ORDER BY id")
      .all<{ name: string }>();
    expect(applied.results.map((row) => row.name)).toEqual([
      "0001_evidence.sql",
      "0002_retention.sql",
      "0003_credentials.sql",
      "0004_uploads.sql",
      "0005_evaluations.sql",
      "0006_feed_horizons.sql",
      "0007_maintenance.sql",
      "0008_retention_state.sql",
    ]);
    const tables = await db()
      .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
      .all<{ name: string }>();
    expect(tables.results.map((row) => row.name)).toEqual(
      expect.arrayContaining([
        "repositories",
        "observations",
        "observation_events",
        "collections",
        "collection_members",
        "report_locations",
        "location_events",
        "retention_certificates",
        "retention_jobs",
        "retention_job_members",
        "release_pins",
        "cell_locks",
        "credentials",
        "credential_grants",
        "audit_log",
        "uploads",
        "upload_parts",
        "service_state",
        "quota_reservations",
        "capacity_baseline",
        "evaluations",
        "retention_supersessions",
      ]),
    );
  });

  it("refuses a payload whose generated gate is outside the gate vocabulary", async () => {
    const repo = await newRepository();
    const bad = observation(hex(64), { gate: "nightly" });
    const message = await rejection(ingestCollectionBatch(db(), collection(repo, [bad])));
    expect(message).toMatch(/CHECK constraint failed/);
    const state = await repositoryState(repo);
    expect(state.observations).toEqual([]);
    expect(state.collections).toEqual([]);
  });

  it("refuses an observation whose column ID differs from its payload ID", async () => {
    const repo = await newRepository();
    const mismatched = { ...observation(), observation_id: hex(64) };
    await rejection(ingestCollectionBatch(db(), collection(repo, [mismatched])));
    expect((await repositoryState(repo)).observations).toEqual([]);
  });

  it("refuses UPDATE on observations, collections, members, and both event streams", async () => {
    const repo = await newRepository();
    const [id] = await seed(repo);
    const d = db();
    const statements = [
      "UPDATE observations SET received_at_ms = 0 WHERE repository_id = ?1",
      "UPDATE collections SET received_at_ms = 0 WHERE repository_id = ?1",
      "UPDATE collection_members SET position = position WHERE repository_id = ?1",
      "UPDATE observation_events SET recorded_at_ms = 0 WHERE repository_id = ?1",
    ];
    for (const sql of statements) {
      expect(await rejection(d.prepare(sql).bind(repo).run())).toMatch(/immutable/);
    }
    await d
      .prepare(
        `INSERT INTO report_locations(repository_id, event_id, observation_id, reference_kind,
           reference_digest, report_path, location_key, location_kind, locator_json, state,
           verification_method, checked_at_ms, valid_until_ms, checked_by)
         VALUES (?1, ?2, ?3, 'report', ?4, 'junit.xml', ?4, 'object', '{"kind":"object"}',
           'verified', 'bytes', 1, 2, 'verifier:test')`,
      )
      .bind(repo, crypto.randomUUID(), id, hex(64))
      .run();
    for (const sql of [
      "UPDATE report_locations SET state = 'withdrawn' WHERE repository_id = ?1",
      "UPDATE location_events SET recorded_at_ms = 0 WHERE repository_id = ?1",
    ]) {
      expect(await rejection(d.prepare(sql).bind(repo).run())).toMatch(/immutable/);
    }
    const repoRow = await d
      .prepare("SELECT evidence_revision, location_event_watermark FROM repositories WHERE repository_id = ?1")
      .bind(repo)
      .first<{ evidence_revision: number; location_event_watermark: number }>();
    expect(repoRow?.evidence_revision).toBe(2);
    expect(repoRow?.location_event_watermark).toBeGreaterThan(0);
  });

  it("refuses a replayed location event UUID with a different body", async () => {
    const repo = await newRepository();
    const [id] = await seed(repo);
    const eventId = crypto.randomUUID();
    const key = hex(64);
    const insert = (state: string) =>
      db()
        .prepare(
          `INSERT INTO report_locations(repository_id, event_id, observation_id, reference_kind,
             reference_digest, report_path, location_key, location_kind, locator_json, state,
             checked_at_ms, checked_by)
           VALUES (?1, ?2, ?3, 'report', ?4, 'junit.xml', ?4, 'object', '{}', ?5, 1, 'verifier:test')
           ON CONFLICT(repository_id, event_id) DO NOTHING`,
        )
        .bind(repo, eventId, id, key, state)
        .run();
    const count = async () =>
      (await db()
        .prepare("SELECT COUNT(*) AS n FROM report_locations WHERE repository_id = ?1")
        .bind(repo)
        .first<{ n: number }>())?.n;
    await insert("unavailable");
    await insert("unavailable");
    expect(await count()).toBe(1);
    expect(await rejection(insert("withdrawn"))).toMatch(/identity_conflict/);
    expect(await count()).toBe(1);
  });

  describe("deletion guard", () => {
    it("refuses DELETE on an observation with no authorizing job member", async () => {
      const repo = await newRepository();
      const [id] = await seed(repo);
      const del = () =>
        db()
          .prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id = ?2")
          .bind(repo, id)
          .run();
      expect(await rejection(del())).toMatch(/deletion_not_authorized/);

      // A member of a job that has not reached `deleting-rows` is not authority either.
      await retentionJob(repo, "scheduled", [id!]);
      expect(await rejection(del())).toMatch(/deletion_not_authorized/);
      expect((await repositoryState(repo)).observations).toEqual([id]);
    });

    it("permits a job in deleting-rows and appends a pruned tombstone", async () => {
      const repo = await newRepository();
      const [id] = await seed(repo);
      const before = await repositoryState(repo);
      const jobId = await retentionJob(repo, "deleting-rows", [id!]);
      await db()
        .prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id = ?2")
        .bind(repo, id)
        .run();
      const after = await repositoryState(repo);
      expect(after.observations).toEqual([]);
      expect(after.revision).toBe(before.revision + 1);
      expect(after.events).toEqual([
        { kind: "accepted", observation_id: id },
        { kind: "pruned", observation_id: id },
      ]);
      const tombstone = await db()
        .prepare("SELECT job_id FROM observation_events WHERE repository_id = ?1 AND kind = 'pruned'")
        .bind(repo)
        .first<{ job_id: string }>();
      expect(tombstone?.job_id).toBe(jobId);
      // Membership survives so an export can report the collection as pruned.
      expect(after.members).toBe(1);
    });

    it("never admits a release-pinned observation as a job member", async () => {
      const repo = await newRepository();
      const [id] = await seed(repo);
      await db()
        .prepare("INSERT INTO release_pins(repository_id, release, observation_id, pinned_at_ms) VALUES (?1, 'v1.0.0', ?2, 1)")
        .bind(repo, id)
        .run();
      const jobId = await retentionJob(repo, "scheduled", []);
      const message = await rejection(
        db()
          .prepare(
            `INSERT INTO retention_job_members(job_id, repository_id, observation_id, package,
               environment, gate, superseded_at_ms)
             VALUES (?1, ?2, ?3, 'biscuit-hash', 'ubuntu-latest', 'L1', 0)`,
          )
          .bind(jobId, repo, id)
          .run(),
      );
      expect(message).toMatch(/release_pinned/);
    });

    it("refuses to pin an observation an unfinished job will delete", async () => {
      const repo = await newRepository();
      const [id] = await seed(repo);
      await retentionJob(repo, "withdrawing", [id!]);
      const message = await rejection(
        db()
          .prepare("INSERT INTO release_pins(repository_id, release, observation_id, pinned_at_ms) VALUES (?1, 'v1.0.0', ?2, 1)")
          .bind(repo, id)
          .run(),
      );
      expect(message).toMatch(/retention_member/);
    });
  });

  it("refuses new observations in a locked cell but not an identical replay", async () => {
    const repo = await newRepository();
    const existing = observation();
    const first = collection(repo, [existing]);
    expect((await ingestCollectionBatch(db(), first)).kind).toBe("ok");
    const jobId = await retentionJob(repo, "deleting-rows", []);
    await db()
      .prepare(
        `INSERT INTO cell_locks(repository_id, package, environment, gate, job_id, acquired_at_ms)
         VALUES (?1, 'biscuit-hash', 'ubuntu-latest', 'L1', ?2, 1)`,
      )
      .bind(repo, jobId)
      .run();
    const replay = await ingestCollectionBatch(db(), { ...first, request_id: crypto.randomUUID() });
    expect(replay.kind).toBe("ok");
    const message = await rejection(
      ingestCollectionBatch(db(), collection(repo, [existing, observation()], { head: HEAD, tree: TREE })),
    );
    expect(message).toMatch(/cell_locked/);
    // A different cell is unaffected.
    const other = await ingestCollectionBatch(db(), collection(repo, [observation(hex(64), { package: "playa" })]));
    expect(other.kind).toBe("ok");
  });

  it("enforces credential and upload invariants", async () => {
    const repo = await newRepository();
    const d = db();
    const credentialId = hex(32);
    await d
      .prepare(
        `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms, expires_at_ms)
         VALUES (?1, 'operator', 'ops', ?2, 0, 1, 2)`,
      )
      .bind(credentialId, hex(64))
      .run();
    expect(
      await rejection(
        d
          .prepare("INSERT INTO credential_grants(credential_id, repository_id, role) VALUES (?1, ?2, 'reader')")
          .bind(credentialId, repo)
          .run(),
      ),
    ).toMatch(/host credentials/);
    expect(
      await rejection(
        d.prepare("UPDATE credentials SET secret_sha256 = ?2 WHERE credential_id = ?1").bind(credentialId, hex(64)).run(),
      ),
    ).toMatch(/immutable/);

    const uploadId = hex(32);
    const insertUpload = (bytes: number, partCount: number) =>
      d
        .prepare(
          `INSERT INTO uploads(upload_id, repository_id, observation_id, reference_kind, reference_digest,
             path, bytes, part_count, state, staging_key, created_by, created_at_ms, expires_at_ms)
           VALUES (?1, ?2, ?3, 'report', ?4, 'junit.xml', ?5, ?6, 'open', ?1, 'producer:test', 1, 2)`,
        )
        .bind(uploadId, repo, hex(64), hex(64), bytes, partCount)
        .run();
    expect(await rejection(insertUpload(1_048_577, 1))).toMatch(/CHECK constraint failed/);
    await insertUpload(1_048_577, 2);
    expect(
      await rejection(d.prepare("UPDATE uploads SET bytes = 1 WHERE upload_id = ?1").bind(uploadId).run()),
    ).toMatch(/immutable/);
    await d.prepare("UPDATE uploads SET state = 'rejected' WHERE upload_id = ?1").bind(uploadId).run();
    expect(
      await rejection(d.prepare("UPDATE uploads SET state = 'open' WHERE upload_id = ?1").bind(uploadId).run()),
    ).toMatch(/terminal/);
  });
});
