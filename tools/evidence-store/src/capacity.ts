//! Free-tier budget admission and capacity accounting (`quota_reservations`
//! and `capacity_baseline`, migration 0004).
//!
//! Capacity-consuming work is admitted only by reserving its projected cost
//! first, in the same D1 write that records the work, so two concurrent
//! admissions cannot both slip under the threshold. A reservation is refused
//! when the amount already reserved or committed, plus the measured usage of
//! everything else on the account (`capacity_baseline`), plus the new amount
//! would exceed `refuse_ratio` of the configured allowance. A refusal writes
//! nothing and never evicts or deletes existing evidence; collection
//! ingestion is never admission-controlled, so a failure is always recorded.
//!
//! Only proof uploads consume admitted capacity today:
//!
//! - `r2_bytes` is live storage, so it counts every reserved or committed
//!   reservation of any period. It is committed when the upload finalizes and
//!   released when the upload is rejected, expires, or is pruned.
//! - `r2_class_a` (one write per part plus the final object) counts the
//!   current month. Operations are spent once admitted, so these reservations
//!   are committed on finalization and never released.

import type { CapacityQuotas } from "./config.ts";

/** The quota resources this service reserves; the others are reported from the baseline only. */
export type CapacityResource = "r2_bytes" | "r2_class_a" | "r2_class_b";

const MONTHLY: readonly CapacityResource[] = ["r2_class_a", "r2_class_b"];

/** The `YYYY-MM` allowance window containing `nowMs` (UTC). */
export function capacityPeriod(nowMs: number): string {
  return new Date(nowMs).toISOString().slice(0, 7);
}

function allowance(quotas: CapacityQuotas, resource: CapacityResource): number {
  switch (resource) {
    case "r2_bytes":
      return quotas.r2_live_plus_staging_bytes;
    case "r2_class_a":
      return quotas.r2_class_a_per_month;
    case "r2_class_b":
      return quotas.r2_class_b_per_month;
  }
}

/** The highest admitted total: `refuse_ratio` of the allowance, rounded down. */
export function refusalThreshold(quotas: CapacityQuotas, resource: CapacityResource): number {
  return Math.floor(allowance(quotas, resource) * quotas.refuse_ratio);
}

/**
 * SQL for the units of `resource` already counted against `period`:
 * reservations plus the external baseline. Binds `?{periodParam}`.
 */
function usedSql(resource: CapacityResource, periodParam: number): string {
  const periodFilter = MONTHLY.includes(resource) ? ` AND q.period = ?${periodParam}` : "";
  return `(
    (SELECT COALESCE(SUM(q.amount), 0) FROM quota_reservations q
     WHERE q.resource = '${resource}' AND q.state IN ('reserved', 'committed')${periodFilter})
    + (SELECT COALESCE(MAX(b.external_used), 0) FROM capacity_baseline b
       WHERE b.resource = '${resource}' AND b.period = ?${periodParam}))`;
}

export interface Admission {
  readonly r2_bytes: number;
  readonly r2_class_a: number;
}

/**
 * A SQL condition that holds when `admission` fits under every refusal
 * threshold. Its placeholders start at `?{first}`: the period, then one
 * threshold-minus-amount per resource. Returns the condition and its values.
 */
export function admissionCondition(
  quotas: CapacityQuotas,
  admission: Admission,
  nowMs: number,
  first: number,
): { sql: string; values: (string | number)[] } {
  const period = first;
  const sql =
    `${usedSql("r2_bytes", period)} <= ?${first + 1} AND ` +
    `${usedSql("r2_class_a", period)} <= ?${first + 2}`;
  return {
    sql,
    values: [
      capacityPeriod(nowMs),
      refusalThreshold(quotas, "r2_bytes") - admission.r2_bytes,
      refusalThreshold(quotas, "r2_class_a") - admission.r2_class_a,
    ],
  };
}

