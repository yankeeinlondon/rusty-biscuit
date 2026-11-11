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
 * document (including a second document). Implemented in Phase 2.
 */
export type StrictLoader = (bytes: Uint8Array) => ContractResult<JsonValue>;

/**
 * Serialize like Python's `json.dumps(value, sort_keys=True,
 * separators=(",", ":"))`: ASCII-escaped, keys sorted by code point, number
 * lexemes rendered as Python would re-serialize the parsed number.
 */
export type Canonicalizer = (value: JsonValue) => string;
