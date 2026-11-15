// Fixtures shared by the retention tests: synthetic observations with
// controlled heads and retry links, an isolated retention-verifier
// certificate builder, and the D1/R2 state a certificate or job leaves.
import { env } from "cloudflare:workers";
import { expect } from "vitest";
import type { Snapshot } from "../../src/contracts/api.ts";
import { canonical, jsonNumber, loadStrict, observationIdentity, type JsonObject } from "../../src/contracts/index.ts";
import type { IngestObservation } from "../../src/db/ingest.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";
import { db, finalize, harness, hex, newRepository, payload, postJson, seed, sha256Hex, stage, type Harness } from "./helpers.ts";

export const CELL = { package: "biscuit-hash", environment: "ubuntu-latest", gate: "L1" } as const;
export const POLICY = { name: "isolated-test-verifier", version: "1", digest: "8".repeat(64) } as const;
export const DAY = 86_400_000;
export const GRACE = 14 * DAY;

export const headOf = (character: string): string => character.repeat(40);

export interface Seeded {
  readonly id: string;
  readonly head: string;
}

/** A synthetic observation of `CELL` (or another package/gate), optionally a retry of `retryOf`. */
export function synthetic(options: { outcome?: "pass" | "fail"; package?: string; gate?: string; retryOf?: string } = {}): IngestObservation {
  const id = hex(64);
  const payload = JSON.stringify({
    cell: {
      gate: options.gate ?? CELL.gate,
      gate_input_identity: "0123abcd",
      outcome: options.outcome ?? "pass",
      package: options.package ?? CELL.package,
    },
    completion: "complete",
    observation_id: id,
    ...(options.retryOf === undefined ? {} : { retry_of: options.retryOf }),
  });
  return { observation_id: id, payload_json: payload };
}

/** Stores `observation` at `head` (in `CELL.environment`) and returns its ID and head. */
export async function seedAt(repositoryId: string, head: string, observation: IngestObservation = synthetic()): Promise<Seeded> {
  await seed(repositoryId, [observation], { head });
  return { id: observation.observation_id, head };
}

export const certificatesPath = (repositoryId: string): string => `/v1/repos/${repositoryId}/retention/certificates`;
export const jobPath = (repositoryId: string, jobId: string): string => `/v1/repos/${repositoryId}/retention/jobs/${jobId}`;

export async function currentSnapshot(request: Harness, repositoryId: string): Promise<Snapshot> {
  const response = await request(`/v1/repos/${repositoryId}/snapshot`, { as: "retention-verifier" });
  expect(response.status).toBe(200);
  return (await response.json()) as Snapshot;
}

export function ancestry(older: Seeded, newer: Seeded): Record<string, unknown> {
  return {
    superseded_id: older.id,
    superseded_by_id: newer.id,
    evidence: { kind: "strict-ancestry", ancestor_head: older.head, descendant_head: newer.head },
  };
}

export function retry(chain: readonly string[]): Record<string, unknown> {
  return { superseded_id: chain[0], superseded_by_id: chain[chain.length - 1], evidence: { kind: "retry", chain } };
}

export function certifiedCell(
  candidates: readonly Seeded[],
  retained: readonly Seeded[],
  supersessions: readonly Record<string, unknown>[],
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    ...CELL,
    candidate_ids: candidates.map((observation) => observation.id),
    retained_ids: retained.map((observation) => observation.id),
    supersessions,
    ...overrides,
  };
}

export function certificate(
  snapshot: Snapshot,
  cells: readonly Record<string, unknown>[],
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return { certificate_id: crypto.randomUUID(), snapshot, cells, verifier_policy: POLICY, release_pins: [], ...overrides };
}

export function submit(request: Harness, repositoryId: string, body: unknown, as = "retention-verifier"): Promise<Response> {
  return postJson(request, certificatesPath(repositoryId), body, as);
}

export interface RetentionJobBody {
  job_id: string;
  certificate_id: string;
  state: string;
  snapshot: Snapshot;
  member_count: number;
  progress: { locations_withdrawn: number; objects_deleted: number; observations_deleted: number };
  not_before_ms: number;
  created_at_ms: number;
  updated_at_ms: number;
}

/** Every retention row a repository holds, to prove a refusal wrote nothing. */
export async function retentionRows(repositoryId: string): Promise<Record<string, number>> {
  return (await db()
    .prepare(
      `SELECT
         (SELECT COUNT(*) FROM retention_certificates WHERE repository_id = ?1) AS certificates,
         (SELECT COUNT(*) FROM retention_jobs WHERE repository_id = ?1) AS jobs,
         (SELECT COUNT(*) FROM retention_job_members WHERE repository_id = ?1) AS members,
         (SELECT COUNT(*) FROM release_pins WHERE repository_id = ?1) AS pins,
         (SELECT COUNT(*) FROM retention_supersessions WHERE repository_id = ?1) AS supersessions,
         (SELECT COUNT(*) FROM audit_log WHERE repository_id = ?1 AND action LIKE 'retention.%') AS audits`,
    )
    .bind(repositoryId)
    .first())!;
}

export const NO_RETENTION_ROWS = { certificates: 0, jobs: 0, members: 0, pins: 0, supersessions: 0, audits: 0 };

export async function jobState(jobId: string): Promise<string> {
  return (await db().prepare("SELECT state FROM retention_jobs WHERE job_id = ?1").bind(jobId).first<{ state: string }>())!.state;
}

// ---- Authentic observations with report bytes (the rig fixture, macOS) ----

export const RIG_CELL = { package: "biscuit-hash", environment: "macos-latest", gate: "L1" } as const;

