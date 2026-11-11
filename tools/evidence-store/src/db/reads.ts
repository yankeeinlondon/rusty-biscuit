//! Snapshot, history, location, and change-feed reads.
//!
//! Every list read is bounded by a captured snapshot: its rows are selected
//! with `seq <= watermark`, so a write committed after the snapshot never
//! enters one of its pages. `observations.seq`, `observation_events.seq`, and
//! `location_events.seq` are AUTOINCREMENT across every repository, so stream
//! positions of one repository are increasing but not contiguous.

import type { Snapshot } from "../contracts/api.ts";
import type { ObservationRow } from "./ingest.ts";

export interface RepositoryRow extends Snapshot {
  observation_event_horizon: number;
  location_event_horizon: number;
}

/** One primary read of the repository's snapshot and replay horizons. */
export async function readRepository(db: D1Database, repositoryId: string): Promise<RepositoryRow | null> {
  return db
    .prepare(
      `SELECT repository_id, generation, retention_epoch, evidence_revision,
              observation_event_watermark, location_event_watermark,
              observation_event_horizon, location_event_horizon
       FROM repositories WHERE repository_id = ?1`,
    )
    .bind(repositoryId)
    .first<RepositoryRow>();
}

// `sequence` is the observation's latest `accepted` event: an ID ingested
// again after it was pruned has two, and only the later one describes the
// stored row. Heads arrive as one JSON parameter (D1 caps bound parameters at
// 100).
const SELECT_HISTORY = `
SELECT * FROM (
    SELECT o.observation_id, o.environment, o.head, o.tree, o.payload_json,
           o.source_version, o.accepted_principal, o.accepted_request_id,
           o.accepted_collection_id, o.received_at_ms,
           (SELECT MAX(e.seq) FROM observation_events e
            WHERE e.repository_id = o.repository_id
              AND e.observation_id = o.observation_id
              AND e.kind = 'accepted') AS sequence
    FROM observations o
    WHERE o.repository_id = ?1 AND o.package = ?2 AND o.gate = ?3
      AND o.head IN (SELECT value FROM json_each(?4))
)
WHERE sequence > ?5 AND sequence <= ?6
ORDER BY sequence
LIMIT ?7`;

export interface HistoryRange {
  package: string;
  gate: string;
  heads: readonly string[];
  /** Exclusive lower bound (the cursor position, or 0). */
  after: number;
  /** Inclusive upper bound: the snapshot's observation-event watermark. */
  watermark: number;
  limit: number;
}

export async function readHistory(db: D1Database, repositoryId: string, range: HistoryRange): Promise<ObservationRow[]> {
  const { results } = await db
    .prepare(SELECT_HISTORY)
    .bind(repositoryId, range.package, range.gate, JSON.stringify(range.heads), range.after, range.watermark, range.limit)
    .all<ObservationRow>();
  return results;
}

/** Sorted IDs of the observations whose `retry_of` names `observationId`. */
export async function readRetriedBy(db: D1Database, repositoryId: string, observationId: string): Promise<string[]> {
  const { results } = await db
    .prepare("SELECT observation_id FROM observations WHERE repository_id = ?1 AND retry_of = ?2 ORDER BY observation_id")
    .bind(repositoryId, observationId)
    .all<{ observation_id: string }>();
  return results.map((row) => row.observation_id);
}

/** A stored `report_locations` row plus its position in the location stream. */
export interface LocationRow {
  event_id: string;
  observation_id: string;
  reference_kind: string;
  reference_digest: string;
  report_path: string;
  location_key: string;
  /** Canonical locator JSON. */
  locator_json: string;
  state: "verified" | "unavailable" | "withdrawn";
  verification_method: string | null;
  checked_at_ms: number;
  valid_until_ms: number | null;
  /** Authenticated verifier as stored principal JSON. */
  checked_by: string;
  sequence: number;
}

// An event ID reused after its row was pruned has an older stream event and
// a `pruned` tombstone too; the stored row's event is the latest non-pruned one.
const LOCATION_COLUMNS = `
    l.event_id, l.observation_id, l.reference_kind, l.reference_digest,
    l.report_path, l.location_key, l.locator_json, l.state,
    l.verification_method, l.checked_at_ms, l.valid_until_ms, l.checked_by,
    (SELECT MAX(e.seq) FROM location_events e
     WHERE e.repository_id = l.repository_id AND e.event_id = l.event_id
       AND e.kind <> 'pruned') AS sequence`;

export async function readLocationEvent(db: D1Database, repositoryId: string, eventId: string): Promise<LocationRow | null> {
  return db
    .prepare(`SELECT ${LOCATION_COLUMNS} FROM report_locations l WHERE l.repository_id = ?1 AND l.event_id = ?2`)
    .bind(repositoryId, eventId)
    .first<LocationRow>();
}

/**
 * The newest event of every distinct location (reference, path, and locator)
 * of an observation, whatever its state or expiry: an expired or withdrawn
 * newest event hides every older verification of the same location.
 */
export async function readLatestLocations(db: D1Database, repositoryId: string, observationId: string): Promise<LocationRow[]> {
  const { results } = await db
    .prepare(
      `SELECT ${LOCATION_COLUMNS}
       FROM report_locations l
       WHERE l.repository_id = ?1 AND l.observation_id = ?2
         AND l.seq = (
             SELECT MAX(n.seq) FROM report_locations n
             WHERE n.repository_id = l.repository_id
               AND n.observation_id = l.observation_id
               AND n.reference_kind = l.reference_kind
               AND n.reference_digest = l.reference_digest
               AND n.report_path = l.report_path
               AND n.location_key = l.location_key)
       ORDER BY l.seq`,
    )
    .bind(repositoryId, observationId)
    .all<LocationRow>();
  return results;
}

