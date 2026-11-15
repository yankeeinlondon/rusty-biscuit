// Just enough of the evidence-store API (collections, uploads, parts,
// finalize) over an in-memory store for the ingestion CLI tests, plus a
// workspace builder that derives a real-shape collection from the frozen
// parity fixtures with report and proof files whose digests match.
import { createHash, randomBytes } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { canonical, collectionIdentity, loadStrict, observationIdentity, type JsonObject } from "../../../src/contracts/index.ts";

export const PRODUCER_TOKEN = `ebt_${"a".repeat(32)}.${"p".repeat(43)}`;
export const VERIFIER_TOKEN = `ebt_${"b".repeat(32)}.${"v".repeat(43)}`;
export const REPO = "873451209";
const PART = 1024 * 1024;

export interface Recorded {
  readonly method: string;
  readonly path: string;
  readonly authorization: string | undefined;
  readonly body: Uint8Array;
}

interface StoredUpload {
  upload_id: string;
  observation_id: string;
  reference_kind: string;
  reference_digest: string;
  path: string;
  bytes: number;
  part_count: number;
  parts: Uint8Array[];
  state: "open" | "finalized";
}

/** A fault for the next matching request: answer a canned response, or throw before or after the server acts. */
export interface Fault {
  readonly match: (method: string, path: string) => boolean;
  readonly times: number;
  readonly action: { kind: "respond"; status: number; headers?: Record<string, string>; body: unknown } | { kind: "throw-before" } | { kind: "throw-after" };
}

const sha = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");

function errorBody(code: string, status: number, retryAfterMs: number | null = null) {
  return { status, body: { error: { code, message: code, location: null, retryable: status === 429 || status === 503, retry_after_ms: retryAfterMs }, request_id: "req-1" } };
}

export class FakeService {
  readonly requests: Recorded[] = [];
  readonly observations = new Set<string>();
  readonly uploads = new Map<string, StoredUpload>();
  readonly faults: { fault: Fault; left: number }[] = [];
  revision = 0;

  fault(fault: Fault): void {
    this.faults.push({ fault, left: fault.times });
  }

  /** The API itself, independent of transport. */
  handle(method: string, path: string, authorization: string | undefined, body: Uint8Array): { status: number; body: unknown } {
    const token = authorization?.replace(/^Bearer /, "");
    if (token !== PRODUCER_TOKEN && token !== VERIFIER_TOKEN) return errorBody("unauthenticated", 401);
    const prefix = `/v1/repos/${REPO}`;
    if (!path.startsWith(prefix)) return errorBody("not_found", 404);
    const rest = path.slice(prefix.length);
    if (method === "POST" && rest === "/collections") {
      if (token !== PRODUCER_TOKEN) return errorBody("denied", 403);
      const loaded = loadStrict(body);
      if (!loaded.ok) return errorBody("invalid_input", 400);
      const collection = (loaded.value as JsonObject)["collection"] as JsonObject;
      const ids = (collection["observations"] as JsonObject[]).map((o) => o["observation_id"] as string);
      const accepted = ids.filter((id) => !this.observations.has(id));
      const already = ids.filter((id) => this.observations.has(id));
      for (const id of accepted) this.observations.add(id);
      if (accepted.length > 0) this.revision += 1;
      return { status: 200, body: { collection_id: collectionIdentity(collection), accepted, already_present: already, evidence_revision: this.revision } };
    }
    if (method === "POST" && rest === "/uploads") {
      const request = JSON.parse(new TextDecoder().decode(body)) as Omit<StoredUpload, "upload_id" | "part_count" | "parts" | "state">;
      if (!this.observations.has(request.observation_id)) return errorBody("not_found", 404);
      const key = JSON.stringify([request.observation_id, request.reference_kind, request.reference_digest, request.path]);
      const existing = this.uploads.get(key);
      if (existing !== undefined) return { status: 200, body: this.view(existing) };
      const upload: StoredUpload = { ...request, upload_id: randomBytes(16).toString("hex"), part_count: Math.ceil(request.bytes / PART), parts: [], state: "open" };
      this.uploads.set(key, upload);
      return { status: 201, body: this.view(upload) };
    }
    const part = /^\/uploads\/([0-9a-f]{32})\/parts\/([0-9]+)$/.exec(rest);
    if (method === "PUT" && part !== null) {
      const upload = this.byId(part[1]!);
      if (upload === undefined) return errorBody("not_found", 404);
      const index = Number(part[2]);
      if (index > upload.parts.length || index >= upload.part_count) return errorBody("invalid_input", 400);
      if (index < upload.parts.length) {
        if (sha(upload.parts[index]!) !== sha(body)) return errorBody("identity_conflict", 409);
      } else {
        upload.parts.push(body);
      }
      return { status: 200, body: { upload_id: upload.upload_id, index, bytes: body.byteLength, parts_received: upload.parts.length } };
    }
    const fin = /^\/uploads\/([0-9a-f]{32})\/finalize$/.exec(rest);
    if (method === "POST" && fin !== null) {
      if (token !== VERIFIER_TOKEN) return errorBody("denied", 403);
      const upload = this.byId(fin[1]!);
      if (upload === undefined) return errorBody("not_found", 404);
      const whole = Buffer.concat(upload.parts);
      if (upload.parts.length !== upload.part_count || whole.byteLength !== upload.bytes || sha(whole) !== upload.reference_digest) {
        return errorBody("invalid_input", 400);
      }
      upload.state = "finalized";
      return { status: 200, body: { upload: this.view(upload), sha256: sha(whole), location: { kind: "verified" } } };
    }
    return errorBody("not_found", 404);
  }

