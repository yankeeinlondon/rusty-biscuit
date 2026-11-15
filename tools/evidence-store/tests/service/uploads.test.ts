// Uploads through `POST /uploads`, `PUT …/parts/{index}`, and
// `POST …/finalize`, against observations ingested through
// `POST /collections`. Every refusal is checked for its public answer and for
// what D1 and R2 hold afterwards.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import type { RequestContext } from "../../src/ingestion.ts";
import { cleanupStaging, finalizeEventId, finalizeUpload } from "../../src/uploads.ts";
import hostedFixture from "../fixtures/parity/collection-hosted-pass.json?raw";
import {
  db,
  expectError,
  finalize,
  harness,
  hex,
  ingestEdited,
  ingestProof,
  ingestReport,
  ingestRig,
  locationState,
  newRepository,
  PART,
  payload,
  postJson,
  principal,
  putPart,
  r2Keys,
  reportUpload,
  sha256Hex,
  stage,
  uploadRow,
  type Harness,
  type IngestedFixture,
} from "./helpers.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";

const uploadsPath = (repo: string) => `/v1/repos/${repo}/uploads`;
const encoder = new TextEncoder();

async function setup(content: Uint8Array = payload(300)): Promise<{ repo: string; request: Harness; fixture: IngestedFixture }> {
  const repo = await newRepository();
  const request = harness([repo]);
  const fixture = await ingestReport(request, repo, content);
  return { repo, request, fixture };
}

async function uploadCount(repo: string): Promise<number> {
  return (await db().prepare("SELECT COUNT(*) AS n FROM uploads WHERE repository_id = ?1").bind(repo).first<{ n: number }>())!.n;
}

async function partCount(uploadId: string): Promise<number> {
  return (await db().prepare("SELECT COUNT(*) AS n FROM upload_parts WHERE upload_id = ?1").bind(uploadId).first<{ n: number }>())!.n;
}

/** The observation and its collection export, as a reader sees them. */
async function published(request: Harness, repo: string, fixture: IngestedFixture): Promise<[string, string]> {
  const observation = await request(`/v1/repos/${repo}/observations/${fixture.observationId}`);
  const exported = await request(`/v1/repos/${repo}/collections/${fixture.collectionId}`);
  expect([observation.status, exported.status]).toEqual([200, 200]);
  return [await observation.text(), await exported.text()];
}

const RIG = { package: "biscuit-hash", environment: "macos-latest", gate: "L1", head: "b".repeat(40), target: "aarch64-apple-darwin" };

function completionRecord(overrides: Record<string, unknown> = {}): Uint8Array {
  return encoder.encode(
    JSON.stringify({
      schema_version: 1,
      package: RIG.package,
      environment: RIG.environment,
      gate: RIG.gate,
      complete: true,
      head: RIG.head,
      run: "run7",
      attempt: 1,
      nextest_version: "0.9.99",
      reports: ["biscuit-hash-L1.xml"],
      ...overrides,
    }),
  );
}

function expectedManifest(overrides: Record<string, unknown> = {}): Uint8Array {
  return encoder.encode(
    JSON.stringify({
      schema_version: 2,
      environment: RIG.environment,
      tier: RIG.gate,
      target: RIG.target,
      nextest_version: "0.9.99",
      from_archive: false,
      selection: { filter: "not test(/^level2_/)", profile: "ci", test_args: "" },
      packages: { [RIG.package]: { tests: ["biscuit_hash::hashes"], ignored: [], excluded: [] } },
      ...overrides,
    }),
  );
}

function proofUpload(fixture: IngestedFixture, kind: "expected-listing" | "completion-record", bytes: Uint8Array): Record<string, unknown> {
  const proof = fixture.observation["proof"] as Record<string, string>;
  const cell = fixture.observation["cell"] as Record<string, string>;
  return {
    observation_id: fixture.observationId,
    reference_kind: kind,
    reference_digest: kind === "expected-listing" ? proof["expected_listing_digest"] : proof["completion_record_digest"],
    bytes: bytes.byteLength,
    path: `${cell["package"]}-${cell["gate"]}.${kind === "expected-listing" ? "expected" : "completion"}.json`,
  };
}

