//! Proof and report transfer: resumable uploads, verifier finalization,
//! authorized object fetch, and staging cleanup.
//!
//! An upload is bound at allocation to one repository, observation, and
//! reference (kind, digest, original path) with its byte length fixed then,
//! because historical proofs record no size. Parts are sequential 1 MiB
//! chunks, each stored as its own content-addressed R2 object
//! `staging/{upload_id}/{index}-{sha256}`; R2 multipart is not used because it
//! requires parts of at least 5 MiB. Keying a part by its digest makes a race
//! between two different first writes of one index safe: D1's first
//! `upload_parts` row wins and the loser answers 409.
//!
//! Finalization (verifier only) streams the parts in order into the final
//! object `objects/{repository}/{observation}/{kind}/{sha256(digest)}/{upload_id}`
//! while hashing them, checks length and digest, structurally checks a proof
//! (`src/contracts/proofs.ts`), and only then runs one D1 batch that marks the
//! upload finalized and appends a `verified` `object` location event whose
//! `event_id` is derived from the upload ID. A retry after the R2 write
//! succeeded but the batch failed rewrites the same object and inserts the
//! same event once. Finalization asserts byte integrity only, never test
//! coverage. Nothing here ever modifies an observation.
//!
//! Opening an upload reserves its projected R2 storage and write operations
//! (`src/capacity.ts`) in the same D1 write as the upload row; past the
//! refusal threshold it answers 429 `quota_exhausted` and records nothing.
//! Finalization commits the reservation; rejection, expiry, and pruning
//! release its storage. While a retention job holds the observation's cell
//! lock, opening an upload or finalizing one is refused with `cell_locked`.

import { publisherKey } from "./auth.ts";
import { commitUploadStatement } from "./capacity.ts";
import type { CapacityQuotas } from "./config.ts";
import { LIMITS, REFERENCE_KINDS, type ReferenceKind } from "./contracts/api.ts";
import { identity } from "./contracts/canonical.ts";
import type { JsonNumber } from "./contracts/json.ts";
import { proofCellOf, proofProblems } from "./contracts/proofs.ts";
import { readObservation } from "./db/ingest.ts";
import { readLocationEvent } from "./db/reads.ts";
import {
  closeUpload,
  expireUploads,
  finalizeStatement,
  insertPart,
  insertUpload,
  readFetchableObject,
  readPart,
  readParts,
  readReusableUpload,
  readUpload,
  readUploadStates,
  type PartRow,
  type UploadRow,
} from "./db/uploads.ts";
import type { Env } from "./env.ts";
import { ApiError, jsonResponse, rawJsonResponse, readBody } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import { checkReference, locationInsert } from "./locations.ts";
import { renderLocationEvent } from "./reads.ts";
import { FORMATS, readRequestObject, requiredInteger, requiredString } from "./requests.ts";

const HOUR_MS = 3_600_000;
const PART_BYTES = LIMITS.upload_part_bytes;

const CREATE_KEYS = ["observation_id", "reference_kind", "reference_digest", "bytes", "path"] as const;

function hex(bytes: ArrayBuffer | Uint8Array): string {
  return Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  return hex(await crypto.subtle.digest("SHA-256", bytes));
}

function partKey(uploadId: string, part: Pick<PartRow, "part_index" | "sha256">): string {
  return `staging/${uploadId}/${part.part_index}-${part.sha256}`;
}

/** The finalization event's UUID-shaped ID, a pure function of the upload, so a retry inserts it once. */
export function finalizeEventId(uploadId: string): string {
  const digest = identity(`finalize:${uploadId}`);
  return `${digest.slice(0, 8)}-${digest.slice(8, 12)}-${digest.slice(12, 16)}-${digest.slice(16, 20)}-${digest.slice(20, 32)}`;
}

/** The OpenAPI `Upload` shape. */
function renderUpload(row: UploadRow): Record<string, unknown> {
  return {
    upload_id: row.upload_id,
    observation_id: row.observation_id,
    reference_kind: row.reference_kind,
    reference_digest: row.reference_digest,
    path: row.path,
    bytes: row.bytes,
    part_bytes: row.part_bytes,
    part_count: row.part_count,
    parts_received: row.parts_received,
    state: row.state,
    expires_at_ms: row.expires_at_ms,
  };
}

function expiredError(uploadId: string): ApiError {
  return new ApiError("not_found", `upload ${uploadId} has expired; open a new upload`);
}

