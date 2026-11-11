//! Verifier-only report-location events.
//!
//! An event says that one typed location of one report or proof file that an
//! observation references was verified, found unavailable, or withdrawn. The
//! route admits only the `verifier` role, so no producer can attest a
//! location. Events are keyed by repository and caller-supplied `event_id`;
//! the `report_locations_identity_guard` trigger (migration 0001) refuses a
//! different event under a used ID, and the `report_locations_appended`
//! trigger advances the evidence revision and appends the stream event.

import { publisherKey } from "./auth.ts";
import { LIMITS, LOCATION_EVENT_KINDS, LOCATOR_KINDS, REFERENCE_KINDS, type LocatorKind } from "./contracts/api.ts";
import { canonical, identity } from "./contracts/canonical.ts";
import { PROOF_COMPLETION_FILE, PROOF_EXPECTED_FILE } from "./contracts/evidence.ts";
import { isJsonObject, memberOf as member, type JsonObject } from "./contracts/json.ts";
import { loadStrict } from "./contracts/loader.ts";
import { readObservation } from "./db/ingest.ts";
import { readLocationEvent, uploadMatches } from "./db/reads.ts";
import { ApiError, rawJsonResponse } from "./http.ts";
import type { RequestContext } from "./ingestion.ts";
import { renderLocationEvent } from "./reads.ts";
import { FORMATS, optionalString, readRequestObject, requiredObject, requiredString, requireObject } from "./requests.ts";

const REQUEST_KEYS = [
  "event_id",
  "observation_id",
  "reference_kind",
  "reference_digest",
  "path",
  "locator",
  "kind",
  "verification_method",
] as const;

const LOCATOR_KEYS: Record<LocatorKind, readonly string[]> = {
  "host-dir": ["kind", "host", "directory"],
  artifact: ["kind", "repository_id", "run_id", "artifact_id", "member"],
  object: ["kind", "upload_id"],
};
const ANY_LOCATOR_KEY = [...new Set(Object.values(LOCATOR_KEYS).flat())];

const VERIFICATION_METHODS = ["bytes", "foreign-host-provenance"] as const;

interface LocationRequest {
  event_id: string;
  observation_id: string;
  reference_kind: (typeof REFERENCE_KINDS)[number];
  reference_digest: string;
  path: string;
  locator: Record<string, string>;
  kind: (typeof LOCATION_EVENT_KINDS)[number];
  verification_method: string | null;
}

function readLocator(body: JsonObject, repositoryId: string): Record<string, string> {
  const value = requiredObject(body, "locator", "", ANY_LOCATOR_KEY);
  const kind = requiredString(value, "kind", "/locator", { oneOf: LOCATOR_KINDS }) as LocatorKind;
  requireObject(value, "/locator", LOCATOR_KEYS[kind]);
  const numeric = { pattern: FORMATS.repositoryId };
  switch (kind) {
    case "host-dir":
      return {
        kind,
        host: requiredString(value, "host", "/locator"),
        directory: requiredString(value, "directory", "/locator"),
      };
    case "artifact": {
      const locator = {
        kind,
        repository_id: requiredString(value, "repository_id", "/locator", numeric),
        run_id: requiredString(value, "run_id", "/locator", numeric),
        artifact_id: requiredString(value, "artifact_id", "/locator", numeric),
        member: requiredString(value, "member", "/locator"),
      };
      if (locator.repository_id !== repositoryId) {
        throw new ApiError("invalid_input", "an artifact locator must name this repository", "/locator/repository_id");
      }
      return locator;
    }
    case "object":
      return { kind, upload_id: requiredString(value, "upload_id", "/locator", { pattern: FORMATS.serverId }) };
  }
}

function readLocationRequest(bytes: Uint8Array, repositoryId: string): LocationRequest {
  const body = readRequestObject(bytes, REQUEST_KEYS);
  const request: LocationRequest = {
    event_id: requiredString(body, "event_id", "", { pattern: FORMATS.uuid }),
    observation_id: requiredString(body, "observation_id", "", { pattern: FORMATS.observationId }),
    reference_kind: requiredString(body, "reference_kind", "", { oneOf: REFERENCE_KINDS }) as LocationRequest["reference_kind"],
    reference_digest: requiredString(body, "reference_digest", "", { maxLength: 256 }),
    path: requiredString(body, "path", ""),
    locator: readLocator(body, repositoryId),
    kind: requiredString(body, "kind", "", { oneOf: LOCATION_EVENT_KINDS }) as LocationRequest["kind"],
    verification_method: optionalString(body, "verification_method", "", { oneOf: VERIFICATION_METHODS }) ?? null,
  };
  if (request.kind === "verified" && request.verification_method === null) {
    throw new ApiError("invalid_input", "verification_method is required on a verified event", "/verification_method");
  }
  if (request.kind !== "verified" && request.verification_method !== null) {
    throw new ApiError("invalid_input", "verification_method is allowed only on a verified event", "/verification_method");
  }
  if (request.kind === "verified" && request.locator["kind"] === "object") {
    // Object bytes are verified only by upload finalization.
    throw new ApiError("invalid_input", "an object location is verified only by finalizing its upload", "/kind");
  }
  return request;
}

