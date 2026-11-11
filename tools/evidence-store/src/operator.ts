//! Operator routes: repository registration, host credential issue and
//! revocation, and the audit trail. The Worker reaches these only with an
//! operator credential; every change is recorded in `audit_log` in the same
//! `batch()` as the change itself.

import type { Publisher } from "./auth.ts";
import type { ServiceConfig } from "./config.ts";
import { LIMITS, type Role } from "./contracts/api.ts";
import { parseCredentialRequest } from "./credential-request.ts";
import { openCursor, queryDigest, signCursor } from "./cursor.ts";
import { auditStatement, auditStatementWhere } from "./db/audit.ts";
import { ApiError, jsonResponse } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import { FORMATS, optionalInteger, optionalString, readRequestObject, requiredString } from "./requests.ts";
import { mintToken, randomId, secretDigest } from "./tokens.ts";

const DAY_MS = 86_400_000;
const SLUG = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;

interface RepositoryRow {
  repository_id: string;
  owner_id: string | null;
  slug: string;
  generation: string;
  retention_epoch: number;
  evidence_revision: number;
  observation_event_watermark: number;
  location_event_watermark: number;
  registered_at_ms: number;
}

async function repositoryRow(db: D1Database, repositoryId: string): Promise<RepositoryRow | null> {
  return db
    .prepare(
      `SELECT repository_id, owner_id, slug, generation, retention_epoch, evidence_revision,
              observation_event_watermark, location_event_watermark, registered_at_ms
       FROM repositories WHERE repository_id = ?1`,
    )
    .bind(repositoryId)
    .first<RepositoryRow>();
}

function repositoryRecord(row: RepositoryRow) {
  return {
    repository_id: row.repository_id,
    owner_id: row.owner_id,
    slug: row.slug,
    registered_at_ms: row.registered_at_ms,
    snapshot: {
      repository_id: row.repository_id,
      generation: row.generation,
      retention_epoch: row.retention_epoch,
      evidence_revision: row.evidence_revision,
      observation_event_watermark: row.observation_event_watermark,
      location_event_watermark: row.location_event_watermark,
    },
  };
}

function isUniqueViolation(error: unknown): boolean {
  return error instanceof Error && /UNIQUE constraint failed/.test(error.message);
}

async function slugHolder(db: D1Database, slug: string, repositoryId: string): Promise<string | null> {
  const row = await db
    .prepare("SELECT repository_id FROM repositories WHERE slug = ?1 AND repository_id != ?2")
    .bind(slug, repositoryId)
    .first<{ repository_id: string }>();
  return row?.repository_id ?? null;
}

/**
 * Registers a repository named in the configuration allowlist, or records a
 * rename. The owner is fixed at first registration.
 */
