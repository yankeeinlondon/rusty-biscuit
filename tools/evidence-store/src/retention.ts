//! Verifier-driven retention: certificates, persisted prune jobs, and the
//! bounded job runner. `api/README.md`, "Retention", describes the contract.
//!
//! A retention verifier (a separately provisioned credential; none exists in
//! production yet) submits a certificate naming an exact current snapshot and,
//! per `{package, environment, gate}` cell, every stored observation, the ones
//! to retain, and the supersession edges that justify pruning the rest. The
//! service cannot see Git history, so it trusts the verifier's ancestry
//! claims but binds them to the stored heads, checks retry chains against the
//! stored `retry_of` links, and refuses anything else it can check: a
//! partial or stale candidate set, an unsuperseded (maximal) observation left
//! out of the retained set, a cycle, a cross-cell edge, or a release-pinned
//! member. Supersession is never inferred from arrival time or producer
//! fields.
//!
//! An accepted certificate becomes a job that the daily run advances:
//!
//! scheduled → withdrawing → deleting-objects → deleting-rows → completed
//!
//! or scheduled → invalidated when the repository's revision moved before the
//! job started. Bytes become unreachable (withdrawn) before they are deleted,
//! and rows are deleted last, so a crash at any point never leaves a row
//! advertising deleted bytes; every step is idempotent and resumes from the
//! persisted state and counters.

import { publisherKey, type Principal, type Publisher } from "./auth.ts";
import { LIMITS, type Snapshot } from "./contracts/api.ts";
import { canonical, identity } from "./contracts/canonical.ts";
import { ENVIRONMENTS, GATES } from "./contracts/evidence.ts";
import { childPointer, type JsonObject, type JsonValue } from "./contracts/json.ts";
import type { AuditActor, AuditEntry } from "./db/audit.ts";
import { readRepository } from "./db/reads.ts";
import {
  advanceObjectsDeleted,
  appendWithdrawals,
  completeJob,
  deleteMemberRows,
  lockAndStartDeleting,
  readCellObservations,
  readJob,
  readJobByCertificate,
  readJobState,
  readMembersWithRows,
  readMemberUploads,
  readObservationFacts,
  readPendingWithdrawals,
  readUnfinishedJobs,
  readUploadsOf,
  releaseTerminalLocks,
  startDeletingRows,
  startOrInvalidate,
  TERMINAL_STATES,
  writeCertificate,
  type CellObservationRow,
  type MemberUpload,
  type RetentionJobRow,
  type RetentionJobState,
} from "./db/retention.ts";
import type { Env } from "./env.ts";
import { ApiError, jsonResponse } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import { readMaintenance } from "./maintenance.ts";
import {
  FORMATS,
  readRequestObject,
  requiredInteger,
  requiredObject,
  requiredString,
  requiredStringArray,
  requireObject,
} from "./requests.ts";

const DAY_MS = 86_400_000;
const GRACE_MS = LIMITS.retention_grace_days * DAY_MS;

// ---- Cell locks --------------------------------------------------------------

/** True when `error` is a D1 failure raised by a cell-lock trigger (migrations 0002 and 0008). */
export function isCellLocked(error: unknown): boolean {
  return error instanceof Error && error.message.includes("cell_locked");
}

/** How long a writer refused by a cell lock waits; the job's bounded batches finish and release it. */
const CELL_LOCK_RETRY_MS = 60_000;

/**
 * The answer to a write into a cell a retention job is deleting in: 503
 * `unavailable`, retryable, because the lock is released when the job's
 * bounded deletion batches finish. Nothing was recorded.
 */
export function cellLockedError(): ApiError {
  return new ApiError("unavailable", "a retention job is pruning this cell; nothing was recorded, retry later", null, CELL_LOCK_RETRY_MS);
}

// ---- Certificate parsing --------------------------------------------------------

