// The batch repository abstraction against local D1: acceptance, idempotent
// replay, mixed batches, empty and unknown-repository collections, identity
// conflicts that must roll back the whole batch, and concurrent deliveries.
import { describe, expect, it } from "vitest";
import {
  ingestCollectionBatch,
  readCollectionManifest,
  readCollectionMembers,
  readObservation,
} from "../../src/db/ingest.ts";
import { repositoryExists } from "../../src/db/repositories.ts";
import {
  collection,
  db,
  hex,
  newRepository,
  observation,
  repositoryState,
  retentionJob,
} from "./helpers.ts";

describe("ingestCollectionBatch", () => {
  it("accepts a new collection with one revision step and one event per observation", async () => {
    const repo = await newRepository();
    const input = collection(repo, [observation(), observation(), observation()]);
    const outcome = await ingestCollectionBatch(db(), input);
    const ids = input.observations.map((o) => o.observation_id);
    expect(outcome).toEqual({
      kind: "ok",
      result: { collection_id: input.collection_id, accepted: ids, already_present: [], evidence_revision: 3 },
    });
    const state = await repositoryState(repo);
    expect(state.observations).toEqual(ids);
    expect(state.collections).toEqual([input.collection_id]);
    expect(state.members).toBe(3);
    expect(state.events).toEqual(ids.map((id) => ({ kind: "accepted", observation_id: id })));
    expect(state.observation_watermark).toBeGreaterThan(0);

    const manifest = await readCollectionManifest(db(), repo, input.collection_id);
    expect(manifest).toMatchObject({
      collection_id: input.collection_id,
      environment: "ubuntu-latest",
      observation_count: 3,
      source_version: 3,
      accepted_principal: input.principal,
      missing_member_count: 0,
    });
    const page = await readCollectionMembers(db(), repo, input.collection_id, -1, 2);
    expect(page.missing).toBe(false);
    expect(page.members.map((m) => m.position)).toEqual([0, 1]);
    expect(page.members.map((m) => m.payload_json)).toEqual(
      input.observations.slice(0, 2).map((o) => o.payload_json),
    );
    const rest = await readCollectionMembers(db(), repo, input.collection_id, 1, 2);
    expect(rest.members.map((m) => m.observation_id)).toEqual([ids[2]]);
  });

  it("treats an identical replay as already present and preserves the first publisher", async () => {
    const repo = await newRepository();
    const first = collection(repo, [observation(), observation()], { source_version: 2 });
    await ingestCollectionBatch(db(), first);
    const before = await repositoryState(repo);

    const replay = {
      ...first,
      principal: "github:other-workflow",
      request_id: crypto.randomUUID(),
      source_version: 3 as const,
      now_ms: first.now_ms + 60_000,
    };
    const outcome = await ingestCollectionBatch(db(), replay);
    expect(outcome).toEqual({
      kind: "ok",
      result: {
        collection_id: first.collection_id,
        accepted: [],
        already_present: first.observations.map((o) => o.observation_id),
        evidence_revision: before.revision,
      },
    });
    expect(await repositoryState(repo)).toEqual(before);

    for (const o of first.observations) {
      expect(await readObservation(db(), repo, o.observation_id)).toMatchObject({
        payload_json: o.payload_json,
        source_version: 2,
        accepted_principal: first.principal,
        accepted_request_id: first.request_id,
        accepted_collection_id: first.collection_id,
        received_at_ms: first.now_ms,
      });
    }
    expect(await readCollectionManifest(db(), repo, first.collection_id)).toMatchObject({
      source_version: 2,
      accepted_principal: first.principal,
      accepted_request_id: first.request_id,
      received_at_ms: first.now_ms,
    });
  });

  it("splits a mixed batch and advances the revision once per new observation", async () => {
    const repo = await newRepository();
    const old = [observation(), observation()];
    await ingestCollectionBatch(db(), collection(repo, old));
    const fresh = [observation(), observation(), observation()];
    const input = collection(repo, [...old, ...fresh]);
    const outcome = await ingestCollectionBatch(db(), input);
    if (outcome.kind !== "ok") throw new Error(outcome.kind);
    const order = input.observations.map((o) => o.observation_id);
    const freshIds = new Set(fresh.map((o) => o.observation_id));
    expect(outcome.result.accepted).toEqual(order.filter((id) => freshIds.has(id)));
    expect(outcome.result.already_present).toEqual(order.filter((id) => !freshIds.has(id)));
    expect(outcome.result.evidence_revision).toBe(5);
    const state = await repositoryState(repo);
    expect(state.events).toHaveLength(5);
    expect(state.members).toBe(2 + 5);
    // The previously present observations keep their original collection.
    const kept = await readObservation(db(), repo, old[0]!.observation_id);
    expect(kept?.accepted_collection_id).not.toBe(input.collection_id);
  });

  it("stores an empty collection without a revision step or event", async () => {
    const repo = await newRepository();
    const input = collection(repo, []);
    expect(await ingestCollectionBatch(db(), input)).toEqual({
      kind: "ok",
      result: { collection_id: input.collection_id, accepted: [], already_present: [], evidence_revision: 0 },
    });
    expect(await ingestCollectionBatch(db(), { ...input, request_id: crypto.randomUUID() })).toMatchObject({
      kind: "ok",
      result: { evidence_revision: 0 },
    });
    const state = await repositoryState(repo);
    expect(state).toMatchObject({ revision: 0, observation_watermark: 0, members: 0, events: [] });
    expect(state.collections).toEqual([input.collection_id]);
    expect(await readCollectionManifest(db(), repo, input.collection_id)).toMatchObject({
      observation_count: 0,
      missing_member_count: 0,
    });
  });

  it("writes nothing for an unknown repository", async () => {
    const repo = "9" + String(Date.now()).slice(-12);
    expect(await repositoryExists(db(), repo)).toBe(false);
    const input = collection(repo, [observation(), observation()]);
    expect(await ingestCollectionBatch(db(), input)).toEqual({ kind: "unknown_repository" });
    for (const table of ["observations", "collections", "collection_members", "observation_events"]) {
      const row = await db()
        .prepare(`SELECT COUNT(*) AS n FROM ${table} WHERE repository_id = ?1`)
        .bind(repo)
        .first<{ n: number }>();
      expect(row?.n, table).toBe(0);
    }
  });

  describe("identity conflicts roll back the whole batch", () => {
    async function expectRollback(
      repo: string,
      conflicting: Parameters<typeof ingestCollectionBatch>[1],
    ): Promise<void> {
      const before = await repositoryState(repo);
      expect(await ingestCollectionBatch(db(), conflicting)).toEqual({ kind: "identity_conflict" });
      expect(await repositoryState(repo)).toEqual(before);
      expect(await readCollectionManifest(db(), repo, conflicting.collection_id)).toBeNull();
    }

    it("same observation ID with different canonical bytes", async () => {
      const repo = await newRepository();
      const original = observation();
      await ingestCollectionBatch(db(), collection(repo, [original]));
      const altered = observation(original.observation_id, { variant: "different" });
      await expectRollback(repo, collection(repo, [altered, observation(), observation()]));
    });

    for (const [field, value] of [
      ["head", "c".repeat(40)],
      ["tree", "d".repeat(40)],
      ["environment", "macos-latest"],
    ] as const) {
      it(`same payload with a different ${field}`, async () => {
        const repo = await newRepository();
        const original = observation();
        await ingestCollectionBatch(db(), collection(repo, [original]));
        await expectRollback(repo, collection(repo, [original, observation()], { [field]: value }));
      });
    }

    it("a reused collection ID with a different context", async () => {
      const repo = await newRepository();
      const first = collection(repo, [observation()]);
      await ingestCollectionBatch(db(), first);
      const before = await repositoryState(repo);
      const reused = collection(repo, [observation()], { collection_id: first.collection_id, head: "e".repeat(40) });
      expect(await ingestCollectionBatch(db(), reused)).toEqual({ kind: "identity_conflict" });
      expect(await repositoryState(repo)).toEqual(before);
    });
  });

  describe("concurrent deliveries", () => {
    it("a duplicate race accepts once and reports the other as already present", async () => {
      const repo = await newRepository();
      const base = collection(repo, [observation(), observation(), observation()]);
      const [a, b] = await Promise.all([
        ingestCollectionBatch(db(), base),
        ingestCollectionBatch(db(), { ...base, request_id: crypto.randomUUID(), principal: "credential:producer-b" }),
      ]);
      if (a.kind !== "ok" || b.kind !== "ok") throw new Error("both deliveries must succeed");
      const ids = base.observations.map((o) => o.observation_id);
      const results = [a.result, b.result].sort((x, y) => y.accepted.length - x.accepted.length);
      expect(results[0]).toMatchObject({ accepted: ids, already_present: [] });
      expect(results[1]).toMatchObject({ accepted: [], already_present: ids });
      expect(results[0]?.evidence_revision).toBe(3);
      expect(results[1]?.evidence_revision).toBe(3);
      const state = await repositoryState(repo);
      expect(state.observations).toEqual(ids);
      expect(state.collections).toEqual([base.collection_id]);
      expect(state.members).toBe(3);
      expect(state.events).toHaveLength(3);
    });

    it("a conflicting pair with fresh siblings leaves nothing of the losing batch", async () => {
      const repo = await newRepository();
      for (let pair = 0; pair < 4; pair += 1) {
        const shared = hex(64);
        const left = collection(repo, [observation(shared), observation(), observation()]);
        const right = collection(repo, [observation(shared, { variant: "other" }), observation(), observation()]);
        const before = await repositoryState(repo);
        const [l, r] = await Promise.all([ingestCollectionBatch(db(), left), ingestCollectionBatch(db(), right)]);
        const kinds = [l.kind, r.kind].sort();
        expect(kinds).toEqual(["identity_conflict", "ok"]);
        const [winner, loser] = l.kind === "ok" ? [left, right] : [right, left];
        const after = await repositoryState(repo);
        const loserFresh = loser.observations
          .map((o) => o.observation_id)
          .filter((id) => id !== shared);
        for (const id of loserFresh) expect(after.observations).not.toContain(id);
        expect(after.collections).toContain(winner.collection_id);
        expect(after.collections).not.toContain(loser.collection_id);
        expect(after.revision).toBe(before.revision + 3);
        expect(after.events).toHaveLength(before.events.length + 3);
        expect(after.members).toBe(before.members + 3);
      }
    });
  });

  it("reports pruned members on export reads", async () => {
    const repo = await newRepository();
    const input = collection(repo, [observation(), observation()]);
    await ingestCollectionBatch(db(), input);
    const pruned = input.observations[1]!.observation_id;
    await retentionJob(repo, "deleting-rows", [pruned]);
    await db()
      .prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id = ?2")
      .bind(repo, pruned)
      .run();
    expect(await readCollectionManifest(db(), repo, input.collection_id)).toMatchObject({
      observation_count: 2,
      missing_member_count: 1,
    });
    const page = await readCollectionMembers(db(), repo, input.collection_id, -1, 10);
    expect(page.missing).toBe(true);
    expect(page.members[1]).toEqual({ position: 1, observation_id: pruned, payload_json: null });
    expect(await readObservation(db(), repo, pruned)).toBeNull();
  });
});
