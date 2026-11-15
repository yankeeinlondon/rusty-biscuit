//! Authenticated identity and the role check every route applies.
//!
//! A `Principal` is the only source of repository and role permissions: grants
//! are never derived from submitted evidence such as `producer` fields. A
//! bearer token is either a host/operator token (`ebt_…`, checked against
//! primary D1 on every request) or a GitHub Actions OIDC JWT (checked against
//! the service configuration's exact workflow allowlist and the registered
//! repository). `api/README.md`, "Credentials" and "GitHub Actions (OIDC)".

import { createRemoteJWKSet, jwtVerify, type JWTPayload, type JWTVerifyGetKey } from "jose";
import { serviceConfig, type ServiceConfig } from "./config.ts";
import type { OperationId, Role } from "./contracts/api.ts";
import { OPERATIONS } from "./contracts/api.ts";
import type { Env } from "./env.ts";
import { ApiError } from "./http.ts";
import { FORMATS } from "./requests.ts";
import { constantTimeEqual, parseToken, secretDigest, TOKEN_PREFIX } from "./tokens.ts";

/** The `Principal` schema of `api/openapi.yaml`; persisted as the actual publisher. */
export type Publisher =
  | { kind: "credential"; credential_id: string }
  | {
      kind: "github-oidc";
      repository_id: string;
      workflow: string;
      ref: string;
      event_name: string;
      run_id: string;
      run_attempt: number;
    };

export interface Principal {
  readonly publisher: Publisher;
  /** Repository ID → roles held there. An empty map grants nothing. */
  readonly grants: ReadonlyMap<string, ReadonlySet<Role>>;
  /** Set only for operator credentials, which hold no repository grants. */
  readonly operator?: true;
}

export type Authenticator = (request: Request, env: Env, nowMs: number) => Promise<Principal>;

/** Stored and compared verbatim as the publisher of accepted evidence. */
export function publisherKey(publisher: Publisher): string {
  return JSON.stringify(publisher);
}

export const refuseAll: Authenticator = async () => {
  throw new ApiError("unauthenticated", "no credential verifier is configured");
};

export const GITHUB_JWKS_URL = "https://token.actions.githubusercontent.com/.well-known/jwks";

export interface AuthenticatorOptions {
  /** Key source for OIDC signatures; defaults to GitHub's remote JWKS, created on first use. */
  readonly jwks?: JWTVerifyGetKey;
}

const unauthenticated = (message: string) => new ApiError("unauthenticated", message);
const denied = (message: string) => new ApiError("denied", message);

/** The bearer value; its text never appears in an error. */
function bearerToken(request: Request): string {
  const header = request.headers.get("authorization");
  if (header === null) throw unauthenticated("an Authorization: Bearer credential is required");
  const match = /^Bearer ([^\s]+)$/i.exec(header);
  if (match === null) throw unauthenticated("the Authorization header is not a bearer credential");
  return match[1]!;
}

interface CredentialRow {
  kind: "host" | "operator";
  secret_sha256: string;
  expires_at_ms: number;
  revoked_at_ms: number | null;
}

async function hostPrincipal(db: D1Database, config: ServiceConfig, token: string, nowMs: number): Promise<Principal> {
  const parsed = parseToken(token);
  if (parsed === null) throw unauthenticated("the host token is malformed");
  const row = await db
    .prepare("SELECT kind, secret_sha256, expires_at_ms, revoked_at_ms FROM credentials WHERE credential_id = ?1")
    .bind(parsed.credentialId)
    .first<CredentialRow>();
  const digest = await secretDigest(parsed.secret);
  // Unknown ID and wrong secret answer identically, so the ID alone proves nothing.
  if (row === null || !constantTimeEqual(digest, row.secret_sha256)) throw unauthenticated("the host token is not valid");
  if (row.revoked_at_ms !== null) throw unauthenticated(`credential ${parsed.credentialId} is revoked`);
  if (nowMs >= row.expires_at_ms) throw unauthenticated(`credential ${parsed.credentialId} has expired`);
  const publisher: Publisher = { kind: "credential", credential_id: parsed.credentialId };
  if (row.kind === "operator") return { publisher, grants: new Map(), operator: true };

  const allowed = new Set(config.repositories.map((repository) => repository.repository_id));
  const rows = await db
    .prepare("SELECT repository_id, role FROM credential_grants WHERE credential_id = ?1")
    .bind(parsed.credentialId)
    .all<{ repository_id: string; role: Role }>();
  const grants = new Map<string, Set<Role>>();
  for (const { repository_id, role } of rows.results) {
    // A grant on a repository outside the configured allowlist is inert.
    if (!allowed.has(repository_id)) continue;
    let roles = grants.get(repository_id);
    if (roles === undefined) grants.set(repository_id, (roles = new Set()));
    roles.add(role);
  }
  return { publisher, grants };
}

/** jose failures that mean the token itself is bad; anything else is the key source failing. */
const TOKEN_FAILURES = new Set([
  "ERR_JWT_CLAIM_VALIDATION_FAILED",
  "ERR_JWT_EXPIRED",
  "ERR_JWT_INVALID",
  "ERR_JWS_INVALID",
  "ERR_JWS_SIGNATURE_VERIFICATION_FAILED",
  "ERR_JOSE_ALG_NOT_ALLOWED",
  "ERR_JOSE_NOT_SUPPORTED",
  "ERR_JWKS_NO_MATCHING_KEY",
  "ERR_JWKS_MULTIPLE_MATCHING_KEYS",
]);

