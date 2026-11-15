// Assembling one retained nightly cell (a real `queue` L1 pass on
// windows-latest, copied byte for byte) into a collection ingestion accepts.
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { assembleHostedCollection, run } from "../../cli/hosted-collection.ts";
import { planUploads } from "../../cli/ingest.ts";
import type { OperatorIo } from "../../cli/operator.ts";
import { parseCollection, type JsonObject } from "../../src/contracts/index.ts";

const FIXTURE = join(import.meta.dirname, "..", "fixtures", "hosted-cell");
const TREE = "a".repeat(40);
const OBSERVATION_ID = "1db1f9b866e71c85721d2e61fa66f48d4f7fd0553d420ed9a091643cc31b8f9f";

let root: string;
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "hosted-collection-"));
  cpSync(FIXTURE, join(root, "in"), { recursive: true });
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

const inputs = (overrides: Record<string, string> = {}) => ({
  observation: join(root, "in", "observation"),
  junit: join(root, "in", "junit"),
  completion: join(root, "in", "completion"),
  tree: TREE,
  out: join(root, "out"),
  ...overrides,
});

function io() {
  const out: string[] = [];
  const err: string[] = [];
  const value: OperatorIo = {
    env: {},
    fetch: (async () => {
      throw new Error("no network");
    }) as typeof fetch,
    spawn: () => ({ status: 1, stdout: "", stderr: "" }),
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => 0,
  };
  return { io: value, stdout: () => out.join(""), stderr: () => err.join("") };
}

describe("hosted-collection", () => {
  it("keeps the original observation and lays out every cited file where ingestion verifies it", () => {
    const result = assembleHostedCollection(inputs());
    expect(result.observation_id).toBe(OBSERVATION_ID);

    const parsed = parseCollection(readFileSync(result.collection));
    if (!parsed.ok) throw new Error(JSON.stringify(parsed.problems));
    expect(parsed.value.observation_ids).toEqual([OBSERVATION_ID]);
    const collection = parsed.value.collection as JsonObject;
    expect([collection["environment"], collection["head"], collection["tree"]]).toEqual(["windows-latest", "c26823f69310efb3ef13ad6d9ffa92da6dda7e2f", TREE]);

    // planUploads hashes each file against the digest the observation cites.
    const uploads = planUploads(collection, join(root, "out", "files"));
    expect(uploads.map((upload) => [upload.reference_kind, upload.path])).toEqual([
      ["report", "L1/queue.xml"],
      ["expected-listing", "queue-L1.expected.json"],
      ["completion-record", "queue-L1.completion.json"],
    ]);
  });

  it("requires the completion artifact when the proof cites a completion record", () => {
    const { completion: _omitted, ...withoutCompletion } = inputs();
    expect(() => assembleHostedCollection(withoutCompletion)).toThrow("--completion is required");
  });

  it("refuses an envelope that would not form a valid collection, writing nothing", async () => {
    const envelope = join(root, "in", "observation", "observation.json");
    writeFileSync(envelope, readFileSync(envelope, "utf8").replace('"outcome":"pass"', '"outcome":"maybe"'));
    const fake = io();
    expect(await run(["--observation", inputs().observation, "--junit", inputs().junit, "--completion", inputs().completion, "--tree", TREE, "--out", inputs().out], fake.io)).toBe(2);
    expect(fake.stderr()).toContain("does not form a valid collection");
    expect(() => readFileSync(join(inputs().out, "collection.json"))).toThrow();
  });

  it("names a cited file the artifacts lack", () => {
    rmSync(join(root, "in", "junit", "L1", "queue.xml"));
    expect(() => assembleHostedCollection(inputs())).toThrow("the artifacts lack");
  });

  it("rejects a tree that is not a commit tree ID and names missing flags", async () => {
    expect(() => assembleHostedCollection(inputs({ tree: "HEAD" }))).toThrow("--tree must be");
    const fake = io();
    expect(await run(["--observation", "x"], fake.io)).toBe(2);
    expect(fake.stderr()).toContain("missing --junit, --tree, --out");
  });
});
