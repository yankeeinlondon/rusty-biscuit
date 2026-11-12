//! The one validator for a `CredentialRequest`, shared by `issueCredential`
//! and the operator CLI's grant file (`api/schemas/credential-grant.schema.json`),
//! which is the same request plus the local `secret_file`.
//!
//! An empty `repository_ids` or `roles` is valid and yields a credential that
//! is denied everything. `operator` is never grantable here.

import { LIMITS, type Role } from "./contracts/api.ts";
import type { JsonObject } from "./contracts/json.ts";
import {
  FORMATS,
  optionalInteger,
  optionalString,
  readRequestObject,
  requiredString,
  requiredStringArray,
} from "./requests.ts";

export const GRANTABLE_ROLES = ["reader", "producer", "verifier", "retention-verifier"] as const satisfies readonly Role[];
export type GrantableRole = (typeof GRANTABLE_ROLES)[number];

const REQUEST_FIELDS = ["label", "repository_ids", "roles", "expires_in_days", "replaces"] as const;

export interface CredentialRequest {
  readonly label: string;
  readonly repository_ids: readonly string[];
  readonly roles: readonly GrantableRole[];
  /** Absent in the request means `LIMITS.credential_expiry_days`. */
  readonly expires_in_days: number;
  readonly replaces?: string;
}

export interface CredentialGrant extends CredentialRequest {
  /** Native local path the CLI creates for the token; never sent to the service. */
  readonly secret_file: string;
}

function readFields(object: JsonObject): CredentialRequest {
  const label = requiredString(object, "label", "", { maxLength: 128 });
  const repository_ids = requiredStringArray(object, "repository_ids", "", { pattern: FORMATS.repositoryId }, { unique: true });
  const roles = requiredStringArray(object, "roles", "", { oneOf: GRANTABLE_ROLES }, { unique: true }) as GrantableRole[];
  const expires_in_days =
    optionalInteger(object, "expires_in_days", "", { min: 1, max: LIMITS.credential_expiry_days }) ?? LIMITS.credential_expiry_days;
  const replaces = optionalString(object, "replaces", "", { pattern: FORMATS.serverId });
  return replaces === undefined
    ? { label, repository_ids, roles, expires_in_days }
    : { label, repository_ids, roles, expires_in_days, replaces };
}

/** A request body; throws 400 `invalid_input` located at the offending field. */
export function parseCredentialRequest(bytes: Uint8Array): CredentialRequest {
  return readFields(readRequestObject(bytes, REQUEST_FIELDS));
}

/** A grant file; the same problems as `parseCredentialRequest`, plus `secret_file`. */
export function parseCredentialGrant(bytes: Uint8Array): CredentialGrant {
  const object = readRequestObject(bytes, [...REQUEST_FIELDS, "$schema", "secret_file"]);
  optionalString(object, "$schema", "", { minLength: 0 });
  const request = readFields(object);
  return { ...request, secret_file: requiredString(object, "secret_file", "") };
}

/** The body `issueCredential` receives for `request`. */
export function credentialRequestBody(request: CredentialRequest): string {
  const { label, repository_ids, roles, expires_in_days, replaces } = request;
  return JSON.stringify(replaces === undefined ? { label, repository_ids, roles, expires_in_days } : { label, repository_ids, roles, expires_in_days, replaces });
}
