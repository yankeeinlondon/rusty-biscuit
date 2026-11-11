//! Snapshot, observation, candidate-history, and change-feed reads.
//!
//! A list read is bounded by one snapshot: the current repository row, or a
//! snapshot the caller pinned from `GET /snapshot`. Its first page captures
//! the snapshot, and the signed cursor carries those bounds to every later
//! page, so a listing terminates at what it captured however many writes
//! arrive meanwhile. Stored canonical observation text is spliced into
//! responses verbatim so number lexemes survive.

import { LIMITS, type Snapshot } from "./contracts/api.ts";
import { GATES } from "./contracts/evidence.ts";
import type { JsonObject } from "./contracts/json.ts";
import { openCursor, queryDigest, signCursor, type CursorState } from "./cursor.ts";
import { readObservation, type ObservationRow } from "./db/ingest.ts";
import {
  readHistory,
  readLatestLocations,
  readLocationStream,
  readObservationEvents,
  readRepository,
  readRetriedBy,
  type LocationRow,
  type LocationStreamRow,
  type ObservationEventRow,
  type RepositoryRow,
} from "./db/reads.ts";
import { ApiError, jsonResponse, rawJsonResponse } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import {
  FORMATS,
  optionalInteger,
  optionalObject,
  optionalString,
  readRequestObject,
  requiredInteger,
  requiredString,
  requiredStringArray,
} from "./requests.ts";

const PAGE_SIZE = { min: 1, max: LIMITS.page_max_records } as const;
const COUNTER = { min: 0, max: Number.MAX_SAFE_INTEGER } as const;
const CURSOR_SPEC = { maxLength: 2048 } as const;
const SNAPSHOT_KEYS = [
  "repository_id",
  "generation",
  "retention_epoch",
  "evidence_revision",
  "observation_event_watermark",
  "location_event_watermark",
] as const;

/** Room left for the largest page trailer: `has_more` plus a full cursor. */
const TRAILER_RESERVE = 2_200;

const encoder = new TextEncoder();
const byteLength = (text: string): number => encoder.encode(text).byteLength;

async function repository(context: RequestContext, repositoryId: string): Promise<RepositoryRow> {
  const row = await readRepository(context.db, repositoryId);
  if (row === null) throw new ApiError("not_found", `repository ${repositoryId} is not registered`);
  return row;
}

/** The `Snapshot` fields of a repository row, in schema order. */
function snapshotOf(row: Snapshot): Snapshot {
  return {
    repository_id: row.repository_id,
    generation: row.generation,
    retention_epoch: row.retention_epoch,
    evidence_revision: row.evidence_revision,
    observation_event_watermark: row.observation_event_watermark,
    location_event_watermark: row.location_event_watermark,
  };
}

export async function getSnapshot(context: RequestContext, repositoryId: string): Promise<Response> {
  return jsonResponse(snapshotOf(await repository(context, repositoryId)), 200, context.requestId);
}

function readPinnedSnapshot(body: JsonObject, repositoryId: string): Snapshot | undefined {
  const pinned = optionalObject(body, "snapshot", "", SNAPSHOT_KEYS);
  if (pinned === undefined) return undefined;
  const snapshot: Snapshot = {
    repository_id: requiredString(pinned, "repository_id", "/snapshot", { pattern: FORMATS.repositoryId }),
    generation: requiredString(pinned, "generation", "/snapshot"),
    retention_epoch: requiredInteger(pinned, "retention_epoch", "/snapshot", COUNTER),
    evidence_revision: requiredInteger(pinned, "evidence_revision", "/snapshot", COUNTER),
    observation_event_watermark: requiredInteger(pinned, "observation_event_watermark", "/snapshot", COUNTER),
    location_event_watermark: requiredInteger(pinned, "location_event_watermark", "/snapshot", COUNTER),
  };
  if (snapshot.repository_id !== repositoryId) {
    throw new ApiError("invalid_input", "snapshot belongs to another repository", "/snapshot/repository_id");
  }
  return snapshot;
}