export async function registerRepository(context: RequestContext, config: ServiceConfig, body: Uint8Array): Promise<Response> {
  const object = readRequestObject(body, ["repository_id", "owner_id", "slug"]);
  const repositoryId = requiredString(object, "repository_id", "", { pattern: FORMATS.repositoryId });
  const ownerId = requiredString(object, "owner_id", "", { pattern: FORMATS.repositoryId });
  const slug = requiredString(object, "slug", "", { pattern: SLUG });
  const allowance = config.repositories.find((repository) => repository.repository_id === repositoryId);
  if (allowance === undefined || allowance.owner_id !== ownerId) {
    throw new ApiError("denied", `repository ${repositoryId} with owner ${ownerId} is not in the service configuration`);
  }

  const { db } = context;
  const slugTaken = async () => {
    const holder = await slugHolder(db, slug, repositoryId);
    return holder === null ? null : new ApiError("identity_conflict", `slug ${slug} belongs to repository ${holder}`, "/slug");
  };
  // A concurrent registration can win between the read and the write; the
  // second pass then sees its row and answers as for an existing repository.
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const existing = await repositoryRow(db, repositoryId);
    if (existing === null) {
      const conflict = await slugTaken();
      if (conflict !== null) throw conflict;
      try {
        await db.batch([
          db
            .prepare(
              `INSERT INTO repositories(repository_id, owner_id, slug, generation, registered_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5)`,
            )
            .bind(repositoryId, ownerId, slug, randomId(), context.nowMs),
          auditStatement(db, context, { action: "repository.register", repositoryId, detail: { owner_id: ownerId, slug } }),
        ]);
      } catch (error) {
        if (isUniqueViolation(error)) continue;
        throw error;
      }
      return jsonResponse(repositoryRecord((await repositoryRow(db, repositoryId))!), 201, context.requestId);
    }
    if (existing.owner_id !== ownerId) {
      throw new ApiError("identity_conflict", `repository ${repositoryId} is registered to another owner`, "/owner_id");
    }
    if (existing.slug !== slug) {
      const conflict = await slugTaken();
      if (conflict !== null) throw conflict;
      try {
        await db.batch([
          db.prepare("UPDATE repositories SET slug = ?2 WHERE repository_id = ?1").bind(repositoryId, slug),
          auditStatement(db, context, { action: "repository.rename", repositoryId, detail: { from: existing.slug, to: slug } }),
        ]);
      } catch (error) {
        if (isUniqueViolation(error)) throw (await slugTaken()) ?? error;
        throw error;
      }
      return jsonResponse(repositoryRecord((await repositoryRow(db, repositoryId))!), 200, context.requestId);
    }
    return jsonResponse(repositoryRecord(existing), 200, context.requestId);
  }
  throw new ApiError("unavailable", "repository registration did not settle", null, 1_000);
}

interface CredentialRow {
  credential_id: string;
  kind: "host" | "operator";
  label: string;
  issued_at_ms: number;
  rotate_at_ms: number;
  expires_at_ms: number;
  replaces: string | null;
  revoked_at_ms: number | null;
  revocation_reason: string | null;
  repository_ids_json: string;
  roles_json: string;
}

async function credentialRow(db: D1Database, credentialId: string): Promise<CredentialRow | null> {
  return db
    .prepare(
      `SELECT credential_id, kind, label, issued_at_ms, rotate_at_ms, expires_at_ms, replaces,
              revoked_at_ms, revocation_reason, repository_ids_json, roles_json
       FROM credentials WHERE credential_id = ?1`,
    )
    .bind(credentialId)
    .first<CredentialRow>();
}

/** The public `Credential` record: never the secret or its digest. */
function credentialRecord(row: CredentialRow) {
  const roles: Role[] = row.kind === "operator" ? ["operator"] : (JSON.parse(row.roles_json) as Role[]);
  return {
    credential_id: row.credential_id,
    label: row.label,
    repository_ids: JSON.parse(row.repository_ids_json) as string[],
    roles,
    issued_at_ms: row.issued_at_ms,
    expires_at_ms: row.expires_at_ms,
    rotate_at_ms: row.rotate_at_ms,
    ...(row.replaces === null ? {} : { replaces: row.replaces }),
    ...(row.revoked_at_ms === null ? {} : { revoked_at_ms: row.revoked_at_ms, revocation_reason: row.revocation_reason }),
  };
}

/**
 * Issues a host credential. Naming `replaces` shortens the replaced
 * credential to at most `credential_overlap_max_days` from now. One batch
 * writes the credential, its grants, the shortened expiry, and the audit rows.
 */
