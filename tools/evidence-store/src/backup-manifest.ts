//! The one validator for a backup manifest
//! (`api/schemas/backup-manifest.schema.json`), shared by `backup` (which
//! writes it), `verify-backup`, and `restore`.
//!
//! The manifest is read through the shared strict loader and the field readers
//! of `requests.ts`, so a duplicate key, trailing content, an explicit `null`,
//! an unknown field, or a wrong type is refused at its JSON pointer like any
//! request. Every list is required (an empty list is valid and means none);
//! no element is ever skipped. File names are confined to the backup
//! directory: plain relative segments, none starting with a dot.

import type { JsonObject, JsonValue } from "./contracts/json.ts";
import { childPointer } from "./contracts/json.ts";
import { ApiError } from "./http.ts";
import {
  checkInteger,
  FORMATS,
  optionalInteger,
  optionalString,
  readRequestObject,
  requiredInteger,
  requiredObject,
  requiredString,
  requireObject,
} from "./requests.ts";

export const BACKUP_FORMAT_VERSION = 1;
export const MANIFEST_FILE = "manifest.json";

/** A relative path inside the backup directory; no segment may start with a dot, so `..` cannot escape it. */
export const BACKUP_FILE = /^[A-Za-z0-9_-][A-Za-z0-9._-]*(?:\/[A-Za-z0-9_-][A-Za-z0-9._-]*)*$/;

const ENVIRONMENTS = ["development", "restore-test"] as const;
const JOB_STATES = ["scheduled", "withdrawing", "deleting-objects", "deleting-rows", "completed", "invalidated"] as const;
const REFERENCE_KINDS = ["report", "expected-listing", "completion-record"] as const;
const GRANT_ROLES = ["reader", "producer", "verifier", "retention-verifier"] as const;
const OBJECT_MAX_BYTES = 67_108_864;
const COUNT = { min: 0, max: Number.MAX_SAFE_INTEGER } as const;

export interface BackupFile {
  readonly file: string;
  readonly sha256: string;
  readonly bytes: number;
}

export interface RepositoryCounters {
  readonly repository_id: string;
  readonly generation: string;
  readonly retention_epoch: number;
  readonly evidence_revision: number;
  readonly observation_event_watermark: number;
  readonly location_event_watermark: number;
}

/** A finalized upload's object, copied into the backup and digest-checked against `uploads.sha256`. */
export interface BackupObject extends BackupFile {
  readonly upload_id: string;
  readonly repository_id: string;
  readonly key: string;
}

/** A finalized upload whose object could not be copied intact; restore marks it `unavailable`. */
export interface MissingObject {
  readonly upload_id: string;
  readonly repository_id: string;
  readonly key: string;
  readonly sha256: string;
  readonly bytes: number;
  readonly reason: "absent" | "digest-mismatch";
}

/** An upload still open at the boundary: pending, never acknowledged. Its staged parts are not copied. */
export interface PendingUpload {
  readonly upload_id: string;
  readonly repository_id: string;
  readonly observation_id: string;
  readonly reference_kind: string;
  readonly bytes: number;
  readonly created_at_ms: number;
  readonly expires_at_ms: number;
}

export interface RetentionJobRecord {
  readonly job_id: string;
  readonly repository_id: string;
  readonly state: (typeof JOB_STATES)[number];
  readonly generation: string;
  readonly retention_epoch: number;
  readonly evidence_revision: number;
  readonly updated_at_ms: number;
}

/** A credential's digest and lifetime. Never the bearer secret, which the database does not hold. */
export interface CredentialRecord {
  readonly credential_id: string;
  readonly kind: "host" | "operator";
  readonly secret_sha256: string;
  readonly expires_at_ms: number;
  /** Present exactly when the credential was revoked at the boundary. */
  readonly revoked_at_ms?: number;
}

export interface GrantRecord {
  readonly credential_id: string;
  readonly repository_id: string;
  readonly role: (typeof GRANT_ROLES)[number];
}

/** A `credential.revoke` audit record from the quiesced window, which restore re-applies. */
export interface WindowRevocation {
  readonly credential_id: string;
  readonly at_ms: number;
}

export interface BackupManifest {
  readonly format_version: 1;
  readonly backup_id: string;
  readonly created_at_ms: number;
  readonly boundary: { readonly boundary_id: string; readonly quiesced_at_ms: number; readonly resumed_at_ms: number };
  readonly source: { readonly environment: (typeof ENVIRONMENTS)[number]; readonly d1_database_name: string; readonly d1_database_id: string; readonly r2_bucket: string };
  readonly migration: number;
  readonly repositories: readonly RepositoryCounters[];
  readonly database: BackupFile;
  readonly objects: readonly BackupObject[];
  readonly missing_objects: readonly MissingObject[];
  readonly pending_uploads: readonly PendingUpload[];
  readonly retention_jobs: readonly RetentionJobRecord[];
  readonly credentials: readonly CredentialRecord[];
  readonly grants: readonly GrantRecord[];
  readonly window_revocations: readonly WindowRevocation[];
}

