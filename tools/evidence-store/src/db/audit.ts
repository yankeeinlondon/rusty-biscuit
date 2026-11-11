//! Audit trail rows (`audit_log`, append-only by trigger).
//!
//! `actor` is `publisherKey(principal.publisher)`: the API's `Principal`
//! object as JSON text, parsed back by `listAudit`. A row never carries a
//! token or a secret digest.

import { publisherKey, type Principal } from "../auth.ts";
import type { RequestContext } from "../ingestion.ts";

export type AuditAction =
  | "repository.register"
  | "repository.rename"
  | "credential.issue"
  | "credential.revoke"
  | "credential.denied"
  | "operator.bootstrap";

export interface AuditEntry {
  readonly action: AuditAction;
  readonly repositoryId?: string | null;
  readonly targetCredentialId?: string | null;
  /** Serialized to `detail_json`; never a secret. */
  readonly detail?: unknown;
}

export const AUDIT_COLUMNS = "at_ms, action, actor, actor_credential_id, repository_id, target_credential_id, request_id, detail_json";

export interface AuditActor {
  readonly principal: Principal;
  readonly requestId: string;
  readonly nowMs: number;
}

function values(actor: AuditActor, entry: AuditEntry): (string | number | null)[] {
  const { publisher } = actor.principal;
  return [
    actor.nowMs,
    entry.action,
    publisherKey(publisher),
    publisher.kind === "credential" ? publisher.credential_id : null,
    entry.repositoryId ?? null,
    entry.targetCredentialId ?? null,
    actor.requestId,
    entry.detail === undefined ? null : JSON.stringify(entry.detail),
  ];
}

/** An insert for use inside a `batch()` alongside the change it records. */
export function auditStatement(db: D1Database, actor: AuditActor, entry: AuditEntry): D1PreparedStatement {
  return db.prepare(`INSERT INTO audit_log(${AUDIT_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`).bind(...values(actor, entry));
}

/**
 * An insert that happens only when `condition` (SQL over positional `?`
 * placeholders bound to `bindings`) holds when the statement runs, so a
 * batch can record a change exactly when its guarded update applies.
 */
export function auditStatementWhere(
  db: D1Database,
  actor: AuditActor,
  entry: AuditEntry,
  condition: string,
  bindings: readonly (string | number | null)[],
): D1PreparedStatement {
  return db
    .prepare(`INSERT INTO audit_log(${AUDIT_COLUMNS}) SELECT ?, ?, ?, ?, ?, ?, ?, ? WHERE ${condition}`)
    .bind(...values(actor, entry), ...bindings);
}

/**
 * Records a role refusal. Best-effort: a failed audit write is swallowed so a
 * 403 never becomes a 503.
 */
export async function auditDenied(context: RequestContext, repositoryId: string | null, operation: string): Promise<void> {
  try {
    await auditStatement(context.db, context, { action: "credential.denied", repositoryId, detail: { operation } }).run();
  } catch {
    // Deliberately ignored; see above.
  }
}
