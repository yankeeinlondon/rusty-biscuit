// Worker harness for the maintenance and recovery tests: the shared
// principals plus an operator, against any `Env` (so a test can wrap D1).
import { env } from "cloudflare:workers";
import type { Principal } from "../../src/auth.ts";
import { createWorker, type Env } from "../../src/worker.ts";
import { credentialPublisher, db, hex, principal } from "./helpers.ts";

export const OPERATOR_ID = "0f".repeat(16);

export interface RecoveryHarness {
  (path: string, init?: RequestInit & { as?: string }): Promise<Response>;
  clock: number;
}

/** Like `harness`, plus `operator`; `storage` replaces the bindings when given. */
export function recoveryHarness(repositories: readonly string[], storage: () => Env = () => env as unknown as Env): RecoveryHarness {
  const operator: Principal = { publisher: { kind: "credential", credential_id: OPERATOR_ID }, grants: new Map(), operator: true };
  const principals: Record<string, Principal> = {
    reader: principal("cccc", repositories, ["reader"]),
    producer: principal("aaaa", repositories, ["producer"]),
    verifier: principal("dddd", repositories, ["verifier"]),
    operator,
  };
  const worker = createWorker({
    authenticate: async (incoming) => {
      const who = principals[incoming.headers.get("x-test-principal") ?? ""];
      if (who === undefined) throw new Error("test request without a principal");
      return who;
    },
    now: () => request.clock,
  });
  const request = ((path: string, init: RequestInit & { as?: string } = {}) =>
    worker.fetch!(
      new Request(`https://evidence.test${path}`, {
        ...init,
        headers: { "x-test-principal": init.as ?? "reader", ...(init.headers as Record<string, string>) },
      }) as never,
      storage(),
      {} as ExecutionContext,
    )) as RecoveryHarness;
  request.clock = 1_800_000_000_000;
  return request;
}

export function post(request: RecoveryHarness, path: string, body: unknown, as: string): Promise<Response> {
  return request(path, { method: "POST", body: typeof body === "string" ? body : JSON.stringify(body), as });
}

/** Puts the store back in `open` without the API, so one test's quiesce never leaks into the next. */
export async function forceOpen(): Promise<void> {
  await db()
    .prepare("UPDATE service_state SET maintenance_mode = 'open', maintenance_reason = NULL, maintenance_since_ms = NULL, maintenance_boundary_id = NULL WHERE id = 1")
    .run();
}

export async function forceQuiesce(): Promise<void> {
  await db()
    .prepare("UPDATE service_state SET maintenance_mode = 'quiesced', maintenance_reason = 'test', maintenance_since_ms = 1, maintenance_boundary_id = NULL WHERE id = 1")
    .run();
}

/** Every row of every table but the audit log, plus every R2 key: what "writes nothing" compares. */
export async function storeContents(): Promise<{ rows: Record<string, unknown[]>; keys: string[] }> {
  const { results } = await db()
    .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%' AND name != 'audit_log' ORDER BY name")
    .all<{ name: string }>();
  const rows: Record<string, unknown[]> = {};
  for (const { name } of results) {
    rows[name] = (await db().prepare(`SELECT * FROM "${name}" ORDER BY rowid`).all()).results;
  }
  const keys: string[] = [];
  let cursor: string | undefined;
  do {
    const listed = await env.EVIDENCE_BUCKET.list(cursor === undefined ? {} : { cursor });
    keys.push(...listed.objects.map((object) => object.key));
    cursor = listed.truncated ? listed.cursor : undefined;
  } while (cursor !== undefined);
  return { rows, keys: keys.sort() };
}

/**
 * A retention certificate and job in `state` for `observationIds`, recorded
 * at the repository's current generation, epoch, and revision (so the job is
 * current) and submitted by a renderable retention-verifier publisher.
 */
export async function currentRetentionJob(repositoryId: string, state: string, observationIds: readonly string[], notBeforeMs = 0): Promise<string> {
  const repository = (await db()
    .prepare("SELECT generation, retention_epoch, evidence_revision FROM repositories WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<{ generation: string; retention_epoch: number; evidence_revision: number }>())!;
  const certificateId = crypto.randomUUID();
  const jobId = hex(32);
  const submitter = credentialPublisher("bbbb");
  await db().batch([
    db()
      .prepare(
        `INSERT INTO retention_certificates(repository_id, certificate_id, certificate_json, certificate_sha256, generation,
           retention_epoch, evidence_revision, verifier_policy_digest, submitted_by, submitted_request_id, submitted_at_ms)
         VALUES (?1, ?2, '{}', ?3, ?4, ?5, ?6, ?3, ?7, 'req', 1)`,
      )
      .bind(repositoryId, certificateId, hex(64), repository.generation, repository.retention_epoch, repository.evidence_revision, submitter),
    db()
      .prepare(
        `INSERT INTO retention_jobs(job_id, repository_id, certificate_id, state, generation, retention_epoch, evidence_revision,
           member_count, not_before_ms, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, 2)`,
      )
      .bind(jobId, repositoryId, certificateId, state, repository.generation, repository.retention_epoch, repository.evidence_revision, observationIds.length, notBeforeMs),
    ...observationIds.map((id) =>
      db()
        .prepare(
          `INSERT INTO retention_job_members(job_id, repository_id, observation_id, package, environment, gate, superseded_at_ms)
           VALUES (?1, ?2, ?3, 'biscuit-hash', 'ubuntu-latest', 'L1', 0)`,
        )
        .bind(jobId, repositoryId, id),
    ),
  ]);
  return jobId;
}
