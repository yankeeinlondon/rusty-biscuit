//! Upload and upload-part rows (migration 0004).
//!
//! An upload's binding (repository, observation, reference, path, length) is
//! fixed at allocation and its state leaves `open` at most once
//! (`uploads_fixed_binding`, `uploads_terminal_state`), so every state change
//! here is conditional on `state = 'open'` and a lost race is read back rather
//! than overwritten.

import { admissionCondition, releaseClosedUploadsStatement, reserveUploadStatements, type Admission } from "../capacity.ts";
import type { CapacityQuotas } from "../config.ts";

export type UploadState = "open" | "finalized" | "rejected" | "expired";

export interface UploadRow {
  upload_id: string;
  repository_id: string;
  observation_id: string;
  reference_kind: string;
  reference_digest: string;
  path: string;
  bytes: number;
  part_bytes: number;
  part_count: number;
  state: UploadState;
  /** The final object's R2 key; parts are staged under `staging/{upload_id}/`. */
  staging_key: string;
  sha256: string | null;
  created_by: string;
  created_at_ms: number;
  expires_at_ms: number;
  finalized_at_ms: number | null;
  /** Count of `upload_parts` rows; parts are sequential, so also the next index. */
  parts_received: number;
}

const UPLOAD_COLUMNS = `
    u.upload_id, u.repository_id, u.observation_id, u.reference_kind,
    u.reference_digest, u.path, u.bytes, u.part_bytes, u.part_count, u.state,
    u.staging_key, u.sha256, u.created_by, u.created_at_ms, u.expires_at_ms,
    u.finalized_at_ms,
    (SELECT COUNT(*) FROM upload_parts p WHERE p.upload_id = u.upload_id) AS parts_received`;

export async function readUpload(db: D1Database, repositoryId: string, uploadId: string): Promise<UploadRow | null> {
  return db
    .prepare(`SELECT ${UPLOAD_COLUMNS} FROM uploads u WHERE u.upload_id = ?1 AND u.repository_id = ?2`)
    .bind(uploadId, repositoryId)
    .first<UploadRow>();
}

export interface UploadBinding {
  observation_id: string;
  reference_kind: string;
  reference_digest: string;
  path: string;
}

/** The newest open or finalized upload of exactly this reference and path, which a repeated request reuses. */
export async function readReusableUpload(db: D1Database, repositoryId: string, binding: UploadBinding): Promise<UploadRow | null> {
  return db
    .prepare(
      `SELECT ${UPLOAD_COLUMNS} FROM uploads u
       WHERE u.repository_id = ?1 AND u.observation_id = ?2 AND u.reference_kind = ?3
         AND u.reference_digest = ?4 AND u.path = ?5 AND u.state IN ('open', 'finalized')
       ORDER BY u.state = 'finalized' DESC, u.created_at_ms DESC
       LIMIT 1`,
    )
    .bind(repositoryId, binding.observation_id, binding.reference_kind, binding.reference_digest, binding.path)
    .first<UploadRow>();
}

export interface NewUpload extends UploadBinding {
  upload_id: string;
  repository_id: string;
  bytes: number;
  part_count: number;
  staging_key: string;
  created_by: string;
  created_at_ms: number;
  expires_at_ms: number;
}

/**
 * Inserts the upload and reserves its capacity in one D1 write, only when
 * the reservation fits under every refusal threshold (`src/capacity.ts`).
 * Returns false, having written nothing, when it does not. A locked cell
 * (migration 0008) refuses the insert with `cell_locked`.
 */
export async function insertUpload(
  db: D1Database,
  upload: NewUpload,
  quotas: CapacityQuotas,
  admission: Admission,
): Promise<boolean> {
  const condition = admissionCondition(quotas, admission, upload.created_at_ms, 13);
  const results = await db.batch([
    db
      .prepare(
        `INSERT INTO uploads(upload_id, repository_id, observation_id, reference_kind, reference_digest,
           path, bytes, part_count, state, staging_key, created_by, created_at_ms, expires_at_ms)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'open', ?9, ?10, ?11, ?12
         WHERE ${condition.sql}`,
      )
      .bind(
        upload.upload_id,
        upload.repository_id,
        upload.observation_id,
        upload.reference_kind,
        upload.reference_digest,
        upload.path,
        upload.bytes,
        upload.part_count,
        upload.staging_key,
        upload.created_by,
        upload.created_at_ms,
        upload.expires_at_ms,
        ...condition.values,
      ),
    ...reserveUploadStatements(db, upload.repository_id, upload.upload_id, admission, upload.created_at_ms),
  ]);
  return results[0]!.meta.changes === 1;
}

/**
 * Moves an open upload to `rejected` or `expired` and releases its
 * live-storage reservation; a no-op once it has left `open`.
 */