describe("POST /uploads", () => {
  it("opens an upload (201) and answers a repeat, by producer or verifier, with the same upload (200)", async () => {
    const { repo, request, fixture } = await setup();
    request.clock = 1_900_000_000_000;
    const first = await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer");
    expect(first.status).toBe(201);
    const upload = (await first.json()) as Record<string, unknown>;
    expect(upload).toEqual({
      upload_id: expect.stringMatching(/^[0-9a-f]{32}$/),
      observation_id: fixture.observationId,
      reference_kind: "report",
      reference_digest: fixture.report["sha256"],
      path: fixture.report["path"],
      bytes: 300,
      part_bytes: PART,
      part_count: 1,
      parts_received: 0,
      state: "open",
      expires_at_ms: 1_900_000_000_000 + 24 * 3_600_000,
    });
    const row = await uploadRow(upload["upload_id"] as string);
    expect(row?.staging_key).toBe(
      `objects/${repo}/${fixture.observationId}/report/${await sha256Hex(encoder.encode(fixture.report["sha256"] as string))}/${upload["upload_id"]}`,
    );

    const replay = await postJson(request, uploadsPath(repo), reportUpload(fixture), "verifier");
    expect(replay.status).toBe(200);
    expect(await replay.json()).toEqual(upload);
    expect(await uploadCount(repo)).toBe(1);
  });

  it("refuses a repeat with a different length as an identity conflict and writes nothing", async () => {
    // A report pins its length, so only a proof reference can be asked for with another one.
    const repo = await newRepository();
    const request = harness([repo]);
    const proofFixture = await ingestProof(request, repo, { expected: expectedManifest(), completion: completionRecord() });
    const body = proofUpload(proofFixture, "expected-listing", expectedManifest());
    expect((await postJson(request, uploadsPath(repo), body, "producer")).status).toBe(201);
    await expectError(await postJson(request, uploadsPath(repo), { ...body, bytes: 5 }, "producer"), 409, "identity_conflict", "/bytes");
    expect(await uploadCount(repo)).toBe(1);
  });

  it("answers 404 for an unknown observation and 400 for a reference the observation does not carry", async () => {
    const { repo, request, fixture } = await setup();
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, { observation_id: hex(64) }), "producer"), 404, "not_found", "/observation_id");
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, { reference_digest: "0".repeat(64) }), "producer"), 400, "invalid_input", "/reference_digest");
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, { path: "other.xml" }), "producer"), 400, "invalid_input", "/path");
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, { bytes: 299 }), "producer"), 400, "invalid_input", "/bytes");
    expect(await uploadCount(repo)).toBe(0);
  });

  it("refuses a proof upload for an observation that records no proof", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const fixture = await ingestEdited(request, repo, rigFixture, (observation) => {
      // A passing rig observation must name a proof; a hook observation need not.
      (observation["producer"] as Record<string, unknown>)["kind"] = "hook";
      delete observation["proof"];
    });
    const body = { observation_id: fixture.observationId, reference_kind: "expected-listing", reference_digest: "0".repeat(64), bytes: 10, path: "biscuit-hash-L1.expected.json" };
    await expectError(await postJson(request, uploadsPath(repo), body, "producer"), 400, "invalid_input", "/reference_digest");
    expect(await uploadCount(repo)).toBe(0);
  });

  it("answers 413 above 64 MiB while the observation stays recorded and exported", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const fixture = await ingestRig(request, repo);
    const before = await published(request, repo, fixture);
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, { bytes: 64 * PART + 1 }), "producer"), 413, "size_limit", "/bytes");
    expect(await uploadCount(repo)).toBe(0);
    expect(await published(request, repo, fixture)).toEqual(before);
  });

  it("accepts a zero-byte upload only for a referenced empty file", async () => {
    const empty = await setup(new Uint8Array());
    const response = await postJson(empty.request, uploadsPath(empty.repo), reportUpload(empty.fixture), "producer");
    expect(response.status).toBe(201);
    const upload = (await response.json()) as { upload_id: string; part_count: number };
    expect(upload.part_count).toBe(0);
    const finalized = await finalize(empty.request, empty.repo, upload.upload_id);
    expect(finalized.status).toBe(200);
    expect(((await finalized.json()) as { sha256: string }).sha256).toBe(await sha256Hex(new Uint8Array()));

    const nonEmpty = await setup(payload(10));
    await expectError(await postJson(nonEmpty.request, uploadsPath(nonEmpty.repo), reportUpload(nonEmpty.fixture, { bytes: 0 }), "producer"), 400, "invalid_input", "/bytes");
  });

  it("refuses a reader", async () => {
    const { repo, request, fixture } = await setup();
    await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture), "reader"), 403, "denied");
    expect(await uploadCount(repo)).toBe(0);
  });

  it("wires each field through the shared readers (one invalid shape per field)", async () => {
    const { repo, request, fixture } = await setup();
    const cases: [Record<string, unknown>, string][] = [
      [{ observation_id: "XYZ" }, "/observation_id"],
      [{ reference_kind: "log" }, "/reference_kind"],
      [{ reference_digest: "" }, "/reference_digest"],
      [{ bytes: "300" }, "/bytes"],
      [{ path: 7 }, "/path"],
      [{ extra: true }, "/extra"],
    ];
    for (const [override, location] of cases) {
      await expectError(await postJson(request, uploadsPath(repo), reportUpload(fixture, override), "producer"), 400, "invalid_input", location);
    }
    const { bytes: _, ...missing } = reportUpload(fixture);
    await expectError(await postJson(request, uploadsPath(repo), missing, "producer"), 400, "invalid_input", "/bytes");
    expect(await uploadCount(repo)).toBe(0);
    expect((await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer")).status).toBe(201);
  });
});

