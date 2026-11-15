//! Atomic collection ingestion and the collection/observation reads that
//! ingestion and export need.
//!
//! One collection is one D1 `batch()`, which D1 runs as a single transaction.
//! The `observations_identity_guard` trigger (migration 0001) is the conflict
//! authority: it aborts the whole batch with `identity_conflict` when a stored
//! observation with the same ID differs in canonical bytes or in
//! environment/head/tree, so a rejected collection leaves no sibling
//! observation, manifest, member, event, or revision step behind. There is no
//! preflight SELECT. Arrays are bound as one JSON parameter and expanded with
//! `json_each(?)`, which keeps every statement under D1's 100-parameter cap.

import type { IngestResult } from "../contracts/api.ts";

export interface IngestObservation {
  observation_id: string;
  /** Canonical observation bytes; stored and compared verbatim. */
  payload_json: string;
}

export interface IngestCollectionInput {
  repository_id: string;
  /** SHA-256 of the canonical version-3 collection. */
  collection_id: string;
  environment: string;
  head: string;
  tree: string;
  source_version: 2 | 3;
  /** Authenticated publisher, recorded only on first acceptance. */
  principal: string;
  /**
   * Unique per call. Rows stamped with it are reported as `accepted`; reusing
   * a request ID across calls would misreport earlier rows as accepted.
   */
  request_id: string;
  now_ms: number;
  /** Validated, unique, and sorted by `observation_id`; position is the index. */
  observations: IngestObservation[];
}

export type IngestOutcome =
  | { kind: "ok"; result: IngestResult }
  | { kind: "identity_conflict" }
  | { kind: "unknown_repository" };

// Every insert selects FROM repositories, so an unknown repository inserts
// nothing without needing a guard or an error.
const INSERT_OBSERVATIONS = `
INSERT INTO observations(
    repository_id, observation_id, environment, head, tree, payload_json,
    source_version, accepted_principal, accepted_request_id,
    accepted_collection_id, received_at_ms
)
SELECT r.repository_id, json_extract(j.value, '$.observation_id'), ?2, ?3, ?4,
       json_extract(j.value, '$.payload_json'), ?5, ?6, ?7, ?8, ?9
FROM repositories r, json_each(?10) j
WHERE r.repository_id = ?1
ORDER BY j.key
ON CONFLICT(repository_id, observation_id) DO NOTHING`;

const INSERT_COLLECTION = `
INSERT INTO collections(
    repository_id, collection_id, environment, head, tree, observation_count,
    source_version, accepted_principal, accepted_request_id, received_at_ms
)
SELECT repository_id, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
FROM repositories
WHERE repository_id = ?1
ON CONFLICT(repository_id, collection_id) DO NOTHING`;

const INSERT_MEMBERS = `
INSERT INTO collection_members(repository_id, collection_id, position, observation_id)
SELECT c.repository_id, c.collection_id, CAST(j.key AS INTEGER), j.value
FROM collections c, json_each(?3) j
WHERE c.repository_id = ?1 AND c.collection_id = ?2
ON CONFLICT DO NOTHING`;

const SELECT_OUTCOME = `
SELECT o.observation_id, o.accepted_request_id = ?2 AS accepted
FROM json_each(?3) j
JOIN observations o ON o.repository_id = ?1 AND o.observation_id = j.value
ORDER BY j.key`;

const SELECT_REVISION = "SELECT evidence_revision FROM repositories WHERE repository_id = ?1";

/**
 * Inserts a validated collection, its manifest, and its membership in one
 * transaction.
 *
 * - New observations are `accepted`; each advances the evidence revision by
 *   one and appends one `accepted` change event.
 * - Identical observations are `already_present`; they keep their first
 *   publisher, request ID, collection, timestamp, and source version, and add
 *   no revision step or event.
 * - The manifest is stored once, including for an empty collection; storing
 *   or replaying a manifest never advances the revision.
 *
 * Throws on any D1 failure other than an identity conflict; the caller answers
 * 503 `unavailable`.
 */