export interface ObservationEventRow {
  seq: number;
  kind: "accepted" | "pruned";
  observation_id: string;
  environment: string;
  package: string;
  gate: string;
  head: string;
  tree: string;
  outcome: string;
  completion: string;
  job_id: string | null;
  recorded_at_ms: number;
}

export interface StreamRange {
  /** Exclusive lower bound. */
  after: number;
  /** Inclusive upper bound: the snapshot's watermark for the stream. */
  watermark: number;
  limit: number;
}

export async function readObservationEvents(db: D1Database, repositoryId: string, range: StreamRange): Promise<ObservationEventRow[]> {
  const { results } = await db
    .prepare(
      `SELECT seq, kind, observation_id, environment, package, gate, head, tree,
              outcome, completion, job_id, recorded_at_ms
       FROM observation_events
       WHERE repository_id = ?1 AND seq > ?2 AND seq <= ?3
       ORDER BY seq
       LIMIT ?4`,
    )
    .bind(repositoryId, range.after, range.watermark, range.limit)
    .all<ObservationEventRow>();
  return results;
}

/**
 * One location-stream event. A `pruned` tombstone, or an event whose row a
 * later tombstone removed, carries `tombstone_*` and no `row`; every other
 * event carries the stored row it announced.
 */
export interface LocationStreamRow {
  seq: number;
  kind: "verified" | "unavailable" | "withdrawn" | "pruned";
  observation_id: string;
  location_key: string;
  tombstone_job_id: string | null;
  tombstone_recorded_at_ms: number | null;
  row: Omit<LocationRow, "sequence"> | null;
}

// The tombstone that removed an event's row is the first `pruned` event for
// its event ID after it; checking the tombstone first keeps an event ID that
// was reused after pruning from being rendered with the newer row.
const SELECT_LOCATION_STREAM = `
SELECT e.seq, e.kind, e.observation_id, e.location_key,
       CASE WHEN e.kind = 'pruned' THEN e.job_id ELSE p.job_id END AS tombstone_job_id,
       CASE WHEN e.kind = 'pruned' THEN e.recorded_at_ms ELSE p.recorded_at_ms END
           AS tombstone_recorded_at_ms,
       l.event_id AS row_event_id, l.observation_id AS row_observation_id,
       l.reference_kind, l.reference_digest, l.report_path,
       l.location_key AS row_location_key, l.locator_json, l.state,
       l.verification_method, l.checked_at_ms, l.valid_until_ms, l.checked_by
FROM location_events e
LEFT JOIN location_events p
  ON e.kind <> 'pruned'
 AND p.seq = (SELECT MIN(q.seq) FROM location_events q
              WHERE q.repository_id = e.repository_id AND q.event_id = e.event_id
                AND q.kind = 'pruned' AND q.seq > e.seq)
LEFT JOIN report_locations l
  ON e.kind <> 'pruned' AND p.seq IS NULL
 AND l.repository_id = e.repository_id AND l.event_id = e.event_id
WHERE e.repository_id = ?1 AND e.seq > ?2 AND e.seq <= ?3
ORDER BY e.seq
LIMIT ?4`;

interface LocationStreamFlat {
  seq: number;
  kind: LocationStreamRow["kind"];
  observation_id: string;
  location_key: string;
  tombstone_job_id: string | null;
  tombstone_recorded_at_ms: number | null;
  row_event_id: string | null;
  row_observation_id: string;
  reference_kind: string;
  reference_digest: string;
  report_path: string;
  row_location_key: string;
  locator_json: string;
  state: LocationRow["state"];
  verification_method: string | null;
  checked_at_ms: number;
  valid_until_ms: number | null;
  checked_by: string;
}

export async function readLocationStream(db: D1Database, repositoryId: string, range: StreamRange): Promise<LocationStreamRow[]> {
  const { results } = await db
    .prepare(SELECT_LOCATION_STREAM)
    .bind(repositoryId, range.after, range.watermark, range.limit)
    .all<LocationStreamFlat>();
  return results.map((flat) => ({
    seq: flat.seq,
    kind: flat.kind,
    observation_id: flat.observation_id,
    location_key: flat.location_key,
    tombstone_job_id: flat.tombstone_job_id,
    tombstone_recorded_at_ms: flat.tombstone_recorded_at_ms,
    row:
      flat.row_event_id === null
        ? null
        : {
            event_id: flat.row_event_id,
            observation_id: flat.row_observation_id,
            reference_kind: flat.reference_kind,
            reference_digest: flat.reference_digest,
            report_path: flat.report_path,
            location_key: flat.row_location_key,
            locator_json: flat.locator_json,
            state: flat.state,
            verification_method: flat.verification_method,
            checked_at_ms: flat.checked_at_ms,
            valid_until_ms: flat.valid_until_ms,
            checked_by: flat.checked_by,
          },
  }));
}

/** Whether an upload is bound to exactly this repository, observation, and reference. */
export async function uploadMatches(
  db: D1Database,
  repositoryId: string,
  uploadId: string,
  observationId: string,
  referenceKind: string,
  referenceDigest: string,
): Promise<boolean> {
  const row = await db
    .prepare(
      `SELECT 1 AS present FROM uploads
       WHERE upload_id = ?1 AND repository_id = ?2 AND observation_id = ?3
         AND reference_kind = ?4 AND reference_digest = ?5`,
    )
    .bind(uploadId, repositoryId, observationId, referenceKind, referenceDigest)
    .first<{ present: number }>();
  return row !== null;
}
