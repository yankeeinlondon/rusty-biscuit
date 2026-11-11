// Verifier-only location events through `POST /locations`, against an
// authentic fixture ingested through `POST /collections` so the observation
// carries real reports and proof. Every refusal is checked for its code, its
// location, and zero writes.
import { describe, expect, it } from "vitest";
import { canonical, identity, type JsonObject } from "../../src/contracts/index.ts";
import {
  credentialPublisher,
  db,
  expectError,
  harness,
  hex,
  ingestRig,
  newRepository,
  postJson,
  reportEvent,
  type Harness,
  type IngestedFixture,
} from "./helpers.ts";

const locationsPath = (repo: string) => `/v1/repos/${repo}/locations`;

async function setup(): Promise<{ repo: string; request: Harness; fixture: IngestedFixture }> {
  const repo = await newRepository();
  const request = harness([repo]);
  const fixture = await ingestRig(request, repo);
  return { repo, request, fixture };
}

async function stored(repo: string): Promise<{ revision: number; locations: number; events: number }> {
  const row = await db()
    .prepare(
      `SELECT evidence_revision AS revision,
              (SELECT COUNT(*) FROM report_locations WHERE repository_id = ?1) AS locations,
              (SELECT COUNT(*) FROM location_events WHERE repository_id = ?1) AS events
       FROM repositories WHERE repository_id = ?1`,
    )
    .bind(repo)
    .first<{ revision: number; locations: number; events: number }>();
  return row!;
}

/** Sends `body` as `as` and asserts the refusal wrote nothing. */
async function refused(
  context: { repo: string; request: Harness },
  body: unknown,
  status: number,
  code: string,
  location?: string | null,
  as = "verifier",
) {
  const before = await stored(context.repo);
  await expectError(await postJson(context.request, locationsPath(context.repo), body, as), status, code, location);
  expect(await stored(context.repo)).toEqual(before);
}

