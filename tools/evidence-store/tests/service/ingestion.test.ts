// Ingestion and exact export through the Worker's public `fetch`, against
// local D1 built from `migrations/`. Inputs are the frozen authentic parity
// fixtures with one edit per case; every rejection is checked for its code,
// its location, and zero writes.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import type { Principal } from "../../src/auth.ts";
import type { Role } from "../../src/contracts/api.ts";
import {
  canonical,
  identity,
  jsonNumber,
  loadStrict,
  observationIdentity,
  type JsonObject,
  type JsonValue,
} from "../../src/contracts/index.ts";
import { readObservation } from "../../src/db/ingest.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import emptyFixture from "../fixtures/parity/collection-empty.json?raw";
import hostedFailFixture from "../fixtures/parity/collection-hosted-fail.json?raw";
import retryChainFixture from "../fixtures/parity/collection-retry-chain.json?raw";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";
import seedFixture from "../fixtures/parity/collection-seed.json?raw";
import adaptedV2Fixture from "../fixtures/parity/adapter-hook-v2.canonical?raw";
import { db, newRepository, repositoryState, retentionJob } from "./helpers.ts";

const PRODUCER = "producer";
const READER = "reader";
const OTHER_PRODUCER = "producer-b";

function principal(id: string, repositories: readonly string[], roles: readonly Role[]): Principal {
  return {
    publisher: { kind: "credential", credential_id: id.padEnd(32, "0").slice(0, 32) },
    grants: new Map(repositories.map((repo) => [repo, new Set(roles)])),
  };
}

let clock = 1_800_000_000_000;

function workerFor(repositories: readonly string[], storage: Env = env as unknown as Env) {
  const principals: Record<string, Principal> = {
    [PRODUCER]: principal("aaaa", repositories, ["producer"]),
    [OTHER_PRODUCER]: principal("bbbb", repositories, ["producer"]),
    [READER]: principal("cccc", repositories, ["reader"]),
  };
  const worker = createWorker({
    authenticate: async (request) => {
      const who = principals[request.headers.get("x-test-principal") ?? ""];
      if (who === undefined) throw new Error("test request without a principal");
      return who;
    },
    now: () => clock,
  });
  return (path: string, init: RequestInit & { as?: string } = {}) =>
    worker.fetch!(
      new Request(`https://evidence.test${path}`, {
        ...init,
        headers: { "x-test-principal": init.as ?? PRODUCER, ...(init.headers as Record<string, string>) },
      }) as never,
      storage,
      {} as ExecutionContext,
    );
}

const encoder = new TextEncoder();

function load(text: string): JsonObject {
  const loaded = loadStrict(encoder.encode(text));
  if (!loaded.ok) throw new Error(JSON.stringify(loaded.problems));
  return loaded.value as JsonObject;
}

/** Canonical submission text: number lexemes in the fixture survive verbatim. */
function submission(collection: JsonValue, sourceVersion = 3): string {
  return `{"source_version":${sourceVersion},"collection":${canonical(collection)}}`;
}

function observations(collection: JsonObject): JsonObject[] {
  return collection["observations"] as JsonObject[];
}

function ids(collection: JsonObject): string[] {
  return observations(collection).map((o) => o["observation_id"] as string);
}

/** Re-identifies an observation after an edit so it stays contract-valid. */
function reidentify(observation: JsonObject): JsonObject {
  observation["observation_id"] = observationIdentity(observation);
  return observation;
}

function sortedCollection(base: JsonObject, members: JsonObject[], context: Partial<Record<"head" | "tree", string>> = {}) {
  const sorted = [...members].sort((a, b) => ((a["observation_id"] as string) < (b["observation_id"] as string) ? -1 : 1));
  return { ...base, ...context, observations: sorted } as JsonObject;
}

async function post(request: ReturnType<typeof workerFor>, repo: string, body: string, as = PRODUCER) {
  return request(`/v1/repos/${repo}/collections`, { method: "POST", body, as });
}

interface ErrorEnvelope {
  error: { code: string; location: string | null; retryable: boolean; message: string; retry_after_ms: number | null };
  request_id: string;
}

async function expectError(response: Response, status: number, code: string, location?: string | null) {
  expect(response.status).toBe(status);
  const body = (await response.json()) as ErrorEnvelope;
  expect(body.error.code).toBe(code);
  if (location !== undefined) expect(body.error.location).toBe(location);
  expect(body.request_id).toBe(response.headers.get("x-request-id"));
  return body;
}

