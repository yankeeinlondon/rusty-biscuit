//! Retention certificate, job, member, pin, lock, and supersession rows
//! (migrations 0002 and 0008), and the bounded statements the job runner
//! applies.
//!
//! Every write is a D1 `batch()` (one transaction) whose statements are
//! conditional on the state they expect, so a statement that a concurrent run
//! or a crashed earlier attempt already applied changes nothing. Progress
//! counters advance with SQLite's `changes()`, which reports the rows the
//! immediately preceding statement of the same batch inserted or deleted
//! (trigger writes excluded), so a counter moves exactly when its rows did.
//! ID sets are bound as one JSON parameter and expanded with `json_each(?)`.

import { auditStatementWhere, type AuditActor, type AuditEntry } from "./audit.ts";
import { releasePrunedUploadsStatement } from "../capacity.ts";

export type RetentionJobState = "scheduled" | "withdrawing" | "deleting-objects" | "deleting-rows" | "completed" | "invalidated";

export const TERMINAL_STATES: readonly RetentionJobState[] = ["completed", "invalidated"];

export interface RetentionJobRow {
  job_id: string;
  repository_id: string;
  certificate_id: string;
  state: RetentionJobState;
  generation: string;
  retention_epoch: number;
  evidence_revision: number;
  member_count: number;
  locations_withdrawn: number;
  objects_deleted: number;
  observations_deleted: number;
  not_before_ms: number;
  created_at_ms: number;
  updated_at_ms: number;
  /** The certificate as stored (canonical JSON). */
  certificate_json: string;
  certificate_sha256: string;
  /** The submitting retention verifier as stored principal JSON. */
  submitted_by: string;
}

const JOB_COLUMNS = `
    j.job_id, j.repository_id, j.certificate_id, j.state, j.generation,
    j.retention_epoch, j.evidence_revision, j.member_count,
    j.locations_withdrawn, j.objects_deleted, j.observations_deleted,
    j.not_before_ms, j.created_at_ms, j.updated_at_ms,
    c.certificate_json, c.certificate_sha256, c.submitted_by`;

const JOB_FROM = `
    FROM retention_jobs j
    JOIN retention_certificates c
      ON c.repository_id = j.repository_id AND c.certificate_id = j.certificate_id`;

export async function readJob(db: D1Database, repositoryId: string, jobId: string): Promise<RetentionJobRow | null> {
  return db
    .prepare(`SELECT ${JOB_COLUMNS} ${JOB_FROM} WHERE j.job_id = ?1 AND j.repository_id = ?2`)
    .bind(jobId, repositoryId)
    .first<RetentionJobRow>();
}

export async function readJobByCertificate(db: D1Database, repositoryId: string, certificateId: string): Promise<RetentionJobRow | null> {
  return db
    .prepare(`SELECT ${JOB_COLUMNS} ${JOB_FROM} WHERE j.repository_id = ?1 AND j.certificate_id = ?2`)
    .bind(repositoryId, certificateId)
    .first<RetentionJobRow>();
}

/** Unfinished jobs, oldest first; `jobId` narrows to one. */
export async function readUnfinishedJobs(db: D1Database, jobId: string | undefined): Promise<RetentionJobRow[]> {
  const { results } = await db
    .prepare(
      `SELECT ${JOB_COLUMNS} ${JOB_FROM}
       WHERE j.state NOT IN ('completed', 'invalidated') AND (?1 IS NULL OR j.job_id = ?1)
       ORDER BY j.created_at_ms, j.job_id`,
    )
    .bind(jobId ?? null)
    .all<RetentionJobRow>();
  return results;
}

// ---- Certificate checks ------------------------------------------------------

export interface CellObservationRow {
  observation_id: string;
  package: string;
  environment: string;
  gate: string;
  head: string;
  retry_of: string | null;
}

export interface CellKey {
  package: string;
  environment: string;
  gate: string;
}

/**
 * The stored observations of the named cells, at most `limit` rows. A
 * caller passing one more than the IDs it expects learns that an extra one
 * exists without reading an unbounded history.
 */
