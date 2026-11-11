// The two durable change streams through `POST /changes/query`, against local
// D1 built from `migrations/`. Location events are appended through the
// public `POST /locations` route against an authentic ingested fixture.
import { describe, expect, it } from "vitest";
import {
  db,
  expectError,
  harness,
  ingestRig,
  newRepository,
  observation,
  postJson,
  prune,
  reportEvent,
  seed,
  type Harness,
} from "./helpers.ts";

interface ChangesPage {
  stream: string;
  snapshot: { observation_event_watermark: number; location_event_watermark: number };
  last_sequence: number;
  items: ({ type: string; sequence: number; observation_id?: string } & Record<string, unknown>)[];
  has_more: boolean;
  next_cursor: string | null;
}

const changesPath = (repo: string) => `/v1/repos/${repo}/changes/query`;

async function changes(request: Harness, repo: string, body: Record<string, unknown>): Promise<ChangesPage> {
  const response = await postJson(request, changesPath(repo), body);
  expect(response.status).toBe(200);
  const page = (await response.json()) as ChangesPage;
  expect(page.has_more).toBe(page.next_cursor !== null);
  return page;
}

async function follow(request: Harness, repo: string, query: Record<string, unknown>, first?: ChangesPage) {
  let page = first ?? (await changes(request, repo, query));
  const items = [...page.items];
  while (page.next_cursor !== null) {
    page = await changes(request, repo, { stream: query["stream"], page_size: query["page_size"], cursor: page.next_cursor });
    items.push(...page.items);
  }
  return { items, last: page };
}

async function appendEvent(request: Harness, repo: string, body: Record<string, unknown>) {
  const response = await postJson(request, `/v1/repos/${repo}/locations`, body, "verifier");
  expect(response.status).toBe(201);
  return (await response.json()) as { event_id: string; sequence: number };
}

describe("POST /changes/query: observations", () => {
  it("lists accepted events with their cell context, ending at the snapshot watermark", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const seeded = [observation(), observation(undefined, { outcome: "fail" })].sort((a, b) =>
      a.observation_id < b.observation_id ? -1 : 1,
    );
    await seed(repo, seeded);
    const page = await changes(request, repo, { stream: "observations" });
    expect(page.stream).toBe("observations");
    expect(page.items.map((item) => item.observation_id)).toEqual(seeded.map((o) => o.observation_id));
    const first = page.items[0]!;
    expect(Object.keys(first).sort()).toEqual(
      ["type", "sequence", "observation_id", "environment", "package", "gate", "head", "tree", "outcome", "completion", "recorded_at_ms"].sort(),
    );
    expect(first).toMatchObject({ type: "accepted", environment: "ubuntu-latest", package: "biscuit-hash", gate: "L1", completion: "complete" });
    expect(page.last_sequence).toBe(page.snapshot.observation_event_watermark);
    expect(page.last_sequence).toBe(page.items[1]!.sequence);
  });

  it("resumes with after_sequence, replays identically, and reports last_sequence for full and empty pages", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const empty = await changes(request, repo, { stream: "observations" });
    expect(empty).toMatchObject({ items: [], last_sequence: 0, has_more: false, next_cursor: null });

    for (let index = 0; index < 3; index += 1) await seed(repo, [observation()]);
    const first = await changes(request, repo, { stream: "observations", page_size: 2 });
    expect(first.items).toHaveLength(2);
    expect(first.has_more).toBe(true);
    expect(first.last_sequence).toBe(first.items[1]!.sequence);

    const resumed = await changes(request, repo, { stream: "observations", after_sequence: first.last_sequence });
    const replayed = await changes(request, repo, { stream: "observations", after_sequence: first.last_sequence });
    expect(resumed.items).toHaveLength(1);
    expect(replayed.items).toEqual(resumed.items);
    expect(resumed.last_sequence).toBe(resumed.snapshot.observation_event_watermark);

    const caughtUp = await changes(request, repo, { stream: "observations", after_sequence: resumed.last_sequence });
    expect(caughtUp).toMatchObject({ items: [], last_sequence: resumed.last_sequence, has_more: false });
  });

  it("refuses after_sequence beyond the watermark (400) and before the replay horizon (409)", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    for (let index = 0; index < 2; index += 1) await seed(repo, [observation()]);
    const { snapshot, items } = await changes(request, repo, { stream: "observations" });
    const watermark = snapshot.observation_event_watermark;
    await expectError(
      await postJson(request, changesPath(repo), { stream: "observations", after_sequence: watermark + 1 }),
      400,
      "invalid_input",
      "/after_sequence",
    );
    await expectError(await postJson(request, changesPath(repo), { stream: "locations", after_sequence: 1 }), 400, "invalid_input", "/after_sequence");

    await db().prepare("UPDATE repositories SET observation_event_horizon = ?2 WHERE repository_id = ?1").bind(repo, items[0]!.sequence).run();
    for (const body of [{ stream: "observations" }, { stream: "observations", after_sequence: items[0]!.sequence - 1 }]) {
      await expectError(await postJson(request, changesPath(repo), body), 409, "cursor_expired", "/after_sequence");
    }
    const atHorizon = await changes(request, repo, { stream: "observations", after_sequence: items[0]!.sequence });
    expect(atHorizon.items.map((item) => item.sequence)).toEqual([items[1]!.sequence]);
    // The location stream has its own horizon.
    expect((await changes(request, repo, { stream: "locations" })).items).toEqual([]);
  });

  it("refuses a cursor on another stream or page size, and a cursor with after_sequence or a snapshot", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    for (let index = 0; index < 3; index += 1) await seed(repo, [observation()]);
    const { next_cursor: cursor, snapshot } = await changes(request, repo, { stream: "observations", page_size: 1 });
    const send = (body: Record<string, unknown>) => postJson(request, changesPath(repo), body);
    await expectError(await send({ stream: "locations", page_size: 1, cursor }), 400, "invalid_cursor", "/cursor");
    await expectError(await send({ stream: "observations", page_size: 2, cursor }), 400, "invalid_cursor", "/cursor");
    await expectError(await send({ stream: "observations", page_size: 1, cursor, after_sequence: 0 }), 400, "invalid_input", "/after_sequence");
    await expectError(await send({ stream: "observations", page_size: 1, cursor, snapshot: { ...snapshot, repository_id: repo } }), 400, "invalid_input", "/snapshot");
    await expectError(await send({ stream: "everything" }), 400, "invalid_input", "/stream");
  });

  it("keeps events written after the snapshot out of its pages", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const original = [observation(), observation(), observation()];
    for (const item of original) await seed(repo, [item]);
    const first = await changes(request, repo, { stream: "observations", page_size: 1 });
    await seed(repo, [observation(), observation()]);
    const { items, last } = await follow(request, repo, { stream: "observations", page_size: 1 }, first);
    expect(items.map((item) => item.observation_id)).toEqual(original.map((o) => o.observation_id));
    expect(last.last_sequence).toBe(first.snapshot.observation_event_watermark);
    expect(last.has_more).toBe(false);

    const fresh = await changes(request, repo, { stream: "observations", after_sequence: last.last_sequence });
    expect(fresh.items).toHaveLength(2);
  });

  it("never shows another repository's events", async () => {
    const repo = await newRepository();
    const other = await newRepository();
    const request = harness([repo, other]);
    const mine = [observation(), observation()];
    await seed(repo, [mine[0]!]);
    await seed(other, [observation()]);
    await seed(repo, [mine[1]!]);
    const otherFixture = await ingestRig(request, other);
    await appendEvent(request, other, reportEvent(otherFixture));

    const page = await changes(request, repo, { stream: "observations" });
    expect(page.items.map((item) => item.observation_id)).toEqual(mine.map((o) => o.observation_id));
    expect((await changes(request, repo, { stream: "locations" })).items).toEqual([]);
  });
});