describe("PUT /uploads/{upload_id}/parts/{index}", () => {
  async function open(content: Uint8Array) {
    const context = await setup(content);
    const response = await postJson(context.request, uploadsPath(context.repo), reportUpload(context.fixture), "producer");
    const { upload_id: uploadId } = (await response.json()) as { upload_id: string };
    return { ...context, uploadId };
  }

  it("accepts sequential parts of exactly 1 MiB and the remainder", async () => {
    const content = payload(2 * PART + 100);
    const { repo, request, uploadId } = await open(content);
    for (const [index, slice] of [content.slice(0, PART), content.slice(PART, 2 * PART), content.slice(2 * PART)].entries()) {
      const response = await putPart(request, repo, uploadId, index, slice);
      expect(response.status).toBe(200);
      expect(await response.json()).toEqual({ upload_id: uploadId, index, bytes: slice.byteLength, parts_received: index + 1 });
    }
    expect(await r2Keys(`staging/${uploadId}/`)).toHaveLength(3);
    expect(await partCount(uploadId)).toBe(3);
  });

  it("refuses an index gap, a wrong part size, and a part beyond the last", async () => {
    const content = payload(PART + 10);
    const { repo, request, uploadId } = await open(content);
    await expectError(await putPart(request, repo, uploadId, 1, content.slice(PART)), 400, "invalid_input");
    await expectError(await putPart(request, repo, uploadId, 0, content.slice(0, 100)), 400, "invalid_input");
    expect(await partCount(uploadId)).toBe(0);
    expect(await r2Keys(`staging/${uploadId}/`)).toEqual([]);
    expect((await putPart(request, repo, uploadId, 0, content.slice(0, PART))).status).toBe(200);
    await expectError(await putPart(request, repo, uploadId, 1, content.slice(PART, PART + 9)), 400, "invalid_input");
    expect((await putPart(request, repo, uploadId, 1, content.slice(PART))).status).toBe(200);
    await expectError(await putPart(request, repo, uploadId, 2, content.slice(PART)), 400, "invalid_input");
    expect(await partCount(uploadId)).toBe(2);
  });

  it("answers an identical replay with 200 and a different one with 409, changing nothing", async () => {
    const content = payload(500);
    const { repo, request, uploadId } = await open(content);
    expect((await putPart(request, repo, uploadId, 0, content)).status).toBe(200);
    const keys = await r2Keys(`staging/${uploadId}/`);
    const replay = await putPart(request, repo, uploadId, 0, content, "verifier");
    expect(replay.status).toBe(200);
    expect(await replay.json()).toEqual({ upload_id: uploadId, index: 0, bytes: 500, parts_received: 1 });
    await expectError(await putPart(request, repo, uploadId, 0, payload(500, 9)), 409, "identity_conflict");
    expect(await r2Keys(`staging/${uploadId}/`)).toEqual(keys);
    expect(await partCount(uploadId)).toBe(1);
  });

  it("leaves the observation, its export, and its locations unchanged when an upload is interrupted", async () => {
    const content = payload(2 * PART + 1);
    const { repo, request, fixture, uploadId } = await open(content);
    const before = await published(request, repo, fixture);
    const locations = await locationState(repo);
    expect((await putPart(request, repo, uploadId, 0, content.slice(0, PART))).status).toBe(200);
    expect(await published(request, repo, fixture)).toEqual(before);
    expect(await locationState(repo)).toEqual(locations);
    await expectError(await finalize(request, repo, uploadId), 400, "invalid_input");
    expect((await uploadRow(uploadId))?.state).toBe("open");
  });

  it("answers 404 and marks the upload expired after 24 hours", async () => {
    const content = payload(20);
    const { repo, request, uploadId } = await open(content);
    request.clock += 24 * 3_600_000;
    await expectError(await putPart(request, repo, uploadId, 0, content), 404, "not_found");
    expect((await uploadRow(uploadId))?.state).toBe("expired");
    expect(await r2Keys(`staging/${uploadId}/`)).toEqual([]);
  });

  it("answers 413 for a part above 1 MiB and 404 for a malformed upload ID", async () => {
    const { repo, request, uploadId } = await open(payload(20));
    await expectError(await putPart(request, repo, uploadId, 0, payload(PART + 1)), 413, "size_limit");
    await expectError(await putPart(request, repo, "XYZ", 0, payload(20)), 404, "not_found");
    await expectError(await putPart(request, repo, hex(32), 0, payload(20)), 404, "not_found");
  });
});