export async function readCellObservations(
  db: D1Database,
  repositoryId: string,
  cells: readonly CellKey[],
  limit: number,
): Promise<CellObservationRow[]> {
  const { results } = await db
    .prepare(
      `SELECT o.observation_id, o.package, o.environment, o.gate, o.head, o.retry_of
       FROM json_each(?2) j
       JOIN observations o
         ON o.repository_id = ?1
        AND o.package = json_extract(j.value, '$.package')
        AND o.gate = json_extract(j.value, '$.gate')
        AND o.environment = json_extract(j.value, '$.environment')
       ORDER BY o.observation_id
       LIMIT ?3`,
    )
    .bind(repositoryId, JSON.stringify(cells), limit)
    .all<CellObservationRow>();
  return results;
}

/** Which of `observationIds` are stored, pinned, or members of an unfinished job. */
export interface ObservationFacts {
  stored: Set<string>;
  pinned: Map<string, string>;
  scheduled: Map<string, string>;
}

export async function readObservationFacts(db: D1Database, repositoryId: string, observationIds: readonly string[]): Promise<ObservationFacts> {
  const ids = JSON.stringify(observationIds);
  const [stored, pinned, scheduled] = await db.batch<{ observation_id: string; detail: string }>([
    db
      .prepare(
        `SELECT o.observation_id, '' AS detail FROM observations o
         WHERE o.repository_id = ?1 AND o.observation_id IN (SELECT value FROM json_each(?2))`,
      )
      .bind(repositoryId, ids),
    db
      .prepare(
        `SELECT p.observation_id, MIN(p.release) AS detail FROM release_pins p
         WHERE p.repository_id = ?1 AND p.observation_id IN (SELECT value FROM json_each(?2))
         GROUP BY p.observation_id`,
      )
      .bind(repositoryId, ids),
    db
      .prepare(
        `SELECT m.observation_id, MIN(m.job_id) AS detail FROM retention_job_members m
         JOIN retention_jobs j ON j.job_id = m.job_id
         WHERE m.repository_id = ?1 AND m.observation_id IN (SELECT value FROM json_each(?2))
           AND j.state NOT IN ('completed', 'invalidated')
         GROUP BY m.observation_id`,
      )
      .bind(repositoryId, ids),
  ]);
  return {
    stored: new Set(stored!.results.map((row) => row.observation_id)),
    pinned: new Map(pinned!.results.map((row) => [row.observation_id, row.detail])),
    scheduled: new Map(scheduled!.results.map((row) => [row.observation_id, row.detail])),
  };
}

// ---- Certificate submission ---------------------------------------------------

export interface CertificateWrite {
  repository_id: string;
  certificate_id: string;
  certificate_json: string;
  certificate_sha256: string;
  snapshot: {
    generation: string;
    retention_epoch: number;
    evidence_revision: number;
    observation_event_watermark: number;
    location_event_watermark: number;
  };
  verifier_policy_digest: string;
  submitted_by: string;
  request_id: string;
  now_ms: number;
  job_id: string;
  grace_ms: number;
  /** Every `superseded_id` the certificate names; the first certification of each is recorded. */
  superseded_ids: string[];
  members: { observation_id: string; package: string; environment: string; gate: string }[];
  pins: { release: string; observation_id: string }[];
}

export type CertificateWriteOutcome = "written" | "stale";

/**
 * Persists a checked certificate, its supersession times, its job and
 * members, its release pins, and its audit row in one transaction. The
 * certificate insert is conditional on the repository row still equaling the
 * certificate's snapshot, and every later statement on that insert, so a
 * write that raced the checks inserts nothing (`stale`). A concurrent insert
 * of the same certificate ID throws (primary key); the caller reads it back.
 */