async function verifyJwt(token: string, keys: JWTVerifyGetKey, config: ServiceConfig, nowMs: number): Promise<JWTPayload> {
  try {
    const { payload } = await jwtVerify(token, keys, {
      issuer: config.oidc.issuer,
      audience: config.oidc.audience,
      algorithms: ["RS256"],
      clockTolerance: config.oidc.clock_tolerance_seconds,
      currentDate: new Date(nowMs),
      requiredClaims: ["exp"],
    });
    return payload;
  } catch (error) {
    const code = (error as { code?: unknown } | null)?.code;
    if (typeof code === "string" && TOKEN_FAILURES.has(code)) throw unauthenticated("the OIDC token is not valid");
    throw new ApiError("unavailable", "the GitHub OIDC signing keys are unavailable", null, 1_000);
  }
}

function claim(payload: JWTPayload, name: string): string {
  const value = payload[name];
  if (typeof value !== "string" || value.length === 0) throw denied(`the OIDC token lacks a ${name} claim`);
  return value;
}

/** `<repository>/<path>@<ref>` → `<path>`, refusing a repository or ref that disagrees with its own claims. */
function pinnedWorkflow(payload: JWTPayload, repository: string, ref: string): string {
  const workflowRef = claim(payload, "workflow_ref");
  const prefix = `${repository}/`;
  const at = workflowRef.indexOf("@", prefix.length);
  if (!workflowRef.startsWith(prefix) || at === -1) throw denied("workflow_ref does not name this repository's workflow");
  if (workflowRef.slice(at + 1) !== ref) throw denied("workflow_ref does not match the ref claim");
  return workflowRef.slice(prefix.length, at);
}

async function oidcPrincipal(db: D1Database, config: ServiceConfig, keys: JWTVerifyGetKey, token: string, nowMs: number): Promise<Principal> {
  const payload = await verifyJwt(token, keys, config, nowMs);
  const repositoryId = claim(payload, "repository_id");
  const ownerId = claim(payload, "repository_owner_id");
  if (!FORMATS.repositoryId.test(repositoryId)) throw denied("repository_id is not a numeric GitHub ID");
  if (!FORMATS.repositoryId.test(ownerId)) throw denied("repository_owner_id is not a numeric GitHub ID");
  const eventName = claim(payload, "event_name");
  if (eventName === "pull_request" || eventName === "pull_request_target") throw denied("pull-request contexts are never trusted");
  for (const name of ["head_ref", "base_ref"]) {
    const value = payload[name];
    if (value !== undefined && value !== null && value !== "") throw denied("pull-request and fork contexts are never trusted");
  }
  const ref = claim(payload, "ref");
  const workflow = pinnedWorkflow(payload, claim(payload, "repository"), ref);
  const runId = claim(payload, "run_id");
  const runAttempt = Number(claim(payload, "run_attempt"));
  if (!FORMATS.repositoryId.test(runId) || !Number.isSafeInteger(runAttempt) || runAttempt < 1) {
    throw denied("run_id or run_attempt is malformed");
  }

  const roles = new Set<Role>();
  let trusted = false;
  for (const entry of config.oidc.trusted_workflows) {
    if (entry.repository_id === repositoryId && entry.workflow === workflow && entry.ref === ref && entry.event_name === eventName) {
      trusted = true;
      for (const role of entry.roles) roles.add(role);
    }
  }
  if (!trusted) throw denied("this workflow, ref, and event are not trusted");
  const allowance = config.repositories.find((repository) => repository.repository_id === repositoryId);
  if (allowance === undefined || allowance.owner_id !== ownerId) throw denied(`repository ${repositoryId} is not allowed for this owner`);
  const registered = await db
    .prepare("SELECT owner_id FROM repositories WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<{ owner_id: string | null }>();
  if (registered === null || registered.owner_id !== ownerId) throw denied(`repository ${repositoryId} is not registered for this owner`);

  return {
    publisher: { kind: "github-oidc", repository_id: repositoryId, workflow, ref, event_name: eventName, run_id: runId, run_attempt: runAttempt },
    grants: new Map([[repositoryId, roles]]),
  };
}

/**
 * The production authenticator. A token that fails verification answers 401;
 * a verified OIDC token from an untrusted context answers 403; an unreachable
 * key source answers 503.
 */
export function createAuthenticator(options: AuthenticatorOptions = {}): Authenticator {
  let keys = options.jwks;
  return async (request, env, nowMs) => {
    const config = serviceConfig(env);
    const token = bearerToken(request);
    if (token.startsWith(TOKEN_PREFIX)) return hostPrincipal(env.EVIDENCE_DB, config, token, nowMs);
    keys ??= createRemoteJWKSet(new URL(GITHUB_JWKS_URL));
    return oidcPrincipal(env.EVIDENCE_DB, config, keys, token, nowMs);
  };
}

/**
 * Unknown repositories answer 404 only after authorization succeeds, so a
 * caller without a grant cannot probe which repositories exist.
 */
export function authorize(principal: Principal, repositoryId: string, operation: OperationId): void {
  const held = principal.grants.get(repositoryId);
  const allowed: readonly Role[] = OPERATIONS[operation].roles;
  if (held === undefined || !allowed.some((role) => held.has(role))) {
    throw new ApiError("denied", `this credential may not call ${operation} on repository ${repositoryId}`);
  }
}
