// The ingestion CLI's exported commands in process, against an in-memory
// fake of the API with injected fetch, sleep, and randomness.
import { existsSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { checkEndpoint, planUploads, run, type IngestIo, type ResumeState } from "../../cli/ingest.ts";
import { createNodeReportReader } from "../../src/contracts/node-report-reader.ts";
import { loadStrict, type JsonObject } from "../../src/contracts/index.ts";
import { FakeService, parityPath, PRODUCER_TOKEN, REPO, rewriteCollection, rigWorkspace, unavailable, VERIFIER_TOKEN, type Workspace } from "../fixtures/ingest/fake-service.ts";

const MIB = 1024 * 1024;
let dir: string;
let service: FakeService;
let sleeps: number[];
let out: string[];
let err: string[];
let random: number;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "evidence-ingest-"));
  writeFileSync(join(dir, "producer.token"), `${PRODUCER_TOKEN}\n`);
  writeFileSync(join(dir, "verifier.token"), `${VERIFIER_TOKEN}\n`);
  service = new FakeService();
  sleeps = [];
  out = [];
  err = [];
  random = 0.5;
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

function io(home = dir): IngestIo {
  return {
    fetch: service.fetch,
    sleep: async (ms) => void sleeps.push(ms),
    random: () => random,
    now: () => 1_800_000_000_000,
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    readReport: createNodeReportReader({ home: () => home }),
  };
}

const statePath = () => join(dir, "state.json");
const readState = () => JSON.parse(readFileSync(statePath(), "utf8")) as ResumeState;
const flags = (extra: string[] = []) => [
  "--endpoint",
  "https://evidence.test/",
  "--repo",
  REPO,
  "--token-file",
  join(dir, "producer.token"),
  "--state",
  statePath(),
  ...extra,
];
const ingestRig = (w: Workspace, extra: string[] = []) => run(["ingest", w.input, ...flags(["--proof-dir", w.proofDir, ...extra])], io());
const paths = (method: string) => service.requests.filter((request) => request.method === method).map((request) => request.path.replace(`/v1/repos/${REPO}`, ""));

