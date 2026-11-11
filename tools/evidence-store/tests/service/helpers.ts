// Synthetic fixtures for the storage tests. Payloads are canonical-shaped
// (sorted keys, no whitespace) but are not real contract output: storage
// treats them as opaque bytes apart from the generated columns.
import { env } from "cloudflare:workers";
import { expect } from "vitest";
import type { Principal } from "../../src/auth.ts";
import type { Role } from "../../src/contracts/api.ts";
import { canonical, loadStrict, type JsonObject } from "../../src/contracts/index.ts";
import { ingestCollectionBatch, type IngestCollectionInput, type IngestObservation } from "../../src/db/ingest.ts";
import { insertRepository } from "../../src/db/repositories.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";

export const db = (): D1Database => env.EVIDENCE_DB;

export function hex(length: number): string {
  const bytes = crypto.getRandomValues(new Uint8Array(Math.ceil(length / 2)));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0"))
    .join("")
    .slice(0, length);
}

/** A fresh repository per test, so tests never depend on storage isolation. */
export async function newRepository(): Promise<string> {
  const digits = crypto.getRandomValues(new Uint32Array(2));
  const repositoryId = `${1 + (digits[0]! % 9)}${String(digits[1]).padStart(10, "0")}`;
  await insertRepository(db(), {
    repository_id: repositoryId,
    slug: `owner/repo-${repositoryId}`,
    generation: hex(32),
  });
  return repositoryId;
}

export interface CellShape {
  package?: string;
  gate?: string;
  outcome?: string;
  variant?: string;
}

export function observation(id: string = hex(64), cell: CellShape = {}): IngestObservation {
  const payload =
    `{"cell":{"gate":${JSON.stringify(cell.gate ?? "L1")},` +
    `"gate_input_identity":"0123abcd",` +
    `"outcome":${JSON.stringify(cell.outcome ?? "pass")},` +
    `"package":${JSON.stringify(cell.package ?? "biscuit-hash")}},` +
    `"completion":"complete",` +
    `"observation_id":${JSON.stringify(id)}` +
    (cell.variant === undefined ? "" : `,"variant":${JSON.stringify(cell.variant)}`) +
    "}";
  return { observation_id: id, payload_json: payload };
}

export function sorted(observations: IngestObservation[]): IngestObservation[] {
  return [...observations].sort((a, b) => (a.observation_id < b.observation_id ? -1 : 1));
}

export const HEAD = "a".repeat(40);
export const TREE = "b".repeat(40);

export function collection(
  repositoryId: string,
  observations: IngestObservation[],
  overrides: Partial<IngestCollectionInput> = {},
): IngestCollectionInput {
  return {
    repository_id: repositoryId,
    collection_id: hex(64),
    environment: "ubuntu-latest",
    head: HEAD,
    tree: TREE,
    source_version: 3,
    principal: "credential:producer-a",
    request_id: crypto.randomUUID(),
    now_ms: 1_700_000_000_000,
    observations: sorted(observations),
    ...overrides,
  };
}

export interface RepositoryState {
  revision: number;
  observation_watermark: number;
  observations: string[];
  collections: string[];
  members: number;
  events: { kind: string; observation_id: string }[];
}

