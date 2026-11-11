//! The lossless JSON value model and the strict-loader seam.
//!
//! `JSON.parse` collapses `1` and `1.0`, rounds integers past 2^53, and lets a
//! duplicate key silently win; Python's `json.loads` keeps the first two
//! distinctions, so an identity computed from `JSON.parse` output would not
//! match `schema.observation_identity`. Values therefore keep their number
//! lexemes until canonicalization.

/** A JSON number exactly as written. `integer` is true for a token with no fraction or exponent. */
export interface JsonNumber {
  readonly kind: "number";
  readonly lexeme: string;
  readonly integer: boolean;
}

export type JsonValue =
  | null
  | boolean
  | string
  | JsonNumber
  | JsonValue[]
  | { [key: string]: JsonValue };

export type JsonObject = { [key: string]: JsonValue };

/**
 * One located reason input was rejected. `location` is an RFC 6901 JSON
 * pointer into the submitted document (`""` is the whole document), or, for
 * a loader failure, the pointer of the innermost value being read.
 */
export interface ContractProblem {
  readonly code: string;
  readonly location: string;
  readonly message: string;
}

export type ContractResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly problems: readonly ContractProblem[] };

/** Loader rejection codes; every strict JSON consumer reports these. */
export const LOADER_PROBLEM_CODES = [
  "invalid-utf8",
  "byte-order-mark",
  "empty-document",
  "syntax",
  "duplicate-key",
  "nonfinite-number",
  "trailing-content",
] as const;
export type LoaderProblemCode = (typeof LOADER_PROBLEM_CODES)[number];

/**
 * Parse exactly one JSON document from raw bytes. Refuses invalid UTF-8, a
 * BOM, blank input, duplicate keys, `NaN`/`Infinity`, and anything after the
 * document (including a second document). `loadStrict` in `loader.ts`.
 */
export type StrictLoader = (bytes: Uint8Array) => ContractResult<JsonValue>;

/**
 * Serialize like Python's `json.dumps(value, sort_keys=True,
 * separators=(",", ":"))`: ASCII-escaped, keys sorted by code point, number
 * lexemes rendered as Python would re-serialize the parsed number. `canonical`
 * in `canonical.ts`.
 */
export type Canonicalizer = (value: JsonValue) => string;

/**
 * The only `JsonNumber` implementation. Discriminating by class rather than
 * by shape keeps a JSON object that happens to carry `kind: "number"` (a
 * producer object, say) from being read as a number.
 */
class LosslessNumber implements JsonNumber {
  readonly kind = "number" as const;
  constructor(
    readonly lexeme: string,
    readonly integer: boolean,
  ) {
    Object.freeze(this);
  }
}

const NUMBER_LEXEME = /^-?(?:0|[1-9][0-9]*)(\.[0-9]+)?([eE][-+]?[0-9]+)?$/;

/** A number token. Throws on a lexeme that is not JSON number grammar: that is a caller defect. */
export function jsonNumber(lexeme: string): JsonNumber {
  const match = NUMBER_LEXEME.exec(lexeme);
  if (match === null) throw new TypeError(`not a JSON number lexeme: ${lexeme}`);
  return new LosslessNumber(lexeme, match[1] === undefined && match[2] === undefined);
}

export function isJsonNumber(value: JsonValue | undefined): value is JsonNumber {
  return value instanceof LosslessNumber;
}

export function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value) && !(value instanceof LosslessNumber);
}

const KEY_ORDER = new WeakMap<JsonObject, readonly string[]>();

/**
 * A prototype-free object whose keys keep their document order, so a key named
 * `__proto__` is an ordinary own key and integer-like keys are not hoisted the
 * way `Object.keys` would order them. Throws on a repeated key.
 */
export function jsonObject(entries: Iterable<readonly [string, JsonValue]>): JsonObject {
  const object = Object.create(null) as JsonObject;
  const order: string[] = [];
  for (const [key, value] of entries) {
    if (Object.hasOwn(object, key)) throw new TypeError(`repeated key ${JSON.stringify(key)}`);
    Object.defineProperty(object, key, { value, enumerable: true, writable: true, configurable: true });
    order.push(key);
  }
  KEY_ORDER.set(object, order);
  return object;
}

/**
 * Own keys in insertion order (Python dict order), which only message text
 * that renders a whole object depends on. Keys added after construction follow
 * the recorded ones; removed keys are skipped.
 */
export function objectKeys(object: JsonObject): string[] {
  const present = Object.keys(object);
  const recorded = KEY_ORDER.get(object);
  if (recorded === undefined) return present;
  const remaining = new Set(present);
  const keys = recorded.filter((key) => remaining.delete(key));
  return [...keys, ...remaining];
}

/** The member named `key`, or `undefined` when absent (never an inherited property). */
export function memberOf(object: JsonObject, key: string): JsonValue | undefined {
  return Object.hasOwn(object, key) ? object[key] : undefined;
}

/** `pointer` extended by one RFC 6901 reference token. */
export function childPointer(pointer: string, token: string | number): string {
  return `${pointer}/${String(token).replaceAll("~", "~0").replaceAll("/", "~1")}`;
}