describe("ingest a version-3 collection", () => {
  it("submits the canonical bytes once, stages every mapped file, and a verifier finalizes", async () => {
    const w = rigWorkspace(dir);
    expect(await ingestRig(w)).toBe(0);

    const posts = service.requests.filter((request) => request.path.endsWith("/collections"));
    expect(posts).toHaveLength(1);
    expect(new TextDecoder().decode(posts[0]!.body)).toBe(`{"source_version":3,"collection":${w.canonical}}`);
    expect(posts[0]!.authorization).toBe(`Bearer ${PRODUCER_TOKEN}`);
    expect(paths("POST").filter((path) => path === "/uploads")).toHaveLength(3);
    expect(paths("PUT")).toHaveLength(3);

    const state = readState();
    expect(state.ingest).toEqual({ state: "acknowledged", evidence_revision: 1, accepted: [w.observationId], already_present: [] });
    expect(state.submission).toMatchObject({ source_version: 3, collection_id: w.collectionId });
    expect(state.uploads.map((upload) => [upload.reference_kind, upload.path, upload.state, upload.parts_acknowledged])).toEqual([
      ["report", "biscuit-hash-L1.xml", "staged", 1],
      ["expected-listing", "biscuit-hash-L1.expected.json", "staged", 1],
      ["completion-record", "biscuit-hash-L1.completion.json", "staged", 1],
    ]);
    expect(readFileSync(`${statePath()}.collection.json`, "utf8")).toBe(w.canonical);
    expect(out.join("")).toContain(`collection ${w.collectionId}`);
    expect(out.join("")).toContain("1 accepted, 0 already present");
    expect(out.join("")).toContain(`3 upload(s) staged; pending verifier finalization: just evidence-finalize ${statePath()} --token-file <verifier token path>`);

    // The producer credential cannot finalize.
    expect(await run(["finalize", statePath(), "--token-file", join(dir, "producer.token")], io())).toBe(1);
    expect(err.join("")).toContain("code: denied");
    expect(readState().uploads.every((upload) => upload.state === "staged")).toBe(true);

    service.requests.length = 0;
    expect(await run(["finalize", statePath(), "--token-file", join(dir, "verifier.token")], io())).toBe(0);
    expect(paths("POST")).toHaveLength(3);
    expect(service.requests.every((request) => request.authorization === `Bearer ${VERIFIER_TOKEN}`)).toBe(true);
    expect(readState().uploads.every((upload) => upload.state === "finalized")).toBe(true);
    expect(out.join("")).toContain("3 upload(s) finalized");
  });

  it("submits without uploads when --proof-dir is absent", async () => {
    const w = rigWorkspace(dir);
    expect(await run(["ingest", w.input, ...flags()], io())).toBe(0);
    expect(paths("POST")).toEqual(["/collections"]);
    expect(readState().uploads).toEqual([]);
    expect(out.join("")).toContain("no uploads requested");
  });

  it("splits a file of 1 MiB + 3 bytes into parts of exactly 1 MiB and 3 bytes", async () => {
    const w = rigWorkspace(dir, MIB + 3);
    expect(await ingestRig(w)).toBe(0);
    const reportParts = service.requests.filter((request) => request.method === "PUT").slice(0, 2);
    expect(reportParts.map((request) => [request.path.split("/").pop(), request.body.byteLength])).toEqual([
      ["0", MIB],
      ["1", 3],
    ]);
    expect(readState().uploads[0]).toMatchObject({ bytes: MIB + 3, parts_acknowledged: 2, state: "staged" });
  });

  it("stages a zero-byte report with no parts", async () => {
    const w = rigWorkspace(dir, 0);
    expect(await ingestRig(w)).toBe(0);
    expect(paths("PUT")).toHaveLength(2);
    expect(readState().uploads[0]).toMatchObject({ bytes: 0, parts_acknowledged: 0, state: "staged" });
  });

  it("collapses duplicate references to one upload", () => {
    const w = rigWorkspace(dir);
    const loaded = loadStrict(readFileSync(w.input));
    if (!loaded.ok) throw new Error("fixture does not load");
    const observation = ((loaded.value as JsonObject)["observations"] as JsonObject[])[0]!;
    const reports = observation["reports"] as JsonObject[];
    observation["reports"] = [reports[0]!, reports[0]!];
    expect(planUploads(loaded.value as JsonObject, w.proofDir).map((upload) => upload.reference_kind)).toEqual(["report", "expected-listing", "completion-record"]);
  });
});

describe("ingest a version-2 receipt", () => {
  it("adapts it on this host, submits source_version 2, and preserves the original input", async () => {
    const home = parityPath("adapter-hook-v2.home");
    const input = parityPath("adapter-hook-v2.json");
    const proofDir = join(home, "ci-evidence", "bbbbbbbbb");
    expect(await run(["ingest", input, ...flags(["--proof-dir", proofDir])], io(home))).toBe(0);

    const canonicalText = readFileSync(parityPath("adapter-hook-v2.canonical"), "utf8").replace(/\n$/, "");
    const post = service.requests.find((request) => request.path.endsWith("/collections"))!;
    expect(new TextDecoder().decode(post.body)).toBe(`{"source_version":2,"collection":${canonicalText}}`);
    expect(readFileSync(`${statePath()}.input.json`)).toEqual(readFileSync(input));
    const state = readState();
    expect(state.input.source).toBe(input);
    expect(state.submission.source_version).toBe(2);
    expect(state.uploads.map((upload) => upload.path).sort()).toEqual(["claudine-L1.xml", "darkmatter-L1.xml"]);
    expect(state.uploads.every((upload) => upload.state === "staged")).toBe(true);
  });
});

