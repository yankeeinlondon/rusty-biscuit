// `GET /objects/{observation_id}/{reference_kind}/{reference_digest}`: only a
// finalized object whose newest object location event is `verified` is
// served, and a withdrawal denies the next retrieval but not one already
// streaming.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import {
  expectError,
  finalize,
  harness,
  ingestReport,
  newRepository,
  payload,
  postJson,
  reportEvent,
  reportUpload,
  sha256Hex,
  stage,
  uploadRow,
  type Harness,
  type IngestedFixture,
} from "./helpers.ts";

const objectPath = (repo: string, fixture: IngestedFixture) =>
  `/v1/repos/${repo}/objects/${fixture.observationId}/report/${encodeURIComponent(fixture.report["sha256"] as string)}`;

async function finalized(content: Uint8Array): Promise<{ repo: string; request: Harness; fixture: IngestedFixture; uploadId: string }> {
  const repo = await newRepository();
  const request = harness([repo]);
  const fixture = await ingestReport(request, repo, content);
  const uploadId = await stage(request, repo, reportUpload(fixture), content);
  expect((await finalize(request, repo, uploadId)).status).toBe(200);
  return { repo, request, fixture, uploadId };
}

function withdraw(request: Harness, repo: string, fixture: IngestedFixture, locator: Record<string, string>): Promise<Response> {
  return postJson(request, `/v1/repos/${repo}/locations`, reportEvent(fixture, { kind: "withdrawn", locator }), "verifier");
}

describe("GET /objects/…", () => {
  it("streams the exact bytes of a finalized object with its length and digest headers", async () => {
    const content = payload(1500);
    const { repo, request, fixture } = await finalized(content);
    for (const as of ["reader", "verifier"]) {
      const response = await request(objectPath(repo, fixture), { as });
      expect(response.status).toBe(200);
      expect(response.headers.get("content-type")).toBe("application/octet-stream");
      expect(response.headers.get("content-length")).toBe("1500");
      expect(response.headers.get("x-content-sha256")).toBe(await sha256Hex(content));
      expect(response.headers.get("x-request-id")).toMatch(/^[0-9a-f-]{36}$/);
      expect(new Uint8Array(await response.arrayBuffer())).toEqual(content);
    }
  });

  it("refuses a producer", async () => {
    const { repo, request, fixture } = await finalized(payload(10));
    await expectError(await request(objectPath(repo, fixture), { as: "producer" }), 403, "denied");
  });

  it("answers 404 for an object that is staged but not finalized, and for a malformed path", async () => {
    const content = payload(10);
    const repo = await newRepository();
    const request = harness([repo]);
    const fixture = await ingestReport(request, repo, content);
    await stage(request, repo, reportUpload(fixture), content);
    await expectError(await request(objectPath(repo, fixture)), 404, "not_found");
    await expectError(await request(`/v1/repos/${repo}/objects/${fixture.observationId}/log/abc`), 404, "not_found");
    await expectError(await request(`/v1/repos/${repo}/objects/${fixture.observationId}/report/%E0%A4%A`), 404, "not_found");
  });

  it("denies retrieval after a withdrawal, and a finalize replay does not revive the object", async () => {
    const { repo, request, fixture, uploadId } = await finalized(payload(10));
    expect((await withdraw(request, repo, fixture, { kind: "object", upload_id: uploadId })).status).toBe(201);
    await expectError(await request(objectPath(repo, fixture)), 404, "not_found");
    expect((await finalize(request, repo, uploadId)).status).toBe(200);
    await expectError(await request(objectPath(repo, fixture)), 404, "not_found");
    // The bytes stay stored; only retrieval is denied.
    expect(await env.EVIDENCE_BUCKET.head((await uploadRow(uploadId))!.staging_key)).not.toBeNull();
  });

  it("completes a retrieval already streaming when a withdrawal lands, then denies the next", async () => {
    const content = payload(4096);
    const { repo, request, fixture, uploadId } = await finalized(content);
    const streaming = await request(objectPath(repo, fixture));
    expect(streaming.status).toBe(200);
    expect((await withdraw(request, repo, fixture, { kind: "object", upload_id: uploadId })).status).toBe(201);
    expect(new Uint8Array(await streaming.arrayBuffer())).toEqual(content);
    await expectError(await request(objectPath(repo, fixture)), 404, "not_found");
  });

  it("keeps the object retrievable when an independent host-dir location is withdrawn", async () => {
    const { repo, request, fixture } = await finalized(payload(10));
    const hostDir = { kind: "host-dir", host: "rig-mac", directory: "/reports/run7" };
    expect((await postJson(request, `/v1/repos/${repo}/locations`, reportEvent(fixture, { locator: hostDir }), "verifier")).status).toBe(201);
    expect((await withdraw(request, repo, fixture, hostDir)).status).toBe(201);
    expect((await request(objectPath(repo, fixture))).status).toBe(200);
  });

  it("ignores the attestation expiry of an object location", async () => {
    const { repo, request, fixture } = await finalized(payload(10));
    request.clock += 30 * 24 * 3_600_000;
    expect((await request(objectPath(repo, fixture))).status).toBe(200);
  });

  it("answers 503 when the rows say the bytes exist but R2 has lost them", async () => {
    const { repo, request, fixture, uploadId } = await finalized(payload(10));
    await env.EVIDENCE_BUCKET.delete((await uploadRow(uploadId))!.staging_key);
    await expectError(await request(objectPath(repo, fixture)), 503, "unavailable");
  });
});