/** Follows every page and rebuilds the collection exactly as the CLI will. */
async function exportAll(request: ReturnType<typeof workerFor>, repo: string, collectionId: string, pageSize?: number) {
  const items: JsonValue[] = [];
  let cursor: string | null = null;
  let header: JsonObject | undefined;
  let pages = 0;
  do {
    const query = new URLSearchParams();
    if (pageSize !== undefined) query.set("page_size", String(pageSize));
    if (cursor !== null) query.set("cursor", cursor);
    const response = await request(`/v1/repos/${repo}/collections/${collectionId}?${query}`, { as: READER });
    expect(response.status).toBe(200);
    const page = load(await response.text());
    header ??= page;
    items.push(...(page["items"] as JsonValue[]));
    cursor = page["next_cursor"] as string | null;
    expect(page["has_more"]).toBe(cursor !== null);
    pages += 1;
  } while (cursor !== null);
  const rebuilt: JsonObject = {
    schema_version: jsonNumber("3"),
    environment: header!["environment"]!,
    head: header!["head"]!,
    tree: header!["tree"]!,
    observations: items,
  };
  return { header: header!, rebuilt, pages };
}

describe("POST /collections", () => {
  it("accepts an authentic collection and exports it byte-exactly (write/read/write/read)", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const collection = load(rigFixture);
    const response = await post(request, repo, submission(collection));
    expect(response.status).toBe(200);
    const result = (await response.json()) as { collection_id: string; accepted: string[]; already_present: string[]; evidence_revision: number };
    expect(result).toEqual({
      collection_id: identity(canonical(collection)),
      accepted: ids(collection),
      already_present: [],
      evidence_revision: 1,
    });

    const first = await exportAll(request, repo, result.collection_id);
    expect(identity(canonical(first.rebuilt))).toBe(result.collection_id);
    expect(first.header["acceptance"]).toEqual({
      source_version: jsonNumber("3"),
      accepted_at_ms: jsonNumber(String(clock)),
      accepted_by: { kind: "credential", credential_id: "aaaa".padEnd(32, "0") },
    });

    // The exported collection is a valid submission of itself: idempotent.
    const replay = await post(request, repo, submission(first.rebuilt));
    expect(await replay.json()).toEqual({ ...result, accepted: [], already_present: ids(collection) });
    const second = await exportAll(request, repo, result.collection_id);
    expect(canonical(second.rebuilt)).toBe(canonical(first.rebuilt));
    expect((await repositoryState(repo)).events).toHaveLength(1);
  });

  it("accepts a collection adapted from a version-2 receipt and records its source version", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const collection = load(adaptedV2Fixture);
    const response = await post(request, repo, submission(collection, 2));
    expect(response.status).toBe(200);
    const result = (await response.json()) as { collection_id: string; accepted: string[] };
    expect(result.collection_id).toBe(identity(adaptedV2Fixture));
    expect(result.accepted).toEqual(ids(collection));
    const exported = await exportAll(request, repo, result.collection_id);
    expect((exported.header["acceptance"] as JsonObject)["source_version"]).toEqual(jsonNumber("2"));
  });

  it("succeeds on an empty collection without a revision step, and refuses an empty version-2 adaptation", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const collection = load(emptyFixture);
    const response = await post(request, repo, submission(collection));
    expect(await response.json()).toEqual({
      collection_id: identity(canonical(collection)),
      accepted: [],
      already_present: [],
      evidence_revision: 0,
    });
    const exported = await exportAll(request, repo, identity(canonical(collection)));
    expect(exported.rebuilt["observations"]).toEqual([]);
    expect(identity(canonical(exported.rebuilt))).toBe(identity(canonical(collection)));

    const other = await newRepository();
    const refused = await post(workerFor([other]), other, submission(collection, 2));
    await expectError(refused, 400, "invalid_input", "/collection/observations");
    const state = await repositoryState(other);
    expect(state.collections).toEqual([]);
    expect(state.revision).toBe(0);
  });

  it("answers 404 for an unknown repository and writes nothing", async () => {
    const repo = "9" + String(Date.now()).slice(-12);
    const request = workerFor([repo]);
    await expectError(await post(request, repo, submission(load(rigFixture))), 404, "not_found");
    await expectError(await request(`/v1/repos/${repo}/collections/${"a".repeat(64)}`, { as: READER }), 404, "not_found");
    const rows = await db()
      .prepare("SELECT (SELECT COUNT(*) FROM observations WHERE repository_id = ?1) + (SELECT COUNT(*) FROM collections WHERE repository_id = ?1) AS n")
      .bind(repo)
      .first<{ n: number }>();
    expect(rows?.n).toBe(0);
  });

  it("rolls back the whole collection when one member conflicts on context", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const rig = load(rigFixture);
    await post(request, repo, submission(rig));
    const before = await repositoryState(repo);

    // Same observation ID under a different head, plus a fresh sibling.
    const sibling = observations(load(seedFixture))[0]!;
    const conflicting = sortedCollection(rig, [observations(rig)[0]!, sibling], { head: "d".repeat(40) });
    const response = await post(request, repo, submission(conflicting));
    await expectError(response, 409, "identity_conflict", "/collection/observations");
    expect(await repositoryState(repo)).toEqual(before);
  });

  it("serves a lost-response retry and a concurrent duplicate as one acceptance", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const chain = load(retryChainFixture);
    const body = submission(chain);
    const [a, b] = await Promise.all([post(request, repo, body), post(request, repo, body)]);
    const results = [(await a.json()) as { accepted: string[]; already_present: string[] }, (await b.json()) as { accepted: string[]; already_present: string[] }];
    expect(results.map((r) => r.accepted.length + r.already_present.length)).toEqual([3, 3]);
    expect(results.flatMap((r) => r.accepted).sort()).toEqual(ids(chain));

    const retry = await post(request, repo, body);
    expect(await retry.json()).toMatchObject({ accepted: [], already_present: ids(chain), evidence_revision: 3 });
    const state = await repositoryState(repo);
    expect(state.events).toHaveLength(3);
    expect(state.collections).toHaveLength(1);
  });

  it("lets exactly one of two concurrent conflicting collections commit, with none of the loser's siblings", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const rig = load(rigFixture);
    const shared = observations(rig)[0]!;
    const siblingA = observations(load(seedFixture))[0]!;
    const siblingB = observations(load(hostedFailFixture))[0]!;
    const a = sortedCollection(rig, [shared, siblingA], { head: "1".repeat(40) });
    const b = sortedCollection(rig, [shared, siblingB], { head: "2".repeat(40) });
    const responses = await Promise.all([post(request, repo, submission(a)), post(request, repo, submission(b))]);
    const statuses = responses.map((r) => r.status).sort();
    expect(statuses).toEqual([200, 409]);
    const winner = responses[0]!.status === 200 ? a : b;
    const loserSibling = winner === a ? siblingB : siblingA;
    const state = await repositoryState(repo);
    expect(state.observations).toEqual(ids(winner));
    expect(state.observations).not.toContain(loserSibling["observation_id"]);
    expect(state.collections).toEqual([identity(canonical(winner))]);
    expect(state.revision).toBe(2);
  });

  it("keeps the first publisher, timestamp, and source version on a replay through another adapter", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const rig = load(rigFixture);
    const firstAt = clock;
    await post(request, repo, submission(rig, 3), PRODUCER);
    clock += 60_000;
    const replay = await post(request, repo, submission(rig, 2), OTHER_PRODUCER);
    expect(await replay.json()).toMatchObject({ accepted: [], already_present: ids(rig), evidence_revision: 1 });
    const row = await readObservation(db(), repo, ids(rig)[0]!);
    expect(row).toMatchObject({
      accepted_principal: JSON.stringify({ kind: "credential", credential_id: "aaaa".padEnd(32, "0") }),
      received_at_ms: firstAt,
      source_version: 3,
    });
    const exported = await exportAll(request, repo, identity(canonical(rig)));
    expect((exported.header["acceptance"] as JsonObject)["accepted_at_ms"]).toEqual(jsonNumber(String(firstAt)));
  });
});