function invalid(location: string, message: string): ApiError {
  return new ApiError("invalid_input", message, location);
}

/** The required list `key` of `object`, each element an object with `allowed` keys. */
export function requiredObjectArray(object: JsonObject, key: string, pointer: string, allowed: readonly string[]): JsonObject[] {
  const location = childPointer(pointer, key);
  if (!Object.hasOwn(object, key)) throw invalid(location, `${key} is required`);
  const value: JsonValue = object[key]!;
  if (value === null) throw invalid(location, `${key} must not be null`);
  if (!Array.isArray(value)) throw invalid(location, `${location} must be an array`);
  return value.map((item, index) => requireObject(item, childPointer(location, index), allowed));
}

const sha256 = { pattern: FORMATS.sha256 } as const;
const repositoryId = { pattern: FORMATS.repositoryId } as const;
const serverId = { pattern: FORMATS.serverId } as const;

function backupFile(object: JsonObject, pointer: string, maxBytes = Number.MAX_SAFE_INTEGER): BackupFile {
  return {
    file: requiredString(object, "file", pointer, { pattern: BACKUP_FILE, maxLength: 256 }),
    sha256: requiredString(object, "sha256", pointer, sha256),
    bytes: requiredInteger(object, "bytes", pointer, { min: 0, max: maxBytes }),
  };
}

function counters(object: JsonObject, pointer: string): RepositoryCounters {
  return {
    repository_id: requiredString(object, "repository_id", pointer, repositoryId),
    generation: requiredString(object, "generation", pointer, { maxLength: 256 }),
    retention_epoch: requiredInteger(object, "retention_epoch", pointer, COUNT),
    evidence_revision: requiredInteger(object, "evidence_revision", pointer, COUNT),
    observation_event_watermark: requiredInteger(object, "observation_event_watermark", pointer, COUNT),
    location_event_watermark: requiredInteger(object, "location_event_watermark", pointer, COUNT),
  };
}

const COUNTER_KEYS = ["repository_id", "generation", "retention_epoch", "evidence_revision", "observation_event_watermark", "location_event_watermark"];
const ROOT_KEYS = [
  "$schema",
  "format_version",
  "backup_id",
  "created_at_ms",
  "boundary",
  "source",
  "migration",
  "repositories",
  "database",
  "objects",
  "missing_objects",
  "pending_uploads",
  "retention_jobs",
  "credentials",
  "grants",
  "window_revocations",
];

