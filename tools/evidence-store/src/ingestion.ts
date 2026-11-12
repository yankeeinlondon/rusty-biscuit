//! Collection ingestion and exact collection export.
//!
//! Ingestion validates the complete submission and applies every bound before
//! it touches D1, then hands the whole collection to one atomic batch
//! (`src/db/ingest.ts`). Export returns the exact submitted collection in
//! bounded pages; stored canonical observation text is spliced into the
//! response verbatim so number lexemes survive and the client can recompute
//! the collection ID from the pages.

import { publisherKey, type Principal } from "./auth.ts";
import { LIMITS, type IngestResult } from "./contracts/api.ts";
import { canonical, identity } from "./contracts/canonical.ts";
import type { SourceVersion } from "./contracts/evidence.ts";
import {
  childPointer,
  isJsonNumber,
  isJsonObject,
  objectKeys,
  type ContractProblem,
  type JsonObject,
} from "./contracts/json.ts";
import { loadStrict } from "./contracts/loader.ts";
import type { ParsedCollection } from "./contracts/parse.ts";
import { validateCollection } from "./contracts/validate.ts";
import { openCursor, queryDigest, signCursor } from "./cursor.ts";
import { ingestCollectionBatch, readCollectionManifest, readCollectionMembers, type IngestObservation } from "./db/ingest.ts";
import { ApiError } from "./http.ts";

export interface ParsedSubmission {
  readonly source_version: SourceVersion;
  readonly collection: ParsedCollection;
  /** Canonical observation text in collection order, as it is stored. */
  readonly observations: readonly IngestObservation[];
}

/** A rejected submission: `size_limit` problems answer 413, the rest 400. */
export interface SubmissionProblem extends ContractProblem {
  readonly status: "invalid_input" | "size_limit";
}

export type SubmissionResult =
  | { readonly ok: true; readonly value: ParsedSubmission }
  | { readonly ok: false; readonly problems: readonly SubmissionProblem[] };

const SUBMISSION_KEYS = ["collection", "source_version"] as const;

function prefixed(problems: readonly ContractProblem[], prefix: string): SubmissionProblem[] {
  return problems.map((problem) => ({ ...problem, location: prefix + problem.location, status: "invalid_input" }));
}

/**
 * Parse an ingestion request body without touching storage. Every loader,
 * contract, and size problem is reported with a JSON pointer into the body;
 * a result that is not `ok` must publish nothing.
 */