export async function writeCertificate(db: D1Database, write: CertificateWrite, actor: AuditActor, audit: AuditEntry): Promise<CertificateWriteOutcome> {
  const mine = "EXISTS (SELECT 1 FROM retention_certificates WHERE repository_id = ? AND certificate_id = ? AND submitted_request_id = ?)";
  const mineValues = [write.repository_id, write.certificate_id, write.request_id];
  const minePositional = `EXISTS (SELECT 1 FROM retention_certificates WHERE repository_id = ?1 AND certificate_id = ?2 AND submitted_request_id = ?3)`;
  const memberIds = JSON.stringify(write.members.map((member) => member.observation_id));
  const s = write.snapshot;
  const results = await db.batch([
    db
      .prepare(
        `INSERT INTO retention_certificates(repository_id, certificate_id, certificate_json, certificate_sha256,
           generation, retention_epoch, evidence_revision, verifier_policy_digest, submitted_by,
           submitted_request_id, submitted_at_ms)
         SELECT r.repository_id, ?2, ?3, ?4, r.generation, r.retention_epoch, r.evidence_revision, ?5, ?6, ?7, ?8
         FROM repositories r
         WHERE r.repository_id = ?1 AND r.generation = ?9 AND r.retention_epoch = ?10
           AND r.evidence_revision = ?11 AND r.observation_event_watermark = ?12
           AND r.location_event_watermark = ?13`,
      )
      .bind(
        write.repository_id,
        write.certificate_id,
        write.certificate_json,
        write.certificate_sha256,
        write.verifier_policy_digest,
        write.submitted_by,
        write.request_id,
        write.now_ms,
        s.generation,
        s.retention_epoch,
        s.evidence_revision,
        s.observation_event_watermark,
        s.location_event_watermark,
      ),
    db
      .prepare(
        `INSERT INTO retention_supersessions(repository_id, observation_id, first_certified_at_ms, certificate_id)
         SELECT ?1, j.value, ?4, ?2 FROM json_each(?5) j
         WHERE ${minePositional}
         ON CONFLICT(repository_id, observation_id) DO NOTHING`,
      )
      .bind(write.repository_id, write.certificate_id, write.request_id, write.now_ms, JSON.stringify(write.superseded_ids)),
    // The grace ends `grace_ms` after the latest first certification of any
    // member; an empty prune set completes at once. A job whose grace has
    // already ended starts here, while its snapshot is still current (the
    // certificate insert above matched only then): a verifier resubmits after
    // the grace, and waiting for the daily run would let any intervening
    // write move the revision and invalidate it.
    db
      .prepare(
        `INSERT INTO retention_jobs(job_id, repository_id, certificate_id, state, generation, retention_epoch,
           evidence_revision, member_count, not_before_ms, created_at_ms, updated_at_ms)
         SELECT ?4, c.repository_id, c.certificate_id,
                CASE WHEN ?5 = 0 THEN 'completed' WHEN g.not_before_ms <= ?6 THEN 'withdrawing' ELSE 'scheduled' END,
                c.generation, c.retention_epoch, c.evidence_revision, ?5, g.not_before_ms, ?6, ?6
         FROM retention_certificates c,
              (SELECT COALESCE((SELECT MAX(s.first_certified_at_ms) FROM retention_supersessions s
                                WHERE s.repository_id = ?1 AND s.observation_id IN (SELECT value FROM json_each(?7))) + ?8, ?6)
                      AS not_before_ms) g
         WHERE c.repository_id = ?1 AND c.certificate_id = ?2 AND c.submitted_request_id = ?3`,
      )
      .bind(write.repository_id, write.certificate_id, write.request_id, write.job_id, write.members.length, write.now_ms, memberIds, write.grace_ms),
    db
      .prepare(
        `INSERT INTO retention_job_members(job_id, repository_id, observation_id, package, environment, gate, superseded_at_ms)
         SELECT ?4, ?1, json_extract(j.value, '$.observation_id'), json_extract(j.value, '$.package'),
                json_extract(j.value, '$.environment'), json_extract(j.value, '$.gate'),
                (SELECT s.first_certified_at_ms FROM retention_supersessions s
                 WHERE s.repository_id = ?1 AND s.observation_id = json_extract(j.value, '$.observation_id'))
         FROM json_each(?5) j
         WHERE ${minePositional}`,
      )
      .bind(write.repository_id, write.certificate_id, write.request_id, write.job_id, JSON.stringify(write.members)),
    db
      .prepare(
        `INSERT INTO release_pins(repository_id, release, observation_id, pinned_at_ms)
         SELECT ?1, json_extract(j.value, '$.release'), json_extract(j.value, '$.observation_id'), ?4
         FROM json_each(?5) j
         WHERE ${minePositional}
         ON CONFLICT(repository_id, release, observation_id) DO NOTHING`,
      )
      .bind(write.repository_id, write.certificate_id, write.request_id, write.now_ms, JSON.stringify(write.pins)),
    auditStatementWhere(db, actor, audit, mine, mineValues),
  ]);
  return results[0]!.meta.changes === 1 ? "written" : "stale";
}