/** Reads a manifest; throws `invalid_input` located at the first offending field. */
export function parseBackupManifest(bytes: Uint8Array): BackupManifest {
  const root = readRequestObject(bytes, ROOT_KEYS);
  optionalString(root, "$schema", "", { minLength: 0 });
  const version = Object.hasOwn(root, "format_version") ? root["format_version"]! : undefined;
  if (version === undefined) throw invalid("/format_version", "format_version is required");
  if (checkInteger(version, "/format_version", COUNT) !== BACKUP_FORMAT_VERSION) {
    throw invalid("/format_version", `format_version must be ${BACKUP_FORMAT_VERSION}`);
  }
  const backupId = requiredString(root, "backup_id", "", serverId);
  const boundaryObject = requiredObject(root, "boundary", "", ["boundary_id", "quiesced_at_ms", "resumed_at_ms"]);
  const boundary = {
    boundary_id: requiredString(boundaryObject, "boundary_id", "/boundary", serverId),
    quiesced_at_ms: requiredInteger(boundaryObject, "quiesced_at_ms", "/boundary", COUNT),
    resumed_at_ms: requiredInteger(boundaryObject, "resumed_at_ms", "/boundary", COUNT),
  };
  if (boundary.boundary_id !== backupId) throw invalid("/boundary/boundary_id", "boundary_id must equal backup_id");
  if (boundary.resumed_at_ms < boundary.quiesced_at_ms) throw invalid("/boundary/resumed_at_ms", "resumed_at_ms precedes quiesced_at_ms");
  const sourceObject = requiredObject(root, "source", "", ["environment", "d1_database_name", "d1_database_id", "r2_bucket"]);
  const source = {
    environment: requiredString(sourceObject, "environment", "/source", { oneOf: ENVIRONMENTS }) as (typeof ENVIRONMENTS)[number],
    d1_database_name: requiredString(sourceObject, "d1_database_name", "/source", { maxLength: 256 }),
    d1_database_id: requiredString(sourceObject, "d1_database_id", "/source", { maxLength: 256 }),
    r2_bucket: requiredString(sourceObject, "r2_bucket", "/source", { maxLength: 256 }),
  };

  const list = (key: string, allowed: readonly string[]) => requiredObjectArray(root, key, "", allowed);
  const at = (key: string, index: number) => `/${key}/${index}`;
  return {
    format_version: BACKUP_FORMAT_VERSION,
    backup_id: backupId,
    created_at_ms: requiredInteger(root, "created_at_ms", "", COUNT),
    boundary,
    source,
    migration: requiredInteger(root, "migration", "", { min: 1, max: 9999 }),
    repositories: list("repositories", COUNTER_KEYS).map((item, index) => counters(item, at("repositories", index))),
    database: backupFile(requiredObject(root, "database", "", ["file", "sha256", "bytes"]), "/database"),
    objects: list("objects", ["upload_id", "repository_id", "key", "file", "sha256", "bytes"]).map((item, index) => {
      const pointer = at("objects", index);
      return {
        upload_id: requiredString(item, "upload_id", pointer, serverId),
        repository_id: requiredString(item, "repository_id", pointer, repositoryId),
        key: requiredString(item, "key", pointer, { maxLength: 1024 }),
        ...backupFile(item, pointer, OBJECT_MAX_BYTES),
      };
    }),
    missing_objects: list("missing_objects", ["upload_id", "repository_id", "key", "sha256", "bytes", "reason"]).map((item, index) => {
      const pointer = at("missing_objects", index);
      return {
        upload_id: requiredString(item, "upload_id", pointer, serverId),
        repository_id: requiredString(item, "repository_id", pointer, repositoryId),
        key: requiredString(item, "key", pointer, { maxLength: 1024 }),
        sha256: requiredString(item, "sha256", pointer, sha256),
        bytes: requiredInteger(item, "bytes", pointer, { min: 0, max: OBJECT_MAX_BYTES }),
        reason: requiredString(item, "reason", pointer, { oneOf: ["absent", "digest-mismatch"] }) as MissingObject["reason"],
      };
    }),
    pending_uploads: list("pending_uploads", ["upload_id", "repository_id", "observation_id", "reference_kind", "bytes", "created_at_ms", "expires_at_ms"]).map(
      (item, index) => {
        const pointer = at("pending_uploads", index);
        return {
          upload_id: requiredString(item, "upload_id", pointer, serverId),
          repository_id: requiredString(item, "repository_id", pointer, repositoryId),
          observation_id: requiredString(item, "observation_id", pointer, { pattern: FORMATS.observationId }),
          reference_kind: requiredString(item, "reference_kind", pointer, { oneOf: REFERENCE_KINDS }),
          bytes: requiredInteger(item, "bytes", pointer, { min: 0, max: OBJECT_MAX_BYTES }),
          created_at_ms: requiredInteger(item, "created_at_ms", pointer, COUNT),
          expires_at_ms: requiredInteger(item, "expires_at_ms", pointer, COUNT),
        };
      },
    ),
    retention_jobs: list("retention_jobs", ["job_id", "repository_id", "state", "generation", "retention_epoch", "evidence_revision", "updated_at_ms"]).map(
      (item, index) => {
        const pointer = at("retention_jobs", index);
        return {
          job_id: requiredString(item, "job_id", pointer, serverId),
          repository_id: requiredString(item, "repository_id", pointer, repositoryId),
          state: requiredString(item, "state", pointer, { oneOf: JOB_STATES }) as RetentionJobRecord["state"],
          generation: requiredString(item, "generation", pointer, { maxLength: 256 }),
          retention_epoch: requiredInteger(item, "retention_epoch", pointer, COUNT),
          evidence_revision: requiredInteger(item, "evidence_revision", pointer, COUNT),
          updated_at_ms: requiredInteger(item, "updated_at_ms", pointer, COUNT),
        };
      },
    ),
    credentials: list("credentials", ["credential_id", "kind", "secret_sha256", "expires_at_ms", "revoked_at_ms"]).map((item, index) => {
      const pointer = at("credentials", index);
      const revoked = optionalInteger(item, "revoked_at_ms", pointer, COUNT);
      const record = {
        credential_id: requiredString(item, "credential_id", pointer, serverId),
        kind: requiredString(item, "kind", pointer, { oneOf: ["host", "operator"] }) as CredentialRecord["kind"],
        secret_sha256: requiredString(item, "secret_sha256", pointer, sha256),
        expires_at_ms: requiredInteger(item, "expires_at_ms", pointer, COUNT),
      };
      return revoked === undefined ? record : { ...record, revoked_at_ms: revoked };
    }),
    grants: list("grants", ["credential_id", "repository_id", "role"]).map((item, index) => {
      const pointer = at("grants", index);
      return {
        credential_id: requiredString(item, "credential_id", pointer, serverId),
        repository_id: requiredString(item, "repository_id", pointer, repositoryId),
        role: requiredString(item, "role", pointer, { oneOf: GRANT_ROLES }) as GrantRecord["role"],
      };
    }),
    window_revocations: list("window_revocations", ["credential_id", "at_ms"]).map((item, index) => {
      const pointer = at("window_revocations", index);
      return {
        credential_id: requiredString(item, "credential_id", pointer, serverId),
        at_ms: requiredInteger(item, "at_ms", pointer, COUNT),
      };
    }),
  };
}