describe("POST /changes/query: locations", () => {
  it("is a separate stream that replays a verification and its withdrawal in order", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const fixture = await ingestRig(request, repo);
    const verified = await appendEvent(request, repo, reportEvent(fixture));
    const withdrawn = await appendEvent(request, repo, reportEvent(fixture, { kind: "withdrawn" }));

    const page = await changes(request, repo, { stream: "locations" });
    expect(page.stream).toBe("locations");
    expect(page.items.map((item) => [item.type, item.sequence])).toEqual([
      ["event", verified.sequence],
      ["event", withdrawn.sequence],
    ]);
    expect((page.items[0]!["event"] as { event_id: string; kind: string })).toMatchObject({ event_id: verified.event_id, kind: "verified" });
    expect(page.items[1]!["event"]).toEqual(withdrawn);
    expect(page.last_sequence).toBe(page.snapshot.location_event_watermark);
    expect(await changes(request, repo, { stream: "locations", after_sequence: 0 })).toEqual(page);

    const observations = await changes(request, repo, { stream: "observations" });
    expect(observations.items.map((item) => item.observation_id)).toEqual([fixture.observationId]);

    const paged = await follow(request, repo, { stream: "locations", page_size: 1 });
    expect(paged.items).toEqual(page.items);
  });

  it("leaves pruned tombstones in both streams after a sanctioned prune", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const fixture = await ingestRig(request, repo);
    const event = await appendEvent(request, repo, reportEvent(fixture));
    const jobId = await prune(repo, [fixture.observationId]);

    const observations = await changes(request, repo, { stream: "observations" });
    expect(observations.items.map((item) => item.type)).toEqual(["accepted", "pruned"]);
    const tombstone = observations.items[1]!;
    expect(tombstone).toMatchObject({ observation_id: fixture.observationId, job_id: jobId });
    expect(tombstone).not.toHaveProperty("outcome");

    const locations = await changes(request, repo, { stream: "locations" });
    expect(locations.items).toHaveLength(2);
    const [removed, pruned] = locations.items;
    // The earlier event's row is gone, so it is rendered as its tombstone.
    expect(removed).toMatchObject({ type: "pruned", sequence: event.sequence, observation_id: fixture.observationId, job_id: jobId });
    expect(pruned).toMatchObject({ type: "pruned", observation_id: fixture.observationId, job_id: jobId });
    expect(pruned!.sequence).toBeGreaterThan(event.sequence);
    expect(removed!["location_key"]).toBe(pruned!["location_key"]);
    expect(Object.keys(pruned!).sort()).toEqual(["job_id", "location_key", "observation_id", "recorded_at_ms", "sequence", "type"]);
  });
});
