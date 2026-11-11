//! The service configuration (`api/schemas/service-config.schema.json`).
//!
//! Read through the shared strict loader and the request field readers, so a
//! configuration problem is located by JSON pointer the same way a request
//! problem is. Every list is an allowlist: an empty `repositories`,
//! `trusted_workflows`, or `roles` is valid and denies everything; it never
//! means unrestricted. Importable from Node: Worker types are type-only.

import { childPointer, isJsonNumber, type ContractResult, type JsonObject, type JsonValue } from "./contracts/json.ts";
import { loadStrict } from "./contracts/loader.ts";
import type { Env } from "./env.ts";
import { ApiError } from "./http.ts";
import {
  FORMATS,
  requireObject,
  requiredInteger,
  requiredObject,
  requiredString,
  requiredStringArray,
  optionalString,
} from "./requests.ts";

export const OIDC_ROLES = ["producer", "reader"] as const;
export type OidcRole = (typeof OIDC_ROLES)[number];

export const TRUSTED_EVENTS = ["workflow_dispatch", "push", "schedule"] as const;

export interface RepositoryAllowance {
  readonly repository_id: string;
  readonly owner_id: string;
  readonly slug: string;
}

export interface TrustedWorkflow {
  readonly repository_id: string;
  readonly workflow: string;
  readonly ref: string;
  readonly event_name: (typeof TRUSTED_EVENTS)[number];
  readonly roles: readonly OidcRole[];
}

export interface CapacityQuotas {
  readonly alert_ratio: number;
  readonly refuse_ratio: number;
  readonly r2_live_plus_staging_bytes: number;
  readonly r2_class_a_per_month: number;
  readonly r2_class_b_per_month: number;
  readonly d1_database_bytes: number;
  readonly d1_rows_read_per_day: number;
  readonly d1_rows_written_per_day: number;
  readonly worker_requests_per_day: number;
}

export interface ServiceConfig {
  readonly api_version: "v1";
  readonly repositories: readonly RepositoryAllowance[];
  readonly oidc: {
    readonly issuer: string;
    readonly audience: string;
    readonly clock_tolerance_seconds: number;
    readonly trusted_workflows: readonly TrustedWorkflow[];
  };
  readonly quotas: {
    readonly rate_mutations_per_minute: number;
    readonly rate_reads_per_minute: number;
    readonly operator_rate_mutations_per_minute: number;
    readonly capacity: CapacityQuotas;
  };
}

export const OIDC_ISSUER = "https://token.actions.githubusercontent.com";
export const OIDC_AUDIENCE = "rusty-biscuit-evidence-store";

const SLUG = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const WORKFLOW = /^\.github\/workflows\/[A-Za-z0-9_.-]+\.ya?ml$/;
const REF = /^refs\/(heads|tags)\/.+$/;

/** The integer capacity allowances and their schema maxima. */
const CAPACITY_LIMITS = {
  r2_live_plus_staging_bytes: 8_589_934_592,
  r2_class_a_per_month: 800_000,
  r2_class_b_per_month: 8_000_000,
  d1_database_bytes: 500_000_000,
  d1_rows_read_per_day: 5_000_000,
  d1_rows_written_per_day: 100_000,
  worker_requests_per_day: 100_000,
} as const;

function invalid(location: string, message: string): ApiError {
  return new ApiError("invalid_input", message, location);
}

/** A required array member; explicit `null` is refused like every other field. */
function requiredArray(object: JsonObject, key: string, pointer: string): JsonValue[] {
  const location = childPointer(pointer, key);
  if (!Object.hasOwn(object, key)) throw invalid(location, `${key} is required`);
  const value = object[key]!;
  if (value === null) throw invalid(location, `${key} must not be null`);
  if (!Array.isArray(value)) throw invalid(location, `${location} must be an array`);
  return value;
}

/** A ratio in `(0, max]`; any JSON number lexeme, integer or not. */
function requiredRatio(object: JsonObject, key: string, pointer: string, max: number): number {
  const location = childPointer(pointer, key);
  if (!Object.hasOwn(object, key)) throw invalid(location, `${key} is required`);
  const value = object[key]!;
  if (!isJsonNumber(value)) throw invalid(location, `${location} must be a number`);
  const number = Number(value.lexeme);
  if (!(number > 0 && number <= max)) throw invalid(location, `${location} must be greater than 0 and at most ${max}`);
  return number;
}

function readRepositories(object: JsonObject): RepositoryAllowance[] {
  const seen = new Set<string>();
  return requiredArray(object, "repositories", "").map((item, index) => {
    const pointer = childPointer("/repositories", index);
    const entry = requireObject(item, pointer, ["repository_id", "owner_id", "slug"]);
    const repository_id = requiredString(entry, "repository_id", pointer, { pattern: FORMATS.repositoryId });
    if (seen.has(repository_id)) throw invalid(childPointer(pointer, "repository_id"), `repository ${repository_id} is listed twice`);
    seen.add(repository_id);
    return {
      repository_id,
      owner_id: requiredString(entry, "owner_id", pointer, { pattern: FORMATS.repositoryId }),
      slug: requiredString(entry, "slug", pointer, { pattern: SLUG }),
    };
  });
}