/** A pinned snapshot is usable only while it is a past state of the current row. */
function checkPinned(pinned: Snapshot, current: Snapshot): void {
  if (pinned.generation !== current.generation) {
    throw new ApiError("stale_snapshot", "snapshot predates a store restore; take a fresh snapshot", "/snapshot/generation");
  }
  if (pinned.retention_epoch < current.retention_epoch) {
    throw new ApiError("snapshot_pruned", "evidence was pruned after this snapshot was taken", "/snapshot/retention_epoch");
  }
  for (const key of ["retention_epoch", "evidence_revision", "observation_event_watermark", "location_event_watermark"] as const) {
    if (pinned[key] > current[key]) {
      throw new ApiError("invalid_input", `snapshot ${key} is ahead of the repository`, `/snapshot/${key}`);
    }
  }
}

interface ReadStart {
  readonly snapshot: Snapshot;
  /** Exclusive lower stream position of this page, when a cursor supplied it. */
  readonly position: number | undefined;
}

/** The bounds of this page: the cursor's, the pinned snapshot, or the current row. */
async function startRead(
  context: RequestContext,
  current: RepositoryRow,
  cursor: string | undefined,
  pinned: Snapshot | undefined,
  digest: string,
): Promise<ReadStart> {
  if (cursor !== undefined) {
    const state = await openCursor(
      context.cursorKey,
      cursor,
      {
        repository_id: current.repository_id,
        generation: current.generation,
        retention_epoch: current.retention_epoch,
        query_digest: digest,
      },
      context.nowMs,
      "/cursor",
    );
    if (state.bounds === undefined) throw new ApiError("invalid_cursor", "cursor carries no snapshot bounds", "/cursor");
    return {
      snapshot: { repository_id: state.repository_id, generation: state.generation, retention_epoch: state.retention_epoch, ...state.bounds },
      position: state.position,
    };
  }
  if (pinned !== undefined) {
    checkPinned(pinned, current);
    return { snapshot: pinned, position: undefined };
  }
  return { snapshot: snapshotOf(current), position: undefined };
}

function cursorState(snapshot: Snapshot, digest: string, position: number, nowMs: number): CursorState {
  return {
    repository_id: snapshot.repository_id,
    generation: snapshot.generation,
    retention_epoch: snapshot.retention_epoch,
    query_digest: digest,
    position,
    issued_at_ms: nowMs,
    bounds: {
      evidence_revision: snapshot.evidence_revision,
      observation_event_watermark: snapshot.observation_event_watermark,
      location_event_watermark: snapshot.location_event_watermark,
    },
  };
}

/**
 * Renders rows into a page until `limit` items or `budget` encoded bytes. A
 * first row that alone exceeds the budget can never be paged and is refused.
 */
function fillPage<T>(rows: readonly T[], limit: number, budget: number, render: (row: T) => string, name: (row: T) => string): string[] {
  const items: string[] = [];
  let used = 0;
  for (const row of rows.slice(0, limit)) {
    const text = render(row);
    const cost = byteLength(text) + (items.length > 0 ? 1 : 0);
    if (used + cost > budget) {
      if (items.length === 0) throw new ApiError("unpageable_record", `${name(row)} alone exceeds the page byte limit`);
      break;
    }
    items.push(text);
    used += cost;
  }
  return items;
}

/** An `ObservationRecord` as JSON text, with the stored payload and principal spliced in. */
function renderRecord(row: ObservationRow): string {
  return (
    `{"sequence":${row.sequence},"environment":${JSON.stringify(row.environment)},` +
    `"head":${JSON.stringify(row.head)},"tree":${JSON.stringify(row.tree)},` +
    `"observation":${row.payload_json},` +
    `"acceptance":{"source_version":${row.source_version},"accepted_at_ms":${row.received_at_ms},` +
    `"accepted_by":${row.accepted_principal},"collection_id":${JSON.stringify(row.accepted_collection_id)}}}`
  );
}

/**
 * A `LocationEvent` as JSON text. `sequence` is the event's position in the
 * location stream; `verification_method` and `valid_until_ms` exist only on
 * `verified` events.
 */