describe("configuration errors before any request", () => {
  it("accepts https and loopback http endpoints only", () => {
    expect(checkEndpoint("https://evidence.test/")).toBe("https://evidence.test");
    expect(checkEndpoint("http://127.0.0.1:8787")).toBe("http://127.0.0.1:8787");
    expect(checkEndpoint("http://localhost:8787/")).toBe("http://localhost:8787");
    expect(checkEndpoint("http://[::1]:8787")).toBe("http://[::1]:8787");
    for (const bad of ["http://evidence.test", "https://evidence.test/v1", "https://user:pw@evidence.test", "ftp://evidence.test", "evidence.test"]) {
      expect(() => checkEndpoint(bad), bad).toThrow(/--endpoint must be/);
    }
  });

  const cases: [string, (w: Workspace) => string[], string][] = [
    ["http to a non-loopback host", (w) => ["ingest", w.input, ...flags(["--proof-dir", w.proofDir])].map((a) => (a === "https://evidence.test/" ? "http://evidence.test" : a)), "--endpoint must be"],
    ["a malformed repository ID", (w) => ["ingest", w.input, ...flags()].map((a) => (a === REPO ? "owner/name" : a)), "--repo must be"],
    ["a missing token file", (w) => ["ingest", w.input, ...flags()].map((a) => (a.endsWith("producer.token") ? join(dir, "absent.token") : a)), "could not read the token file"],
    [
      "an empty token file",
      (w) => {
        writeFileSync(join(dir, "empty.token"), "\n");
        return ["ingest", w.input, ...flags()].map((a) => (a.endsWith("producer.token") ? join(dir, "empty.token") : a));
      },
      "does not hold one token",
    ],
    [
      "input with a duplicate key",
      (w) => {
        writeFileSync(w.input, readFileSync(w.input, "utf8").replace('"environment": "macos-latest"', '"environment": "macos-latest", "environment": "macos-latest"'));
        return ["ingest", w.input, ...flags()];
      },
      'is invalid at "/environment"',
    ],
    [
      "a missing proof file",
      (w) => {
        unlinkSync(join(w.proofDir, "biscuit-hash-L1.completion.json"));
        return ["ingest", w.input, ...flags(["--proof-dir", w.proofDir])];
      },
      '"/observations/0/proof/completion_record_digest" is missing',
    ],
    [
      "a proof whose sha256 differs from its digest",
      (w) => {
        writeFileSync(join(w.proofDir, "biscuit-hash-L1.expected.json"), "{}\n");
        return ["ingest", w.input, ...flags(["--proof-dir", w.proofDir])];
      },
      '"/observations/0/proof/expected_listing_digest" does not match its digest',
    ],
    [
      "a report whose length differs",
      (w) => {
        writeFileSync(join(w.proofDir, "biscuit-hash-L1.xml"), "short");
        return ["ingest", w.input, ...flags(["--proof-dir", w.proofDir])];
      },
      '"/observations/0/reports/0" has 5 bytes',
    ],
    [
      "a report path escaping the proof directory",
      (w) => {
        rewriteCollection(w.input, (document) => {
          (document.observations[0]!["reports"] as { path: string }[])[0]!.path = "../biscuit-hash-L1.xml";
          (document.observations[0]!["cell"] as { report: string }).report = "../biscuit-hash-L1.xml";
        });
        return ["ingest", w.input, ...flags(["--proof-dir", w.proofDir])];
      },
      'the report path at "/observations/0/reports/0" is absolute or contains ".."',
    ],
  ];
  for (const [name, argv, message] of cases) {
    it(`refuses ${name} with exit 2, sending nothing and writing no state`, async () => {
      const w = rigWorkspace(dir);
      expect(await run(argv(w), io())).toBe(2);
      expect(err.join("")).toContain(message);
      expect(service.requests).toEqual([]);
      expect(existsSync(statePath())).toBe(false);
    });
  }
});