export async function closeUpload(db: D1Database, uploadId: string, state: "rejected" | "expired", nowMs: number): Promise<void> {
  await db.batch([
    db.prepare("UPDATE uploads SET state = ?2 WHERE upload_id = ?1 AND state = 'open'").bind(uploadId, state),
    releaseClosedUploadsStatement(db, nowMs),
  ]);
}

/** The finalization half of the one D1 batch that also inserts the verified location. */
export function finalizeStatement(db: D1Database, uploadId: string, sha256: string, nowMs: number): D1PreparedStatement {
  return db
    .prepare("UPDATE uploads SET state = 'finalized', sha256 = ?2, finalized_at_ms = ?3 WHERE upload_id = ?1 AND state = 'open'")
    .bind(uploadId, sha256, nowMs);
}

export interface PartRow {
  part_index: number;
  bytes: number;
  sha256: string;
}

export async function readPart(db: D1Database, uploadId: string, index: number): Promise<PartRow | null> {
  return db
    .prepare("SELECT part_index, bytes, sha256 FROM upload_parts WHERE upload_id = ?1 AND part_index = ?2")
    .bind(uploadId, index)
    .first<PartRow>();
}

export async function readParts(db: D1Database, uploadId: string): Promise<PartRow[]> {
  const { results } = await db
    .prepare("SELECT part_index, bytes, sha256 FROM upload_parts WHERE upload_id = ?1 ORDER BY part_index")
    .bind(uploadId)
    .all<PartRow>();
  return results;
}

/**
 * Records a received part while the upload is open. The first row for an
 * index wins: a racing write of the same index is ignored here and its
 * caller compares digests against the stored row.
 */
export async function insertPart(
  db: D1Database,
  uploadId: string,
  part: PartRow & { r2_etag: string },
  nowMs: number,
): Promise<void> {
  await db
    .prepare(
      `INSERT INTO upload_parts(upload_id, part_index, bytes, sha256, r2_etag, received_at_ms)
       SELECT ?1, ?2, ?3, ?4, ?5, ?6
       WHERE EXISTS (SELECT 1 FROM uploads WHERE upload_id = ?1 AND state = 'open')
       ON CONFLICT(upload_id, part_index) DO NOTHING`,
    )
    .bind(uploadId, part.part_index, part.bytes, part.sha256, part.r2_etag, nowMs)
    .run();
}

export interface FetchableObject {
  upload_id: string;
  bytes: number;
  sha256: string;
  staging_key: string;
}

/**
 * The newest finalized upload of this reference whose newest `object`
 * location event is `verified`. The event's `valid_until_ms` is deliberately
 * not consulted: the bytes sit in this service's own bucket and were
 * verified at finalization, so only a later `withdrawn` or `unavailable`
 * event for the same locator ends retrieval. The locator's `upload_id`
 * identifies it exactly as its `location_key` does.
 */
export async function readFetchableObject(
  db: D1Database,
  repositoryId: string,
  observationId: string,
  referenceKind: string,
  referenceDigest: string,
): Promise<FetchableObject | null> {
  return db
    .prepare(
      `SELECT u.upload_id, u.bytes, u.sha256, u.staging_key
       FROM uploads u
       WHERE u.repository_id = ?1 AND u.observation_id = ?2 AND u.reference_kind = ?3
         AND u.reference_digest = ?4 AND u.state = 'finalized'
         AND (SELECT l.state FROM report_locations l
              WHERE l.repository_id = u.repository_id AND l.observation_id = u.observation_id
                AND l.reference_kind = u.reference_kind AND l.reference_digest = u.reference_digest
                AND l.location_kind = 'object'
                AND json_extract(l.locator_json, '$.upload_id') = u.upload_id
              ORDER BY l.seq DESC LIMIT 1) = 'verified'
       ORDER BY u.finalized_at_ms DESC, u.upload_id
       LIMIT 1`,
    )
    .bind(repositoryId, observationId, referenceKind, referenceDigest)
    .first<FetchableObject>();
}

/**
 * Marks every open upload past its expiry `expired` and releases the
 * live-storage reservation of every closed upload; returns how many expired.
 */
export async function expireUploads(db: D1Database, nowMs: number): Promise<number> {
  const [expired] = await db.batch([
    db.prepare("UPDATE uploads SET state = 'expired' WHERE state = 'open' AND expires_at_ms <= ?1").bind(nowMs),
    releaseClosedUploadsStatement(db, nowMs),
  ]);
  return expired!.meta.changes;
}

export interface UploadStateRow {
  upload_id: string;
  state: UploadState;
  staging_key: string;
  created_at_ms: number;
}

/** States of the named uploads; IDs without a row are absent from the result. */
export async function readUploadStates(db: D1Database, uploadIds: readonly string[]): Promise<UploadStateRow[]> {
  const { results } = await db
    .prepare(
      `SELECT upload_id, state, staging_key, created_at_ms FROM uploads
       WHERE upload_id IN (SELECT value FROM json_each(?1))`,
    )
    .bind(JSON.stringify(uploadIds))
    .all<UploadStateRow>();
  return results;
}