export function renderLocationEvent(row: LocationRow): string {
  const verified = row.state === "verified";
  return JSON.stringify({
    event_id: row.event_id,
    sequence: row.sequence,
    observation_id: row.observation_id,
    reference_kind: row.reference_kind,
    reference_digest: row.reference_digest,
    path: row.report_path,
    location_key: row.location_key,
    locator: JSON.parse(row.locator_json) as unknown,
    kind: row.state,
    ...(verified ? { verification_method: row.verification_method } : {}),
    checked_at_ms: row.checked_at_ms,
    ...(verified ? { valid_until_ms: row.valid_until_ms } : {}),
    checked_by: JSON.parse(row.checked_by) as unknown,
  });
}

export async function getObservation(context: RequestContext, repositoryId: string, observationId: string): Promise<Response> {
  const row = await readObservation(context.db, repositoryId, observationId);
  if (row === null) throw new ApiError("not_found", `observation ${observationId} is not stored in repository ${repositoryId}`);
  const [retriedBy, locations] = await Promise.all([
    readRetriedBy(context.db, repositoryId, observationId),
    readLatestLocations(context.db, repositoryId, observationId),
  ]);
  const body =
    `{"record":${renderRecord(row)},"retried_by":${JSON.stringify(retriedBy)},` +
    `"locations":[${locations.map(renderLocationEvent).join(",")}]}`;
  return rawJsonResponse(body, context.requestId);
}

const HISTORY_KEYS = ["package", "gate", "reachable_heads", "page_size", "cursor", "snapshot"] as const;

/** Rejects `cursor` combined with a field that would restart the listing. */
function refuseWithCursor(body: JsonObject, cursor: string | undefined, key: string): void {
  if (cursor !== undefined && Object.hasOwn(body, key)) {
    throw new ApiError("invalid_input", `${key} is not allowed with cursor; a cursor carries its own bounds`, `/${key}`);
  }
}

export async function queryHistory(context: RequestContext, repositoryId: string, bytes: Uint8Array): Promise<Response> {
  const body = readRequestObject(bytes, HISTORY_KEYS);
  const packageName = requiredString(body, "package", "");
  const gate = requiredString(body, "gate", "", { oneOf: GATES });
  const heads = requiredStringArray(body, "reachable_heads", "", { pattern: FORMATS.gitObjectId }, {
    maxItems: LIMITS.reachable_heads,
    unique: true,
  });
  const limit = optionalInteger(body, "page_size", "", PAGE_SIZE) ?? LIMITS.page_default_records;
  const cursor = optionalString(body, "cursor", "", CURSOR_SPEC);
  refuseWithCursor(body, cursor, "snapshot");
  const pinned = readPinnedSnapshot(body, repositoryId);

  const current = await repository(context, repositoryId);
  const sortedHeads = [...heads].sort();
  const digest = await queryDigest(["queryHistory", packageName, gate, sortedHeads.join(","), limit]);
  const start = await startRead(context, current, cursor, pinned, digest);
  const { snapshot } = start;

  // One extra row tells whether another page exists.
  const rows =
    heads.length === 0
      ? []
      : await readHistory(context.db, repositoryId, {
          package: packageName,
          gate,
          heads: sortedHeads,
          after: start.position ?? 0,
          watermark: snapshot.observation_event_watermark,
          limit: limit + 1,
        });
  const header = `{"snapshot":${JSON.stringify(snapshot)},"items":[`;
  const budget = LIMITS.page_max_encoded_bytes - byteLength(header) - TRAILER_RESERVE;
  const items = fillPage(rows, limit, budget, renderRecord, (row) => `observation ${row.observation_id}`);
  const hasMore = items.length < rows.length;
  const next = hasMore
    ? await signCursor(context.cursorKey, cursorState(snapshot, digest, rows[items.length - 1]!.sequence, context.nowMs))
    : null;
  return rawJsonResponse(`${header}${items.join(",")}],"has_more":${hasMore},"next_cursor":${JSON.stringify(next)}}`, context.requestId);
}

const CHANGES_KEYS = ["stream", "after_sequence", "page_size", "cursor", "snapshot"] as const;
const STREAMS = ["observations", "locations"] as const;
type Stream = (typeof STREAMS)[number];

function renderObservationChange(event: ObservationEventRow): string {
  const context = {
    sequence: event.seq,
    observation_id: event.observation_id,
    environment: event.environment,
    package: event.package,
    gate: event.gate,
    head: event.head,
    tree: event.tree,
  };
  if (event.kind === "pruned") {
    return JSON.stringify({ type: "pruned", ...context, job_id: event.job_id, recorded_at_ms: event.recorded_at_ms });
  }
  return JSON.stringify({
    type: "accepted",
    ...context,
    outcome: event.outcome,
    completion: event.completion,
    recorded_at_ms: event.recorded_at_ms,
  });
}

