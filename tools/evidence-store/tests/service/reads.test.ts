// Snapshot, observation, and candidate-history reads through the Worker's
// public `fetch`, against local D1 built from `migrations/`. History tests
// seed synthetic observations directly; the observation-detail test ingests an
// authentic fixture so the record carries real reports and proof.
import { describe, expect, it } from "vitest";
import { canonical, loadStrict, type JsonObject } from "../../src/contracts/index.ts";
import type { IngestObservation } from "../../src/db/ingest.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";
import {
  credentialPublisher,
  db,
  expectError,
  harness,
  hex,
  newRepository,
  postJson,
  prune,
  seed,
  type Harness,
} from "./helpers.ts";

const H1 = "1".repeat(40);
const H2 = "2".repeat(40);
const H3 = "3".repeat(40);

interface Shape {
  package?: string;
  gate?: string;
  outcome?: string;
  completion?: string;
  retry_of?: string;
  padding?: number;
}

/** A canonical-shaped synthetic observation; `padding` inflates its encoded size. */
function synthetic(shape: Shape = {}, id: string = hex(64)): IngestObservation {
  const payload =
    `{"cell":{"gate":${JSON.stringify(shape.gate ?? "L1")},"gate_input_identity":"0123abcd",` +
    `"outcome":${JSON.stringify(shape.outcome ?? "pass")},"package":${JSON.stringify(shape.package ?? "biscuit-hash")}},` +
    `"completion":${JSON.stringify(shape.completion ?? "complete")},"observation_id":${JSON.stringify(id)}` +
    (shape.retry_of === undefined ? "" : `,"retry_of":${JSON.stringify(shape.retry_of)}`) +
    (shape.padding === undefined ? "" : `,"variant":${JSON.stringify("x".repeat(shape.padding))}`) +
    "}";
  return { observation_id: id, payload_json: payload };
}

interface Snapshot {
  repository_id: string;
  generation: string;
  retention_epoch: number;
  evidence_revision: number;
  observation_event_watermark: number;
  location_event_watermark: number;
}

interface HistoryRecord {
  sequence: number;
  environment: string;
  head: string;
  tree: string;
  observation: { observation_id: string; cell: { outcome: string }; completion: string };
  acceptance: { source_version: number; accepted_at_ms: number; accepted_by: unknown; collection_id: string };
}

interface HistoryPage {
  snapshot: Snapshot;
  items: HistoryRecord[];
  has_more: boolean;
  next_cursor: string | null;
}

const historyPath = (repo: string) => `/v1/repos/${repo}/history/query`;

async function history(request: Harness, repo: string, body: Record<string, unknown>): Promise<HistoryPage> {
  const response = await postJson(request, historyPath(repo), body);
  expect(response.status).toBe(200);
  const page = (await response.json()) as HistoryPage;
  expect(page.has_more).toBe(page.next_cursor !== null);
  return page;
}

/** Follows every cursor and returns the item IDs in order plus the page count. */
async function historyAll(request: Harness, repo: string, query: Record<string, unknown>) {
  const ids: string[] = [];
  const snapshots: Snapshot[] = [];
  let page = await history(request, repo, query);
  let pages = 1;
  for (;;) {
    ids.push(...page.items.map((item) => item.observation.observation_id));
    snapshots.push(page.snapshot);
    if (page.next_cursor === null) break;
    page = await history(request, repo, { ...query, cursor: page.next_cursor });
    pages += 1;
  }
  return { ids, pages, snapshots };
}

async function snapshot(request: Harness, repo: string): Promise<Snapshot> {
  const response = await request(`/v1/repos/${repo}/snapshot`);
  expect(response.status).toBe(200);
  return (await response.json()) as Snapshot;
}

async function acceptedSequence(repo: string, id: string): Promise<number> {
  const row = await db()
    .prepare("SELECT MAX(seq) AS seq FROM observation_events WHERE repository_id = ?1 AND observation_id = ?2 AND kind = 'accepted'")
    .bind(repo, id)
    .first<{ seq: number }>();
  return row!.seq;
}

const BASE = { package: "biscuit-hash", gate: "L1", reachable_heads: [H1, H2] };

describe("GET /snapshot", () => {
  it("returns the repository row's bounds and answers 404 for an unknown repository", async () => {
    const repo = await newRepository();
    const unknown = `9${hex(10).replace(/[a-f]/g, "7")}`;
    const request = harness([repo, unknown]);
    await seed(repo, [synthetic(), synthetic()]);
    const row = await db()
      .prepare("SELECT generation, observation_event_watermark FROM repositories WHERE repository_id = ?1")
      .bind(repo)
      .first<{ generation: string; observation_event_watermark: number }>();
    expect(await snapshot(request, repo)).toEqual({
      repository_id: repo,
      generation: row!.generation,
      retention_epoch: 0,
      evidence_revision: 2,
      observation_event_watermark: row!.observation_event_watermark,
      location_event_watermark: 0,
    });
    await expectError(await request(`/v1/repos/${unknown}/snapshot`), 404, "not_found");
  });
});

