//! Evidence-store Worker entry point and route table.
//!
//! Routing is native `fetch` matching, no framework. Every request gets a
//! request ID and is checked in this order:
//!
//! 1. API version, then the service configuration (invalid configuration
//!    answers 503 to everything rather than running with a partial allowlist);
//! 2. authentication, which derives every grant from the authenticated
//!    identity and never from submitted fields;
//! 3. the operation's role on the repository (operator routes require an
//!    operator credential), before the repository is looked up, so a caller
//!    without a grant cannot probe which repositories exist;
//! 4. the principal's request rate;
//! 5. for a mutation, the persisted maintenance mode (`src/maintenance.ts`):
//!    a quiesced store answers 503 before any work starts, and a write that
//!    races the quiesce is refused by the database itself.
//!
//! A thrown `ApiError` becomes its envelope; any other failure (D1, R2, the
//! runtime, a missing binding) answers 503 `unavailable` without detail, never
//! an empty success.

import { authorize, createAuthenticator, publisherKey, type Authenticator, type Principal } from "./auth.ts";
import { serviceConfig } from "./config.ts";
import { API_VERSION, LIMITS, REFERENCE_KINDS, type OperationId, type ReferenceKind } from "./contracts/api.ts";
import { auditDenied } from "./db/audit.ts";
import type { Env } from "./env.ts";
import { ApiError, errorResponse, jsonResponse, rawJsonResponse, readBody } from "./http.ts";
import { exportCollection, ingest, type RequestContext } from "./ingestion.ts";
import { appendLocation } from "./locations.ts";
import { issueCredential, listAudit, registerRepository, revokeCredential } from "./operator.ts";
import { getObservation, getSnapshot, queryChanges, queryHistory } from "./reads.ts";
import { FORMATS } from "./requests.ts";
import { cellLockedError, getRetentionJob, isCellLocked, submitRetentionCertificate } from "./retention.ts";
import { createUpload, fetchObject, finalizeUpload, putUploadPart } from "./uploads.ts";
import { runDaily } from "./daily.ts";
import { assertWritable, isMaintenanceRefusal, maintenanceError } from "./maintenance.ts";
import { getMaintenance, recoverJobs, setMaintenance } from "./recovery.ts";

export type { Env } from "./env.ts";

export interface WorkerOptions {
  readonly authenticate: Authenticator;
  readonly now?: () => number;
}

type RateClass = "read" | "mutation" | "operator";

const RATE_LIMITERS: Record<RateClass, keyof Pick<Env, "READ_LIMITER" | "MUTATION_LIMITER" | "OPERATOR_LIMITER">> = {
  read: "READ_LIMITER",
  mutation: "MUTATION_LIMITER",
  operator: "OPERATOR_LIMITER",
};

/** Cloudflare's simple rate limits count over a 60-second period. */
const RATE_PERIOD_MS = 60_000;

async function enforceRate(env: Env, principal: Principal, rateClass: RateClass): Promise<void> {
  const limiter = env[RATE_LIMITERS[rateClass]];
  // A missing binding throws here and answers 503: requests are never unmetered.
  const { success } = await limiter.limit({ key: `${rateClass}:${publisherKey(principal.publisher)}` });
  if (!success) throw new ApiError("rate_limited", `request rate exceeded for this credential`, null, RATE_PERIOD_MS);
}

interface RepositoryRoute {
  readonly operation: OperationId;
  readonly rate: RateClass;
  readonly handle: (context: RequestContext, repositoryId: string, request: Request, url: URL, env: Env) => Promise<Response>;
}

const body = (request: Request) => readBody(request, LIMITS.request_json_bytes);

/** A decimal part index without leading zeros; anything else is not a route. */
const PART_INDEX = /^(?:0|[1-9][0-9]{0,8})$/;

/** A percent-decoded path segment, or null when its encoding is malformed. */
function decodeSegment(segment: string): string | null {
  try {
    return decodeURIComponent(segment);
  } catch {
    return null;
  }
}