const encoder = new TextEncoder();

/** Refuses a reference the stored observation does not carry, located at the field that disagrees. */
function checkReference(payload: string, request: LocationRequest): void {
  const loaded = loadStrict(encoder.encode(payload));
  if (!loaded.ok || !isJsonObject(loaded.value)) throw new Error("stored observation payload is not a JSON object");
  const observation = loaded.value;
  const mismatch = (field: "path" | "reference_digest") =>
    new ApiError("invalid_input", `observation ${request.observation_id} carries no such ${request.reference_kind}`, `/${field}`);

  if (request.reference_kind === "report") {
    // Stored payloads passed `validateObservation`, so `reports` is an array of report objects.
    const reports = member(observation, "reports") as JsonObject[];
    const sameDigest = reports.filter((report) => member(report, "sha256") === request.reference_digest);
    if (sameDigest.length === 0) throw mismatch("reference_digest");
    if (!sameDigest.some((report) => member(report, "path") === request.path)) throw mismatch("path");
    return;
  }
  const proof = member(observation, "proof");
  const digestKey = request.reference_kind === "expected-listing" ? "expected_listing_digest" : "completion_record_digest";
  if (!isJsonObject(proof) || member(proof, digestKey) !== request.reference_digest) throw mismatch("reference_digest");
  const cell = member(observation, "cell") as JsonObject;
  const template = request.reference_kind === "expected-listing" ? PROOF_EXPECTED_FILE : PROOF_COMPLETION_FILE;
  const file = template.replace("{package}", member(cell, "package") as string).replace("{gate}", member(cell, "gate") as string);
  if (request.path !== file) throw mismatch("path");
}

// RETURNING yields a row only for a new insert, which is how a first insert
// (201) is told from an identical replay (200).
const INSERT_LOCATION = `
INSERT INTO report_locations(
    repository_id, event_id, observation_id, reference_kind, reference_digest,
    report_path, location_key, location_kind, locator_json, state,
    verification_method, checked_at_ms, valid_until_ms, checked_by
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
ON CONFLICT(repository_id, event_id) DO NOTHING
RETURNING seq`;

export async function appendLocation(context: RequestContext, repositoryId: string, bytes: Uint8Array): Promise<Response> {
  const request = readLocationRequest(bytes, repositoryId);
  const observation = await readObservation(context.db, repositoryId, request.observation_id);
  if (observation === null) {
    throw new ApiError("not_found", `observation ${request.observation_id} is not stored in repository ${repositoryId}`, "/observation_id");
  }
  checkReference(observation.payload_json, request);
  const uploadId = request.locator["upload_id"];
  if (
    uploadId !== undefined &&
    !(await uploadMatches(context.db, repositoryId, uploadId, request.observation_id, request.reference_kind, request.reference_digest))
  ) {
    throw new ApiError("not_found", `upload ${uploadId} does not hold this reference`, "/locator/upload_id");
  }

  const locatorJson = canonical(request.locator);
  const validUntil = request.kind === "verified" ? context.nowMs + LIMITS.location_attestation_hours * 3_600_000 : null;
  let inserted: boolean;
  try {
    const result = await context.db
      .prepare(INSERT_LOCATION)
      .bind(
        repositoryId,
        request.event_id,
        request.observation_id,
        request.reference_kind,
        request.reference_digest,
        request.path,
        identity(locatorJson),
        request.locator["kind"],
        locatorJson,
        request.kind,
        request.verification_method,
        context.nowMs,
        validUntil,
        publisherKey(context.principal.publisher),
      )
      .all();
    inserted = result.results.length > 0;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("identity_conflict")) {
      throw new ApiError("identity_conflict", `event ${request.event_id} is already recorded with a different payload; nothing was recorded`, "/event_id");
    }
    throw error;
  }
  const stored = await readLocationEvent(context.db, repositoryId, request.event_id);
  if (stored === null) throw new Error(`location event ${request.event_id} vanished after insert`);
  return rawJsonResponse(renderLocationEvent(stored), context.requestId, inserted ? 201 : 200);
}