describe("GET /observations/{id}", () => {
  it("returns the record, its acceptance, its retry successors, and the newest event of each location", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const document = loadStrict(new TextEncoder().encode(rigFixture));
    if (!document.ok) throw new Error("fixture");
    const fixture = document.value as JsonObject;
    request.clock = 1_800_000_000_000;
    const ingested = await postJson(request, `/v1/repos/${repo}/collections`, `{"source_version":3,"collection":${canonical(fixture)}}`, "producer");
    expect(ingested.status).toBe(200);
    const { collection_id: collectionId } = (await ingested.json()) as { collection_id: string };
    const observation = (fixture["observations"] as JsonObject[])[0]!;
    const id = observation["observation_id"] as string;
    const report = (observation["reports"] as JsonObject[])[0]!;

    const retryA = synthetic({ retry_of: id, outcome: "fail" });
    const retryB = synthetic({ retry_of: id });
    await seed(repo, [retryA, retryB]);

    const event = (eventId: string, locator: Record<string, string>, kind: string) => ({
      event_id: eventId,
      observation_id: id,
      reference_kind: "report",
      reference_digest: report["sha256"],
      path: report["path"],
      locator,
      kind,
      ...(kind === "verified" ? { verification_method: "bytes" } : {}),
    });
    const hostDir = { kind: "host-dir", host: "rig-mac", directory: "/reports/run7" };
    const artifact = { kind: "artifact", repository_id: repo, run_id: "11", artifact_id: "22", member: "biscuit-hash-L1.xml" };
    const send = (body: unknown) => postJson(request, `/v1/repos/${repo}/locations`, body, "verifier");
    expect((await send(event(crypto.randomUUID(), hostDir, "verified"))).status).toBe(201);
    const withdrawnId = crypto.randomUUID();
    expect((await send(event(withdrawnId, hostDir, "withdrawn"))).status).toBe(201);
    const artifactId = crypto.randomUUID();
    expect((await send(event(artifactId, artifact, "verified"))).status).toBe(201);

    const response = await request(`/v1/repos/${repo}/observations/${id}`);
    expect(response.status).toBe(200);
    const text = await response.text();
    // The stored canonical payload is spliced verbatim.
    expect(text).toContain(`"observation":${canonical(observation)}`);
    const detail = JSON.parse(text) as {
      record: HistoryRecord;
      retried_by: string[];
      locations: { event_id: string; kind: string; verification_method?: string; valid_until_ms?: number; locator: unknown }[];
    };
    expect(detail.record).toMatchObject({
      sequence: await acceptedSequence(repo, id),
      environment: fixture["environment"],
      head: fixture["head"],
      tree: fixture["tree"],
      acceptance: {
        source_version: 3,
        accepted_at_ms: 1_800_000_000_000,
        accepted_by: JSON.parse(credentialPublisher("aaaa")),
        collection_id: collectionId,
      },
    });
    expect(detail.retried_by).toEqual([retryA.observation_id, retryB.observation_id].sort());
    // The withdrawal hides the earlier verification of the same location.
    expect(detail.locations.map((location) => [location.event_id, location.kind])).toEqual([
      [withdrawnId, "withdrawn"],
      [artifactId, "verified"],
    ]);
    expect(detail.locations[0]).not.toHaveProperty("verification_method");
    expect(detail.locations[0]).not.toHaveProperty("valid_until_ms");
    expect(detail.locations[1]!.locator).toEqual(artifact);
  });

  it("answers 404 for a missing observation but 200 with no items for an empty history", async () => {
    const repo = await newRepository();
    const unknown = `8${hex(10).replace(/[a-f]/g, "1")}`;
    const request = harness([repo, unknown]);
    await expectError(await request(`/v1/repos/${repo}/observations/${hex(64)}`), 404, "not_found");
    await expectError(await postJson(request, historyPath(unknown), BASE), 404, "not_found");

    const empty = await history(request, repo, BASE);
    expect(empty).toMatchObject({ items: [], has_more: false, next_cursor: null });
    const noHeads = await history(request, repo, { ...BASE, reachable_heads: [] });
    expect(noHeads).toMatchObject({ items: [], has_more: false, next_cursor: null });
    expect(noHeads.snapshot.repository_id).toBe(repo);
  });
});