export async function repositoryState(repositoryId: string): Promise<RepositoryState> {
  const d = db();
  const repo = await d
    .prepare(
      "SELECT evidence_revision, observation_event_watermark FROM repositories WHERE repository_id = ?1",
    )
    .bind(repositoryId)
    .first<{ evidence_revision: number; observation_event_watermark: number }>();
  const ids = await d
    .prepare("SELECT observation_id FROM observations WHERE repository_id = ?1 ORDER BY observation_id")
    .bind(repositoryId)
    .all<{ observation_id: string }>();
  const collections = await d
    .prepare("SELECT collection_id FROM collections WHERE repository_id = ?1 ORDER BY collection_id")
    .bind(repositoryId)
    .all<{ collection_id: string }>();
  const members = await d
    .prepare("SELECT COUNT(*) AS n FROM collection_members WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<{ n: number }>();
  const events = await d
    .prepare("SELECT kind, observation_id FROM observation_events WHERE repository_id = ?1 ORDER BY seq")
    .bind(repositoryId)
    .all<{ kind: string; observation_id: string }>();
  return {
    revision: repo?.evidence_revision ?? -1,
    observation_watermark: repo?.observation_event_watermark ?? -1,
    observations: ids.results.map((row) => row.observation_id),
    collections: collections.results.map((row) => row.collection_id),
    members: members?.n ?? 0,
    events: events.results,
  };
}

/** Creates a certificate and a job in `state` whose members are `observationIds`. */
export async function retentionJob(
  repositoryId: string,
  state: string,
  observationIds: string[],
): Promise<string> {
  const d = db();
  const certificateId = crypto.randomUUID();
  const jobId = hex(32);
  await d.batch([
    d
      .prepare(
        `INSERT INTO retention_certificates(repository_id, certificate_id, certificate_json,
           certificate_sha256, generation, retention_epoch, evidence_revision,
           verifier_policy_digest, submitted_by, submitted_request_id, submitted_at_ms)
         VALUES (?1, ?2, '{}', ?3, 'g', 0, 0, ?3, 'retention-verifier:test', 'req', 1)`,
      )
      .bind(repositoryId, certificateId, hex(64)),
    d
      .prepare(
        `INSERT INTO retention_jobs(job_id, repository_id, certificate_id, state, generation,
           retention_epoch, evidence_revision, not_before_ms, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, 'g', 0, 0, 0, 1, 2)`,
      )
      .bind(jobId, repositoryId, certificateId, state),
    ...observationIds.map((id) =>
      d
        .prepare(
          `INSERT INTO retention_job_members(job_id, repository_id, observation_id, package,
             environment, gate, superseded_at_ms)
           VALUES (?1, ?2, ?3, 'biscuit-hash', 'ubuntu-latest', 'L1', 0)`,
        )
        .bind(jobId, repositoryId, id),
    ),
  ]);
  return jobId;
}

export async function rejection(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
  throw new Error("expected the statement to be refused");
}

// ---- Worker harness shared by the read, change-feed, and location tests ----

/** Stored principal JSON as `publisherKey` writes it; synthetic rows need it to render. */
export function credentialPublisher(id: string): string {
  return JSON.stringify({ kind: "credential", credential_id: id.padEnd(32, "0").slice(0, 32) });
}

export function principal(id: string, repositories: readonly string[], roles: readonly Role[]): Principal {
  return {
    publisher: { kind: "credential", credential_id: id.padEnd(32, "0").slice(0, 32) },
    grants: new Map(repositories.map((repo) => [repo, new Set(roles)])),
  };
}

export interface Harness {
  /** Sends a request as the principal named by `as` (default `reader`). */
  (path: string, init?: RequestInit & { as?: string }): Promise<Response>;
  /** Milliseconds the worker reads as now; tests may move it. */
  clock: number;
}

/**
 * A Worker whose authenticator resolves the `x-test-principal` header to one
 * of: `reader`, `producer`, `verifier`, `verifier-b`, each granted its role
 * on every repository in `repositories`.
 */
export function harness(repositories: readonly string[]): Harness {
  const principals: Record<string, Principal> = {
    reader: principal("cccc", repositories, ["reader"]),
    producer: principal("aaaa", repositories, ["producer"]),
    verifier: principal("dddd", repositories, ["verifier"]),
    "verifier-b": principal("eeee", repositories, ["verifier"]),
  };
  const request = ((path: string, init: RequestInit & { as?: string } = {}) =>
    worker.fetch!(
      new Request(`https://evidence.test${path}`, {
        ...init,
        headers: { "x-test-principal": init.as ?? "reader", ...(init.headers as Record<string, string>) },
      }) as never,
      env as unknown as Env,
      {} as ExecutionContext,
    )) as Harness;
  request.clock = 1_800_000_000_000;
  const worker = createWorker({
    authenticate: async (incoming) => {
      const who = principals[incoming.headers.get("x-test-principal") ?? ""];
      if (who === undefined) throw new Error("test request without a principal");
      return who;
    },
    now: () => request.clock,
  });
  return request;
}

/** POSTs `body` (an object, serialized with `JSON.stringify`) and returns the response. */
export function postJson(request: Harness, path: string, body: unknown, as?: string): Promise<Response> {
  return request(path, { method: "POST", body: typeof body === "string" ? body : JSON.stringify(body), ...(as === undefined ? {} : { as }) });
}

export interface ErrorEnvelope {
  error: { code: string; location: string | null; retryable: boolean; message: string; retry_after_ms: number | null };
  request_id: string;
}

export async function expectError(response: Response, status: number, code: string, location?: string | null): Promise<ErrorEnvelope> {
  const body = (await response.json()) as ErrorEnvelope;
  expect({ status: response.status, code: body.error.code }).toEqual({ status, code });
  if (location !== undefined) expect(body.error.location).toBe(location);
  return body;
}

/** Ingests synthetic observations directly (bypassing the API) with a renderable publisher. */
export async function seed(
  repositoryId: string,
  observations: IngestObservation[],
  overrides: Partial<IngestCollectionInput> = {},
): Promise<string[]> {
  const outcome = await ingestCollectionBatch(
    db(),
    collection(repositoryId, observations, { principal: credentialPublisher("aaaa"), ...overrides }),
  );
  if (outcome.kind !== "ok") throw new Error(`seed failed: ${outcome.kind}`);
  return outcome.result.accepted;
}

/** Deletes observations (and their location rows) through a sanctioned retention job. */
export async function prune(repositoryId: string, observationIds: string[]): Promise<string> {
  const jobId = await retentionJob(repositoryId, "deleting-rows", observationIds);
  const ids = JSON.stringify(observationIds);
  await db().batch([
    db()
      .prepare("DELETE FROM report_locations WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))")
      .bind(repositoryId, ids),
    db()
      .prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))")
      .bind(repositoryId, ids),
  ]);
  return jobId;
}

