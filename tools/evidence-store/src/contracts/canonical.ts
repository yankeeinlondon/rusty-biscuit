//! Canonical bytes and content identities, as `schema.canonical`,
//! `schema.identity`, and `schema.observation_identity` compute them.

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, utf8ToBytes } from "@noble/hashes/utils.js";
import { isJsonNumber, objectKeys, type Canonicalizer, type JsonNumber, type JsonObject, type JsonValue } from "./json.ts";

/** Orders two strings by Unicode code point (Python `str` order); a lone surrogate is its own code point. */
export function compareCodePoints(left: string, right: string): number {
  const length = Math.min(left.length, right.length);
  for (let index = 0; index < length; ) {
    const a = left.codePointAt(index) ?? 0;
    const b = right.codePointAt(index) ?? 0;
    if (a !== b) return a < b ? -1 : 1;
    index += a > 0xffff ? 2 : 1;
  }
  // Equal up to the shorter length means equal code points up to there, so
  // the shorter string (in units, hence in code points) sorts first.
  return left.length - right.length;
}

/**
 * Python `repr(float)`: the shortest digits that round-trip, positional for a
 * decimal exponent in [-4, 16), `.0` on an integral value, and an exponent of at
 * least two digits otherwise. Infinities render as `json.dumps` writes them.
 */
export function formatPythonFloat(value: number, infinity: "json" | "repr"): string {
  if (Number.isNaN(value)) return infinity === "json" ? "NaN" : "nan";
  if (!Number.isFinite(value)) {
    if (infinity === "json") return value > 0 ? "Infinity" : "-Infinity";
    return value > 0 ? "inf" : "-inf";
  }
  if (value === 0) return Object.is(value, -0) ? "-0.0" : "0.0";
  const sign = value < 0 ? "-" : "";
  const [mantissa = "", exponent = "0"] = Math.abs(value).toString().split("e");
  const [whole = "", fraction = ""] = mantissa.split(".");
  let digits = whole + fraction;
  let point = whole.length + Number(exponent);
  const leading = /^0*/.exec(digits)?.[0].length ?? 0;
  digits = digits.slice(leading);
  point -= leading;
  digits = digits.replace(/0+$/, "");
  if (point > -4 && point <= 16) {
    if (point <= 0) return `${sign}0.${"0".repeat(-point)}${digits}`;
    if (point >= digits.length) return `${sign}${digits}${"0".repeat(point - digits.length)}.0`;
    return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
  }
  const power = point - 1;
  const head = digits.length > 1 ? `${digits[0]}.${digits.slice(1)}` : digits;
  const magnitude = String(Math.abs(power)).padStart(2, "0");
  return `${sign}${head}e${power < 0 ? "-" : "+"}${magnitude}`;
}

/** How Python re-serializes a parsed number: integers verbatim (`-0` is `0`), floats through `repr`. */
export function formatPythonNumber(number: JsonNumber, infinity: "json" | "repr"): string {
  if (number.integer) return number.lexeme === "-0" ? "0" : number.lexeme;
  return formatPythonFloat(Number(number.lexeme), infinity);
}

const SHORT_ESCAPES: Readonly<Record<number, string>> = {
  0x22: '\\"',
  0x5c: "\\\\",
  0x08: "\\b",
  0x0c: "\\f",
  0x0a: "\\n",
  0x0d: "\\r",
  0x09: "\\t",
};

const PLAIN = /^[\x20\x21\x23-\x5b\x5d-\x7e]*$/;

/** `json.dumps` with `ensure_ascii=True`: every UTF-16 unit outside printable ASCII becomes `\uXXXX`. */
function quote(text: string): string {
  // Most evidence strings need no escape; skipping the per-unit loop keeps a
  // 1 MiB collection inside the Worker CPU budget.
  if (PLAIN.test(text)) return `"${text}"`;
  let result = '"';
  for (let index = 0; index < text.length; index += 1) {
    const unit = text.charCodeAt(index);
    const short = SHORT_ESCAPES[unit];
    if (short !== undefined) result += short;
    else if (unit < 0x20 || unit > 0x7e) result += `\\u${unit.toString(16).padStart(4, "0")}`;
    else result += text[index];
  }
  return `${result}"`;
}

function write(value: JsonValue): string {
  if (value === null) return "null";
  if (value === true) return "true";
  if (value === false) return "false";
  if (typeof value === "string") return quote(value);
  if (isJsonNumber(value)) return formatPythonNumber(value, "json");
  if (Array.isArray(value)) return `[${value.map(write).join(",")}]`;
  const object: JsonObject = value;
  const keys = objectKeys(object).sort(compareCodePoints);
  return `{${keys.map((key) => `${quote(key)}:${write(object[key] as JsonValue)}`).join(",")}}`;
}

/**
 * See `Canonicalizer`. Total like the Python: a float lexeme that overflows
 * renders as `Infinity`, which is why `loadStrict` refuses such lexemes.
 */
export const canonical: Canonicalizer = write;

/** Lowercase SHA-256 hex of the UTF-8 bytes of `text`. */
export function identity(text: string): string {
  return bytesToHex(sha256(utf8ToBytes(text)));
}

/** The identity an observation's `observation_id` must equal: its content without that key. */
export function observationIdentity(observation: JsonObject): string {
  const body: JsonObject = Object.create(null) as JsonObject;
  for (const key of objectKeys(observation)) {
    if (key !== "observation_id") body[key] = observation[key] as JsonValue;
  }
  return identity(canonical(body));
}

/** A collection's id: the identity of the whole canonical collection. */
export function collectionIdentity(collection: JsonValue): string {
  return identity(canonical(collection));
}