describe("POST /collections rejects before any write", () => {
  async function refused(body: string | Uint8Array, status: number, code: string, location: string) {
    const repo = await newRepository();
    const response = await post(workerFor([repo]), repo, body as string);
    await expectError(response, status, code, location);
    const state = await repositoryState(repo);
    expect(state).toMatchObject({ revision: 0, observations: [], collections: [], members: 0, events: [] });
  }

  it("refuses a contract violation with its JSON pointer (observation_id mismatch)", async () => {
    const rig = load(rigFixture);
    observations(rig)[0]!["observation_id"] = "0".repeat(64);
    await refused(submission(rig), 400, "invalid_input", "/collection/observations/0/observation_id");
  });

  it("refuses loader failures through the shared strict loader", async () => {
    await refused(`{"source_version":3,"source_version":3,"collection":${canonical(load(emptyFixture))}}`, 400, "invalid_input", "/source_version");
    await refused(`${submission(load(emptyFixture))} {}`, 400, "invalid_input", "");
    await refused(new Uint8Array([0xef, 0xbb, 0xbf, ...encoder.encode(submission(load(emptyFixture)))]), 400, "invalid_input", "");
  });

  it("refuses submission envelope shapes: unknown key, missing collection, non-integer version", async () => {
    const empty = canonical(load(emptyFixture));
    await refused(`{"source_version":3,"collection":${empty},"extra":1}`, 400, "invalid_input", "/extra");
    await refused(`{"source_version":3}`, 400, "invalid_input", "");
    await refused(`{"source_version":3.0,"collection":${empty}}`, 400, "invalid_input", "/source_version");
    await refused(`{"source_version":true,"collection":${empty}}`, 400, "invalid_input", "/source_version");
    await refused(`{"source_version":1,"collection":${empty}}`, 400, "invalid_input", "/source_version");
    await refused(`{"source_version":null,"collection":${empty}}`, 400, "invalid_input", "/source_version");
    await refused(`[]`, 400, "invalid_input", "");
  });

  it("refuses a body over 1 MiB with 413", async () => {
    const body = `{"source_version":3,"collection":${canonical(load(emptyFixture))},"pad":"${"x".repeat(1024 * 1024)}"}`;
    await refused(body, 413, "size_limit", "");
  });

  it("refuses more than 256 observations with 413 before validating them", async () => {
    const base = load(emptyFixture);
    const many = `{"source_version":3,"collection":${canonical(base).replace('"observations":[]', `"observations":[${Array(257).fill("{}").join(",")}]`)}}`;
    await refused(many, 413, "size_limit", "/collection/observations");
  });

  it("refuses an observation whose canonical form exceeds 128 KiB with 413", async () => {
    const collection = load(hostedFailFixture);
    const failing = observations(collection)[0]!;
    const cell = failing["cell"] as JsonObject;
    cell["failed_tests"] = Array.from({ length: 20 }, (_, i) => `t${i}::${"x".repeat(7_000)}`);
    reidentify(failing);
    await refused(submission(collection), 413, "size_limit", "/collection/observations/0");
  });
});