describe("POST /uploads/{upload_id}/finalize", () => {
  it("verifies the bytes and appends exactly one verified object location", async () => {
    const content = payload(2 * PART + 7);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    const before = await locationState(repo);
    const now = (request.clock += 60_000);
    const response = await finalize(request, repo, uploadId);
    expect(response.status).toBe(200);
    const result = (await response.json()) as Record<string, Record<string, unknown>>;
    expect(result["sha256"]).toBe(fixture.report["sha256"]);
    expect(result["upload"]).toMatchObject({ upload_id: uploadId, state: "finalized", parts_received: 3, part_count: 3 });
    expect(result["location"]).toMatchObject({
      event_id: finalizeEventId(uploadId),
      observation_id: fixture.observationId,
      reference_kind: "report",
      reference_digest: fixture.report["sha256"],
      path: fixture.report["path"],
      locator: { kind: "object", upload_id: uploadId },
      kind: "verified",
      verification_method: "bytes",
      checked_at_ms: now,
      valid_until_ms: now + 24 * 3_600_000,
      checked_by: { kind: "credential", credential_id: "dddd".padEnd(32, "0") },
    });
    expect(Object.keys(result).sort()).toEqual(["location", "sha256", "upload"]);
    const after = await locationState(repo);
    expect(after).toEqual({ revision: before.revision + 1, locations: before.locations + 1, events: before.events + 1, watermark: expect.any(Number) });
    expect(after.watermark).toBeGreaterThan(before.watermark);

    const changes = await postJson(request, `/v1/repos/${repo}/changes/query`, { stream: "locations" });
    const page = (await changes.json()) as { items: { event: { event_id: string } }[] };
    expect(page.items.map((item) => item.event.event_id)).toEqual([finalizeEventId(uploadId)]);

    const row = await uploadRow(uploadId);
    expect(row?.sha256).toBe(fixture.report["sha256"]);
    const stored = await env.EVIDENCE_BUCKET.get(row!.staging_key);
    expect(await sha256Hex(new Uint8Array(await stored!.arrayBuffer()))).toBe(await sha256Hex(content));
    expect(await r2Keys(`staging/${uploadId}/`)).toEqual([]);

    request.clock += 3_600_000;
    const replay = await finalize(request, repo, uploadId, "verifier-b");
    expect(replay.status).toBe(200);
    expect(await replay.json()).toEqual(result);
    expect(await locationState(repo)).toEqual(after);
  });

  it("refuses a producer and writes nothing", async () => {
    const content = payload(40);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    const before = await locationState(repo);
    await expectError(await finalize(request, repo, uploadId, "producer"), 403, "denied");
    expect((await uploadRow(uploadId))?.state).toBe("open");
    expect(await locationState(repo)).toEqual(before);
    expect(await env.EVIDENCE_BUCKET.head((await uploadRow(uploadId))!.staging_key)).toBeNull();
  });

  it("rejects bytes that do not match the digest, deletes the object, and leaves the observation untouched", async () => {
    const content = payload(64);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), payload(64, 2));
    const before = await published(request, repo, fixture);
    const locations = await locationState(repo);
    const refusal = await expectError(await finalize(request, repo, uploadId), 400, "invalid_input", null);
    expect(refusal.error.message).toContain("rejected");
    const row = await uploadRow(uploadId);
    expect(row?.state).toBe("rejected");
    expect(await env.EVIDENCE_BUCKET.head(row!.staging_key)).toBeNull();
    expect(await published(request, repo, fixture)).toEqual(before);
    expect(await locationState(repo)).toEqual(locations);
    await expectError(await finalize(request, repo, uploadId), 400, "invalid_input");

    // A rejected upload is not reused: the reference can be uploaded again.
    const again = await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer");
    expect(again.status).toBe(201);
  });

  it("rejects staged parts whose total length was tampered with", async () => {
    const content = payload(PART + 50);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    const [, last] = await r2Keys(`staging/${uploadId}/`);
    await env.EVIDENCE_BUCKET.put(last!, payload(49));
    await expectError(await finalize(request, repo, uploadId), 400, "invalid_input", null);
    const row = await uploadRow(uploadId);
    expect(row?.state).toBe("rejected");
    expect(await env.EVIDENCE_BUCKET.head(row!.staging_key)).toBeNull();
    expect((await locationState(repo)).locations).toBe(0);
  });

  it("refuses a body other than empty or {}", async () => {
    const content = payload(8);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    await expectError(await postJson(request, `/v1/repos/${repo}/uploads/${uploadId}/finalize`, { force: true }, "verifier"), 400, "invalid_input", "/force");
    expect((await postJson(request, `/v1/repos/${repo}/uploads/${uploadId}/finalize`, {}, "verifier")).status).toBe(200);
    await expectError(await finalize(request, repo, hex(32)), 404, "not_found");
  });

  it("finalizes once when the D1 batch fails after the object was written", async () => {
    const content = payload(PART + 3);
    const { repo, request, fixture } = await setup(content);
    const uploadId = await stage(request, repo, reportUpload(fixture), content);
    const real = db();
    const failing = new Proxy(real, {
      get(target, property) {
        if (property === "batch") return async () => Promise.reject(new Error("D1 is unavailable"));
        const value = Reflect.get(target, property) as unknown;
        return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
      },
    });
    const context: RequestContext = {
      db: failing,
      cursorKey: "unused",
      requestId: crypto.randomUUID(),
      nowMs: request.clock,
      principal: principal("dddd", [repo], ["verifier"]),
    };
    await expect(finalizeUpload(context, env.EVIDENCE_BUCKET, repo, uploadId, new Uint8Array())).rejects.toThrow("D1 is unavailable");
    const row = await uploadRow(uploadId);
    expect(row?.state).toBe("open");
    expect(await env.EVIDENCE_BUCKET.head(row!.staging_key)).not.toBeNull();
    expect((await locationState(repo)).locations).toBe(0);

    const retry = await finalize(request, repo, uploadId);
    expect(retry.status).toBe(200);
    const result = await retry.json();
    const again = await finalize(request, repo, uploadId);
    expect(await again.json()).toEqual(result);
    expect((await locationState(repo)).locations).toBe(1);
  });

  describe("proof documents", () => {
    async function proofSetup(documents: { expected: Uint8Array; completion: Uint8Array }, fixtureText?: string) {
      const repo = await newRepository();
      const request = harness([repo]);
      const fixture = await ingestProof(request, repo, documents, fixtureText);
      return { repo, request, fixture };
    }

    it("finalizes a valid completion record and expected listing", async () => {
      const documents = { expected: expectedManifest(), completion: completionRecord() };
      const { repo, request, fixture } = await proofSetup(documents);
      for (const [kind, bytes] of [
        ["expected-listing", documents.expected],
        ["completion-record", documents.completion],
      ] as const) {
        const uploadId = await stage(request, repo, proofUpload(fixture, kind, bytes), bytes);
        const response = await finalize(request, repo, uploadId);
        expect(response.status, kind).toBe(200);
        const result = (await response.json()) as { location: Record<string, unknown> };
        expect(result.location).toMatchObject({ reference_kind: kind, kind: "verified" });
      }
      expect((await locationState(repo)).locations).toBe(2);
    });

    it("accepts a seed or rig completion record proven on another same-tree commit", async () => {
      // A seed run records each cell's own commit; local_evidence.py compares package and gate, not head.
      const completion = completionRecord({ head: "c".repeat(40) });
      const { repo, request, fixture } = await proofSetup({ expected: expectedManifest(), completion });
      const uploadId = await stage(request, repo, proofUpload(fixture, "completion-record", completion), completion);
      expect((await finalize(request, repo, uploadId)).status).toBe(200);
    });

    it("binds a hosted completion record to the producer's head, run, and attempt", async () => {
      const hosted = { package: "alpha", environment: "ubuntu-latest", head: "b".repeat(40) };
      const good = completionRecord({ ...hosted, run: "100", attempt: 1, reports: ["L1/alpha.xml"] });
      const otherRun = completionRecord({ ...hosted, run: "101", attempt: 1, reports: ["L1/alpha.xml"] });
      const otherHead = completionRecord({ ...hosted, head: "c".repeat(40), run: "100", attempt: 1, reports: ["L1/alpha.xml"] });
      for (const [bytes, status] of [
        [good, 200],
        [otherRun, 400],
        [otherHead, 400],
      ] as const) {
        const { repo, request, fixture } = await proofSetup({ expected: expectedManifest(), completion: bytes }, hostedFixture);
        const uploadId = await stage(request, repo, proofUpload(fixture, "completion-record", bytes), bytes);
        expect((await finalize(request, repo, uploadId)).status).toBe(status);
      }
    });

    it("rejects a completion record for another cell", async () => {
      for (const override of [{ package: "other" }, { gate: "L2" }, { environment: "ubuntu-latest" }]) {
        const completion = completionRecord(override);
        const { repo, request, fixture } = await proofSetup({ expected: expectedManifest(), completion });
        const uploadId = await stage(request, repo, proofUpload(fixture, "completion-record", completion), completion);
        const refusal = await expectError(await finalize(request, repo, uploadId), 400, "invalid_input", null);
        expect(refusal.error.message, JSON.stringify(override)).toContain("completion-cell-unknown");
        const row = await uploadRow(uploadId);
        expect(row?.state).toBe("rejected");
        expect(await env.EVIDENCE_BUCKET.head(row!.staging_key)).toBeNull();
        expect((await locationState(repo)).locations).toBe(0);
      }
    });

    it("rejects an expected listing for another target", async () => {
      const expected = expectedManifest({ target: "x86_64-unknown-linux-gnu" });
      const { repo, request, fixture } = await proofSetup({ expected, completion: completionRecord() });
      const uploadId = await stage(request, repo, proofUpload(fixture, "expected-listing", expected), expected);
      const refusal = await expectError(await finalize(request, repo, uploadId), 400, "invalid_input", null);
      expect(refusal.error.message).toContain("completion-manifest-target");
    });

    it("rejects a proof that is not strict JSON without quoting its bytes", async () => {
      const completion = encoder.encode('{"schema_version":1,"schema_version":1,"secret":"do-not-echo"}');
      const { repo, request, fixture } = await proofSetup({ expected: expectedManifest(), completion });
      const uploadId = await stage(request, repo, proofUpload(fixture, "completion-record", completion), completion);
      const refusal = await expectError(await finalize(request, repo, uploadId), 400, "invalid_input", null);
      expect(refusal.error.message).toContain("completion-plan-unreadable");
      expect(refusal.error.message).not.toContain("do-not-echo");
      expect((await uploadRow(uploadId))?.state).toBe("rejected");
    });
  });
});