// ---- Job runner ------------------------------------------------------------------

/** Deletes the locks of finished jobs; returns how many. */
export async function releaseTerminalLocks(db: D1Database): Promise<number> {
  const result = await db
    .prepare("DELETE FROM cell_locks WHERE job_id IN (SELECT job_id FROM retention_jobs WHERE state IN ('completed', 'invalidated'))")
    .run();
  return result.meta.changes;
}

const REPOSITORY_UNCHANGED = `EXISTS (
    SELECT 1 FROM repositories r
    WHERE r.repository_id = retention_jobs.repository_id
      AND r.generation = retention_jobs.generation
      AND r.retention_epoch = retention_jobs.retention_epoch
      AND r.evidence_revision = retention_jobs.evidence_revision)`;

/**
 * Starts a scheduled job whose grace has ended, or invalidates it when the
 * repository's generation, epoch, or revision moved since its certificate.
 * The revision check and the transition are one conditional statement.
 */
export async function startOrInvalidate(db: D1Database, jobId: string, nowMs: number, actor: AuditActor, audit: AuditEntry): Promise<void> {
  await db.batch([
    db
      .prepare(
        `UPDATE retention_jobs SET state = 'withdrawing', updated_at_ms = ?2
         WHERE job_id = ?1 AND state = 'scheduled' AND not_before_ms <= ?2 AND ${REPOSITORY_UNCHANGED}`,
      )
      .bind(jobId, nowMs),
    db
      .prepare(
        `UPDATE retention_jobs SET state = 'invalidated', updated_at_ms = ?2
         WHERE job_id = ?1 AND state = 'scheduled' AND not_before_ms <= ?2 AND NOT ${REPOSITORY_UNCHANGED}`,
      )
      .bind(jobId, nowMs),
    auditStatementWhere(db, actor, audit, "changes() = 1", []),
  ]);
}

export async function readJobState(db: D1Database, jobId: string): Promise<RetentionJobRow["state"]> {
  const row = await db.prepare("SELECT state FROM retention_jobs WHERE job_id = ?1").bind(jobId).first<{ state: RetentionJobState }>();
  if (row === null) throw new Error(`retention job ${jobId} vanished`);
  return row.state;
}

/** A member's location whose newest event is not yet `withdrawn`. */
export interface PendingWithdrawal {
  seq: number;
  observation_id: string;
}

// The newest event of a location is the highest `seq` among rows of the same
// observation, reference, path, and locator (as `readLatestLocations` reads).
const NEWEST_NOT_WITHDRAWN = `
    FROM retention_job_members m
    JOIN report_locations l ON l.repository_id = m.repository_id AND l.observation_id = m.observation_id
    WHERE m.job_id = ?1 AND l.state <> 'withdrawn'
      AND l.seq = (
          SELECT MAX(n.seq) FROM report_locations n
          WHERE n.repository_id = l.repository_id AND n.observation_id = l.observation_id
            AND n.reference_kind = l.reference_kind AND n.reference_digest = l.reference_digest
            AND n.report_path = l.report_path AND n.location_key = l.location_key)`;

export async function readPendingWithdrawals(db: D1Database, jobId: string, limit: number): Promise<PendingWithdrawal[]> {
  const { results } = await db
    .prepare(`SELECT l.seq, l.observation_id ${NEWEST_NOT_WITHDRAWN} ORDER BY l.seq LIMIT ?2`)
    .bind(jobId, limit)
    .all<PendingWithdrawal>();
  return results;
}

/**
 * Appends one `withdrawn` event per pending location, copying the location
 * from its newest row, while the job is `withdrawing`. `withdrawals` pairs
 * each newest row's `seq` with a deterministic event ID, so a replay after a
 * crash inserts nothing new and the counter does not move.
 */