describe("GET /collections/{id}", () => {
  async function chainRepository() {
    const repo = await newRepository();
    const request = workerFor([repo]);
    const chain = load(retryChainFixture);
    await post(request, repo, submission(chain));
    return { repo, request, chain, collectionId: identity(canonical(chain)) };
  }

  it("pages a collection one member at a time and terminates at its size", async () => {
    const { repo, request, chain, collectionId } = await chainRepository();
    const all = await exportAll(request, repo, collectionId, 1);
    expect(all.pages).toBe(3);
    expect(identity(canonical(all.rebuilt))).toBe(collectionId);
    expect(canonical(all.rebuilt)).toBe(canonical(chain));
  });

  async function firstCursor(request: ReturnType<typeof workerFor>, repo: string, collectionId: string) {
    const response = await request(`/v1/repos/${repo}/collections/${collectionId}?page_size=1`, { as: READER });
    return load(await response.text())["next_cursor"] as string;
  }

  it("refuses tampered, foreign-query, and foreign-repository cursors", async () => {
    const { repo, request, collectionId } = await chainRepository();
    const cursor = await firstCursor(request, repo, collectionId);
    const tampered = cursor.slice(0, -2) + (cursor.endsWith("AA") ? "BB" : "AA");
    const path = `/v1/repos/${repo}/collections/${collectionId}`;
    await expectError(await request(`${path}?page_size=1&cursor=${tampered}`, { as: READER }), 400, "invalid_cursor", "/query/cursor");
    await expectError(await request(`${path}?page_size=2&cursor=${cursor}`, { as: READER }), 400, "invalid_cursor", "/query/cursor");
    await expectError(await request(`${path}?page_size=1&cursor=not-a-cursor`, { as: READER }), 400, "invalid_cursor", "/query/cursor");

    const other = await chainRepository();
    const otherRequest = workerFor([repo, other.repo]);
    await expectError(
      await otherRequest(`/v1/repos/${other.repo}/collections/${collectionId}?page_size=1&cursor=${cursor}`, { as: READER }),
      400,
      "invalid_cursor",
    );
  });

  it("refuses expired cursors, cursors from before pruning, and cursors from before a restore", async () => {
    const { repo, request, collectionId } = await chainRepository();
    const path = `/v1/repos/${repo}/collections/${collectionId}?page_size=1&cursor=`;
    const cursor = await firstCursor(request, repo, collectionId);
    clock += 25 * 3_600_000;
    await expectError(await request(path + cursor, { as: READER }), 409, "cursor_expired");
    clock -= 25 * 3_600_000;

    await db().prepare("UPDATE repositories SET retention_epoch = retention_epoch + 1 WHERE repository_id = ?1").bind(repo).run();
    await expectError(await request(path + cursor, { as: READER }), 409, "snapshot_pruned");

    const fresh = await firstCursor(request, repo, collectionId);
    await db().prepare("UPDATE repositories SET generation = ?2 WHERE repository_id = ?1").bind(repo, "f".repeat(32)).run();
    await expectError(await request(path + fresh, { as: READER }), 400, "invalid_cursor");
  });

  it("validates page_size", async () => {
    const { repo, request, collectionId } = await chainRepository();
    for (const size of ["0", "257", "-1", "1.5", "abc", ""]) {
      await expectError(
        await request(`/v1/repos/${repo}/collections/${collectionId}?page_size=${size}`, { as: READER }),
        400,
        "invalid_input",
        "/query/page_size",
      );
    }
  });

  it("answers 410 collection_pruned rather than a partial collection once a member is pruned", async () => {
    const { repo, request, chain, collectionId } = await chainRepository();
    const victim = ids(chain)[1]!;
    await retentionJob(repo, "deleting-rows", [victim]);
    await db().prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id = ?2").bind(repo, victim).run();
    await expectError(await request(`/v1/repos/${repo}/collections/${collectionId}`, { as: READER }), 410, "collection_pruned");
    await expectError(await request(`/v1/repos/${repo}/collections/${collectionId}?page_size=1`, { as: READER }), 410, "collection_pruned");
  });

  it("answers 410 on a later page when a member of an earlier page was pruned", async () => {
    const { repo, request, chain, collectionId } = await chainRepository();
    const cursor = await firstCursor(request, repo, collectionId);
    const victim = ids(chain)[0]!;
    await retentionJob(repo, "deleting-rows", [victim]);
    await db().prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id = ?2").bind(repo, victim).run();
    await expectError(
      await request(`/v1/repos/${repo}/collections/${collectionId}?page_size=1&cursor=${cursor}`, { as: READER }),
      410,
      "collection_pruned",
    );
  });

  it("answers 404 for a collection that was never stored", async () => {
    const { repo, request } = await chainRepository();
    await expectError(await request(`/v1/repos/${repo}/collections/${"e".repeat(64)}`, { as: READER }), 404, "not_found");
    await expectError(await request(`/v1/repos/${repo}/collections/not-hex`, { as: READER }), 404, "not_found");
  });
});