function reservationId(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Inserts the upload's reservations, only once the upload row exists (it is inserted conditionally before). */
export function reserveUploadStatements(
  db: D1Database,
  repositoryId: string,
  uploadId: string,
  admission: Admission,
  nowMs: number,
): D1PreparedStatement[] {
  const period = capacityPeriod(nowMs);
  return (["r2_bytes", "r2_class_a"] as const).map((resource) =>
    db
      .prepare(
        `INSERT INTO quota_reservations(reservation_id, repository_id, resource, period, amount, upload_id,
           state, created_at_ms, updated_at_ms)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, 'reserved', ?7, ?7
         WHERE EXISTS (SELECT 1 FROM uploads WHERE upload_id = ?6)`,
      )
      .bind(reservationId(), repositoryId, resource, period, admission[resource], uploadId, nowMs),
  );
}

/** Commits an upload's reservations once it is finalized. */
export function commitUploadStatement(db: D1Database, uploadId: string, nowMs: number): D1PreparedStatement {
  return db
    .prepare(
      `UPDATE quota_reservations SET state = 'committed', updated_at_ms = ?2
       WHERE upload_id = ?1 AND state = 'reserved'
         AND EXISTS (SELECT 1 FROM uploads WHERE upload_id = ?1 AND state = 'finalized')`,
    )
    .bind(uploadId, nowMs);
}

/**
 * Releases the live-storage reservation of every rejected or expired upload.
 * Idempotent, so it runs after each state change and in the daily sweep.
 */
export function releaseClosedUploadsStatement(db: D1Database, nowMs: number): D1PreparedStatement {
  return db
    .prepare(
      `UPDATE quota_reservations SET state = 'released', updated_at_ms = ?1
       WHERE resource = 'r2_bytes' AND state = 'reserved'
         AND upload_id IN (SELECT upload_id FROM uploads WHERE state IN ('rejected', 'expired'))`,
    )
    .bind(nowMs);
}

/**
 * Detaches the reservations of a pruned observation's uploads (their rows
 * are deleted next) and releases their live storage.
 */
export function releasePrunedUploadsStatement(
  db: D1Database,
  repositoryId: string,
  observationIdsJson: string,
  nowMs: number,
): D1PreparedStatement {
  return db
    .prepare(
      `UPDATE quota_reservations
       SET state = CASE WHEN resource = 'r2_bytes' THEN 'released' ELSE state END,
           upload_id = NULL, updated_at_ms = ?3
       WHERE upload_id IN (
           SELECT upload_id FROM uploads
           WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2)))`,
    )
    .bind(repositoryId, observationIdsJson, nowMs);
}

export interface ResourceUsage {
  readonly resource: CapacityResource;
  readonly period: string;
  /** This service's reserved and committed units. */
  readonly reserved: number;
  /** Measured usage outside this service. */
  readonly external: number;
  readonly allowance: number;
  /** `(reserved + external) / allowance`. */
  readonly ratio: number;
  /** At or above `alert_ratio`. */
  readonly alert: boolean;
  /** At or above `refuse_ratio`: new capacity-consuming work is refused. */
  readonly refusing: boolean;
}

export interface CapacityReport {
  readonly period: string;
  readonly resources: readonly ResourceUsage[];
}

/** Current usage of each reserved resource against its allowance. */
export async function capacityReport(db: D1Database, quotas: CapacityQuotas, nowMs: number): Promise<CapacityReport> {
  const period = capacityPeriod(nowMs);
  const resources: ResourceUsage[] = [];
  for (const resource of ["r2_bytes", "r2_class_a", "r2_class_b"] as const) {
    // A SELECT of scalar subqueries always yields exactly one row.
    const row = (await db
      .prepare(
        `SELECT
           (SELECT COALESCE(SUM(amount), 0) FROM quota_reservations
            WHERE resource = ?1 AND state IN ('reserved', 'committed') AND (?3 = 0 OR period = ?2)) AS reserved,
           (SELECT COALESCE(MAX(external_used), 0) FROM capacity_baseline WHERE resource = ?1 AND period = ?2) AS external`,
      )
      .bind(resource, period, MONTHLY.includes(resource) ? 1 : 0)
      .first<{ reserved: number; external: number }>())!;
    const { reserved, external } = row;
    const total = allowance(quotas, resource);
    const ratio = total === 0 ? (reserved + external > 0 ? Infinity : 0) : (reserved + external) / total;
    resources.push({
      resource,
      period,
      reserved,
      external,
      allowance: total,
      ratio,
      alert: ratio >= quotas.alert_ratio,
      refusing: ratio >= quotas.refuse_ratio,
    });
  }
  return { period, resources };
}