export function parseSubmission(bytes: Uint8Array): SubmissionResult {
  const fail = (problems: SubmissionProblem[]) => ({ ok: false as const, problems });
  if (bytes.byteLength > LIMITS.request_json_bytes) {
    return fail([
      {
        code: "size-limit",
        location: "",
        message: `request body is ${bytes.byteLength} bytes; the limit is ${LIMITS.request_json_bytes}`,
        status: "size_limit",
      },
    ]);
  }
  const loaded = loadStrict(bytes);
  if (!loaded.ok) return fail(prefixed(loaded.problems, ""));
  const body = loaded.value;
  if (!isJsonObject(body)) {
    return fail([{ code: "malformed-submission", location: "", message: "submission must be an object", status: "invalid_input" }]);
  }
  const problems: SubmissionProblem[] = [];
  for (const key of objectKeys(body)) {
    if (!(SUBMISSION_KEYS as readonly string[]).includes(key)) {
      problems.push({
        code: "malformed-submission",
        location: childPointer("", key),
        message: `submission has unknown field ${JSON.stringify(key)}`,
        status: "invalid_input",
      });
    }
  }
  for (const key of SUBMISSION_KEYS) {
    if (!Object.hasOwn(body, key)) {
      problems.push({ code: "malformed-submission", location: "", message: `submission is missing ${key}`, status: "invalid_input" });
    }
  }
  if (problems.length > 0) return fail(problems);

  const version = body["source_version"];
  // Exactly the integer tokens 2 or 3: `3.0`, `true`, and `"3"` are refused.
  if (!isJsonNumber(version) || !version.integer || (version.lexeme !== "2" && version.lexeme !== "3")) {
    return fail([
      { code: "malformed-submission", location: "/source_version", message: "source_version must be the integer 2 or 3", status: "invalid_input" },
    ]);
  }
  const sourceVersion: SourceVersion = version.lexeme === "2" ? 2 : 3;

  const document = body["collection"] as JsonObject;
  const members = isJsonObject(document) ? document["observations"] : undefined;
  if (Array.isArray(members) && members.length > LIMITS.collection_observations) {
    return fail([
      {
        code: "size-limit",
        location: "/collection/observations",
        message: `collection has ${members.length} observations; the limit is ${LIMITS.collection_observations}`,
        status: "size_limit",
      },
    ]);
  }
  const contract = validateCollection(document);
  if (contract.length > 0) return fail(prefixed(contract, "/collection"));
  const observations = members as JsonObject[];
  if (sourceVersion === 2 && observations.length === 0) {
    return fail([
      {
        code: "malformed-submission",
        location: "/collection/observations",
        message: "a collection adapted from a version-2 receipt has at least one observation",
        status: "invalid_input",
      },
    ]);
  }

  const stored: IngestObservation[] = [];
  for (const [index, observation] of observations.entries()) {
    const payload = canonical(observation);
    // Canonical text is pure ASCII, so its length is its byte count.
    if (payload.length > LIMITS.observation_canonical_bytes) {
      return fail([
        {
          code: "size-limit",
          location: `/collection/observations/${index}`,
          message: `observation is ${payload.length} canonical bytes; the limit is ${LIMITS.observation_canonical_bytes}`,
          status: "size_limit",
        },
      ]);
    }
    stored.push({ observation_id: observation["observation_id"] as string, payload_json: payload });
  }
  const text = canonical(document);
  const collection: ParsedCollection = {
    collection: document,
    canonical: text,
    collection_id: identity(text),
    observation_ids: stored.map((observation) => observation.observation_id),
  };
  return { ok: true, value: { source_version: sourceVersion, collection, observations: stored } };
}

function firstProblemError(problems: readonly SubmissionProblem[]): ApiError {
  const first = problems[0]!;
  const more = problems.length > 1 ? ` (and ${problems.length - 1} more problem${problems.length > 2 ? "s" : ""})` : "";
  return new ApiError(first.status, `${first.message}${more}`, first.location);
}

export interface RequestContext {
  readonly db: D1Database;
  readonly cursorKey: string;
  readonly requestId: string;
  readonly nowMs: number;
  readonly principal: Principal;
}

export async function ingest(context: RequestContext, repositoryId: string, body: Uint8Array): Promise<IngestResult> {
  const parsed = parseSubmission(body);
  if (!parsed.ok) throw firstProblemError(parsed.problems);
  const { collection } = parsed.value;
  const document = collection.collection as JsonObject;
  const outcome = await ingestCollectionBatch(context.db, {
    repository_id: repositoryId,
    collection_id: collection.collection_id,
    environment: document["environment"] as string,
    head: document["head"] as string,
    tree: document["tree"] as string,
    source_version: parsed.value.source_version,
    principal: publisherKey(context.principal.publisher),
    request_id: context.requestId,
    now_ms: context.nowMs,
    observations: [...parsed.value.observations],
  });
  switch (outcome.kind) {
    case "ok":
      return outcome.result;
    case "unknown_repository":
      throw new ApiError("not_found", `repository ${repositoryId} is not registered`);
    case "identity_conflict":
      throw new ApiError(
        "identity_conflict",
        "an observation or collection ID is already stored with different content or context; nothing was recorded",
        "/collection/observations",
      );
  }
}

interface RepositoryBounds {
  generation: string;
  retention_epoch: number;
}