export interface IngestedFixture {
  collectionId: string;
  /** The fixture's first observation, as loaded. */
  observation: JsonObject;
  observationId: string;
  /** Its first report entry. */
  report: JsonObject;
}

/** Ingests the authentic `collection-rig` fixture through `POST /collections` as the producer. */
export async function ingestRig(request: Harness, repositoryId: string): Promise<IngestedFixture> {
  const loaded = loadStrict(new TextEncoder().encode(rigFixture));
  if (!loaded.ok) throw new Error("collection-rig fixture does not load");
  const fixture = loaded.value as JsonObject;
  const response = await postJson(request, `/v1/repos/${repositoryId}/collections`, `{"source_version":3,"collection":${canonical(fixture)}}`, "producer");
  if (response.status !== 200) throw new Error(`fixture ingestion answered ${response.status}`);
  const { collection_id: collectionId } = (await response.json()) as { collection_id: string };
  const observation = (fixture["observations"] as JsonObject[])[0]!;
  return {
    collectionId,
    observation,
    observationId: observation["observation_id"] as string,
    report: (observation["reports"] as JsonObject[])[0]!,
  };
}

/** A valid `LocationEventRequest` for `fixture`'s first report at a host directory. */
export function reportEvent(fixture: IngestedFixture, overrides: Record<string, unknown> = {}): Record<string, unknown> {
  const kind = (overrides["kind"] as string | undefined) ?? "verified";
  return {
    event_id: crypto.randomUUID(),
    observation_id: fixture.observationId,
    reference_kind: "report",
    reference_digest: fixture.report["sha256"],
    path: fixture.report["path"],
    locator: { kind: "host-dir", host: "rig-mac", directory: "/reports/run7" },
    kind,
    ...(kind === "verified" ? { verification_method: "bytes" } : {}),
    ...overrides,
  };
}