function readTrustedWorkflows(oidc: JsonObject): TrustedWorkflow[] {
  return requiredArray(oidc, "trusted_workflows", "/oidc").map((item, index) => {
    const pointer = childPointer("/oidc/trusted_workflows", index);
    const entry = requireObject(item, pointer, ["repository_id", "workflow", "ref", "event_name", "roles"]);
    return {
      repository_id: requiredString(entry, "repository_id", pointer, { pattern: FORMATS.repositoryId }),
      workflow: requiredString(entry, "workflow", pointer, { pattern: WORKFLOW }),
      ref: requiredString(entry, "ref", pointer, { pattern: REF }),
      event_name: requiredString(entry, "event_name", pointer, { oneOf: TRUSTED_EVENTS }) as TrustedWorkflow["event_name"],
      roles: requiredStringArray(entry, "roles", pointer, { oneOf: OIDC_ROLES }, { unique: true }) as OidcRole[],
    };
  });
}

function readCapacity(quotas: JsonObject): CapacityQuotas {
  const pointer = "/quotas/capacity";
  const capacity = requiredObject(quotas, "capacity", "/quotas", ["alert_ratio", "refuse_ratio", ...Object.keys(CAPACITY_LIMITS)]);
  const integer = (key: keyof typeof CAPACITY_LIMITS) => requiredInteger(capacity, key, pointer, { min: 0, max: CAPACITY_LIMITS[key] });
  return {
    alert_ratio: requiredRatio(capacity, "alert_ratio", pointer, 0.7),
    refuse_ratio: requiredRatio(capacity, "refuse_ratio", pointer, 0.8),
    r2_live_plus_staging_bytes: integer("r2_live_plus_staging_bytes"),
    r2_class_a_per_month: integer("r2_class_a_per_month"),
    r2_class_b_per_month: integer("r2_class_b_per_month"),
    d1_database_bytes: integer("d1_database_bytes"),
    d1_rows_read_per_day: integer("d1_rows_read_per_day"),
    d1_rows_written_per_day: integer("d1_rows_written_per_day"),
    worker_requests_per_day: integer("worker_requests_per_day"),
  };
}

function readServiceConfig(value: JsonValue): ServiceConfig {
  const root = requireObject(value, "", ["$schema", "api_version", "repositories", "oidc", "quotas"]);
  optionalString(root, "$schema", "", { minLength: 0 });
  requiredString(root, "api_version", "", { oneOf: ["v1"] });
  const repositories = readRepositories(root);
  const oidc = requiredObject(root, "oidc", "", ["issuer", "audience", "clock_tolerance_seconds", "trusted_workflows"]);
  const quotas = requiredObject(root, "quotas", "", [
    "rate_mutations_per_minute",
    "rate_reads_per_minute",
    "operator_rate_mutations_per_minute",
    "capacity",
  ]);
  return {
    api_version: "v1",
    repositories,
    oidc: {
      issuer: requiredString(oidc, "issuer", "/oidc", { oneOf: [OIDC_ISSUER] }),
      audience: requiredString(oidc, "audience", "/oidc", { oneOf: [OIDC_AUDIENCE] }),
      clock_tolerance_seconds: requiredInteger(oidc, "clock_tolerance_seconds", "/oidc", { min: 0, max: 60 }),
      trusted_workflows: readTrustedWorkflows(oidc),
    },
    quotas: {
      rate_mutations_per_minute: requiredInteger(quotas, "rate_mutations_per_minute", "/quotas", { min: 1, max: 60 }),
      rate_reads_per_minute: requiredInteger(quotas, "rate_reads_per_minute", "/quotas", { min: 1, max: 300 }),
      operator_rate_mutations_per_minute: requiredInteger(quotas, "operator_rate_mutations_per_minute", "/quotas", { min: 1, max: 30 }),
      capacity: readCapacity(quotas),
    },
  };
}

/** Loader problems keep their loader code; field problems are `invalid-config`. */
export function parseServiceConfig(text: string): ContractResult<ServiceConfig> {
  const loaded = loadStrict(new TextEncoder().encode(text));
  if (!loaded.ok) return loaded;
  try {
    return { ok: true, value: readServiceConfig(loaded.value) };
  } catch (error) {
    if (!(error instanceof ApiError)) throw error;
    return { ok: false, problems: [{ code: "invalid-config", location: error.location ?? "", message: error.message }] };
  }
}

let cached: { readonly text: string; readonly config: ServiceConfig } | undefined;

/**
 * The bound configuration, parsed once per distinct text. A missing or
 * invalid binding throws 503 `unavailable`: the service never runs with a
 * partial allowlist. The message names the binding and the problem's
 * location, never a value.
 */
export function serviceConfig(env: Pick<Env, "SERVICE_CONFIG">): ServiceConfig {
  const text: unknown = env.SERVICE_CONFIG;
  if (typeof text !== "string") throw new ApiError("unavailable", "the SERVICE_CONFIG binding is missing");
  if (cached?.text === text) return cached.config;
  const parsed = parseServiceConfig(text);
  if (!parsed.ok) {
    const problem = parsed.problems[0]!;
    throw new ApiError("unavailable", `the SERVICE_CONFIG binding is invalid (${problem.code} at "${problem.location}")`);
  }
  cached = { text, config: parsed.value };
  return parsed.value;
}
