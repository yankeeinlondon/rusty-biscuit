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
//! 4. the principal's request rate.
//!
//! A thrown `ApiError` becomes its envelope; any other failure (D1, R2, the
//! runtime, a missing binding) answers 503 `unavailable` without detail, never
//! an empty success.

import { authorize, createAuthenticator, publisherKey, type Authenticator, type Principal } from "./auth.ts";
import { serviceConfig } from "./config.ts";
import { API_VERSION, LIMITS, type OperationId } from "./contracts/api.ts";
import { auditDenied } from "./db/audit.ts";
import type { Env } from "./env.ts";
import { ApiError, errorResponse, jsonResponse, rawJsonResponse, readBody } from "./http.ts";
import { exportCollection, ingest, type RequestContext } from "./ingestion.ts";
import { appendLocation } from "./locations.ts";
import { issueCredential, listAudit, registerRepository, revokeCredential } from "./operator.ts";
import { getObservation, getSnapshot, queryChanges, queryHistory } from "./reads.ts";
import { FORMATS } from "./requests.ts";

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
  readonly handle: (context: RequestContext, repositoryId: string, request: Request, url: URL) => Promise<Response>;
}

const body = (request: Request) => readBody(request, LIMITS.request_json_bytes);

/** Matches `rest` (the path after `/v1/repos/{repository_id}`) to a repository operation. */
function matchRepositoryRoute(method: string, rest: readonly string[]): RepositoryRoute | null {
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
  readonly handle: (context: RequestContext, env: Env, request: Request) => Promise<Response>;
}

/** Matches `rest` (the path after `/v1/operator`) to an operator operation. */
function matchOperatorRoute(method: string, rest: readonly string[]): OperatorRoute | null {
  if (method !== "POST") return null;
  const path = rest.join("/");
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
    return operatorRoute.handle(context, env, request);
  }

  if (segments[1] !== "repos") throw notFound();
  const repositoryId = segments[2] ?? "";
  if (!FORMATS.repositoryId.test(repositoryId)) throw notFound();
  const repositoryRoute = matchRepositoryRoute(request.method, segments.slice(3));
  if (repositoryRoute === null) throw notFound();

  const principal = await options.authenticate(request, env, nowMs);
  const context: RequestContext = { db: env.EVIDENCE_DB, cursorKey: env.CURSOR_HMAC_KEY, requestId, nowMs, principal };
  if (repositoryRoute.operation !== "publishEvaluation") {
    try {
      authorize(principal, repositoryId, repositoryRoute.operation);
    } catch (error) {
      if (error instanceof ApiError && error.code === "denied") await auditDenied(context, repositoryId, repositoryRoute.operation);
      throw error;
    }
  }
  await enforceRate(env, principal, repositoryRoute.rate);
  return repositoryRoute.handle(context, repositoryId, request, url);
}

export function createWorker(options: WorkerOptions): ExportedHandler<Env> {
  return {
    async fetch(request, env): Promise<Response> {
      const requestId = crypto.randomUUID();
      try {
        return await route(request as unknown as Request, env, options, requestId);
      } catch (error) {
        if (error instanceof ApiError) return errorResponse(error, requestId);
        // The cause stays out of the response: it may quote stored evidence or a credential.
        return errorResponse(new ApiError("unavailable", "the evidence store is temporarily unavailable", null, 1_000), requestId);
      }
    },
  };
}

export default createWorker({ authenticate: createAuthenticator() });