const CERTIFICATE_KEYS = ["certificate_id", "snapshot", "cells", "verifier_policy", "release_pins"] as const;
const SNAPSHOT_KEYS = [
  "repository_id",
  "generation",
  "retention_epoch",
  "evidence_revision",
  "observation_event_watermark",
  "location_event_watermark",
] as const;
const CELL_KEYS = ["package", "environment", "gate", "candidate_ids", "retained_ids", "supersessions"] as const;
const EDGE_KEYS = ["superseded_id", "superseded_by_id", "evidence"] as const;
const EVIDENCE_KEYS = { "strict-ancestry": ["kind", "ancestor_head", "descendant_head"], retry: ["kind", "chain"] } as const;
const ANY_EVIDENCE_KEY = ["kind", "ancestor_head", "descendant_head", "chain"] as const;
const POLICY_KEYS = ["name", "version", "digest"] as const;
const PIN_KEYS = ["release", "observation_ids"] as const;

const COUNTER = { min: 0, max: Number.MAX_SAFE_INTEGER } as const;
const OBSERVATION_ID = { pattern: FORMATS.observationId } as const;

type Evidence =
  | { readonly kind: "strict-ancestry"; readonly ancestor_head: string; readonly descendant_head: string }
  | { readonly kind: "retry"; readonly chain: readonly string[] };

interface Edge {
  readonly pointer: string;
  readonly superseded_id: string;
  readonly superseded_by_id: string;
  readonly evidence: Evidence;
}

interface CertifiedCell {
  readonly pointer: string;
  readonly package: string;
  readonly environment: string;
  readonly gate: string;
  readonly candidate_ids: readonly string[];
  readonly retained_ids: readonly string[];
  readonly supersessions: readonly Edge[];
}

interface ReleasePin {
  readonly pointer: string;
  readonly release: string;
  readonly observation_ids: readonly string[];
}

export interface Certificate {
  readonly certificate_id: string;
  readonly snapshot: Snapshot;
  readonly cells: readonly CertifiedCell[];
  readonly verifier_policy: { readonly name: string; readonly version: string; readonly digest: string };
  readonly release_pins: readonly ReleasePin[];
  /** The certificate's canonical JSON, stored verbatim; its SHA-256 decides replay equality. */
  readonly canonical: string;
}

function invalid(location: string, message: string): ApiError {
  return new ApiError("invalid_input", message, location);
}

/**
 * A required array member whose elements are validated by the caller.
 * Absent and explicit `null` are refused like every other field; `[]` is
 * valid and an array is never filtered.
 */
function requiredArray(object: JsonObject, key: string, pointer: string): JsonValue[] {
  const location = childPointer(pointer, key);
  if (!Object.hasOwn(object, key)) throw invalid(location, `${key} is required`);
  const value = object[key]!;
  if (value === null) throw invalid(location, `${key} must not be null; omit it instead`);
  if (!Array.isArray(value)) throw invalid(location, `${location} must be an array`);
  return value;
}

function readSnapshot(body: JsonObject, repositoryId: string): Snapshot {
  const snapshot = requiredObject(body, "snapshot", "", SNAPSHOT_KEYS);
  const read: Snapshot = {
    repository_id: requiredString(snapshot, "repository_id", "/snapshot", { pattern: FORMATS.repositoryId }),
    generation: requiredString(snapshot, "generation", "/snapshot"),
    retention_epoch: requiredInteger(snapshot, "retention_epoch", "/snapshot", COUNTER),
    evidence_revision: requiredInteger(snapshot, "evidence_revision", "/snapshot", COUNTER),
    observation_event_watermark: requiredInteger(snapshot, "observation_event_watermark", "/snapshot", COUNTER),
    location_event_watermark: requiredInteger(snapshot, "location_event_watermark", "/snapshot", COUNTER),
  };
  if (read.repository_id !== repositoryId) throw invalid("/snapshot/repository_id", "snapshot belongs to another repository");
  return read;
}

function readEvidence(edge: JsonObject, pointer: string): Evidence {
  const location = childPointer(pointer, "evidence");
  const evidence = requiredObject(edge, "evidence", pointer, ANY_EVIDENCE_KEY);
  const kind = requiredString(evidence, "kind", location, { oneOf: ["strict-ancestry", "retry"] }) as Evidence["kind"];
  requireObject(evidence, location, EVIDENCE_KEYS[kind]);
  if (kind === "strict-ancestry") {
    return {
      kind,
      ancestor_head: requiredString(evidence, "ancestor_head", location, { pattern: FORMATS.gitObjectId }),
      descendant_head: requiredString(evidence, "descendant_head", location, { pattern: FORMATS.gitObjectId }),
    };
  }
  const chain = requiredStringArray(evidence, "chain", location, OBSERVATION_ID, { unique: true });
  if (chain.length < 2) throw invalid(childPointer(location, "chain"), "a retry chain names at least the superseded and the superseding observation");
  return { kind, chain };
}