export async function appendWithdrawals(
  db: D1Database,
  job: Pick<RetentionJobRow, "job_id" | "repository_id" | "submitted_by">,
  withdrawals: readonly { seq: number; event_id: string }[],
  nowMs: number,
): Promise<void> {
  await db.batch([
    db
      .prepare(
        `INSERT INTO report_locations(repository_id, event_id, observation_id, reference_kind, reference_digest,
           report_path, location_key, location_kind, locator_json, state, verification_method,
           checked_at_ms, valid_until_ms, checked_by)
         SELECT ?1, json_extract(w.value, '$.event_id'), l.observation_id, l.reference_kind, l.reference_digest,
                l.report_path, l.location_key, l.location_kind, l.locator_json, 'withdrawn', NULL, ?2, NULL, ?3
         FROM json_each(?4) w
         JOIN report_locations l ON l.repository_id = ?1 AND l.seq = json_extract(w.value, '$.seq')
         WHERE EXISTS (SELECT 1 FROM retention_jobs WHERE job_id = ?5 AND state = 'withdrawing')
         ON CONFLICT(repository_id, event_id) DO NOTHING`,
      )
      .bind(job.repository_id, nowMs, job.submitted_by, JSON.stringify(withdrawals), job.job_id),
    db
      .prepare("UPDATE retention_jobs SET locations_withdrawn = locations_withdrawn + changes(), updated_at_ms = ?2 WHERE job_id = ?1")
      .bind(job.job_id, nowMs),
  ]);
}

const CELL_MATCH = "c.repository_id = m.repository_id AND c.package = m.package AND c.environment = m.environment AND c.gate = m.gate";

/**
 * Locks every member cell (all or none: nothing is taken while another job
 * holds any of them, so two jobs never deadlock), then moves the job to
 * `deleting-objects` only if it holds every lock and every member location's
 * newest event is still `withdrawn`. Under the lock no new `verified` event
 * can land (migration 0008), so a second attempt always succeeds.
 */
export async function lockAndStartDeleting(db: D1Database, jobId: string, nowMs: number): Promise<void> {
  await db.batch([
    db
      .prepare(
        `INSERT INTO cell_locks(repository_id, package, environment, gate, job_id, acquired_at_ms)
         SELECT DISTINCT m.repository_id, m.package, m.environment, m.gate, m.job_id, ?2
         FROM retention_job_members m
         WHERE m.job_id = ?1
           AND EXISTS (SELECT 1 FROM retention_jobs WHERE job_id = ?1 AND state = 'withdrawing')
           AND NOT EXISTS (
               SELECT 1 FROM retention_job_members x
               JOIN cell_locks c ON c.repository_id = x.repository_id AND c.package = x.package
                                AND c.environment = x.environment AND c.gate = x.gate
               WHERE x.job_id = ?1 AND c.job_id <> ?1)
         ON CONFLICT(repository_id, package, environment, gate) DO NOTHING`,
      )
      .bind(jobId, nowMs),
    db
      .prepare(
        `UPDATE retention_jobs SET state = 'deleting-objects', updated_at_ms = ?2
         WHERE job_id = ?1 AND state = 'withdrawing'
           AND NOT EXISTS (
               SELECT 1 FROM retention_job_members m
               WHERE m.job_id = ?1
                 AND NOT EXISTS (SELECT 1 FROM cell_locks c WHERE ${CELL_MATCH} AND c.job_id = ?1))
           AND NOT EXISTS (SELECT 1 ${NEWEST_NOT_WITHDRAWN})`,
      )
      .bind(jobId, nowMs),
  ]);
}

export interface MemberUpload {
  upload_id: string;
  staging_key: string;
}

/** The next `limit` member uploads in upload-ID order after the first `offset`. */
export async function readMemberUploads(db: D1Database, jobId: string, offset: number, limit: number): Promise<MemberUpload[]> {
  const { results } = await db
    .prepare(
      `SELECT u.upload_id, u.staging_key FROM uploads u
       JOIN retention_job_members m ON m.repository_id = u.repository_id AND m.observation_id = u.observation_id
       WHERE m.job_id = ?1
       ORDER BY u.upload_id
       LIMIT ?2 OFFSET ?3`,
    )
    .bind(jobId, limit, offset)
    .all<MemberUpload>();
  return results;
}