describe("POST /locations", () => {
  it("records a verified host-dir event with its location key, expiry, and authenticated verifier", async () => {
    const { repo, request, fixture } = await setup();
    const before = await stored(repo);
    const body = reportEvent(fixture);
    request.clock = 1_900_000_000_000;
    const response = await postJson(request, locationsPath(repo), body, "verifier");
    expect(response.status).toBe(201);
    const event = (await response.json()) as Record<string, unknown>;
    const seq = await db()
      .prepare("SELECT seq FROM location_events WHERE repository_id = ?1 AND event_id = ?2")
      .bind(repo, body["event_id"])
      .first<{ seq: number }>();
    expect(event).toEqual({
      event_id: body["event_id"],
      sequence: seq!.seq,
      observation_id: fixture.observationId,
      reference_kind: "report",
      reference_digest: fixture.report["sha256"],
      path: fixture.report["path"],
      location_key: identity(canonical(body["locator"] as JsonObject)),
      locator: body["locator"],
      kind: "verified",
      verification_method: "bytes",
      checked_at_ms: 1_900_000_000_000,
      valid_until_ms: 1_900_000_000_000 + 24 * 3_600_000,
      checked_by: JSON.parse(credentialPublisher("dddd")),
    });
    expect(await stored(repo)).toEqual({ revision: before.revision + 1, locations: 1, events: 1 });
  });

  it("answers an identical replay with the original record and no new write", async () => {
    const { repo, request, fixture } = await setup();
    const body = reportEvent(fixture);
    const first = await postJson(request, locationsPath(repo), body, "verifier");
    expect(first.status).toBe(201);
    const original = await first.json();
    const after = await stored(repo);

    request.clock += 60_000;
    const replay = await postJson(request, locationsPath(repo), body, "verifier-b");
    expect(replay.status).toBe(200);
    expect(await replay.json()).toEqual(original);
    expect(await stored(repo)).toEqual(after);
  });

  it("refuses a different payload under a used event_id with 409 and writes nothing", async () => {
    const { repo, request, fixture } = await setup();
    const body = reportEvent(fixture);
    expect((await postJson(request, locationsPath(repo), body, "verifier")).status).toBe(201);
    await refused({ repo, request }, { ...body, kind: "unavailable", verification_method: undefined }, 409, "identity_conflict", "/event_id");
    await refused({ repo, request }, { ...body, locator: { kind: "host-dir", host: "other", directory: "/x" } }, 409, "identity_conflict", "/event_id");
  });

  it("answers 404 for an observation the repository does not hold", async () => {
    const context = await setup();
    await refused(context, reportEvent(context.fixture, { observation_id: hex(64) }), 404, "not_found");
  });

  it("refuses a report reference the observation does not carry, located at the field that disagrees", async () => {
    const context = await setup();
    await refused(context, reportEvent(context.fixture, { path: "elsewhere.xml" }), 400, "invalid_input", "/path");
    await refused(context, reportEvent(context.fixture, { reference_digest: "0".repeat(64) }), 400, "invalid_input", "/reference_digest");
  });

  it("accepts expected-listing and completion-record references at their proof file names", async () => {
    const context = await setup();
    const proof = context.fixture.observation["proof"] as JsonObject;
    const cases = [
      ["expected-listing", proof["expected_listing_digest"], "biscuit-hash-L1.expected.json"],
      ["completion-record", proof["completion_record_digest"], "biscuit-hash-L1.completion.json"],
    ] as const;
    for (const [kind, digest, path] of cases) {
      const body = reportEvent(context.fixture, { reference_kind: kind, reference_digest: digest, path });
      expect((await postJson(context.request, locationsPath(context.repo), body, "verifier")).status).toBe(201);
      await refused(context, { ...body, event_id: crypto.randomUUID(), path: "biscuit-hash-L2.expected.json" }, 400, "invalid_input", "/path");
      await refused(context, { ...body, event_id: crypto.randomUUID(), reference_digest: "other" }, 400, "invalid_input", "/reference_digest");
    }
    expect((await stored(context.repo)).locations).toBe(2);
  });

  it("requires verification_method exactly on verified events and refuses null", async () => {
    const context = await setup();
    const { verification_method: _, ...unverified } = reportEvent(context.fixture);
    await refused(context, unverified, 400, "invalid_input", "/verification_method");
    await refused(context, reportEvent(context.fixture, { kind: "withdrawn", verification_method: "bytes" }), 400, "invalid_input", "/verification_method");
    await refused(context, reportEvent(context.fixture, { verification_method: null }), 400, "invalid_input", "/verification_method");
    await refused(context, reportEvent(context.fixture, { verification_method: "trust-me" }), 400, "invalid_input", "/verification_method");
  });

  it("refuses an unknown locator kind, an extra locator key, and an artifact of another repository", async () => {
    const context = await setup();
    const artifact = { kind: "artifact", repository_id: context.repo, run_id: "11", artifact_id: "22", member: "x.xml" };
    await refused(context, reportEvent(context.fixture, { locator: { kind: "url", host: "h" } }), 400, "invalid_input", "/locator/kind");
    await refused(context, reportEvent(context.fixture, { locator: { kind: "host-dir", host: "h", directory: "/d", member: "m" } }), 400, "invalid_input", "/locator/member");
    await refused(context, reportEvent(context.fixture, { locator: { kind: "host-dir", host: "h", directory: "/d", url: "u" } }), 400, "invalid_input", "/locator/url");
    await refused(context, reportEvent(context.fixture, { locator: { ...artifact, repository_id: "42" } }), 400, "invalid_input", "/locator/repository_id");
    await refused(context, reportEvent(context.fixture, { locator: { ...artifact, run_id: "0" } }), 400, "invalid_input", "/locator/run_id");
    expect((await postJson(context.request, locationsPath(context.repo), reportEvent(context.fixture, { locator: artifact }), "verifier")).status).toBe(201);
  });

  it("requires an upload bound to the reference for an object locator, and never verifies one", async () => {
    const context = await setup();
    const uploadId = hex(32);
    const object = { kind: "object", upload_id: uploadId };
    await refused(context, reportEvent(context.fixture, { locator: object, kind: "unavailable" }), 404, "not_found", "/locator/upload_id");
    await refused(context, reportEvent(context.fixture, { locator: object }), 400, "invalid_input", "/kind");

    await db()
      .prepare(
        `INSERT INTO uploads(upload_id, repository_id, observation_id, reference_kind, reference_digest, path,
           bytes, part_count, state, staging_key, created_by, created_at_ms, expires_at_ms)
         VALUES (?1, ?2, ?3, 'report', ?4, ?5, 0, 0, 'open', ?6, 'producer', 1, 2)`,
      )
      .bind(uploadId, context.repo, context.fixture.observationId, context.fixture.report["sha256"], context.fixture.report["path"], hex(32))
      .run();
    await refused(context, reportEvent(context.fixture, { locator: object }), 400, "invalid_input", "/kind");
    const response = await postJson(context.request, locationsPath(context.repo), reportEvent(context.fixture, { locator: object, kind: "unavailable" }), "verifier");
    expect(response.status).toBe(201);
  });

  it("refuses producers and readers: no producer can attest a location", async () => {
    const context = await setup();
    await refused(context, reportEvent(context.fixture), 403, "denied", undefined, "producer");
    await refused(context, reportEvent(context.fixture), 403, "denied", undefined, "reader");
  });

  it("advances the revision exactly once per new event", async () => {
    const { repo, request, fixture } = await setup();
    const start = (await stored(repo)).revision;
    const bodies = [reportEvent(fixture), reportEvent(fixture, { kind: "unavailable" }), reportEvent(fixture, { kind: "withdrawn" })];
    for (const body of bodies) {
      expect((await postJson(request, locationsPath(repo), body, "verifier")).status).toBe(201);
      expect((await postJson(request, locationsPath(repo), body, "verifier")).status).toBe(200);
    }
    expect(await stored(repo)).toEqual({ revision: start + 3, locations: 3, events: 3 });
  });
});