function readCell(item: JsonValue, pointer: string): CertifiedCell {
  const cell = requireObject(item, pointer, CELL_KEYS);
  return {
    pointer,
    package: requiredString(cell, "package", pointer),
    environment: requiredString(cell, "environment", pointer, { oneOf: ENVIRONMENTS }),
    gate: requiredString(cell, "gate", pointer, { oneOf: GATES }),
    candidate_ids: requiredStringArray(cell, "candidate_ids", pointer, OBSERVATION_ID, { unique: true }),
    retained_ids: (() => {
      const retained = requiredStringArray(cell, "retained_ids", pointer, OBSERVATION_ID, { unique: true });
      if (retained.length === 0) throw invalid(childPointer(pointer, "retained_ids"), "retained_ids must name at least one observation");
      return retained;
    })(),
    supersessions: requiredArray(cell, "supersessions", pointer).map((value, index) => {
      const edgePointer = childPointer(childPointer(pointer, "supersessions"), index);
      const edge = requireObject(value, edgePointer, EDGE_KEYS);
      return {
        pointer: edgePointer,
        superseded_id: requiredString(edge, "superseded_id", edgePointer, OBSERVATION_ID),
        superseded_by_id: requiredString(edge, "superseded_by_id", edgePointer, OBSERVATION_ID),
        evidence: readEvidence(edge, edgePointer),
      };
    }),
  };
}

function readPin(item: JsonValue, pointer: string): ReleasePin {
  const pin = requireObject(item, pointer, PIN_KEYS);
  return {
    pointer,
    release: requiredString(pin, "release", pointer),
    observation_ids: requiredStringArray(pin, "observation_ids", pointer, OBSERVATION_ID, { unique: true }),
  };
}

/** Strictly parses a certificate body; every problem is 400 located by JSON pointer. */
export function readCertificate(bytes: Uint8Array, repositoryId: string): Certificate {
  const body = readRequestObject(bytes, CERTIFICATE_KEYS);
  const certificateId = requiredString(body, "certificate_id", "", { pattern: FORMATS.uuid });
  const snapshot = readSnapshot(body, repositoryId);
  const cells = requiredArray(body, "cells", "").map((item, index) => readCell(item, childPointer("/cells", index)));
  const policy = requiredObject(body, "verifier_policy", "", POLICY_KEYS);
  const verifierPolicy = {
    name: requiredString(policy, "name", "/verifier_policy"),
    version: requiredString(policy, "version", "/verifier_policy"),
    digest: requiredString(policy, "digest", "/verifier_policy", { pattern: FORMATS.sha256 }),
  };
  const releases = new Set<string>();
  const pins = requiredArray(body, "release_pins", "").map((item, index) => {
    const pin = readPin(item, childPointer("/release_pins", index));
    if (releases.has(pin.release)) throw invalid(childPointer(pin.pointer, "release"), `release ${JSON.stringify(pin.release)} is listed twice`);
    releases.add(pin.release);
    return pin;
  });
  return { certificate_id: certificateId, snapshot, cells, verifier_policy: verifierPolicy, release_pins: pins, canonical: canonical(body) };
}

// ---- Certificate checks ---------------------------------------------------------

const cellKey = (cell: { package: string; environment: string; gate: string }) => JSON.stringify([cell.package, cell.environment, cell.gate]);

/** A certificate snapshot must be exactly the current repository row. */
function checkSnapshot(snapshot: Snapshot, current: Snapshot): void {
  if (snapshot.generation !== current.generation) {
    throw new ApiError("stale_snapshot", "snapshot predates a store restore; take a fresh snapshot", "/snapshot/generation");
  }
  if (snapshot.retention_epoch < current.retention_epoch) {
    throw new ApiError("snapshot_pruned", "evidence was pruned after this snapshot was taken", "/snapshot/retention_epoch");
  }
  const counters = ["retention_epoch", "evidence_revision", "observation_event_watermark", "location_event_watermark"] as const;
  for (const key of counters) {
    if (snapshot[key] > current[key]) throw invalid(`/snapshot/${key}`, `snapshot ${key} is ahead of the repository`);
  }
  for (const key of counters) {
    if (snapshot[key] !== current[key]) {
      throw new ApiError("stale_snapshot", `the repository changed after this snapshot (${key}); certify a fresh snapshot`, `/snapshot/${key}`);
    }
  }
}