/** The upload, still `open`, or the error its state answers. An open upload past expiry is marked `expired`. */
async function openUpload(context: RequestContext, repositoryId: string, uploadId: string): Promise<UploadRow> {
  const upload = await readUpload(context.db, repositoryId, uploadId);
  if (upload === null) throw new ApiError("not_found", `upload ${uploadId} does not exist`);
  if (upload.state === "expired") throw expiredError(uploadId);
  if (upload.state === "rejected") {
    // Rejection covers a length, digest, or proof-binding failure; the original reason was answered then.
    throw new ApiError("invalid_input", `upload ${uploadId} was rejected when it was finalized; open a new upload`);
  }
  if (upload.state === "open" && context.nowMs >= upload.expires_at_ms) {
    await closeUpload(context.db, uploadId, "expired", context.nowMs);
    throw expiredError(uploadId);
  }
  return upload;
}

// ---- POST /uploads ---------------------------------------------------------

/** How long a client waits before retrying a refused admission; capacity frees only by pruning or a new month. */
const QUOTA_RETRY_MS = 3_600_000;

export async function createUpload(
  context: RequestContext,
  repositoryId: string,
  bytes: Uint8Array,
  quotas: CapacityQuotas,
): Promise<Response> {
  const body = readRequestObject(bytes, CREATE_KEYS);
  const request = {
    observation_id: requiredString(body, "observation_id", "", { pattern: FORMATS.observationId }),
    reference_kind: requiredString(body, "reference_kind", "", { oneOf: REFERENCE_KINDS }) as ReferenceKind,
    reference_digest: requiredString(body, "reference_digest", "", { maxLength: 256 }),
    bytes: requiredInteger(body, "bytes", "", { min: 0, max: Number.MAX_SAFE_INTEGER }),
    path: requiredString(body, "path", ""),
  };
  if (request.bytes > LIMITS.upload_object_max_bytes) {
    // The observation stays recorded; only its bytes cannot be stored here.
    throw new ApiError("size_limit", `an uploaded object is at most ${LIMITS.upload_object_max_bytes} bytes; this one declares ${request.bytes}`, "/bytes");
  }
  const observation = await readObservation(context.db, repositoryId, request.observation_id);
  if (observation === null) {
    throw new ApiError("not_found", `observation ${request.observation_id} is not stored in repository ${repositoryId}`, "/observation_id");
  }
  const { report } = checkReference(observation.payload_json, request);
  if (report !== null && Number((report["bytes"] as JsonNumber).lexeme) !== request.bytes) {
    throw new ApiError("invalid_input", "bytes differs from the length the observation records for this report", "/bytes");
  }

  const existing = await readReusableUpload(context.db, repositoryId, request);
  if (existing !== null && !(existing.state === "open" && context.nowMs >= existing.expires_at_ms)) {
    if (existing.bytes !== request.bytes) {
      throw new ApiError("identity_conflict", `upload ${existing.upload_id} holds this reference with a different length; nothing was recorded`, "/bytes");
    }
    return jsonResponse(renderUpload(existing), 200, context.requestId);
  }
  if (existing !== null) await closeUpload(context.db, existing.upload_id, "expired", context.nowMs);

  const uploadId = hex(crypto.getRandomValues(new Uint8Array(16)));
  const partCount = Math.ceil(request.bytes / PART_BYTES);
  const admitted = await insertUpload(context.db, {
    ...request,
    upload_id: uploadId,
    repository_id: repositoryId,
    part_count: partCount,
    // The digest is hashed so an arbitrary reference string is a safe key segment.
    staging_key: `objects/${repositoryId}/${request.observation_id}/${request.reference_kind}/${identity(request.reference_digest)}/${uploadId}`,
    created_by: publisherKey(context.principal.publisher),
    created_at_ms: context.nowMs,
    expires_at_ms: context.nowMs + LIMITS.upload_expiry_hours * HOUR_MS,
  }, quotas, { r2_bytes: request.bytes, r2_class_a: partCount + 1 });
  if (!admitted) {
    // The observation stays recorded; only its bytes wait for capacity.
    throw new ApiError(
      "quota_exhausted",
      "storing these bytes would pass the free-tier refusal threshold; nothing was recorded and no evidence was removed",
      null,
      QUOTA_RETRY_MS,
    );
  }
  const created = await readUpload(context.db, repositoryId, uploadId);
  if (created === null) throw new Error(`upload ${uploadId} vanished after insert`);
  return jsonResponse(renderUpload(created), 201, context.requestId);
}