export async function ingestCollectionBatch(
  db: D1Database,
  input: IngestCollectionInput,
): Promise<IngestOutcome> {
  const ids = input.observations.map((observation) => observation.observation_id);
  const idsJson = JSON.stringify(ids);
  const repo = input.repository_id;
  let results: D1Result[];
  try {
    results = await db.batch([
      db
        .prepare(INSERT_OBSERVATIONS)
        .bind(
          repo,
          input.environment,
          input.head,
          input.tree,
          input.source_version,
          input.principal,
          input.request_id,
          input.collection_id,
          input.now_ms,
          JSON.stringify(input.observations),
        ),
      db
        .prepare(INSERT_COLLECTION)
        .bind(
          repo,
          input.collection_id,
          input.environment,
          input.head,
          input.tree,
          input.observations.length,
          input.source_version,
          input.principal,
          input.request_id,
          input.now_ms,
        ),
      db.prepare(INSERT_MEMBERS).bind(repo, input.collection_id, idsJson),
      db.prepare(SELECT_OUTCOME).bind(repo, input.request_id, idsJson),
      db.prepare(SELECT_REVISION).bind(repo),
    ]);
  } catch (error) {
    if (isIdentityConflict(error)) return { kind: "identity_conflict" };
    throw error;
  }

  const revision = results[4]?.results[0] as { evidence_revision: number } | undefined;
  if (revision === undefined) return { kind: "unknown_repository" };

  const outcome = (results[3]?.results ?? []) as { observation_id: string; accepted: number }[];
  if (outcome.length !== ids.length) {
    throw new Error(
      `ingestion batch stored ${outcome.length} of ${ids.length} observations for collection ${input.collection_id}`,
    );
  }
  return {
    kind: "ok",
    result: {
      collection_id: input.collection_id,
      accepted: outcome.filter((row) => row.accepted === 1).map((row) => row.observation_id),
      already_present: outcome.filter((row) => row.accepted !== 1).map((row) => row.observation_id),
      evidence_revision: revision.evidence_revision,
    },
  };
}

/** D1 surfaces the trigger's RAISE text inside its error message. */
function isIdentityConflict(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("identity_conflict");
}

export interface CollectionManifest {
  collection_id: string;
  environment: string;
  head: string;
  tree: string;
  observation_count: number;
  source_version: 2 | 3;
  accepted_principal: string;
  accepted_request_id: string;
  received_at_ms: number;
  /** Members across the whole collection whose observation has been pruned. */
  missing_member_count: number;
}

export async function readCollectionManifest(
  db: D1Database,
  repositoryId: string,
  collectionId: string,
): Promise<CollectionManifest | null> {
  return db
    .prepare(
      `SELECT c.collection_id, c.environment, c.head, c.tree, c.observation_count,
              c.source_version, c.accepted_principal, c.accepted_request_id,
              c.received_at_ms,
              (SELECT COUNT(*) FROM collection_members m
               WHERE m.repository_id = c.repository_id
                 AND m.collection_id = c.collection_id
                 AND NOT EXISTS (
                     SELECT 1 FROM observations o
                     WHERE o.repository_id = m.repository_id
                       AND o.observation_id = m.observation_id)
              ) AS missing_member_count
       FROM collections c
       WHERE c.repository_id = ?1 AND c.collection_id = ?2`,
    )
    .bind(repositoryId, collectionId)
    .first<CollectionManifest>();
}

export interface CollectionMember {
  position: number;
  observation_id: string;
  /** Canonical observation bytes, or null when the observation was pruned. */
  payload_json: string | null;
}

export interface CollectionMembersPage {
  members: CollectionMember[];
  /** True when any member on this page has been pruned. */
  missing: boolean;
}

/** Members with `position > afterPosition` in position order; start with -1. */
export async function readCollectionMembers(
  db: D1Database,
  repositoryId: string,
  collectionId: string,
  afterPosition: number,
  limit: number,
): Promise<CollectionMembersPage> {
  const { results } = await db
    .prepare(
      `SELECT m.position, m.observation_id, o.payload_json
       FROM collection_members m
       LEFT JOIN observations o
         ON o.repository_id = m.repository_id AND o.observation_id = m.observation_id
       WHERE m.repository_id = ?1 AND m.collection_id = ?2 AND m.position > ?3
       ORDER BY m.position
       LIMIT ?4`,
    )
    .bind(repositoryId, collectionId, afterPosition, limit)
    .all<CollectionMember>();
  return { members: results, missing: results.some((member) => member.payload_json === null) };
}

export interface ObservationRow {
  observation_id: string;
  environment: string;
  head: string;
  tree: string;
  payload_json: string;
  source_version: 2 | 3;
  accepted_principal: string;
  accepted_request_id: string;
  accepted_collection_id: string;
  received_at_ms: number;
  /**
   * Sequence of the observation's latest `accepted` change event: an ID
   * ingested again after it was pruned has two.
   */
  sequence: number;
}

export async function readObservation(
  db: D1Database,
  repositoryId: string,
  observationId: string,
): Promise<ObservationRow | null> {
  return db
    .prepare(
      `SELECT o.observation_id, o.environment, o.head, o.tree, o.payload_json,
              o.source_version, o.accepted_principal, o.accepted_request_id,
              o.accepted_collection_id, o.received_at_ms,
              (SELECT MAX(e.seq) FROM observation_events e
               WHERE e.repository_id = o.repository_id
                 AND e.observation_id = o.observation_id
                 AND e.kind = 'accepted') AS sequence
       FROM observations o
       WHERE o.repository_id = ?1 AND o.observation_id = ?2`,
    )
    .bind(repositoryId, observationId)
    .first<ObservationRow>();
}