describe("POST /history/query", () => {
  it("includes failures and incomplete attempts in every environment and excludes other cells, heads, and repositories", async () => {
    const repo = await newRepository();
    const other = await newRepository();
    const request = harness([repo, other]);
    const pass = synthetic();
    const fail = synthetic({ outcome: "fail" });
    const partial = synthetic({ completion: "partial", outcome: "fail" });
    const onH2 = synthetic({ completion: "interrupted" });
    await seed(repo, [pass], { head: H1 });
    await seed(repo, [fail], { head: H1, environment: "macos-latest" });
    await seed(repo, [partial], { head: H1, environment: "windows-latest" });
    await seed(repo, [onH2], { head: H2, environment: "wsl2-ubuntu" });
    await seed(repo, [synthetic({ package: "biscuit-file" }), synthetic({ gate: "L2" })], { head: H1 });
    await seed(repo, [synthetic()], { head: H3 });
    await seed(other, [synthetic()], { head: H1 });

    const page = await history(request, repo, BASE);
    expect(page.items.map((item) => item.observation.observation_id)).toEqual([
      pass.observation_id,
      fail.observation_id,
      partial.observation_id,
      onH2.observation_id,
    ]);
    expect(page.items.map((item) => [item.environment, item.head])).toEqual([
      ["ubuntu-latest", H1],
      ["macos-latest", H1],
      ["windows-latest", H1],
      ["wsl2-ubuntu", H2],
    ]);
    const sequences = page.items.map((item) => item.sequence);
    expect(sequences).toEqual([...sequences].sort((a, b) => a - b));
    expect(page.items[0]!.acceptance.accepted_by).toEqual(JSON.parse(credentialPublisher("aaaa")));
  });

  it("ends pages at page_size and terminates", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const seeded = [synthetic(), synthetic(), synthetic(), synthetic(), synthetic()];
    for (const observation of seeded) await seed(repo, [observation], { head: H1 });
    const { ids, pages } = await historyAll(request, repo, { ...BASE, page_size: 2 });
    expect(ids).toEqual(seeded.map((observation) => observation.observation_id));
    expect(pages).toBe(3);
    await expectError(await postJson(request, historyPath(repo), { ...BASE, page_size: 257 }), 400, "invalid_input", "/page_size");
  });

  it("ends a page at the encoded-byte budget before page_size", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const seeded = Array.from({ length: 12 }, () => synthetic({ padding: 100_000 }));
    for (let index = 0; index < seeded.length; index += 4) await seed(repo, seeded.slice(index, index + 4), { head: H1 });
    const first = await postJson(request, historyPath(repo), BASE);
    const text = await first.text();
    expect(new TextEncoder().encode(text).byteLength).toBeLessThanOrEqual(1_048_576);
    const page = JSON.parse(text) as HistoryPage;
    expect(page.items.length).toBeLessThan(12);
    expect(page.items.length).toBeGreaterThan(0);
    expect(page.has_more).toBe(true);
    const { ids } = await historyAll(request, repo, BASE);
    expect(new Set(ids)).toEqual(new Set(seeded.map((observation) => observation.observation_id)));
    expect(ids).toHaveLength(12);
  });

  it("refuses a record that alone exceeds the page byte budget", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    await seed(repo, [synthetic({ padding: 1_100_000 })], { head: H1 });
    await expectError(await postJson(request, historyPath(repo), BASE), 413, "unpageable_record");
  });

  it("refuses more than 256 reachable heads with 413", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const heads = Array.from({ length: 257 }, (_, index) => index.toString(16).padStart(40, "0"));
    await expectError(await postJson(request, historyPath(repo), { ...BASE, reachable_heads: heads }), 413, "size_limit", "/reachable_heads");
  });

  it("refuses cursors from another query, another repository, a tampered cursor, and a cursor with a snapshot", async () => {
    const repo = await newRepository();
    const other = await newRepository();
    const request = harness([repo, other]);
    await seed(repo, [synthetic(), synthetic(), synthetic()], { head: H1 });
    await seed(other, [synthetic(), synthetic(), synthetic()], { head: H1 });
    const { next_cursor: cursor } = await history(request, repo, { ...BASE, page_size: 1 });
    const tampered = cursor!.slice(0, -2) + (cursor!.endsWith("AA") ? "BB" : "AA");
    for (const body of [
      { ...BASE, page_size: 1, package: "biscuit-file", cursor },
      { ...BASE, page_size: 2, cursor },
      { ...BASE, page_size: 1, reachable_heads: [H1], cursor },
      { ...BASE, page_size: 1, cursor: tampered },
    ]) {
      await expectError(await postJson(request, historyPath(repo), body), 400, "invalid_cursor", "/cursor");
    }
    await expectError(await postJson(request, historyPath(other), { ...BASE, page_size: 1, cursor }), 400, "invalid_cursor", "/cursor");
    const pinned = await snapshot(request, repo);
    await expectError(await postJson(request, historyPath(repo), { ...BASE, page_size: 1, cursor, snapshot: pinned }), 400, "invalid_input", "/snapshot");
    // Head order does not change the query.
    const reordered = await history(request, repo, { ...BASE, reachable_heads: [H2, H1], page_size: 1, cursor });
    expect(reordered.items).toHaveLength(1);
  });

  it("refuses a cursor after a restore (400), after pruning (409 snapshot_pruned), and after 24 hours (409 cursor_expired)", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    await seed(repo, [synthetic(), synthetic()], { head: H1 });
    const query = { ...BASE, page_size: 1 };
    const cursor = (await history(request, repo, query)).next_cursor!;

    request.clock += 25 * 3_600_000;
    await expectError(await postJson(request, historyPath(repo), { ...query, cursor }), 409, "cursor_expired", "/cursor");
    request.clock -= 25 * 3_600_000;

    await db().prepare("UPDATE repositories SET retention_epoch = retention_epoch + 1 WHERE repository_id = ?1").bind(repo).run();
    await expectError(await postJson(request, historyPath(repo), { ...query, cursor }), 409, "snapshot_pruned", "/cursor");

    const fresh = (await history(request, repo, query)).next_cursor!;
    await db().prepare("UPDATE repositories SET generation = ?2 WHERE repository_id = ?1").bind(repo, hex(32)).run();
    await expectError(await postJson(request, historyPath(repo), { ...query, cursor: fresh }), 400, "invalid_cursor", "/cursor");
  });

  it("reads a pinned snapshot's bounds and refuses a stale, pruned, foreign, or future one", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const before = synthetic();
    await seed(repo, [before], { head: H1 });
    const pinned = await snapshot(request, repo);
    await seed(repo, [synthetic()], { head: H1 });

    const page = await history(request, repo, { ...BASE, snapshot: pinned });
    expect(page.snapshot).toEqual(pinned);
    expect(page.items.map((item) => item.observation.observation_id)).toEqual([before.observation_id]);

    const query = (snapshot: Snapshot) => postJson(request, historyPath(repo), { ...BASE, snapshot });
    await expectError(await query({ ...pinned, generation: hex(32) }), 409, "stale_snapshot", "/snapshot/generation");
    await expectError(await query({ ...pinned, observation_event_watermark: pinned.observation_event_watermark + 10_000 }), 400, "invalid_input", "/snapshot/observation_event_watermark");
    await expectError(await query({ ...pinned, retention_epoch: 1 }), 400, "invalid_input", "/snapshot/retention_epoch");
    await expectError(await query({ ...pinned, repository_id: "1" }), 400, "invalid_input", "/snapshot/repository_id");
    await db().prepare("UPDATE repositories SET retention_epoch = 1 WHERE repository_id = ?1").bind(repo).run();
    await expectError(await query(pinned), 409, "snapshot_pruned", "/snapshot/retention_epoch");
  });

  it("keeps writes made during a listing out of its later pages, and a fresh query sees them", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const original = [synthetic(), synthetic(), synthetic(), synthetic()];
    for (const observation of original) await seed(repo, [observation], { head: H1 });
    const query = { ...BASE, page_size: 2 };
    let page = await history(request, repo, query);
    const captured = page.snapshot;
    const ids = page.items.map((item) => item.observation.observation_id);

    const concurrent = [synthetic(), synthetic(), synthetic()];
    await seed(repo, concurrent, { head: H1 });
    while (page.next_cursor !== null) {
      page = await history(request, repo, { ...query, cursor: page.next_cursor });
      expect(page.snapshot).toEqual(captured);
      ids.push(...page.items.map((item) => item.observation.observation_id));
    }
    expect(page.has_more).toBe(false);
    expect(ids).toEqual(original.map((observation) => observation.observation_id));

    const fresh = await historyAll(request, repo, query);
    expect(new Set(fresh.ids)).toEqual(new Set([...original, ...concurrent].map((observation) => observation.observation_id)));
  });

  it("drops pruned members from a fresh history", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const seeded = [synthetic(), synthetic(), synthetic()];
    await seed(repo, seeded, { head: H1 });
    const victim = seeded[1]!.observation_id;
    await prune(repo, [victim]);
    const page = await history(request, repo, BASE);
    expect(page.items.map((item) => item.observation.observation_id).sort()).toEqual(
      seeded.map((observation) => observation.observation_id).filter((id) => id !== victim).sort(),
    );
    await expectError(await request(`/v1/repos/${repo}/observations/${victim}`), 404, "not_found");
  });
});
