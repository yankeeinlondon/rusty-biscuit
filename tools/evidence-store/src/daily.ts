//! The daily scheduled run (the Worker's `scheduled` handler, wired to the
//! cron trigger in `wrangler.jsonc`).
//!
//! Each step is bounded and idempotent, so a run cut short is finished by the
//! next one. Nothing runs while the store is quiesced for a backup: every step
//! writes, and the backup boundary must not move. The steps, in order:
//!
//! 1. staging cleanup (`cleanupStaging`): expire open uploads, release their
//!    reserved storage, and remove old unfinalized staging bytes;
//! 2. retention jobs (`advanceRetentionJobs`);
//! 3. tombstone expiry: `pruned` change events older than
//!    `change_tombstone_days` are deleted, together with every earlier event of
//!    the same observation or location event, and the stream's replay horizon
//!    advances to the highest deleted tombstone (never backwards), so a change
//!    query resuming before it answers 409 `cursor_expired`;
//! 4. capacity accounting: usage of each R2 allowance, with a log line of
//!    numbers only (never report bytes or credentials) at the alert ratio.

import { capacityReport, type CapacityReport } from "./capacity.ts";
import { serviceConfig } from "./config.ts";
import { LIMITS } from "./contracts/api.ts";
import { expireTombstones, readExpiredTombstones } from "./db/retention.ts";
import type { Env } from "./env.ts";
import { readMaintenance } from "./maintenance.ts";
import { advanceRetentionJobs, type RetentionRunReport } from "./retention.ts";
import { cleanupStaging, type CleanupResult } from "./uploads.ts";

const DAY_MS = 86_400_000;
/** Tombstones read per expiry round; each round is one bounded D1 write per repository. */
const TOMBSTONE_BATCH = 500;
/** Expiry rounds per stream per run; the next run continues. */
const TOMBSTONE_ROUNDS = 20;

export interface TombstoneExpiry {
  /** Change events deleted from the observation stream. */
  readonly observation_events_deleted: number;
  /** Change events deleted from the location stream. */
  readonly location_events_deleted: number;
}

export interface DailyReport {
  /** True when the run did nothing because the store was quiesced. */
  readonly skipped_for_maintenance: boolean;
  readonly staging: CleanupResult | null;
  readonly retention: RetentionRunReport | null;
  readonly tombstones: TombstoneExpiry | null;
  readonly capacity: CapacityReport | null;
}

async function expireStream(db: D1Database, stream: "observation" | "location", cutoffMs: number): Promise<number> {
  let deleted = 0;
  for (let round = 0; round < TOMBSTONE_ROUNDS; round += 1) {
    const chunks = await readExpiredTombstones(db, stream, cutoffMs, TOMBSTONE_BATCH);
    if (chunks.length === 0) break;
    for (const chunk of chunks) deleted += await expireTombstones(db, stream, chunk.repository_id, chunk.horizon, cutoffMs);
  }
  return deleted;
}

/** Deletes expired `pruned` tombstones and advances both replay horizons. */
export async function expireChangeTombstones(db: D1Database, nowMs: number): Promise<TombstoneExpiry> {
  const cutoff = nowMs - LIMITS.change_tombstone_days * DAY_MS;
  return {
    observation_events_deleted: await expireStream(db, "observation", cutoff),
    location_events_deleted: await expireStream(db, "location", cutoff),
  };
}

export async function runDaily(
  env: Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET" | "SERVICE_CONFIG">,
  nowMs: number,
): Promise<DailyReport> {
  if ((await readMaintenance(env.EVIDENCE_DB)).mode === "quiesced") {
    return { skipped_for_maintenance: true, staging: null, retention: null, tombstones: null, capacity: null };
  }
  const staging = await cleanupStaging(env, nowMs);
  const retention = await advanceRetentionJobs(env, nowMs);
  const tombstones = await expireChangeTombstones(env.EVIDENCE_DB, nowMs);
  const capacity = await capacityReport(env.EVIDENCE_DB, serviceConfig(env).quotas.capacity, nowMs);
  for (const usage of capacity.resources) {
    if (usage.alert) {
      // Numbers and fixed resource names only; an operator decides what to do.
      console.warn(
        JSON.stringify({
          event: "evidence_store.capacity_alert",
          resource: usage.resource,
          period: usage.period,
          used: usage.reserved + usage.external,
          allowance: usage.allowance,
          ratio: usage.ratio,
          refusing: usage.refusing,
        }),
      );
    }
  }
  return { skipped_for_maintenance: false, staging, retention, tombstones, capacity };
}