/** Records that the objects of `count` more uploads are deleted, only if no other run already did. */
export async function advanceObjectsDeleted(db: D1Database, jobId: string, from: number, count: number, nowMs: number): Promise<void> {
  await db
    .prepare(
      `UPDATE retention_jobs SET objects_deleted = ?3, updated_at_ms = ?4
       WHERE job_id = ?1 AND state = 'deleting-objects' AND objects_deleted = ?2`,
    )
    .bind(jobId, from, from + count, nowMs)
    .run();
}

/**
 * Moves the job to `deleting-rows` and advances the repository's retention
 * epoch in the same transaction, exactly once: the epoch statement applies
 * only when the transition just did.
 */
export async function startDeletingRows(db: D1Database, job: Pick<RetentionJobRow, "job_id" | "repository_id">, nowMs: number): Promise<void> {
  await db.batch([
    db
      .prepare("UPDATE retention_jobs SET state = 'deleting-rows', updated_at_ms = ?2 WHERE job_id = ?1 AND state = 'deleting-objects'")
      .bind(job.job_id, nowMs),
    db.prepare("UPDATE repositories SET retention_epoch = retention_epoch + 1 WHERE repository_id = ?1 AND changes() = 1").bind(job.repository_id),
  ]);
}

/** Members that still have an observation or upload row, in ID order. */
export async function readMembersWithRows(db: D1Database, jobId: string, limit: number): Promise<string[]> {
  const { results } = await db
    .prepare(
      `SELECT m.observation_id FROM retention_job_members m
       WHERE m.job_id = ?1 AND (
           EXISTS (SELECT 1 FROM observations o WHERE o.repository_id = m.repository_id AND o.observation_id = m.observation_id)
           OR EXISTS (SELECT 1 FROM uploads u WHERE u.repository_id = m.repository_id AND u.observation_id = m.observation_id))
       ORDER BY m.observation_id
       LIMIT ?2`,
    )
    .bind(jobId, limit)
    .all<{ observation_id: string }>();
  return results.map((row) => row.observation_id);
}

export async function readUploadsOf(db: D1Database, repositoryId: string, observationIds: readonly string[]): Promise<MemberUpload[]> {
  const { results } = await db
    .prepare(
      `SELECT upload_id, staging_key FROM uploads
       WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))
       ORDER BY upload_id`,
    )
    .bind(repositoryId, JSON.stringify(observationIds))
    .all<MemberUpload>();
  return results;
}

/**
 * Deletes one batch of members' rows while the job is `deleting-rows`:
 * releases their uploads' storage reservations, then deletes upload parts,
 * uploads, report locations (each leaving a `pruned` location tombstone),
 * supersession times, and observations (each leaving a `pruned` observation
 * tombstone and a revision step). `updated_at_ms` is set first because the
 * tombstone triggers stamp it as the tombstone time.
 */
export async function deleteMemberRows(
  db: D1Database,
  job: Pick<RetentionJobRow, "job_id" | "repository_id">,
  observationIds: readonly string[],
  nowMs: number,
): Promise<void> {
  const ids = JSON.stringify(observationIds);
  const active = "EXISTS (SELECT 1 FROM retention_jobs WHERE job_id = ?3 AND state = 'deleting-rows')";
  const uploadsOf = "SELECT upload_id FROM uploads WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))";
  await db.batch([
    db.prepare("UPDATE retention_jobs SET updated_at_ms = ?2 WHERE job_id = ?1 AND state = 'deleting-rows'").bind(job.job_id, nowMs),
    releasePrunedUploadsStatement(db, job.repository_id, ids, nowMs),
    db.prepare(`DELETE FROM upload_parts WHERE upload_id IN (${uploadsOf}) AND ${active}`).bind(job.repository_id, ids, job.job_id),
    db
      .prepare(`DELETE FROM uploads WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2)) AND ${active}`)
      .bind(job.repository_id, ids, job.job_id),
    db
      .prepare("DELETE FROM report_locations WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))")
      .bind(job.repository_id, ids),
    db
      .prepare("DELETE FROM retention_supersessions WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))")
      .bind(job.repository_id, ids),
    db
      .prepare("DELETE FROM observations WHERE repository_id = ?1 AND observation_id IN (SELECT value FROM json_each(?2))")
      .bind(job.repository_id, ids),
    db.prepare("UPDATE retention_jobs SET observations_deleted = observations_deleted + changes() WHERE job_id = ?1").bind(job.job_id),
  ]);
}