// ---- PUT /uploads/{upload_id}/parts/{index} ---------------------------------

export async function putUploadPart(
  context: RequestContext,
  bucket: R2Bucket,
  repositoryId: string,
  uploadId: string,
  index: number,
  request: Request,
): Promise<Response> {
  const body = await readBody(request, PART_BYTES);
  const upload = await openUpload(context, repositoryId, uploadId);
  const sha256 = await sha256Hex(body);
  const result = (parts: number) =>
    jsonResponse({ upload_id: uploadId, index, bytes: body.byteLength, parts_received: parts }, 200, context.requestId);
  const conflict = () =>
    new ApiError("identity_conflict", `part ${index} of upload ${uploadId} was already received with different bytes; nothing was recorded`);

  const received = await readPart(context.db, uploadId, index);
  if (received !== null) {
    if (received.sha256 !== sha256) throw conflict();
    return result(upload.parts_received);
  }
  if (upload.state !== "open" || index > upload.parts_received) {
    throw new ApiError("invalid_input", `parts are sequential: upload ${uploadId} expects part ${upload.parts_received} next, not ${index}`);
  }
  if (index >= upload.part_count) {
    throw new ApiError("invalid_input", `upload ${uploadId} has ${upload.part_count} part(s); there is no part ${index}`);
  }
  const expected = index < upload.part_count - 1 ? PART_BYTES : upload.bytes - PART_BYTES * (upload.part_count - 1);
  if (body.byteLength !== expected) {
    throw new ApiError("invalid_input", `part ${index} of upload ${uploadId} must be exactly ${expected} bytes, not ${body.byteLength}`);
  }

  const part = { part_index: index, bytes: body.byteLength, sha256 };
  const stored = await bucket.put(partKey(uploadId, part), body);
  if (stored === null) throw new Error("an unconditional R2 put returned no object");
  await insertPart(context.db, uploadId, { ...part, r2_etag: stored.etag }, context.nowMs);
  const row = await readPart(context.db, uploadId, index);
  // No row: the upload left `open` meanwhile, which `openUpload` reports.
  if (row === null) await openUpload(context, repositoryId, uploadId);
  if (row === null || row.sha256 !== sha256) throw conflict();
  return result(upload.parts_received + 1);
}

// ---- POST /uploads/{upload_id}/finalize --------------------------------------

type Assembly = { readonly ok: true; readonly sha256: string; readonly bytes: Uint8Array | null } | { readonly ok: false; readonly reason: string };

/**
 * Streams the staged parts in order into the final object while hashing
 * them. The object is written through a fixed-length stream, so R2 knows its
 * size and refuses a short or long body; a length problem aborts the write.
 * `keep` also returns the bytes, for proof documents that must be parsed.
 */
async function assemble(bucket: R2Bucket, upload: UploadRow, parts: readonly PartRow[], keep: boolean): Promise<Assembly> {
  // Sizes are checked before the write opens, so a missing or altered part
  // is refused without aborting an R2 put (which only a race can still need).
  for (const part of parts) {
    const head = await bucket.head(partKey(upload.upload_id, part));
    if (head === null) return { ok: false, reason: `staged part ${part.part_index} is missing` };
    if (head.size !== part.bytes) return { ok: false, reason: `staged part ${part.part_index} no longer has its received length` };
  }
  const digest = new crypto.DigestStream("SHA-256");
  const digestWriter = digest.getWriter();
  const fixed = new FixedLengthStream(upload.bytes);
  const objectWriter = fixed.writable.getWriter();
  // An aborted write rejects these too; the outcome is read from `written`.
  objectWriter.closed.catch(() => undefined);
  objectWriter.ready.catch(() => undefined);
  const written = bucket.put(upload.staging_key, fixed.readable);
  const kept: Uint8Array[] = [];
  let length = 0;
  let reason: string | null = null;
  try {
    for (const part of parts) {
      const object = await bucket.get(partKey(upload.upload_id, part));
      if (object === null) {
        reason = `staged part ${part.part_index} is missing`;
        break;
      }
      const reader = object.body.getReader();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        length += value.byteLength;
        if (length > upload.bytes) {
          reason = `the staged parts hold more than the declared ${upload.bytes} bytes`;
          await reader.cancel();
          break;
        }
        await digestWriter.write(value);
        await objectWriter.write(value);
        if (keep) kept.push(value);
      }
      if (reason !== null) break;
    }
    if (reason === null && length !== upload.bytes) reason = `the staged parts hold ${length} bytes, not the declared ${upload.bytes}`;
  } catch (error) {
    await objectWriter.abort(error).catch(() => undefined);
    await written.catch(() => undefined);
    throw error;
  }
  if (reason !== null) {
    await objectWriter.abort(new Error(reason)).catch(() => undefined);
    // The aborted body makes the put fail; nothing was stored.
    await written.catch(() => undefined);
    return { ok: false, reason };
  }
  await objectWriter.close();
  await digestWriter.close();
  await written;
  const sha256 = hex(await digest.digest);
  if (!keep) return { ok: true, sha256, bytes: null };
  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const chunk of kept) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return { ok: true, sha256, bytes };
}