describe("retry policy", () => {
  it("treats a lost collection response as acknowledged when the retry finds it already present", async () => {
    const w = rigWorkspace(dir);
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 1, action: { kind: "throw-after" } });
    expect(await ingestRig(w)).toBe(0);
    expect(paths("POST").filter((path) => path === "/collections")).toHaveLength(2);
    expect(readState().ingest).toEqual({ state: "acknowledged", evidence_revision: 1, accepted: [], already_present: [w.observationId] });
    expect(sleeps).toEqual([500]);
  });

  it("retries a 503 with full jitter, then succeeds", async () => {
    const w = rigWorkspace(dir);
    service.fault({ match: (method, path) => method === "POST" && path.endsWith("/uploads"), times: 2, action: unavailable() });
    expect(await ingestRig(w)).toBe(0);
    expect(sleeps).toEqual([500, 1000]);
  });

  it("honors a larger Retry-After on 429, capped at 60 s", async () => {
    const w = rigWorkspace(dir);
    random = 0;
    const limited = (seconds: string) => ({ kind: "respond" as const, status: 429, headers: { "retry-after": seconds }, body: { error: { code: "rate_limited", message: "slow down", location: null, retryable: true, retry_after_ms: null }, request_id: "r" } });
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 1, action: limited("2") });
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 1, action: limited("120") });
    expect(await ingestRig(w)).toBe(0);
    expect(sleeps).toEqual([2000, 60_000]);
  });

  it("gives up after five attempts, keeps the state, and prints the resume command", async () => {
    const w = rigWorkspace(dir);
    random = 1;
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 5, action: unavailable() });
    expect(await ingestRig(w)).toBe(1);
    expect(service.requests).toHaveLength(5);
    expect(sleeps).toEqual([1000, 2000, 4000, 8000]);
    expect(sleeps.every((ms) => ms <= 30_000)).toBe(true);
    expect(err.join("")).toContain("gave up after 5 attempts");
    expect(err.join("")).toContain(`resume with: just evidence-resume ${statePath()}`);
    expect(readState().ingest.state).toBe("pending");
  });

  it("does not retry a 400 and prints the error body", async () => {
    const w = rigWorkspace(dir);
    service.fault({
      match: (_m, path) => path.endsWith("/collections"),
      times: 1,
      action: { kind: "respond", status: 400, body: { error: { code: "invalid_input", message: "bad cell", location: "/collection/observations/0", retryable: false, retry_after_ms: null }, request_id: "req-9" } },
    });
    expect(await ingestRig(w)).toBe(1);
    expect(service.requests).toHaveLength(1);
    expect(sleeps).toEqual([]);
    const stderr = err.join("");
    for (const text of ["code: invalid_input", "message: bad cell", "location: /collection/observations/0", "request_id: req-9", "just evidence-resume"]) expect(stderr).toContain(text);
  });
});

describe("resume", () => {
  it("continues from the server's parts_received without repeating the collection or earlier parts", async () => {
    const w = rigWorkspace(dir, 3 * MIB + 3);
    service.fault({ match: (method, path) => method === "PUT" && path.endsWith("/parts/2"), times: 5, action: { kind: "throw-before" } });
    expect(await ingestRig(w)).toBe(1);
    expect(readState().uploads[0]).toMatchObject({ parts_acknowledged: 2, state: "pending" });
    expect(readState().ingest.state).toBe("acknowledged");

    service.requests.length = 0;
    expect(await run(["resume", statePath()], io())).toBe(0);
    expect(paths("POST")[0]).toBe("/uploads");
    expect(paths("POST")).not.toContain("/collections");
    const reportId = readState().uploads[0]!.upload_id!;
    expect(paths("PUT").filter((path) => path.includes(reportId))).toEqual([`/uploads/${reportId}/parts/2`, `/uploads/${reportId}/parts/3`]);
    expect(readState().uploads.every((upload) => upload.state === "staged")).toBe(true);
  });

  it("refuses a tampered retained collection before any request", async () => {
    const w = rigWorkspace(dir);
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 5, action: unavailable() });
    expect(await ingestRig(w)).toBe(1);
    writeFileSync(`${statePath()}.collection.json`, w.canonical.replace("macos-latest", "ubuntu-latest"));
    service.requests.length = 0;
    expect(await run(["resume", statePath()], io())).toBe(2);
    expect(err.join("")).toContain("no longer matches collection");
    expect(service.requests).toEqual([]);
  });

  it("stops when the service holds different bytes for a replayed part", async () => {
    const w = rigWorkspace(dir);
    service.fault({
      match: (method) => method === "PUT",
      times: 1,
      action: { kind: "respond", status: 409, body: { error: { code: "identity_conflict", message: "part differs", location: null, retryable: false, retry_after_ms: null }, request_id: "r" } },
    });
    expect(await ingestRig(w)).toBe(1);
    expect(err.join("")).toContain("already holds different bytes for part 0");
    expect(sleeps).toEqual([]);
  });
});