interface Member {
  readonly observation_id: string;
  readonly package: string;
  readonly environment: string;
  readonly gate: string;
  /** Where the certificate names it, for a located refusal. */
  readonly pointer: string;
}

/** Throws at the first edge that closes a cycle in one cell's supersession graph. */
function checkAcyclic(cell: CertifiedCell): void {
  const next = new Map<string, string[]>();
  for (const edge of cell.supersessions) next.set(edge.superseded_id, [...(next.get(edge.superseded_id) ?? []), edge.superseded_by_id]);
  const state = new Map<string, "open" | "done">();
  const visit = (id: string): boolean => {
    if (state.get(id) === "done") return false;
    if (state.get(id) === "open") return true;
    state.set(id, "open");
    for (const successor of next.get(id) ?? []) if (visit(successor)) return true;
    state.set(id, "done");
    return false;
  };
  for (const id of next.keys()) {
    if (visit(id)) throw invalid(childPointer(cell.pointer, "supersessions"), "supersession edges form a cycle");
  }
}

function checkEdge(cell: CertifiedCell, edge: Edge, stored: ReadonlyMap<string, CellObservationRow>, seen: Set<string>): void {
  const at = (field: string) => childPointer(edge.pointer, field);
  const candidates = new Set(cell.candidate_ids);
  for (const field of ["superseded_id", "superseded_by_id"] as const) {
    if (!candidates.has(edge[field])) throw invalid(at(field), `${edge[field]} is not a candidate of this cell; supersession never crosses cells`);
  }
  if (edge.superseded_id === edge.superseded_by_id) throw invalid(at("superseded_by_id"), "an observation cannot supersede itself");
  const pair = `${edge.superseded_id}>${edge.superseded_by_id}`;
  if (seen.has(pair)) throw invalid(edge.pointer, "this supersession edge is listed twice");
  seen.add(pair);

  const evidencePointer = at("evidence");
  const superseded = stored.get(edge.superseded_id)!;
  const superseding = stored.get(edge.superseded_by_id)!;
  if (edge.evidence.kind === "strict-ancestry") {
    // Ancestry itself is the verifier's claim; the heads it names must be the stored ones.
    if (edge.evidence.ancestor_head !== superseded.head) {
      throw invalid(childPointer(evidencePointer, "ancestor_head"), `ancestor_head is not the stored head of ${edge.superseded_id}`);
    }
    if (edge.evidence.descendant_head !== superseding.head) {
      throw invalid(childPointer(evidencePointer, "descendant_head"), `descendant_head is not the stored head of ${edge.superseded_by_id}`);
    }
    if (superseded.head === superseding.head) {
      throw invalid(childPointer(evidencePointer, "descendant_head"), "both observations share one head: that is a conflict, never a strict-ancestry supersession");
    }
    return;
  }
  const { chain } = edge.evidence;
  const chainPointer = childPointer(evidencePointer, "chain");
  if (chain[0] !== edge.superseded_id) throw invalid(childPointer(chainPointer, 0), "a retry chain starts at the superseded observation");
  if (chain[chain.length - 1] !== edge.superseded_by_id) {
    throw invalid(childPointer(chainPointer, chain.length - 1), "a retry chain ends at the superseding observation");
  }
  for (const [index, id] of chain.entries()) {
    if (!candidates.has(id)) throw invalid(childPointer(chainPointer, index), `${id} is not a candidate of this cell`);
    if (index === 0) continue;
    const link = stored.get(id)!.retry_of;
    if (link !== chain[index - 1]) {
      throw invalid(childPointer(chainPointer, index), `the stored retry_of of ${id} is ${link === null ? "absent" : link}, not ${chain[index - 1]}`);
    }
  }
}

