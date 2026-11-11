// The ingestion CLI spawned for real, against a loopback HTTP server over the
// same in-memory fake: one control run and the shared loader's wiring for the
// two documents it reads (input and resume state).
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FakeService, PRODUCER_TOKEN, REPO, rigWorkspace } from "../fixtures/ingest/fake-service.ts";

const script = fileURLToPath(new URL("../../cli/ingest.ts", import.meta.url));
const tsx = fileURLToPath(new URL("../../node_modules/tsx/dist/cli.mjs", import.meta.url));

let dir: string;
let service: FakeService;
let server: Server;
let endpoint: string;

beforeEach(async () => {
  dir = mkdtempSync(join(tmpdir(), "evidence ingest cli "));
  writeFileSync(join(dir, "producer.token"), `${PRODUCER_TOKEN}\n`);
  service = new FakeService();
  server = createServer((request, response) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      const body = new Uint8Array(Buffer.concat(chunks));
      const path = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
      service.requests.push({ method: request.method ?? "", path, authorization: request.headers.authorization, body });
      const answer = service.handle(request.method ?? "", path, request.headers.authorization, body);
      response.writeHead(answer.status, { "content-type": "application/json" });
      response.end(JSON.stringify(answer.body));
    });
  });
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  endpoint = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
});
afterEach(async () => {
  await new Promise<void>((done) => server.close(() => done()));
  rmSync(dir, { recursive: true, force: true });
});

function cli(args: string[]): Promise<{ status: number | null; stdout: string; stderr: string }> {
  return new Promise((done) => {
    const child = spawn(process.execPath, [tsx, script, ...args], { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("close", (status) => done({ status, stdout, stderr }));
  });
}

const ingestArgs = (input: string, proofDir: string) => [
  "ingest",
  input,
  "--endpoint",
  endpoint,
  "--repo",
  REPO,
  "--token-file",
  join(dir, "producer.token"),
  "--state",
  join(dir, "state.json"),
  "--proof-dir",
  proofDir,
];

describe("ingest CLI (spawned)", () => {
  it("ingests and stages over loopback http, with paths containing spaces", async () => {
    const w = rigWorkspace(dir);
    const result = await cli(ingestArgs(w.input, w.proofDir));
    expect(result.stderr).toBe("");
    expect(result.status).toBe(0);
    expect(result.stdout).toContain(`collection ${w.collectionId}`);
    expect(result.stdout).toContain("3 upload(s) staged");
    expect(result.stdout).toContain(`just evidence-finalize "${join(dir, "state.json")}"`);
    const state = JSON.parse(readFileSync(join(dir, "state.json"), "utf8")) as { uploads: { state: string }[] };
    expect(state.uploads.map((upload) => upload.state)).toEqual(["staged", "staged", "staged"]);
  });

  it("refuses a malformed input file with exit 2 and its location, sending nothing", async () => {
    const w = rigWorkspace(dir);
    writeFileSync(w.input, readFileSync(w.input, "utf8").replace('"head":', '"tree": "x", "head":'));
    const result = await cli(ingestArgs(w.input, w.proofDir));
    expect(result.status).toBe(2);
    expect(result.stderr).toContain('is invalid at "/tree"');
    expect(service.requests).toEqual([]);
  });

  it("refuses a malformed resume state with exit 2 and its location", async () => {
    writeFileSync(join(dir, "state.json"), '{"format_version": 1, "format_version": 1}');
    const result = await cli(["resume", join(dir, "state.json")]);
    expect(result.status).toBe(2);
    expect(result.stderr).toContain('is invalid at "/format_version"');
    expect(service.requests).toEqual([]);
  });
});