describe("resume state loader wiring", () => {
  async function stateAfterFailure(): Promise<string> {
    const w = rigWorkspace(dir);
    service.fault({ match: (_m, path) => path.endsWith("/collections"), times: 5, action: unavailable() });
    expect(await ingestRig(w)).toBe(1);
    service.requests.length = 0;
    err.length = 0;
    return readFileSync(statePath(), "utf8");
  }

  const fields: [string, (state: Record<string, unknown>) => void, string][] = [
    ["an absent field", (s) => delete s["endpoint"], '"/endpoint": is required'],
    ["a null field", (s) => (s["repository_id"] = null), '"/repository_id": must be a string, not null'],
    ["a wrong type", (s) => (s["updated_at_ms"] = "soon"), '"/updated_at_ms": must be an integer, not a string'],
    ["a wrong element", (s) => (s["uploads"] = [5]), '"/uploads/0": must be an object, not a number'],
    ["an empty string", (s) => (s["token_file"] = ""), '"/token_file": must not be empty'],
  ];
  for (const [name, edit, message] of fields) {
    it(`refuses ${name}, located, before any request`, async () => {
      const state = JSON.parse(await stateAfterFailure()) as Record<string, unknown>;
      edit(state);
      writeFileSync(statePath(), JSON.stringify(state));
      expect(await run(["resume", statePath()], io())).toBe(2);
      expect(err.join("")).toContain(message);
      expect(service.requests).toEqual([]);
    });
  }

  const documents: [string, (text: string) => string | Uint8Array, string][] = [
    ["a duplicate key", (text) => text.replace('"format_version": 1', '"format_version": 1, "format_version": 1'), 'invalid at "/format_version"'],
    ["trailing garbage", (text) => `${text} garbage`, 'invalid at ""'],
    ["a byte order mark", (text) => `﻿${text}`, "byte order mark"],
    ["a blank document", () => "  \n", 'invalid at ""'],
    ["concatenated documents", (text) => `${text}${text}`, 'invalid at ""'],
  ];
  for (const [name, edit, message] of documents) {
    it(`refuses a state document with ${name}`, async () => {
      writeFileSync(statePath(), edit(await stateAfterFailure()));
      expect(await run(["resume", statePath()], io())).toBe(2);
      expect(err.join("")).toContain(message);
      expect(service.requests).toEqual([]);
    });
  }
});

describe("secrets and report bytes", () => {
  it("never appear in output or the state file", async () => {
    const w = rigWorkspace(dir);
    expect(await ingestRig(w)).toBe(0);
    expect(await run(["finalize", statePath(), "--token-file", join(dir, "verifier.token")], io())).toBe(0);
    service.fault({ match: () => true, times: 5, action: unavailable() });
    rmSync(statePath());
    expect(await ingestRig(w)).toBe(1);
    const reportText = new TextDecoder().decode(w.report.slice(0, 40));
    for (const text of [out.join(""), err.join(""), readFileSync(statePath(), "utf8")]) {
      expect(text).not.toContain(PRODUCER_TOKEN);
      expect(text).not.toContain(VERIFIER_TOKEN);
      expect(text).not.toContain(reportText);
    }
  });
});