  private byId(id: string): StoredUpload | undefined {
    return [...this.uploads.values()].find((upload) => upload.upload_id === id);
  }

  private view(upload: StoredUpload) {
    const { parts, ...rest } = upload;
    return { ...rest, part_bytes: PART, parts_received: parts.length, expires_at_ms: 1 };
  }

  /** A `fetch` over `handle`, applying any queued faults. */
  readonly fetch = (async (input: string | URL, init?: RequestInit) => {
    const url = new URL(String(input));
    const method = init?.method ?? "GET";
    const headers = new Headers(init?.headers);
    const raw = init?.body;
    const body = typeof raw === "string" ? new TextEncoder().encode(raw) : raw instanceof Uint8Array ? raw : new Uint8Array();
    this.requests.push({ method, path: url.pathname, authorization: headers.get("authorization") ?? undefined, body });
    const pending = this.faults.find((entry) => entry.left > 0 && entry.fault.match(method, url.pathname));
    if (pending !== undefined) {
      pending.left -= 1;
      const action = pending.fault.action;
      if (action.kind === "throw-before") throw new TypeError("fetch failed");
      if (action.kind === "respond") return new Response(JSON.stringify(action.body), { status: action.status, headers: action.headers ?? {} });
      this.handle(method, url.pathname, headers.get("authorization") ?? undefined, body);
      throw new TypeError("fetch failed");
    }
    const answer = this.handle(method, url.pathname, headers.get("authorization") ?? undefined, body);
    return new Response(JSON.stringify(answer.body), { status: answer.status, headers: { "content-type": "application/json" } });
  }) as typeof fetch;
}

export const unavailable = (retryAfter?: string) => ({
  kind: "respond" as const,
  status: 503,
  ...(retryAfter === undefined ? {} : { headers: { "retry-after": retryAfter } }),
  body: errorBody("unavailable", 503).body,
});

export interface Workspace {
  readonly input: string;
  readonly proofDir: string;
  readonly report: Uint8Array;
  readonly collectionId: string;
  readonly canonical: string;
  readonly observationId: string;
}

const PARITY = new URL("../parity/", import.meta.url);

/**
 * `collection-rig.json` with its report and proof digests rebound to real
 * files in `dir/proofs` (the report `reportBytes` long) and its
 * observation ID recomputed, so the collection validates and every mapped
 * file matches.
 */
export function rigWorkspace(dir: string, reportBytes = 346): Workspace {
  const proofDir = join(dir, "proofs");
  mkdirSync(proofDir, { recursive: true });
  const report = new Uint8Array(reportBytes).map((_, index) => 65 + (index % 26));
  const expected = new TextEncoder().encode('{"tests":["biscuit_hash::a"]}\n');
  const completion = new TextEncoder().encode('{"complete":true}\n');
  writeFileSync(join(proofDir, "biscuit-hash-L1.xml"), report);
  writeFileSync(join(proofDir, "biscuit-hash-L1.expected.json"), expected);
  writeFileSync(join(proofDir, "biscuit-hash-L1.completion.json"), completion);

  const document = JSON.parse(readFileSync(new URL("collection-rig.json", PARITY), "utf8")) as {
    observations: { observation_id: string; reports: { bytes: number; sha256: string }[]; proof: Record<string, string> }[];
  };
  const observation = document.observations[0]!;
  observation.reports[0]!.bytes = report.byteLength;
  observation.reports[0]!.sha256 = sha(report);
  observation.proof["expected_listing_digest"] = sha(expected);
  observation.proof["completion_record_digest"] = sha(completion);
  const loaded = loadStrict(new TextEncoder().encode(JSON.stringify(document)));
  if (!loaded.ok) throw new Error("fixture does not load");
  observation.observation_id = observationIdentity(((loaded.value as JsonObject)["observations"] as JsonObject[])[0]!);
  const input = join(dir, "collection.json");
  writeFileSync(input, `${JSON.stringify(document, null, 2)}\n`);
  const final = loadStrict(readFileSync(input));
  if (!final.ok) throw new Error("fixture does not load");
  return {
    input,
    proofDir,
    report,
    collectionId: collectionIdentity(final.value),
    canonical: canonical(final.value),
    observationId: observation.observation_id,
  };
}

/** The frozen parity file `name`. */
export function parityPath(name: string): string {
  return fileURLToPath(new URL(name, PARITY));
}

/** Rewrites the collection at `path` with `edit`, then recomputes every observation ID. */
export function rewriteCollection(path: string, edit: (document: { observations: Record<string, unknown>[] }) => void): void {
  const document = JSON.parse(readFileSync(path, "utf8")) as { observations: Record<string, unknown>[] };
  edit(document);
  const loaded = loadStrict(new TextEncoder().encode(JSON.stringify(document)));
  if (!loaded.ok) throw new Error("rewritten fixture does not load");
  ((loaded.value as JsonObject)["observations"] as JsonObject[]).forEach((observation, index) => {
    document.observations[index]!["observation_id"] = observationIdentity(observation);
  });
  writeFileSync(path, JSON.stringify(document));
}