export interface RigObservation extends Seeded {
  /** The `POST /collections` body. */
  readonly body: string;
  readonly report: { sha256: string; bytes: number; path: string };
}

/**
 * The rig fixture's observation at `head`, its first report rewritten to
 * describe `content`; `variant` makes two otherwise equal observations
 * differ, so two observations can cite identical bytes.
 */
export async function rigObservation(head: string, content: Uint8Array, variant: number): Promise<RigObservation> {
  const loaded = loadStrict(new TextEncoder().encode(rigFixture.replace(`"head": "${headOf("b")}"`, `"head": "${head}"`)));
  if (!loaded.ok) throw new Error("rig fixture does not load");
  const fixture = loaded.value as JsonObject;
  const observation = (fixture["observations"] as JsonObject[])[0]!;
  const report = (observation["reports"] as JsonObject[])[0]!;
  const sha256 = await sha256Hex(content);
  report["sha256"] = sha256;
  report["bytes"] = jsonNumber(String(content.byteLength));
  (observation["cell"] as JsonObject)["duration_s"] = jsonNumber(String(variant));
  observation["observation_id"] = observationIdentity(observation);
  return {
    id: observation["observation_id"] as string,
    head,
    body: `{"source_version":3,"collection":${canonical(fixture)}}`,
    report: { sha256, bytes: content.byteLength, path: report["path"] as string },
  };
}

export async function ingestRigObservation(request: Harness, repositoryId: string, observation: RigObservation): Promise<void> {
  const response = await postJson(request, `/v1/repos/${repositoryId}/collections`, observation.body, "producer");
  if (response.status !== 200) throw new Error(`ingestion answered ${response.status}: ${await response.text()}`);
}

export function rigUpload(observation: RigObservation): Record<string, unknown> {
  return {
    observation_id: observation.id,
    reference_kind: "report",
    reference_digest: observation.report.sha256,
    bytes: observation.report.bytes,
    path: observation.report.path,
  };
}

export const objectPath = (repositoryId: string, observation: RigObservation): string =>
  `/v1/repos/${repositoryId}/objects/${observation.id}/report/${observation.report.sha256}`;

/**
 * The invariant every crash must preserve: each finalized upload whose
 * newest object location event is `verified` still has its bytes in R2.
 */
export async function expectNoDanglingAvailability(repositoryId: string): Promise<void> {
  const { results } = await db()
    .prepare(
      `SELECT u.upload_id, u.staging_key FROM uploads u
       WHERE u.repository_id = ?1 AND u.state = 'finalized'
         AND (SELECT l.state FROM report_locations l
              WHERE l.repository_id = u.repository_id AND l.observation_id = u.observation_id
                AND l.location_kind = 'object' AND json_extract(l.locator_json, '$.upload_id') = u.upload_id
              ORDER BY l.seq DESC LIMIT 1) = 'verified'`,
    )
    .bind(repositoryId)
    .all<{ upload_id: string; staging_key: string }>();
  for (const row of results) {
    expect(await env.EVIDENCE_BUCKET.head(row.staging_key), `upload ${row.upload_id} is advertised but its bytes are gone`).not.toBeNull();
  }
}

// ---- A prune of one observation with bytes ----

export interface PruneScenario {
  repo: string;
  request: Harness;
  older: RigObservation;
  newer: RigObservation;
  /** Retained, maximal, with no upload: used to probe the cell lock. */
  extra: RigObservation;
  olderUpload: string;
  newerUpload: string;
  jobId: string;
  /** The snapshot the certificate certified, captured before pruning. */
  snapshot: Awaited<ReturnType<typeof currentSnapshot>>;
  /** When the grace has ended. */
  runAt: number;
}

/**
 * Two observations of one cell citing identical report bytes, each with its
 * own finalized object; `older` also has a host-directory location and a
 * stray staged part. A certificate prunes `older` (superseded on a
 * descendant head) and retains `newer` and `extra`.
 */
export async function pruneScenario(): Promise<PruneScenario> {
  const repo = await newRepository();
  const request = harness([repo]);
  const content = payload(700, 41);
  const older = await rigObservation(headOf("1"), content, 1);
  const newer = await rigObservation(headOf("2"), content, 2);
  const extra = await rigObservation(headOf("3"), payload(20, 42), 3);
  for (const observation of [older, newer, extra]) await ingestRigObservation(request, repo, observation);
  const olderUpload = await stage(request, repo, rigUpload(older), content);
  const newerUpload = await stage(request, repo, rigUpload(newer), content);
  for (const upload of [olderUpload, newerUpload]) expect((await finalize(request, repo, upload)).status).toBe(200);
  await env.EVIDENCE_BUCKET.put(`staging/${olderUpload}/9-${"0".repeat(64)}`, payload(5));
  const hostDir = await postJson(
    request,
    `/v1/repos/${repo}/locations`,
    {
      event_id: crypto.randomUUID(),
      observation_id: older.id,
      reference_kind: "report",
      reference_digest: older.report.sha256,
      path: older.report.path,
      locator: { kind: "host-dir", host: "rig-mac", directory: "/reports/run7" },
      kind: "verified",
      verification_method: "bytes",
    },
    "verifier",
  );
  expect(hostDir.status).toBe(201);
  const snapshot = await currentSnapshot(request, repo);
  const cells = [{ ...certifiedCell([older, newer, extra], [newer, extra], [ancestry(older, newer)]), ...RIG_CELL }];
  const response = await submit(request, repo, certificate(snapshot, cells));
  expect(response.status).toBe(202);
  const { job_id: jobId } = (await response.json()) as RetentionJobBody;
  return { repo, request, older, newer, extra, olderUpload, newerUpload, jobId, snapshot, runAt: request.clock + GRACE };
}