describe("cleanupStaging", () => {
  const HOUR = 3_600_000;

  it("removes old unfinalized staging, keeps young uploads, and strips parts from finalized ones", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const start = 2_000_000_000_000;
    request.clock = start;

    const open = async (content: Uint8Array, parts: number) => {
      const fixture = await ingestReport(request, repo, content);
      const created = await postJson(request, uploadsPath(repo), reportUpload(fixture), "producer");
      const { upload_id: uploadId } = (await created.json()) as { upload_id: string };
      for (let index = 0; index < parts; index += 1) {
        expect((await putPart(request, repo, uploadId, index, content.slice(index * PART, (index + 1) * PART))).status).toBe(200);
      }
      return { fixture, uploadId };
    };

    const abandoned = await open(payload(PART + 5, 11), 1);
    const rejectedFixture = await ingestReport(request, repo, payload(30, 12));
    const rejected = await stage(request, repo, reportUpload(rejectedFixture), payload(30, 13));
    await expectError(await finalize(request, repo, rejected), 400, "invalid_input");
    const finalizedContent = payload(30, 14);
    const finalizedFixture = await ingestReport(request, repo, finalizedContent);
    const finalized = await stage(request, repo, reportUpload(finalizedFixture), finalizedContent);
    expect((await finalize(request, repo, finalized)).status).toBe(200);
    // A part a best-effort delete missed (or a race's losing write).
    await env.EVIDENCE_BUCKET.put(`staging/${finalized}/0-${"0".repeat(64)}`, payload(30, 15));

    request.clock = start + 20 * HOUR;
    const young = await open(payload(PART + 5, 16), 1);

    const result = await cleanupStaging(env, start + 49 * HOUR);
    expect(result.expired).toBeGreaterThanOrEqual(2);
    expect(result.cleaned_uploads).toBeGreaterThanOrEqual(3);

    expect((await uploadRow(abandoned.uploadId))?.state).toBe("expired");
    expect(await r2Keys(`staging/${abandoned.uploadId}/`)).toEqual([]);
    expect(await r2Keys(`staging/${rejected}/`)).toEqual([]);
    expect(await r2Keys(`staging/${finalized}/`)).toEqual([]);
    expect(await env.EVIDENCE_BUCKET.head((await uploadRow(finalized))!.staging_key)).not.toBeNull();
    expect((await uploadRow(young.uploadId))?.state).toBe("expired");
    expect(await r2Keys(`staging/${young.uploadId}/`)).toHaveLength(1);
  });
});