describe("routing, authorization, and failure envelopes", () => {
  it("refuses roles and repositories the principal does not hold", async () => {
    const repo = await newRepository();
    const other = await newRepository();
    const request = workerFor([repo]);
    const body = submission(load(rigFixture));
    await expectError(await post(request, repo, body, READER), 403, "denied");
    await expectError(await post(request, other, body), 403, "denied");
    await expectError(await request(`/v1/repos/${repo}/collections/${"a".repeat(64)}`), 403, "denied");
    expect((await repositoryState(repo)).observations).toEqual([]);
  });

  it("refuses an unauthenticated request in the deployed build", async () => {
    const repo = await newRepository();
    const worker = (await import("../../src/worker.ts")).default;
    const response = await worker.fetch!(
      new Request(`https://evidence.test/v1/repos/${repo}/collections`, { method: "POST", body: "{}" }) as never,
      env as unknown as Env,
      {} as ExecutionContext,
    );
    await expectError(response, 401, "unauthenticated");
  });

  it("answers unsupported_api_version and not_found for unknown versions and routes", async () => {
    const repo = await newRepository();
    const request = workerFor([repo]);
    await expectError(await request(`/v2/repos/${repo}/collections`, { method: "POST" }), 400, "unsupported_api_version");
    await expectError(await request(`/v1/repos/${repo}/nothing`), 404, "not_found");
    await expectError(await request(`/v1/repos/0123/collections`), 404, "not_found");
  });

  it("maps a storage failure to a retryable 503 that leaks no detail", async () => {
    const repo = await newRepository();
    const failing = new Proxy(env.EVIDENCE_DB, {
      get(target, property) {
        if (property === "batch") return async () => { throw new Error("D1_ERROR: secret internal detail"); };
        const value = Reflect.get(target, property) as unknown;
        return typeof value === "function" ? value.bind(target) : value;
      },
    });
    const request = workerFor([repo], { ...(env as unknown as Env), EVIDENCE_DB: failing });
    const response = await post(request, repo, submission(load(rigFixture)));
    const body = await expectError(response, 503, "unavailable", null);
    expect(body.error.retryable).toBe(true);
    expect(body.error.retry_after_ms).toBe(1_000);
    expect(body.error.message).not.toContain("secret");
  });
});
