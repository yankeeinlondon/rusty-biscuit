//! Strict reading of the small JSON request bodies: history and change
//! queries, location events, and operator requests.
//!
//! Every body goes through the shared strict loader (`loadStrict`), so
//! duplicate keys, trailing content, nonfinite numbers, invalid UTF-8, a BOM,
//! and blank input are refused the same way as for evidence. The field
//! readers below are the one place each field shape is decided:
//!
//! - an unknown key, a wrong type, or an out-of-range value is 400
//!   `invalid_input` located at the field's JSON pointer;
//! - an optional field is either absent (`undefined`) or valid; an explicit
//!   `null` is refused, never read as absent;
//! - an array is never filtered: one invalid element refuses the request.

import { childPointer, isJsonNumber, isJsonObject, objectKeys, type JsonObject, type JsonValue } from "./contracts/json.ts";
import { loadStrict } from "./contracts/loader.ts";
import { ApiError } from "./http.ts";

function invalid(location: string, message: string): ApiError {
  return new ApiError("invalid_input", message, location);
}

/**
 * Load `bytes` as one JSON object whose keys are all in `allowed`. Loader
 * failures keep the loader's own location and message.
 */
export function readRequestObject(bytes: Uint8Array, allowed: readonly string[]): JsonObject {
  const loaded = loadStrict(bytes);
  if (!loaded.ok) {
    const first = loaded.problems[0]!;
    throw invalid(first.location, `${first.code}: ${first.message}`);
  }
  return requireObject(loaded.value, "", allowed);
}

/** `value` as an object at `pointer` whose keys are all in `allowed`. */
export function requireObject(value: JsonValue | undefined, pointer: string, allowed: readonly string[]): JsonObject {
  if (!isJsonObject(value)) throw invalid(pointer, `${describe(pointer)} must be an object`);
  for (const key of objectKeys(value)) {
    if (!allowed.includes(key)) throw invalid(childPointer(pointer, key), `unknown field ${JSON.stringify(key)}`);
  }
  return value;
}

function describe(pointer: string): string {
  return pointer === "" ? "request body" : pointer;
}

/** Reads `key` of `object`: absent → `undefined`; explicit `null` → refused. */
function member(object: JsonObject, key: string, pointer: string, required: boolean): JsonValue | undefined {
  const location = childPointer(pointer, key);
  if (!Object.hasOwn(object, key)) {
    if (required) throw invalid(location, `${key} is required`);
    return undefined;
  }
  const value = object[key]!;
  if (value === null) throw invalid(location, `${key} must not be null; omit it instead`);
  return value;
}

export interface StringSpec {
  readonly pattern?: RegExp;
  readonly oneOf?: readonly string[];
  readonly minLength?: number;
  readonly maxLength?: number;
}

/** Validates one string value at `location`. */
export function checkString(value: JsonValue, location: string, spec: StringSpec = {}): string {
  if (typeof value !== "string") throw invalid(location, `${location} must be a string`);
  const min = spec.minLength ?? 1;
  if (value.length < min) throw invalid(location, `${location} must be at least ${min} character(s)`);
  if (spec.maxLength !== undefined && value.length > spec.maxLength) {
    throw invalid(location, `${location} must be at most ${spec.maxLength} characters`);
  }
  if (spec.oneOf !== undefined && !spec.oneOf.includes(value)) {
    throw invalid(location, `${location} must be one of ${spec.oneOf.join(", ")}`);
  }
  if (spec.pattern !== undefined && !spec.pattern.test(value)) {
    throw invalid(location, `${location} has an invalid format`);
  }
  return value;
}

export function requiredString(object: JsonObject, key: string, pointer: string, spec?: StringSpec): string {
  return checkString(member(object, key, pointer, true)!, childPointer(pointer, key), spec);
}

export function optionalString(object: JsonObject, key: string, pointer: string, spec?: StringSpec): string | undefined {
  const value = member(object, key, pointer, false);
  return value === undefined ? undefined : checkString(value, childPointer(pointer, key), spec);
}

export interface IntegerSpec {
  readonly min: number;
  readonly max: number;
}

/** An integer token (`3`, never `3.0`, `true`, or `"3"`) within `[min, max]`. */
export function checkInteger(value: JsonValue, location: string, spec: IntegerSpec): number {
  if (!isJsonNumber(value) || !value.integer) throw invalid(location, `${location} must be an integer`);
  const number = Number(value.lexeme);
  if (!Number.isSafeInteger(number) || number < spec.min || number > spec.max) {
    throw invalid(location, `${location} must be an integer from ${spec.min} to ${spec.max}`);
  }
  return number;
}

export function requiredInteger(object: JsonObject, key: string, pointer: string, spec: IntegerSpec): number {
  return checkInteger(member(object, key, pointer, true)!, childPointer(pointer, key), spec);
}

export function optionalInteger(object: JsonObject, key: string, pointer: string, spec: IntegerSpec): number | undefined {
  const value = member(object, key, pointer, false);
  return value === undefined ? undefined : checkInteger(value, childPointer(pointer, key), spec);
}

export interface ArraySpec {
  readonly maxItems?: number;
  /** Refuse repeated elements (compared after element validation). */
  readonly unique?: boolean;
}

/** A string array whose every element satisfies `element`; `[]` is valid. */
export function checkStringArray(value: JsonValue, location: string, element: StringSpec, spec: ArraySpec = {}): string[] {
  if (!Array.isArray(value)) throw invalid(location, `${location} must be an array`);
  if (spec.maxItems !== undefined && value.length > spec.maxItems) {
    throw new ApiError("size_limit", `${location} has ${value.length} items; the limit is ${spec.maxItems}`, location);
  }
  const seen = new Set<string>();
  const strings = value.map((item, index) => {
    const text = checkString(item, childPointer(location, index), element);
    if (spec.unique === true) {
      if (seen.has(text)) throw invalid(childPointer(location, index), `${childPointer(location, index)} repeats an earlier item`);
      seen.add(text);
    }
    return text;
  });
  return strings;
}

export function requiredStringArray(
  object: JsonObject,
  key: string,
  pointer: string,
  element: StringSpec,
  spec?: ArraySpec,
): string[] {
  return checkStringArray(member(object, key, pointer, true)!, childPointer(pointer, key), element, spec);
}

export function optionalStringArray(
  object: JsonObject,
  key: string,
  pointer: string,
  element: StringSpec,
  spec?: ArraySpec,
): string[] | undefined {
  const value = member(object, key, pointer, false);
  return value === undefined ? undefined : checkStringArray(value, childPointer(pointer, key), element, spec);
}

/** The member as an object with `allowed` keys, or `undefined` when absent. */
export function optionalObject(object: JsonObject, key: string, pointer: string, allowed: readonly string[]): JsonObject | undefined {
  const value = member(object, key, pointer, false);
  return value === undefined ? undefined : requireObject(value, childPointer(pointer, key), allowed);
}

export function requiredObject(object: JsonObject, key: string, pointer: string, allowed: readonly string[]): JsonObject {
  return requireObject(member(object, key, pointer, true), childPointer(pointer, key), allowed);
}

/** Shared identifier formats (see `api/openapi.yaml` components). */
export const FORMATS = {
  repositoryId: /^[1-9][0-9]{0,19}$/,
  sha256: /^[0-9a-f]{64}$/,
  gitObjectId: /^[0-9a-f]{40}$/,
  observationId: /^[0-9a-f]{8,64}$/,
  serverId: /^[0-9a-f]{32}$/,
  uuid: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
} as const;