/**
 * Checks every cell against the stored observations and returns the prune
 * set. A candidate set that differs from the store answers 409
 * `stale_snapshot` (the verifier's view is partial or stale); every other
 * problem is the certificate's own and answers 400.
 */
async function checkCells(db: D1Database, repositoryId: string, certificate: Certificate): Promise<Member[]> {
  const seenCells = new Set<string>();
  for (const cell of certificate.cells) {
    if (seenCells.has(cellKey(cell))) throw invalid(cell.pointer, "this cell is listed twice");
    seenCells.add(cellKey(cell));
  }
  const total = certificate.cells.reduce((sum, cell) => sum + cell.candidate_ids.length, 0);
  const rows =
    certificate.cells.length === 0
      ? []
      : await readCellObservations(
          db,
          repositoryId,
          certificate.cells.map((cell) => ({ package: cell.package, environment: cell.environment, gate: cell.gate })),
          total + 1,
        );
  const byCell = new Map<string, Map<string, CellObservationRow>>();
  for (const row of rows) {
    const key = cellKey(row);
    if (!byCell.has(key)) byCell.set(key, new Map());
    byCell.get(key)!.set(row.observation_id, row);
  }

  const members: Member[] = [];
  for (const cell of certificate.cells) {
    const stored = byCell.get(cellKey(cell)) ?? new Map<string, CellObservationRow>();
    const candidates = new Set(cell.candidate_ids);
    for (const [index, id] of cell.candidate_ids.entries()) {
      if (!stored.has(id)) {
        throw new ApiError(
          "stale_snapshot",
          `missing store data: observation ${id} is not stored in this cell; certify only what the snapshot holds`,
          childPointer(childPointer(cell.pointer, "candidate_ids"), index),
        );
      }
    }
    for (const id of stored.keys()) {
      if (!candidates.has(id)) {
        throw new ApiError(
          "stale_snapshot",
          `candidate_ids omits stored observation ${id}; a certificate must name every observation of the cell`,
          childPointer(cell.pointer, "candidate_ids"),
        );
      }
    }
    for (const [index, id] of cell.retained_ids.entries()) {
      if (!candidates.has(id)) throw invalid(childPointer(childPointer(cell.pointer, "retained_ids"), index), `${id} is not a candidate of this cell`);
    }
    const seenEdges = new Set<string>();
    for (const edge of cell.supersessions) checkEdge(cell, edge, stored, seenEdges);
    checkAcyclic(cell);

    const superseded = new Set(cell.supersessions.map((edge) => edge.superseded_id));
    const retained = new Set(cell.retained_ids);
    for (const [index, id] of cell.candidate_ids.entries()) {
      if (retained.has(id)) continue;
      const pointer = childPointer(childPointer(cell.pointer, "candidate_ids"), index);
      if (!superseded.has(id)) {
        throw invalid(pointer, `observation ${id} is neither retained nor superseded; an unsuperseded observation must be retained`);
      }
      members.push({ observation_id: id, package: cell.package, environment: cell.environment, gate: cell.gate, pointer });
    }
  }
  return members;
}

/** Release pins must name stored observations, and no pinned observation may be pruned. */
async function checkPins(db: D1Database, repositoryId: string, certificate: Certificate, members: readonly Member[]): Promise<void> {
  const pinned = certificate.release_pins.flatMap((pin) => pin.observation_ids);
  const facts = await readObservationFacts(db, repositoryId, [...new Set([...pinned, ...members.map((member) => member.observation_id)])]);
  const memberIds = new Set(members.map((member) => member.observation_id));
  for (const pin of certificate.release_pins) {
    for (const [index, id] of pin.observation_ids.entries()) {
      const pointer = childPointer(childPointer(pin.pointer, "observation_ids"), index);
      if (!facts.stored.has(id)) throw invalid(pointer, `release pin names observation ${id}, which is not stored in this repository`);
      if (memberIds.has(id)) throw invalid(pointer, `observation ${id} is pinned by release ${JSON.stringify(pin.release)} and cannot be pruned`);
      const job = facts.scheduled.get(id);
      if (job !== undefined) throw invalid(pointer, `observation ${id} is a member of unfinished retention job ${job} and cannot be pinned until it ends`);
    }
  }
  for (const member of members) {
    const release = facts.pinned.get(member.observation_id);
    if (release !== undefined) {
      throw invalid(member.pointer, `observation ${member.observation_id} is pinned by release ${JSON.stringify(release)} and cannot be pruned`);
    }
  }
}