export async function issueCredential(context: RequestContext, body: Uint8Array): Promise<Response> {
  const request = parseCredentialRequest(body);
  const { db, nowMs } = context;
  const repositoryIds = JSON.stringify(request.repository_ids);
  const roles = JSON.stringify(request.roles);

  const unknown = await db
    .prepare(
      `SELECT key, value FROM json_each(?1)
       WHERE value NOT IN (SELECT repository_id FROM repositories) ORDER BY key LIMIT 1`,
    )
    .bind(repositoryIds)
    .first<{ key: number; value: string }>();
  if (unknown !== null) {
    throw new ApiError("not_found", `repository ${unknown.value} is not registered`, `/repository_ids/${unknown.key}`);
  }

  const overlapEndMs = nowMs + LIMITS.credential_overlap_max_days * DAY_MS;
  if (request.replaces !== undefined) {
    const replaced = await credentialRow(db, request.replaces);
    if (replaced === null) throw new ApiError("not_found", `credential ${request.replaces} does not exist`, "/replaces");
    if (replaced.kind !== "host") throw new ApiError("invalid_input", `credential ${request.replaces} is not a host credential`, "/replaces");
    if (replaced.revoked_at_ms !== null) throw new ApiError("invalid_input", `credential ${request.replaces} is revoked`, "/replaces");
  }

  const minted = mintToken();
  const expiresAtMs = nowMs + request.expires_in_days * DAY_MS;
  const rotateAtMs = Math.min(nowMs + LIMITS.credential_rotation_day * DAY_MS, expiresAtMs);
  const detail = {
    label: request.label,
    roles: request.roles,
    expires_at_ms: expiresAtMs,
    ...(request.replaces === undefined ? {} : { replaces: request.replaces }),
  };
  const auditTargets = request.repository_ids.length === 0 ? [null] : request.repository_ids;
  const statements: D1PreparedStatement[] = [
    db
      .prepare(
        `INSERT INTO credentials(credential_id, kind, label, secret_sha256, issued_at_ms, rotate_at_ms,
           expires_at_ms, replaces, repository_ids_json, roles_json)
         VALUES (?1, 'host', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)`,
      )
      .bind(minted.credentialId, request.label, await secretDigest(minted.secret), nowMs, rotateAtMs, expiresAtMs, request.replaces ?? null, repositoryIds, roles),
    db
      .prepare(
        `INSERT INTO credential_grants(credential_id, repository_id, role)
         SELECT ?1, repository.value, role.value FROM json_each(?2) AS repository, json_each(?3) AS role`,
      )
      .bind(minted.credentialId, repositoryIds, roles),
    ...auditTargets.map((repositoryId) =>
      auditStatement(db, context, { action: "credential.issue", repositoryId, targetCredentialId: minted.credentialId, detail }),
    ),
  ];
  if (request.replaces !== undefined) {
    statements.push(
      db
        .prepare(
          `UPDATE credentials
           SET expires_at_ms = MIN(expires_at_ms, ?2), rotate_at_ms = MIN(rotate_at_ms, expires_at_ms, ?2)
           WHERE credential_id = ?1`,
        )
        .bind(request.replaces, overlapEndMs),
    );
  }
  await db.batch(statements);
  const issued = (await credentialRow(db, minted.credentialId))!;
  return jsonResponse({ credential: credentialRecord(issued), secret: minted.token }, 201, context.requestId);
}

/** Revokes immediately; revoking twice answers the original revocation and audits once. */
export async function revokeCredential(context: RequestContext, credentialId: string, body: Uint8Array): Promise<Response> {
  const object = readRequestObject(body, ["reason"]);
  const reason = requiredString(object, "reason", "", { maxLength: 512 });
  const { db } = context;
  const existing = await credentialRow(db, credentialId);
  if (existing === null) throw new ApiError("not_found", `credential ${credentialId} does not exist`);
  if (existing.revoked_at_ms === null) {
    // The audit insert runs first and only while the credential is still
    // unrevoked, so two racing revocations record one audit row between them.
    await db.batch([
      auditStatementWhere(
        db,
        context,
        { action: "credential.revoke", targetCredentialId: credentialId, detail: { reason } },
        "EXISTS (SELECT 1 FROM credentials WHERE credential_id = ? AND revoked_at_ms IS NULL)",
        [credentialId],
      ),
      db
        .prepare("UPDATE credentials SET revoked_at_ms = ?2, revocation_reason = ?3 WHERE credential_id = ?1 AND revoked_at_ms IS NULL")
        .bind(credentialId, context.nowMs, reason),
    ]);
  }
  return jsonResponse(credentialRecord((await credentialRow(db, credentialId))!), 200, context.requestId);
}