/** Upload and object routes, whose paths are deeper than the rest. */
function matchTransferRoute(method: string, rest: readonly string[]): RepositoryRoute | null {
  const [first, second, third, fourth] = rest;
  if (first === "uploads" && rest.length === 1 && method === "POST") {
    return {
      operation: "createUpload",
      rate: "mutation",
      handle: async (context, repo, request, _url, env) => createUpload(context, repo, await body(request), serviceConfig(env).quotas.capacity),
    };
  }
  const uploadId = second ?? "";
  const missingUpload = () => new ApiError("not_found", `upload ${uploadId} does not exist`);
  if (first === "uploads" && rest.length === 4 && third === "parts" && method === "PUT") {
    const index = fourth ?? "";
    return {
      operation: "putUploadPart",
      rate: "mutation",
      handle: async (context, repo, request, _url, env) => {
        if (!FORMATS.serverId.test(uploadId)) throw missingUpload();
        if (!PART_INDEX.test(index)) throw new ApiError("not_found", "no such route");
        return putUploadPart(context, env.EVIDENCE_BUCKET, repo, uploadId, Number(index), request);
      },
    };
  }
  if (first === "uploads" && rest.length === 3 && third === "finalize" && method === "POST") {
    return {
      operation: "finalizeUpload",
      rate: "mutation",
      handle: async (context, repo, request, _url, env) => {
        if (!FORMATS.serverId.test(uploadId)) throw missingUpload();
        return finalizeUpload(context, env.EVIDENCE_BUCKET, repo, uploadId, await body(request));
      },
    };
  }
  if (first === "objects" && rest.length === 4 && method === "GET") {
    return {
      operation: "fetchObject",
      rate: "read",
      handle: async (context, repo, _request, _url, env) => {
        const observationId = second ?? "";
        const kind = third ?? "";
        const digest = decodeSegment(fourth ?? "");
        if (
          !FORMATS.observationId.test(observationId) ||
          !(REFERENCE_KINDS as readonly string[]).includes(kind) ||
          digest === null ||
          digest.length === 0 ||
          digest.length > 256
        ) {
          throw new ApiError("not_found", "no such object");
        }
        return fetchObject(context, env.EVIDENCE_BUCKET, repo, observationId, kind as ReferenceKind, digest);
      },
    };
  }
  return null;
}

/** Matches `rest` (the path after `/v1/repos/{repository_id}`) to a repository operation. */
function matchRepositoryRoute(method: string, rest: readonly string[]): RepositoryRoute | null {
  if (rest[0] === "uploads" || rest[0] === "objects") return matchTransferRoute(method, rest);
  if (rest[0] === "retention") {
    if (rest.length === 2 && rest[1] === "certificates" && method === "POST") {
      return {
        operation: "submitRetentionCertificate",
        rate: "mutation",
        handle: async (context, repo, request) => submitRetentionCertificate(context, repo, await body(request)),
      };
    }
    if (rest.length === 3 && rest[1] === "jobs" && method === "GET") {
      const jobId = rest[2]!;
      return {
        operation: "getRetentionJob",
        rate: "read",
        handle: async (context, repo) => {
          if (!FORMATS.serverId.test(jobId)) throw new ApiError("not_found", `retention job ${jobId} does not exist`);
          return getRetentionJob(context, repo, jobId);
        },
      };
    }
    return null;
  }
  const [first, second, ...more] = rest;
  if (more.length > 0) return null;
  if (first === "collections" && second === undefined && method === "POST") {
    return {
      operation: "ingestCollection",
      rate: "mutation",
      handle: async (context, repo, request) => jsonResponse(await ingest(context, repo, await body(request)), 200, context.requestId),
    };
  }
  if (first === "collections" && second !== undefined && method === "GET") {
    return {
      operation: "exportCollection",
      rate: "read",
      handle: async (context, repo, _request, url) => {
        if (!FORMATS.sha256.test(second)) throw new ApiError("not_found", `collection ${second} does not exist`);
        return rawJsonResponse(await exportCollection(context, repo, second, url.searchParams), context.requestId);
      },
    };
  }
  if (first === "observations" && second !== undefined && method === "GET") {
    return {
      operation: "getObservation",
      rate: "read",
      handle: async (context, repo) => {
        if (!FORMATS.observationId.test(second)) throw new ApiError("not_found", `observation ${second} does not exist`);
        return getObservation(context, repo, second);
      },
    };
  }
  if (first === "history" && second === "query" && method === "POST") {
    return { operation: "queryHistory", rate: "read", handle: async (context, repo, request) => queryHistory(context, repo, await body(request)) };
  }
  if (first === "snapshot" && second === undefined && method === "GET") {
    return { operation: "getSnapshot", rate: "read", handle: async (context, repo) => getSnapshot(context, repo) };
  }
  if (first === "changes" && second === "query" && method === "POST") {
    return { operation: "queryChanges", rate: "read", handle: async (context, repo, request) => queryChanges(context, repo, await body(request)) };
  }
  if (first === "locations" && second === undefined && method === "POST") {
    return { operation: "appendLocation", rate: "mutation", handle: async (context, repo, request) => appendLocation(context, repo, await body(request)) };
  }
  if (first === "evaluations" && second === undefined && method === "POST") {
    return {
      operation: "publishEvaluation",
      rate: "mutation",
      handle: async () => {
        throw new ApiError("evaluation_publication_disabled", "evaluation publication is disabled until a trusted verifier policy exists");
      },
    };
  }
  return null;
}

interface OperatorRoute {
  readonly rate: RateClass;
  /** Set on the routes that must work while the store is quiesced (maintenance control and credential revocation). */
  readonly duringMaintenance?: true;
  readonly handle: (context: RequestContext, env: Env, request: Request) => Promise<Response>;
}