async function repositoryBounds(db: D1Database, repositoryId: string): Promise<RepositoryBounds> {
  const row = await db
    .prepare("SELECT generation, retention_epoch FROM repositories WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<RepositoryBounds>();
  if (row === null) throw new ApiError("not_found", `repository ${repositoryId} is not registered`);
  return row;
}

function pageSize(raw: string | null): number {
  if (raw === null) return LIMITS.page_default_records;
  if (!/^[1-9][0-9]{0,2}$/.test(raw) || Number(raw) > LIMITS.page_max_records) {
    throw new ApiError("invalid_input", `page_size must be an integer from 1 to ${LIMITS.page_max_records}`, "/query/page_size");
  }
  return Number(raw);
}

/**
 * One page of an exact collection export, as response text. A collection
 * with any pruned member answers 410 `collection_pruned` on every page, never
 * a partial collection that would hash to a different ID.
 */
export async function exportCollection(
  context: RequestContext,
  repositoryId: string,
  collectionId: string,
  query: URLSearchParams,
): Promise<string> {
  const limit = pageSize(query.get("page_size"));
  const bounds = await repositoryBounds(context.db, repositoryId);
  const digest = await queryDigest(["exportCollection", collectionId, limit]);
  let after = -1;
  const cursor = query.get("cursor");
  if (cursor !== null) {
    const state = await openCursor(
      context.cursorKey,
      cursor,
      { repository_id: repositoryId, generation: bounds.generation, retention_epoch: bounds.retention_epoch, query_digest: digest },
      context.nowMs,
      "/query/cursor",
    );
    after = state.position;
  }
  const manifest = await readCollectionManifest(context.db, repositoryId, collectionId);
  if (manifest === null) throw new ApiError("not_found", `collection ${collectionId} is not stored in repository ${repositoryId}`);
  if (manifest.missing_member_count > 0) {
    throw new ApiError("collection_pruned", `${manifest.missing_member_count} member(s) of collection ${collectionId} were pruned`);
  }

  // One extra row tells whether another page exists.
  const page = await readCollectionMembers(context.db, repositoryId, collectionId, after, limit + 1);
  if (page.missing) {
    throw new ApiError("collection_pruned", `a member of collection ${collectionId} was pruned`);
  }
  const header =
    `{"collection_id":${JSON.stringify(manifest.collection_id)},"schema_version":3,` +
    `"environment":${JSON.stringify(manifest.environment)},"head":${JSON.stringify(manifest.head)},` +
    `"tree":${JSON.stringify(manifest.tree)},"member_count":${manifest.observation_count},` +
    `"acceptance":{"source_version":${manifest.source_version},"accepted_at_ms":${manifest.received_at_ms},` +
    `"accepted_by":${manifest.accepted_principal}},"items":[`;
  // Reserve room for the largest possible trailer (a full cursor).
  const budget = LIMITS.page_max_encoded_bytes - header.length - 2_200;
  const items: string[] = [];
  let used = 0;
  let last = after;
  for (const member of page.members.slice(0, limit)) {
    const payload = member.payload_json!;
    const cost = payload.length + (items.length > 0 ? 1 : 0);
    if (used + cost > budget) {
      if (items.length === 0) {
        throw new ApiError("unpageable_record", `observation ${member.observation_id} alone exceeds the page byte limit`);
      }
      break;
    }
    items.push(payload);
    used += cost;
    last = member.position;
  }
  const hasMore = last < manifest.observation_count - 1;
  const next = hasMore
    ? await signCursor(context.cursorKey, {
        repository_id: repositoryId,
        generation: bounds.generation,
        retention_epoch: bounds.retention_epoch,
        query_digest: digest,
        position: last,
        issued_at_ms: context.nowMs,
      })
    : null;
  return `${header}${items.join(",")}],"has_more":${hasMore},"next_cursor":${JSON.stringify(next)}}`;
}