const AUDIT_ACTIONS = [
  "repository.register",
  "repository.rename",
  "credential.issue",
  "credential.revoke",
  "credential.denied",
  "retention.certificate",
  "retention.job",
  "retention.recover",
  "operator.bootstrap",
  "restore.generation",
  "maintenance.quiesce",
  "maintenance.resume",
] as const;

/** Audit cursors reuse `CursorState`; these fixed values stand in for a repository snapshot. */
const AUDIT_CURSOR_SCOPE = { repository_id: "operator-audit", generation: "audit", retention_epoch: 0 } as const;

interface AuditRow {
  seq: number;
  at_ms: number;
  action: string;
  actor: string;
  repository_id: string | null;
  target_credential_id: string | null;
  request_id: string;
}

function auditRecord(row: AuditRow) {
  return {
    sequence: row.seq,
    at_ms: row.at_ms,
    action: row.action,
    actor: JSON.parse(row.actor) as Publisher,
    ...(row.repository_id === null ? {} : { repository_id: row.repository_id }),
    ...(row.target_credential_id === null ? {} : { target_credential_id: row.target_credential_id }),
    request_id: row.request_id,
  };
}

/** Pages `audit_log` in sequence order. `credential_id` matches the target or the acting credential. */
export async function listAudit(context: RequestContext, body: Uint8Array): Promise<Response> {
  const object = readRequestObject(body, ["repository_id", "credential_id", "action", "since_ms", "until_ms", "page_size", "cursor"]);
  const timestamp = { min: 0, max: Number.MAX_SAFE_INTEGER };
  const repositoryId = optionalString(object, "repository_id", "", { pattern: FORMATS.repositoryId }) ?? null;
  const credentialId = optionalString(object, "credential_id", "", { pattern: FORMATS.serverId }) ?? null;
  const action = optionalString(object, "action", "", { oneOf: AUDIT_ACTIONS }) ?? null;
  const sinceMs = optionalInteger(object, "since_ms", "", timestamp) ?? null;
  const untilMs = optionalInteger(object, "until_ms", "", timestamp) ?? null;
  const pageSize = optionalInteger(object, "page_size", "", { min: 1, max: LIMITS.page_max_records }) ?? LIMITS.page_default_records;
  const cursor = optionalString(object, "cursor", "", { maxLength: 2048 });

  const digest = await queryDigest([repositoryId ?? "", credentialId ?? "", action ?? "", sinceMs ?? -1, untilMs ?? -1, pageSize]);
  const after =
    cursor === undefined
      ? 0
      : (await openCursor(context.cursorKey, cursor, { ...AUDIT_CURSOR_SCOPE, query_digest: digest }, context.nowMs, "/cursor")).position;

  const rows = await context.db
    .prepare(
      `SELECT seq, at_ms, action, actor, repository_id, target_credential_id, request_id FROM audit_log
       WHERE seq > ?1
         AND (?2 IS NULL OR repository_id = ?2)
         AND (?3 IS NULL OR target_credential_id = ?3 OR actor_credential_id = ?3)
         AND (?4 IS NULL OR action = ?4)
         AND (?5 IS NULL OR at_ms >= ?5)
         AND (?6 IS NULL OR at_ms <= ?6)
       ORDER BY seq LIMIT ?7`,
    )
    .bind(after, repositoryId, credentialId, action, sinceMs, untilMs, pageSize + 1)
    .all<AuditRow>();
  const page = rows.results.slice(0, pageSize);
  const hasMore = rows.results.length > pageSize;
  const nextCursor = hasMore
    ? await signCursor(context.cursorKey, {
        ...AUDIT_CURSOR_SCOPE,
        query_digest: digest,
        position: page[page.length - 1]!.seq,
        issued_at_ms: context.nowMs,
      })
    : null;
  return jsonResponse({ items: page.map(auditRecord), has_more: hasMore, next_cursor: nextCursor }, 200, context.requestId);
}