// ---- Routes ------------------------------------------------------------------------

/** The OpenAPI `RetentionJob` shape. */
function renderJob(job: RetentionJobRow): Record<string, unknown> {
  const stored = JSON.parse(job.certificate_json) as { snapshot: Snapshot };
  const { snapshot } = stored;
  return {
    job_id: job.job_id,
    certificate_id: job.certificate_id,
    state: job.state,
    snapshot: {
      repository_id: snapshot.repository_id,
      generation: snapshot.generation,
      retention_epoch: snapshot.retention_epoch,
      evidence_revision: snapshot.evidence_revision,
      observation_event_watermark: snapshot.observation_event_watermark,
      location_event_watermark: snapshot.location_event_watermark,
    },
    member_count: job.member_count,
    progress: {
      locations_withdrawn: job.locations_withdrawn,
      objects_deleted: job.objects_deleted,
      observations_deleted: job.observations_deleted,
    },
    not_before_ms: job.not_before_ms,
    created_at_ms: job.created_at_ms,
    updated_at_ms: job.updated_at_ms,
  };
}

/** An earlier submission of the same certificate ID: identical → 200, different → 409. */
function replay(context: RequestContext, existing: RetentionJobRow, certificate: Certificate): Response {
  if (existing.certificate_sha256 !== identity(certificate.canonical)) {
    throw new ApiError(
      "identity_conflict",
      `certificate ${certificate.certificate_id} is already recorded with different content; nothing was recorded`,
      "/certificate_id",
    );
  }
  return jsonResponse(renderJob(existing), 200, context.requestId);
}

