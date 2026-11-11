//! The HTTP API's version, bounds, roles, and error vocabulary.
//!
//! `api/openapi.yaml` publishes the same values; `tests/node/api-contract.test.ts`
//! fails when the two drift.

export const API_VERSION = "v1";
export const API_PREFIX = `/${API_VERSION}/repos/{repository_id}`;

const KIB = 1024;
const MIB = 1024 * KIB;

/** Applied before any mutation. A collection or retry graph is never split. */
export const LIMITS = {
  request_json_bytes: 1 * MIB,
  observation_canonical_bytes: 128 * KIB,
  collection_observations: 256,
  reachable_heads: 256,
  page_default_records: 100,
  page_max_records: 256,
  page_max_encoded_bytes: 1 * MIB,
  upload_part_bytes: 1 * MIB,
  upload_object_max_bytes: 64 * MIB,
  upload_expiry_hours: 24,
  staging_cleanup_hours: 48,
  cursor_expiry_hours: 24,
  change_tombstone_days: 30,
  location_attestation_hours: 24,
  retention_grace_days: 14,
  credential_expiry_days: 90,
  credential_rotation_day: 60,
  credential_overlap_max_days: 7,
  oidc_clock_tolerance_seconds: 60,
  rate_mutations_per_minute: 60,
  rate_reads_per_minute: 300,
  operator_rate_mutations_per_minute: 30,
} as const;

export const ROLES = ["reader", "producer", "verifier", "retention-verifier", "operator"] as const;
export type Role = (typeof ROLES)[number];

/**
 * Every API operation and the roles allowed to call it. Evaluation publication
 * is listed with no role: it answers 403 `evaluation_publication_disabled` to
 * every caller until a trusted verifier policy exists.
 */
export const OPERATIONS = {
  ingestCollection: { method: "POST", path: "/collections", roles: ["producer"] },
  exportCollection: { method: "GET", path: "/collections/{collection_id}", roles: ["reader", "verifier", "retention-verifier"] },
  getObservation: { method: "GET", path: "/observations/{observation_id}", roles: ["reader", "verifier", "retention-verifier"] },
  queryHistory: { method: "POST", path: "/history/query", roles: ["reader", "verifier", "retention-verifier"] },
  getSnapshot: { method: "GET", path: "/snapshot", roles: ["reader", "verifier", "retention-verifier"] },
  appendLocation: { method: "POST", path: "/locations", roles: ["verifier"] },
  queryChanges: { method: "POST", path: "/changes/query", roles: ["reader", "verifier", "retention-verifier"] },
  createUpload: { method: "POST", path: "/uploads", roles: ["producer", "verifier"] },
  putUploadPart: { method: "PUT", path: "/uploads/{upload_id}/parts/{index}", roles: ["producer", "verifier"] },
  finalizeUpload: { method: "POST", path: "/uploads/{upload_id}/finalize", roles: ["verifier"] },
  fetchObject: { method: "GET", path: "/objects/{observation_id}/{reference_kind}/{reference_digest}", roles: ["reader", "verifier", "retention-verifier"] },
  submitRetentionCertificate: { method: "POST", path: "/retention/certificates", roles: ["retention-verifier"] },
  getRetentionJob: { method: "GET", path: "/retention/jobs/{job_id}", roles: ["retention-verifier", "operator"] },
  publishEvaluation: { method: "POST", path: "/evaluations", roles: [] },
  registerRepository: { method: "POST", path: "/operator/repositories", roles: ["operator"] },
  issueCredential: { method: "POST", path: "/operator/credentials", roles: ["operator"] },
  revokeCredential: { method: "POST", path: "/operator/credentials/{credential_id}/revoke", roles: ["operator"] },
  listAudit: { method: "POST", path: "/operator/audit/query", roles: ["operator"] },
} as const satisfies Record<string, { method: string; path: string; roles: readonly Role[] }>;
export type OperationId = keyof typeof OPERATIONS;

/** Operator routes are not repository-scoped and live under `/v1`, not `API_PREFIX`. */
export const OPERATOR_PATH_PREFIX = `/${API_VERSION}`;

/** Which reference a proof upload or object fetch names. */
export const REFERENCE_KINDS = ["report", "expected-listing", "completion-record"] as const;
export type ReferenceKind = (typeof REFERENCE_KINDS)[number];

/** Typed locators; the Worker never resolves an arbitrary URL or path. */
export const LOCATOR_KINDS = ["host-dir", "artifact", "object"] as const;
export type LocatorKind = (typeof LOCATOR_KINDS)[number];

export const LOCATION_EVENT_KINDS = ["verified", "unavailable", "withdrawn"] as const;
export type LocationEventKind = (typeof LOCATION_EVENT_KINDS)[number];

/** Stable error codes and the HTTP status each answers with. */
export const ERROR_CODES = {
  invalid_input: { status: 400, retryable: false },
  invalid_cursor: { status: 400, retryable: false },
  unsupported_api_version: { status: 400, retryable: false },
  unauthenticated: { status: 401, retryable: false },
  denied: { status: 403, retryable: false },
  evaluation_publication_disabled: { status: 403, retryable: false },
  not_found: { status: 404, retryable: false },
  identity_conflict: { status: 409, retryable: false },
  stale_snapshot: { status: 409, retryable: false },
  snapshot_pruned: { status: 409, retryable: false },
  cursor_expired: { status: 409, retryable: false },
  collection_pruned: { status: 410, retryable: false },
  size_limit: { status: 413, retryable: false },
  unpageable_record: { status: 413, retryable: false },
  rate_limited: { status: 429, retryable: true },
  quota_exhausted: { status: 429, retryable: true },
  unavailable: { status: 503, retryable: true },
} as const satisfies Record<string, { status: number; retryable: boolean }>;
export type ErrorCode = keyof typeof ERROR_CODES;

/** The body of every non-2xx response. Never carries credentials or report bytes. */
export interface ErrorBody {
  error: {
    code: ErrorCode;
    message: string;
    /** JSON pointer into the request, or null when the error is not located in it. */
    location: string | null;
    retryable: boolean;
    retry_after_ms: number | null;
  };
  request_id: string;
}

/** Every list response carries both; `next_cursor` is null exactly when `has_more` is false. */
export interface Page<T> {
  items: T[];
  has_more: boolean;
  next_cursor: string | null;
}

/**
 * One primary-database read. Every later page is bounded by these values;
 * `generation` changes on restore and `retention_epoch` on pruning.
 */
export interface Snapshot {
  repository_id: string;
  generation: string;
  retention_epoch: number;
  evidence_revision: number;
  observation_event_watermark: number;
  location_event_watermark: number;
}

export interface IngestResult {
  collection_id: string;
  accepted: string[];
  already_present: string[];
  evidence_revision: number;
}

/** Client-side retry policy (only 429, 503, and network failures retry). */
export const CLIENT_RETRY = {
  max_attempts: 5,
  backoff_cap_ms: 30_000,
  retry_after_cap_ms: 60_000,
  request_timeout_ms: 60_000,
} as const;

/** Capacity guard ratios applied to every free-tier allowance. */
export const CAPACITY = {
  alert_ratio: 0.7,
  refuse_ratio: 0.8,
  r2_live_plus_staging_bytes: 8 * 1024 * MIB,
  r2_class_a_per_month: 800_000,
  r2_class_b_per_month: 8_000_000,
  d1_database_bytes: 500 * 1000 * 1000,
  d1_rows_read_per_day: 5_000_000,
  d1_rows_written_per_day: 100_000,
  worker_requests_per_day: 100_000,
} as const;
