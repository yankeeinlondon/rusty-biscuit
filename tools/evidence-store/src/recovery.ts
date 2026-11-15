//! Operator recovery routes: persisted maintenance mode (the backup boundary)
//! and recovery of interrupted retention jobs.
//!
//! `POST /v1/operator/maintenance` quiesces or reopens the store. Quiescing
//! commits the `service_state` change and its audit row in one batch, then
//! reads every repository's counters: because the 0007 triggers refuse any
//! write that commits after the quiesce, those counters are the backup
//! boundary, and every write acknowledged before it is in a backup taken
//! while quiesced. `GET /v1/operator/maintenance` reads the same record.

import { auditStatement, auditStatementWhere } from "./db/audit.ts";
import { expireUploads } from "./db/uploads.ts";
import type { Env } from "./env.ts";
import { ApiError, jsonResponse } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import { readMaintenance, type MaintenanceState } from "./maintenance.ts";
import { FORMATS, optionalString, readRequestObject, requiredString } from "./requests.ts";
import { advanceRetentionJobs } from "./retention.ts";

/** One repository's counters at the moment the maintenance record was read. */
export interface BoundaryRepository {
  readonly repository_id: string;
  readonly generation: string;
  readonly retention_epoch: number;
  readonly evidence_revision: number;
  readonly observation_event_watermark: number;
  readonly location_event_watermark: number;
}

/** Applied as the maintenance reason when a quiesce request names none. */
export const DEFAULT_MAINTENANCE_REASON = "operator maintenance";

async function boundaryRepositories(db: D1Database): Promise<BoundaryRepository[]> {
  const { results } = await db
    .prepare(
      `SELECT repository_id, generation, retention_epoch, evidence_revision,
              observation_event_watermark, location_event_watermark
       FROM repositories ORDER BY repository_id`,
    )
    .all<BoundaryRepository>();
  return results;
}

async function maintenanceRecord(db: D1Database, changed: boolean) {
  const state = await readMaintenance(db);
  return { maintenance: state, changed, repositories: await boundaryRepositories(db) };
}

function boundaryConflict(state: MaintenanceState, requested: string | null): ApiError {
  const held = state.boundary_id === null ? "no boundary" : `boundary ${state.boundary_id}`;
  const wanted = requested === null ? "no boundary" : `boundary ${requested}`;
  return new ApiError("identity_conflict", `the store is quiesced for ${held}, not ${wanted}; nothing changed`, "/boundary_id");
}

/**
 * Quiesces or reopens the store. Quiescing an already quiesced store for the
 * same boundary (or both without one) answers the current record; for a
 * different boundary it answers 409. Reopening with `boundary_id` reopens
 * only the quiesce for that boundary; reopening an open store is a no-op.
 * A state change is audited in the same batch, and only when it applies.
 */
export async function setMaintenance(context: RequestContext, body: Uint8Array): Promise<Response> {
  const object = readRequestObject(body, ["mode", "reason", "boundary_id"]);
  const mode = requiredString(object, "mode", "", { oneOf: ["open", "quiesced"] });
  const reason = optionalString(object, "reason", "", { maxLength: 512 });
  const boundaryId = optionalString(object, "boundary_id", "", { pattern: FORMATS.serverId }) ?? null;
  const { db, nowMs } = context;

  if (mode === "quiesced") {
    const applied = await db.batch([
      auditStatementWhere(
        db,
        context,
        { action: "maintenance.quiesce", detail: { reason: reason ?? DEFAULT_MAINTENANCE_REASON, boundary_id: boundaryId } },
        "EXISTS (SELECT 1 FROM service_state WHERE id = 1 AND maintenance_mode = 'open')",
        [],
      ),
      db
        .prepare(
          `UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = ?1,
             maintenance_since_ms = ?2, maintenance_boundary_id = ?3
           WHERE id = 1 AND maintenance_mode = 'open'`,
        )
        .bind(reason ?? DEFAULT_MAINTENANCE_REASON, nowMs, boundaryId),
    ]);
    const record = await maintenanceRecord(db, applied[1]!.meta.changes > 0);
    if (record.maintenance.boundary_id !== boundaryId) throw boundaryConflict(record.maintenance, boundaryId);
    return jsonResponse(record, 200, context.requestId);
  }

  const current = await readMaintenance(db);
  if (current.mode === "quiesced" && boundaryId !== null && current.boundary_id !== boundaryId) throw boundaryConflict(current, boundaryId);
  // `?1 IS NULL` reopens whatever quiesce holds; a named boundary reopens only itself.
  const condition = "maintenance_mode = 'quiesced' AND (?1 IS NULL OR maintenance_boundary_id IS ?1)";
  const applied = await db.batch([
    auditStatementWhere(
      db,
      context,
      { action: "maintenance.resume", detail: { reason: reason ?? null, boundary_id: current.boundary_id, quiesced_at_ms: current.since_ms } },
      `EXISTS (SELECT 1 FROM service_state WHERE id = 1 AND ${condition.replaceAll("?1", "?")})`,
      [boundaryId, boundaryId],
    ),
    db
      .prepare(
        `UPDATE service_state SET maintenance_mode = 'open', maintenance_reason = NULL,
           maintenance_since_ms = NULL, maintenance_boundary_id = NULL
         WHERE id = 1 AND ${condition}`,
      )
      .bind(boundaryId),
  ]);
  const record = await maintenanceRecord(db, applied[1]!.meta.changes > 0);
  // A concurrent quiesce for another boundary can land between the read and the batch.
  if (record.maintenance.mode === "quiesced") throw boundaryConflict(record.maintenance, boundaryId);
  return jsonResponse(record, 200, context.requestId);
}

/** The current maintenance record; works while quiesced. */
export async function getMaintenance(context: RequestContext): Promise<Response> {
  return jsonResponse(await maintenanceRecord(context.db, false), 200, context.requestId);
}

/**
 * Recovers interrupted operational work: expires open uploads past their
 * expiry, advances every unfinished retention job by bounded batches, and
 * releases cell locks still held by a completed or invalidated job. Every
 * step is idempotent, so rerunning after a crash finishes what is left. The
 * Worker refuses this route while the store is quiesced.
 */
export async function recoverJobs(context: RequestContext, env: Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET">, body: Uint8Array): Promise<Response> {
  if (body.byteLength > 0) readRequestObject(body, []);
  const { db, nowMs } = context;
  const expiredUploads = await expireUploads(db, nowMs);
  const run = await advanceRetentionJobs(env, nowMs);
  const released = await db
    .prepare(
      `DELETE FROM cell_locks WHERE job_id IN (
         SELECT job_id FROM retention_jobs WHERE state IN ('completed', 'invalidated'))`,
    )
    .run();
  const report = {
    steps: run.steps,
    released_locks: run.released_locks + released.meta.changes,
    expired_uploads: expiredUploads,
  };
  await auditStatement(db, context, {
    action: "retention.recover",
    detail: { steps: report.steps.length, released_locks: report.released_locks, expired_uploads: expiredUploads },
  }).run();
  return jsonResponse(report, 200, context.requestId);
}