function randomServerId(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** `POST /retention/certificates`: every check runs before the single write. */
export async function submitRetentionCertificate(context: RequestContext, repositoryId: string, bytes: Uint8Array): Promise<Response> {
  const certificate = readCertificate(bytes, repositoryId);
  const current = await readRepository(context.db, repositoryId);
  if (current === null) throw new ApiError("not_found", `repository ${repositoryId} is not registered`);
  const existing = await readJobByCertificate(context.db, repositoryId, certificate.certificate_id);
  if (existing !== null) return replay(context, existing, certificate);
  checkSnapshot(certificate.snapshot, current);
  const members = await checkCells(context.db, repositoryId, certificate);
  await checkPins(context.db, repositoryId, certificate, members);

  const jobId = randomServerId();
  const supersededIds = [...new Set(certificate.cells.flatMap((cell) => cell.supersessions.map((edge) => edge.superseded_id)))];
  const pins = certificate.release_pins.flatMap((pin) => pin.observation_ids.map((id) => ({ release: pin.release, observation_id: id })));
  let outcome;
  try {
    outcome = await writeCertificate(
      context.db,
      {
        repository_id: repositoryId,
        certificate_id: certificate.certificate_id,
        certificate_json: certificate.canonical,
        certificate_sha256: identity(certificate.canonical),
        snapshot: certificate.snapshot,
        verifier_policy_digest: certificate.verifier_policy.digest,
        submitted_by: publisherKey(context.principal.publisher),
        request_id: context.requestId,
        now_ms: context.nowMs,
        job_id: jobId,
        grace_ms: GRACE_MS,
        superseded_ids: supersededIds,
        members: members.map(({ observation_id, package: name, environment, gate }) => ({ observation_id, package: name, environment, gate })),
        pins,
      },
      context,
      {
        action: "retention.certificate",
        repositoryId,
        detail: { certificate_id: certificate.certificate_id, job_id: jobId, member_count: members.length, verifier_policy: certificate.verifier_policy },
      },
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/UNIQUE|PRIMARY KEY/.test(message)) {
      // A concurrent submission of the same certificate ID won.
      const raced = await readJobByCertificate(context.db, repositoryId, certificate.certificate_id);
      if (raced !== null) return replay(context, raced, certificate);
    }
    if (message.includes("release_pinned") || message.includes("retention_member")) {
      throw invalid("/release_pins", "a release pin and a scheduled prune now name the same observation; nothing was recorded");
    }
    throw error;
  }
  if (outcome === "stale") {
    throw new ApiError("stale_snapshot", "the repository changed while the certificate was checked; certify a fresh snapshot", "/snapshot");
  }
  const job = await readJob(context.db, repositoryId, jobId);
  if (job === null) throw new Error(`retention job ${jobId} vanished after insert`);
  return jsonResponse(renderJob(job), 202, context.requestId);
}

/** `GET /retention/jobs/{job_id}`. */
export async function getRetentionJob(context: RequestContext, repositoryId: string, jobId: string): Promise<Response> {
  const job = await readJob(context.db, repositoryId, jobId);
  if (job === null) throw new ApiError("not_found", `retention job ${jobId} does not exist in repository ${repositoryId}`);
  return jsonResponse(renderJob(job), 200, context.requestId);
}

// ---- Job runner ------------------------------------------------------------------------

export interface RetentionJobStep {
  readonly job_id: string;
  readonly repository_id: string;
  readonly from_state: string;
  readonly to_state: string;
}

export interface RetentionRunReport {
  /** Every job this run moved, in the order it moved them. */
  readonly steps: readonly RetentionJobStep[];
  /** Cell locks released because their job was completed or invalidated. */
  readonly released_locks: number;
  /** True when the run did nothing because the store was quiesced. */
  readonly skipped_for_maintenance: boolean;
  /** Bounded batches performed. */
  readonly batches: number;
  /** True when `maxBatches` ran out before every startable job finished; the next run continues. */
  readonly exhausted: boolean;
}

export interface RetentionRunOptions {
  /** Advance only this job. */
  readonly jobId?: string;
  /** Upper bound on bounded batches this run performs. */
  readonly maxBatches?: number;
}

/** Default batch budget of one run; each batch is one bounded D1 or R2 round of work. */
export const DEFAULT_MAX_BATCHES = 400;
const WITHDRAWAL_BATCH = 100;
const UPLOAD_BATCH = 50;
const MEMBER_BATCH = 25;
const R2_DELETE_LIMIT = 1000;

/** The UUID-shaped event ID of the job's withdrawal of one newest location row; a replay reuses it. */
export function withdrawalEventId(jobId: string, locationSeq: number): string {
  const digest = identity(`withdraw:${jobId}:${locationSeq}`);
  return `${digest.slice(0, 8)}-${digest.slice(8, 12)}-${digest.slice(12, 16)}-${digest.slice(16, 20)}-${digest.slice(20, 32)}`;
}

/** The submitting retention verifier is the actor of the job's audit rows. */
function jobActor(job: RetentionJobRow, nowMs: number): AuditActor {
  const principal: Principal = { publisher: JSON.parse(job.submitted_by) as Publisher, grants: new Map() };
  return { principal, requestId: `retention-job:${job.job_id}`, nowMs };
}

function jobAudit(job: RetentionJobRow, state: RetentionJobState): AuditEntry {
  return { action: "retention.job", repositoryId: job.repository_id, detail: { job_id: job.job_id, certificate_id: job.certificate_id, state } };
}

/** Deletes every final object and staged part of `uploads` (R2 deletion is idempotent). */
async function deleteUploadObjects(bucket: R2Bucket, uploads: readonly MemberUpload[]): Promise<void> {
  const keys = uploads.map((upload) => upload.staging_key);
  for (const upload of uploads) {
    // Listed rather than read from `upload_parts`, so a part whose row was never written is found too.
    let cursor: string | undefined;
    do {
      const page = await bucket.list({ prefix: `staging/${upload.upload_id}/`, ...(cursor === undefined ? {} : { cursor }) });
      keys.push(...page.objects.map((object) => object.key));
      cursor = page.truncated ? page.cursor : undefined;
    } while (cursor !== undefined);
  }
  for (let start = 0; start < keys.length; start += R2_DELETE_LIMIT) {
    await bucket.delete(keys.slice(start, start + R2_DELETE_LIMIT));
  }
}

interface Advance {
  /** True when the job cannot move now (grace not over, or a cell lock is held by another job). */
  readonly wait: boolean;
  readonly releasedLocks: number;
}

/** Performs one bounded batch of `job`'s current state. */
async function advanceOnce(env: Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET">, job: RetentionJobRow, nowMs: number): Promise<Advance> {
  const db = env.EVIDENCE_DB;
  switch (job.state) {
    case "scheduled":
      if (nowMs < job.not_before_ms) return { wait: true, releasedLocks: 0 };
      await startOrInvalidate(db, job.job_id, nowMs, jobActor(job, nowMs), jobAudit(job, "invalidated"));
      return { wait: false, releasedLocks: 0 };
    case "withdrawing": {
      const pending = await readPendingWithdrawals(db, job.job_id, WITHDRAWAL_BATCH);
      if (pending.length > 0) {
        await appendWithdrawals(db, job, pending.map((row) => ({ seq: row.seq, event_id: withdrawalEventId(job.job_id, row.seq) })), nowMs);
        return { wait: false, releasedLocks: 0 };
      }
      await lockAndStartDeleting(db, job.job_id, nowMs);
      if ((await readJobState(db, job.job_id)) !== "withdrawing") return { wait: false, releasedLocks: 0 };
      // Still withdrawing: either a location was verified meanwhile (withdraw it next) or another job holds a cell.
      return { wait: (await readPendingWithdrawals(db, job.job_id, 1)).length === 0, releasedLocks: 0 };
    }
    case "deleting-objects": {
      const uploads = await readMemberUploads(db, job.job_id, job.objects_deleted, UPLOAD_BATCH);
      if (uploads.length === 0) {
        await startDeletingRows(db, job, nowMs);
        return { wait: false, releasedLocks: 0 };
      }
      await deleteUploadObjects(env.EVIDENCE_BUCKET, uploads);
      await advanceObjectsDeleted(db, job.job_id, job.objects_deleted, uploads.length, nowMs);
      return { wait: false, releasedLocks: 0 };
    }
    case "deleting-rows": {
      const members = await readMembersWithRows(db, job.job_id, MEMBER_BATCH);
      if (members.length === 0) {
        return { wait: false, releasedLocks: await completeJob(db, job.job_id, nowMs, jobActor(job, nowMs), jobAudit(job, "completed")) };
      }
      // Again, in case an upload finalized after the object phase left bytes behind (its event was refused under the lock).
      await deleteUploadObjects(env.EVIDENCE_BUCKET, await readUploadsOf(db, job.repository_id, members));
      await deleteMemberRows(db, job, members, nowMs);
      return { wait: false, releasedLocks: 0 };
    }
    case "completed":
    case "invalidated":
      return { wait: true, releasedLocks: 0 };
  }
}

/**
 * Advances every unfinished retention job by bounded, idempotent batches:
 * scheduled → withdrawing → deleting-objects → deleting-rows → completed,
 * or scheduled → invalidated when its revision is no longer current. Safe to
 * rerun after a crash at any boundary. Does nothing while the store is
 * quiesced. A thrown D1 or R2 failure leaves every job resumable.
 */
export async function advanceRetentionJobs(
  env: Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET">,
  nowMs: number,
  options: RetentionRunOptions = {},
): Promise<RetentionRunReport> {
  const db = env.EVIDENCE_DB;
  if ((await readMaintenance(db)).mode === "quiesced") {
    return { steps: [], released_locks: 0, skipped_for_maintenance: true, batches: 0, exhausted: false };
  }
  let releasedLocks = await releaseTerminalLocks(db);
  const budget = options.maxBatches ?? DEFAULT_MAX_BATCHES;
  let batches = 0;
  const steps: RetentionJobStep[] = [];
  for (const queued of await readUnfinishedJobs(db, options.jobId)) {
    let job: RetentionJobRow | null = queued;
    while (job !== null && !TERMINAL_STATES.includes(job.state) && batches < budget) {
      batches += 1;
      const advance = await advanceOnce(env, job, nowMs);
      releasedLocks += advance.releasedLocks;
      const after: RetentionJobRow | null = await readJob(db, job.repository_id, job.job_id);
      if (after !== null && after.state !== job.state) {
        steps.push({ job_id: job.job_id, repository_id: job.repository_id, from_state: job.state, to_state: after.state });
      }
      if (advance.wait) break;
      job = after;
    }
  }
  const exhausted = batches >= budget;
  return { steps, released_locks: releasedLocks, skipped_for_maintenance: false, batches, exhausted };
}
