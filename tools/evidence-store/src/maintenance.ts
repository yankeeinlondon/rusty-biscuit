//! Persisted mutation maintenance mode (migration 0007).
//!
//! While `service_state.maintenance_mode` is `quiesced`, every mutation route
//! answers 503 `unavailable` before it starts, and the migration's triggers
//! refuse any write that was already past that check with `maintenance_mode`.
//! The triggers are the authority: a write either committed before the
//! quiesce (and is in the backup) or is refused (and was never acknowledged).

import { ApiError } from "./http.ts";

export type MaintenanceMode = "open" | "quiesced";

export interface MaintenanceState {
  readonly mode: MaintenanceMode;
  readonly reason: string | null;
  readonly since_ms: number | null;
  readonly boundary_id: string | null;
}

/** The SQLite error text the 0007 triggers raise. */
export const MAINTENANCE_ERROR = "maintenance_mode";

/** Seconds a client waits before retrying a refused mutation. */
export const MAINTENANCE_RETRY_MS = 60_000;

export async function readMaintenance(db: D1Database): Promise<MaintenanceState> {
  const row = await db
    .prepare("SELECT maintenance_mode, maintenance_reason, maintenance_since_ms, maintenance_boundary_id FROM service_state WHERE id = 1")
    .first<{ maintenance_mode: MaintenanceMode; maintenance_reason: string | null; maintenance_since_ms: number | null; maintenance_boundary_id: string | null }>();
  // The migration inserts the row and a trigger forbids deleting it.
  if (row === null) throw new Error("service_state row is missing");
  return { mode: row.maintenance_mode, reason: row.maintenance_reason, since_ms: row.maintenance_since_ms, boundary_id: row.maintenance_boundary_id };
}

export function maintenanceError(): ApiError {
  return new ApiError("unavailable", "the evidence store is in maintenance; nothing was recorded, retry later", null, MAINTENANCE_RETRY_MS);
}

/** Refuses with 503 while quiesced, so a mutation never starts work it cannot commit. */
export async function assertWritable(db: D1Database): Promise<void> {
  if ((await readMaintenance(db)).mode === "quiesced") throw maintenanceError();
}

/** True when `error` is a D1 failure raised by a 0007 maintenance trigger. */
export function isMaintenanceRefusal(error: unknown): boolean {
  return error instanceof Error && error.message.includes(MAINTENANCE_ERROR);
}