async function finalizeResult(context: RequestContext, repositoryId: string, upload: UploadRow): Promise<Response> {
  const event = await readLocationEvent(context.db, repositoryId, finalizeEventId(upload.upload_id));
  if (event === null || upload.sha256 === null) throw new Error(`finalized upload ${upload.upload_id} has no verification event`);
  const body = `{"upload":${JSON.stringify(renderUpload(upload))},"sha256":${JSON.stringify(upload.sha256)},"location":${renderLocationEvent(event)}}`;
  return rawJsonResponse(body, context.requestId);
}

/**
 * Verifies and publishes a fully received upload. A mismatch deletes the
 * final object, marks the upload `rejected`, and answers 400 with a message
 * that names the failed check but never quotes the uploaded bytes.
 */
export async function finalizeUpload(
  context: RequestContext,
  bucket: R2Bucket,
  repositoryId: string,
  uploadId: string,
  bytes: Uint8Array,
): Promise<Response> {
  if (bytes.byteLength > 0) readRequestObject(bytes, []);
  const existing = await readUpload(context.db, repositoryId, uploadId);
  if (existing?.state === "finalized") return finalizeResult(context, repositoryId, existing);
  const upload = await openUpload(context, repositoryId, uploadId);
  if (upload.parts_received < upload.part_count) {
    throw new ApiError(
      "invalid_input",
      `upload ${uploadId} has parts_received ${upload.parts_received} of part_count ${upload.part_count}; send the missing parts first`,
    );
  }
  const observation = await readObservation(context.db, repositoryId, upload.observation_id);
  if (observation === null) throw new ApiError("not_found", `observation ${upload.observation_id} is no longer stored`);

  const kind = upload.reference_kind as ReferenceKind;
  const assembled = await assemble(bucket, upload, await readParts(context.db, uploadId), kind !== "report");
  let reason: string | null = assembled.ok ? null : assembled.reason;
  if (assembled.ok && assembled.sha256 !== upload.reference_digest) reason = "the bytes do not match the reference digest";
  if (assembled.ok && reason === null && assembled.bytes !== null) {
    const { observation: loaded } = checkReference(observation.payload_json, upload as UploadRow & { reference_kind: ReferenceKind });
    const problems = proofProblems(kind, assembled.bytes, proofCellOf(loaded, observation));
    if (problems.length > 0) {
      const more = problems.length > 1 ? ` (and ${problems.length - 1} more problem${problems.length > 2 ? "s" : ""})` : "";
      reason = `${problems[0]!.message}${more}`;
    }
  }
  if (reason !== null) {
    await bucket.delete(upload.staging_key);
    await closeUpload(context.db, uploadId, "rejected", context.nowMs);
    throw new ApiError("invalid_input", `upload ${uploadId} was rejected: ${reason}`);
  }

  const sha256 = (assembled as { sha256: string }).sha256;
  await context.db.batch([
    finalizeStatement(context.db, uploadId, sha256, context.nowMs),
    locationInsert(
      context.db,
      repositoryId,
      {
        event_id: finalizeEventId(uploadId),
        observation_id: upload.observation_id,
        reference_kind: kind,
        reference_digest: upload.reference_digest,
        path: upload.path,
        locator: { kind: "object", upload_id: uploadId },
        kind: "verified",
        verification_method: "bytes",
        checked_at_ms: context.nowMs,
        valid_until_ms: context.nowMs + LIMITS.location_attestation_hours * HOUR_MS,
        checked_by: publisherKey(context.principal.publisher),
      },
      uploadId,
    ),
    commitUploadStatement(context.db, uploadId, context.nowMs),
  ]);
  const finalized = await readUpload(context.db, repositoryId, uploadId);
  if (finalized?.state !== "finalized") throw new Error(`upload ${uploadId} did not finalize`);
  // Leftover parts are only storage; `cleanupStaging` removes any this misses.
  await bucket.delete((await readParts(context.db, uploadId)).map((part) => partKey(uploadId, part))).catch(() => undefined);
  return finalizeResult(context, repositoryId, finalized);
}