/**
 * A location-stream item. An event whose row was later pruned can no longer
 * be rendered, so it appears as the tombstone that removed it.
 */
function renderLocationChange(event: LocationStreamRow): string {
  if (event.kind === "pruned" || event.tombstone_job_id !== null) {
    return JSON.stringify({
      type: "pruned",
      sequence: event.seq,
      observation_id: event.observation_id,
      location_key: event.location_key,
      job_id: event.tombstone_job_id,
      recorded_at_ms: event.tombstone_recorded_at_ms,
    });
  }
  if (event.row === null) {
    // Rows leave only through a sanctioned prune, which always writes a tombstone.
    throw new Error(`location event ${event.seq} has neither a row nor a tombstone`);
  }
  return `{"type":"event","sequence":${event.seq},"event":${renderLocationEvent({ ...event.row, sequence: event.seq })}}`;
}

export async function queryChanges(context: RequestContext, repositoryId: string, bytes: Uint8Array): Promise<Response> {
  const body = readRequestObject(bytes, CHANGES_KEYS);
  const stream = requiredString(body, "stream", "", { oneOf: STREAMS }) as Stream;
  const limit = optionalInteger(body, "page_size", "", PAGE_SIZE) ?? LIMITS.page_default_records;
  const cursor = optionalString(body, "cursor", "", CURSOR_SPEC);
  refuseWithCursor(body, cursor, "after_sequence");
  refuseWithCursor(body, cursor, "snapshot");
  const afterSequence = optionalInteger(body, "after_sequence", "", COUNTER);
  const pinned = readPinnedSnapshot(body, repositoryId);

  const current = await repository(context, repositoryId);
  const digest = await queryDigest(["queryChanges", stream, limit]);
  const start = await startRead(context, current, cursor, pinned, digest);
  const { snapshot } = start;
  const observations = stream === "observations";
  const watermark = observations ? snapshot.observation_event_watermark : snapshot.location_event_watermark;
  const horizon = observations ? current.observation_event_horizon : current.location_event_horizon;
  const after = start.position ?? afterSequence ?? 0;
  const location = cursor !== undefined ? "/cursor" : "/after_sequence";
  if (after > watermark) {
    throw new ApiError("invalid_input", `after_sequence ${after} is beyond the snapshot watermark ${watermark}`, location);
  }
  if (after < horizon) {
    throw new ApiError(
      "cursor_expired",
      `the ${stream} stream is complete only after position ${horizon}; start again from a fresh snapshot`,
      location,
    );
  }

  const range = { after, watermark, limit: limit + 1 };
  const header = `{"stream":${JSON.stringify(stream)},"snapshot":${JSON.stringify(snapshot)},"last_sequence":`;
  const budget = LIMITS.page_max_encoded_bytes - byteLength(header) - TRAILER_RESERVE;
  let items: string[];
  let sequences: number[];
  let total: number;
  if (observations) {
    const rows = await readObservationEvents(context.db, repositoryId, range);
    items = fillPage(rows, limit, budget, renderObservationChange, (row) => `observation event ${row.seq}`);
    sequences = rows.map((row) => row.seq);
    total = rows.length;
  } else {
    const rows = await readLocationStream(context.db, repositoryId, range);
    items = fillPage(rows, limit, budget, renderLocationChange, (row) => `location event ${row.seq}`);
    sequences = rows.map((row) => row.seq);
    total = rows.length;
  }
  const hasMore = items.length < total;
  // A finished page reports the watermark, not its last item, so a consumer
  // that persists it never rereads a gap the stream skipped.
  const last = hasMore ? sequences[items.length - 1]! : watermark;
  const next = hasMore ? await signCursor(context.cursorKey, cursorState(snapshot, digest, last, context.nowMs)) : null;
  return rawJsonResponse(
    `${header}${last},"items":[${items.join(",")}],"has_more":${hasMore},"next_cursor":${JSON.stringify(next)}}`,
    context.requestId,
  );
}