/** Matches `rest` (the path after `/v1/operator`) to an operator operation. */
function matchOperatorRoute(method: string, rest: readonly string[]): OperatorRoute | null {
  const path = rest.join("/");
  if (path === "maintenance" && method === "GET") {
    return { rate: "read", handle: async (context) => getMaintenance(context) };
  }
  if (method !== "POST") return null;
  if (path === "maintenance") {
    return { rate: "operator", duringMaintenance: true, handle: async (context, _env, request) => setMaintenance(context, await body(request)) };
  }
  if (path === "jobs/recover") {
    return { rate: "operator", handle: async (context, env, request) => recoverJobs(context, env, await body(request)) };
  }
  if (path === "repositories") {
    return { rate: "operator", handle: async (context, env, request) => registerRepository(context, serviceConfig(env), await body(request)) };
  }
  if (path === "credentials") {
    return { rate: "operator", handle: async (context, _env, request) => issueCredential(context, await body(request)) };
  }
  if (rest.length === 3 && rest[0] === "credentials" && rest[2] === "revoke") {
    const credentialId = rest[1]!;
    return {
      rate: "operator",
      // Stopping a leaked credential outranks the backup boundary (0007 does not block revocation).
      duringMaintenance: true,
      handle: async (context, _env, request) => {
        if (!FORMATS.serverId.test(credentialId)) throw new ApiError("not_found", `credential ${credentialId} does not exist`);
        return revokeCredential(context, credentialId, await body(request));
      },
    };
  }
  if (path === "audit/query") {
    return { rate: "read", handle: async (context, _env, request) => listAudit(context, await body(request)) };
  }
  return null;
}

async function route(request: Request, env: Env, options: WorkerOptions, requestId: string): Promise<Response> {
  const url = new URL(request.url);
  const segments = url.pathname.split("/").slice(1);
  const version = segments[0];
  if (version !== undefined && /^v[0-9]+$/.test(version) && version !== API_VERSION) {
    throw new ApiError("unsupported_api_version", `API version ${version} is not supported; use ${API_VERSION}`);
  }
  const notFound = () => new ApiError("not_found", "no such route");
  if (version !== API_VERSION) throw notFound();
  serviceConfig(env);
  const nowMs = (options.now ?? Date.now)();

  if (segments[1] === "operator") {
    const operatorRoute = matchOperatorRoute(request.method, segments.slice(2));
    if (operatorRoute === null) throw notFound();
    const principal = await options.authenticate(request, env, nowMs);
    const context: RequestContext = { db: env.EVIDENCE_DB, cursorKey: env.CURSOR_HMAC_KEY, requestId, nowMs, principal };
    if (principal.operator !== true) {
      await auditDenied(context, null, "operator");
      throw new ApiError("denied", "operator routes require an operator credential");
    }
    await enforceRate(env, principal, operatorRoute.rate);
    if (operatorRoute.rate !== "read" && operatorRoute.duringMaintenance !== true) await assertWritable(env.EVIDENCE_DB);
    return operatorRoute.handle(context, env, request);
  }

  if (segments[1] !== "repos") throw notFound();
  const repositoryId = segments[2] ?? "";
  if (!FORMATS.repositoryId.test(repositoryId)) throw notFound();
  const repositoryRoute = matchRepositoryRoute(request.method, segments.slice(3));
  if (repositoryRoute === null) throw notFound();

  const principal = await options.authenticate(request, env, nowMs);
  const context: RequestContext = { db: env.EVIDENCE_DB, cursorKey: env.CURSOR_HMAC_KEY, requestId, nowMs, principal };
  // Operator credentials hold no repository grants; reading a retention job is
  // the one repository operation they may call.
  const operatorRead = repositoryRoute.operation === "getRetentionJob" && principal.operator === true;
  if (repositoryRoute.operation !== "publishEvaluation" && !operatorRead) {
    try {
      authorize(principal, repositoryId, repositoryRoute.operation);
    } catch (error) {
      if (error instanceof ApiError && error.code === "denied") await auditDenied(context, repositoryId, repositoryRoute.operation);
      throw error;
    }
  }
  await enforceRate(env, principal, repositoryRoute.rate);
  if (repositoryRoute.rate === "mutation") await assertWritable(env.EVIDENCE_DB);
  try {
    return await repositoryRoute.handle(context, repositoryId, request, url, env);
  } catch (error) {
    // A retention job is deleting in the cell; the database refused the write.
    if (isCellLocked(error)) throw cellLockedError();
    throw error;
  }
}

export function createWorker(options: WorkerOptions): ExportedHandler<Env> {
  return {
    async fetch(request, env): Promise<Response> {
      const requestId = crypto.randomUUID();
      try {
        return await route(request as unknown as Request, env, options, requestId);
      } catch (error) {
        if (error instanceof ApiError) return errorResponse(error, requestId);
        if (isMaintenanceRefusal(error)) return errorResponse(maintenanceError(), requestId);
        // The cause stays out of the response: it may quote stored evidence or a credential.
        return errorResponse(new ApiError("unavailable", "the evidence store is temporarily unavailable", null, 1_000), requestId);
      }
    },
    async scheduled(controller, env): Promise<void> {
      // A thrown step fails the invocation, which Cloudflare records; the next run resumes it.
      await runDaily(env, controller.scheduledTime);
    },
  };
}

export default createWorker({ authenticate: createAuthenticator() });