// ---- GET /objects/{observation_id}/{reference_kind}/{reference_digest} -------

/**
 * Streams the newest finalized object of a reference whose newest object
 * location event is `verified`. A retrieval already streaming completes even
 * if a withdrawal lands meanwhile; the next fetch answers 404.
 */
export async function fetchObject(
  context: RequestContext,
  bucket: R2Bucket,
  repositoryId: string,
  observationId: string,
  referenceKind: ReferenceKind,
  referenceDigest: string,
): Promise<Response> {
  const found = await readFetchableObject(context.db, repositoryId, observationId, referenceKind, referenceDigest);
  if (found === null) {
    throw new ApiError("not_found", `no finalized, currently available ${referenceKind} of observation ${observationId} with this digest`);
  }
  const object = await bucket.get(found.staging_key);
  // The rows say the bytes exist: a missing object is a storage failure, not an absence.
  if (object === null) throw new ApiError("unavailable", "the object's bytes are temporarily unavailable", null, 1_000);
  return new Response(object.body, {
    status: 200,
    headers: {
      "content-type": "application/octet-stream",
      "content-length": String(found.bytes),
      "x-content-sha256": found.sha256,
      "x-request-id": context.requestId,
    },
  });
}

// ---- Staging cleanup -----------------------------------------------------------

export interface CleanupResult {
  /** Open uploads moved to `expired`. */
  expired: number;
  /** Uploads whose staging objects were removed. */
  cleaned_uploads: number;
  /** R2 objects deleted. */
  deleted_objects: number;
}

/**
 * Expires open uploads past their expiry (releasing the storage reserved
 * for every closed upload), then walks every staged part once:
 * a finalized upload loses its leftover parts; an expired or rejected one
 * created at least `staging_cleanup_hours` ago loses its parts and any final
 * object a failed finalization left; an open or younger one is kept. A part
 * whose upload row is gone (pruned) is removed once it is that old.
 */
export async function cleanupStaging(env: Pick<Env, "EVIDENCE_DB" | "EVIDENCE_BUCKET">, nowMs: number): Promise<CleanupResult> {
  const expired = await expireUploads(env.EVIDENCE_DB, nowMs);
  const cutoff = nowMs - LIMITS.staging_cleanup_hours * HOUR_MS;

  const staged = new Map<string, { key: string; uploaded: number }[]>();
  let cursor: string | undefined;
  do {
    const page = await env.EVIDENCE_BUCKET.list({ prefix: "staging/", ...(cursor === undefined ? {} : { cursor }) });
    for (const object of page.objects) {
      const uploadId = object.key.split("/")[1] ?? "";
      staged.set(uploadId, [...(staged.get(uploadId) ?? []), { key: object.key, uploaded: object.uploaded.getTime() }]);
    }
    cursor = page.truncated ? page.cursor : undefined;
  } while (cursor !== undefined);

  const states = new Map((await readUploadStates(env.EVIDENCE_DB, [...staged.keys()])).map((row) => [row.upload_id, row]));
  const doomed: string[] = [];
  let cleaned = 0;
  for (const [uploadId, objects] of staged) {
    const row = states.get(uploadId);
    let remove: boolean;
    if (row === undefined) remove = objects.every((object) => object.uploaded <= cutoff);
    else if (row.state === "finalized") remove = true;
    else remove = row.state !== "open" && row.created_at_ms <= cutoff;
    if (!remove) continue;
    cleaned += 1;
    doomed.push(...objects.map((object) => object.key));
    if (row !== undefined && row.state !== "finalized") doomed.push(row.staging_key);
  }
  // R2 deletes at most 1000 keys per call.
  for (let start = 0; start < doomed.length; start += 1000) {
    await env.EVIDENCE_BUCKET.delete(doomed.slice(start, start + 1000));
  }
  return { expired, cleaned_uploads: cleaned, deleted_objects: doomed.length };
}