/**
 * Completes the job once no member row remains, audits it, and releases its
 * cell locks; returns how many locks were released.
 */
export async function completeJob(db: D1Database, jobId: string, nowMs: number, actor: AuditActor, audit: AuditEntry): Promise<number> {
  const results = await db.batch([
    db
      .prepare(
        `UPDATE retention_jobs SET state = 'completed', updated_at_ms = ?2
         WHERE job_id = ?1 AND state = 'deleting-rows'
           AND NOT EXISTS (
               SELECT 1 FROM retention_job_members m
               WHERE m.job_id = ?1 AND (
                   EXISTS (SELECT 1 FROM observations o WHERE o.repository_id = m.repository_id AND o.observation_id = m.observation_id)
                   OR EXISTS (SELECT 1 FROM uploads u WHERE u.repository_id = m.repository_id AND u.observation_id = m.observation_id)))`,
      )
      .bind(jobId, nowMs),
    auditStatementWhere(db, actor, audit, "changes() = 1", []),
    db.prepare("DELETE FROM cell_locks WHERE job_id = ?1 AND EXISTS (SELECT 1 FROM retention_jobs WHERE job_id = ?1 AND state = 'completed')").bind(jobId),
  ]);
  return results[2]!.meta.changes;
}

// ---- Tombstone expiry ---------------------------------------------------------------

export interface ExpiredTombstones {
  repository_id: string;
  /** Highest sequence among this chunk's expired tombstones: the new horizon. */
  horizon: number;
  count: number;
}

type Stream = "observation" | "location";

const STREAM = {
  observation: { table: "observation_events", key: "observation_id", horizon: "observation_event_horizon" },
  location: { table: "location_events", key: "event_id", horizon: "location_event_horizon" },
} as const;

/**
 * The oldest `limit` `pruned` tombstones of one stream recorded at or before
 * `cutoffMs`, grouped by repository.
 */
export async function readExpiredTombstones(db: D1Database, stream: Stream, cutoffMs: number, limit: number): Promise<ExpiredTombstones[]> {
  const { table } = STREAM[stream];
  const { results } = await db
    .prepare(
      `SELECT repository_id, MAX(seq) AS horizon, COUNT(*) AS count FROM (
           SELECT repository_id, seq FROM ${table}
           WHERE kind = 'pruned' AND recorded_at_ms <= ?1
           ORDER BY seq LIMIT ?2)
       GROUP BY repository_id`,
    )
    .bind(cutoffMs, limit)
    .all<ExpiredTombstones>();
  return results;
}

/**
 * Advances one repository's replay horizon to `horizon` (never backwards),
 * then deletes that stream's expired tombstones at or below it together with
 * every earlier event of the same observation (observation stream) or event
 * ID (location stream). Those events describe deleted rows and lie below the
 * horizon, where no change query can reach them. The horizon moves first in
 * the same transaction, so a reader never resumes across a deleted
 * tombstone. Returns how many events were deleted.
 */
export async function expireTombstones(db: D1Database, stream: Stream, repositoryId: string, horizon: number, cutoffMs: number): Promise<number> {
  const { table, key, horizon: column } = STREAM[stream];
  const results = await db.batch([
    db.prepare(`UPDATE repositories SET ${column} = MAX(${column}, ?2) WHERE repository_id = ?1`).bind(repositoryId, horizon),
    db
      .prepare(
        `DELETE FROM ${table}
         WHERE repository_id = ?1 AND seq <= ?2
           AND EXISTS (
               SELECT 1 FROM ${table} t
               WHERE t.repository_id = ?1 AND t.kind = 'pruned' AND t.recorded_at_ms <= ?3
                 AND t.seq <= ?2 AND t.${key} = ${table}.${key} AND t.seq >= ${table}.seq)`,
      )
      .bind(repositoryId, horizon, cutoffMs),
  ]);
  return results[1]!.meta.changes;
}
