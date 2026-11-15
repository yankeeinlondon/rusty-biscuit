//! The strict JSON loader: `schema.loads_strict` over strict UTF-8 decoding.
//!
//! Accepts and rejects what the Python accepts and rejects, except that a float
//! lexeme overflowing a double is refused as `nonfinite-number` (Python yields
//! `inf`) and nesting deeper than `MAX_DEPTH` is refused rather than risking a
//! stack overflow; `docs/contract-inventory.md`, "Departures from the Python".
//! An integer of more than 4300 digits is refused as `syntax`, where Python
//! 3.12's integer-conversion limit also refuses it.

import {
  childPointer,
  jsonNumber,
  jsonObject,
  type ContractResult,
  type JsonValue,
  type LoaderProblemCode,
  type StrictLoader,
} from "./json.ts";

/** Evidence documents nest fewer than ten levels; Python's own limit is near 10,000. */
export const MAX_DEPTH = 1000;

/** CPython's default `sys.get_int_max_str_digits()`; `json.loads` raises past it. */
const MAX_INTEGER_DIGITS = 4300;

const NUMBER = /-?(?:0|[1-9][0-9]*)(\.[0-9]+)?([eE][-+]?[0-9]+)?/y;
const HEX4 = /^[0-9a-fA-F]{4}$/;
const SIMPLE_ESCAPES: Readonly<Record<string, string>> = {
  '"': '"',
  "\\": "\\",
  "/": "/",
  b: "\b",
  f: "\f",
  n: "\n",
  r: "\r",
  t: "\t",
};

class LoaderFailure extends Error {
  constructor(
    readonly code: LoaderProblemCode,
    readonly location: string,
    message: string,
  ) {
    super(message);
  }
}

class Parser {
  private position = 0;

  constructor(private readonly text: string) {}

  document(): JsonValue {
    this.skipWhitespace();
    if (this.position === this.text.length) {
      throw new LoaderFailure("empty-document", "", "the document is empty or only whitespace");
    }
    const value = this.value("", 0);
    this.skipWhitespace();
    if (this.position !== this.text.length) {
      throw new LoaderFailure("trailing-content", "", `content follows the document at ${this.where()}`);
    }
    return value;
  }

  private value(pointer: string, depth: number): JsonValue {
    this.skipWhitespace();
    const text = this.text;
    const char = text[this.position];
    switch (char) {
      case "{":
        return this.object(pointer, depth + 1);
      case "[":
        return this.array(pointer, depth + 1);
      case '"':
        return this.string(pointer);
      case "t":
        return this.literal("true", true, pointer);
      case "f":
        return this.literal("false", false, pointer);
      case "n":
        return this.literal("null", null, pointer);
      case "N":
        return this.constant("NaN", pointer);
      case "I":
        return this.constant("Infinity", pointer);
      case "-":
        if (text.startsWith("-Infinity", this.position)) return this.constant("-Infinity", pointer);
        return this.number(pointer);
      default:
        if (char !== undefined && char >= "0" && char <= "9") return this.number(pointer);
        throw this.syntax("expecting a value", pointer);
    }
  }

  private object(pointer: string, depth: number): JsonValue {
    this.checkDepth(pointer, depth);
    this.position += 1;
    const entries: [string, JsonValue][] = [];
    const seen = new Set<string>();
    let duplicate: string | undefined;
    this.skipWhitespace();
    if (this.text[this.position] === "}") {
      this.position += 1;
      return jsonObject(entries);
    }
    for (;;) {
      this.skipWhitespace();
      if (this.text[this.position] !== '"') {
        throw this.syntax("expecting a property name enclosed in double quotes", pointer);
      }
      const key = this.string(pointer);
      const child = childPointer(pointer, key);
      this.skipWhitespace();
      if (this.text[this.position] !== ":") throw this.syntax("expecting ':' delimiter", child);
      this.position += 1;
      const value = this.value(child, depth);
      if (seen.has(key)) duplicate ??= key;
      else {
        seen.add(key);
        entries.push([key, value]);
      }
      this.skipWhitespace();
      const next = this.text[this.position];
      this.position += 1;
      if (next === "}") break;
      if (next !== ",") {
        this.position -= 1;
        throw this.syntax("expecting ',' delimiter", pointer);
      }
    }
    // Python's `object_pairs_hook` sees the pairs only once the object closes,
    // so any error inside a later member is reported before the duplicate.
    if (duplicate !== undefined) {
      throw new LoaderFailure("duplicate-key", childPointer(pointer, duplicate), `duplicate key ${JSON.stringify(duplicate)}`);
    }
    return jsonObject(entries);
  }

  private array(pointer: string, depth: number): JsonValue {
    this.checkDepth(pointer, depth);
    this.position += 1;
    const items: JsonValue[] = [];
    this.skipWhitespace();
    if (this.text[this.position] === "]") {
      this.position += 1;
      return items;
    }
    for (;;) {
      items.push(this.value(childPointer(pointer, items.length), depth));
      this.skipWhitespace();
      const next = this.text[this.position];
      this.position += 1;
      if (next === "]") return items;
      if (next !== ",") {
        this.position -= 1;
        throw this.syntax("expecting ',' delimiter", pointer);
      }
    }
  }

  private string(pointer: string): string {
    const text = this.text;
    this.position += 1;
    let result = "";
    let start = this.position;
    for (;;) {
      const code = text.charCodeAt(this.position);
      if (Number.isNaN(code)) throw this.syntax("unterminated string", pointer);
      if (code === 0x22) {
        result += text.slice(start, this.position);
        this.position += 1;
        return result;
      }
      if (code < 0x20) throw this.syntax("invalid control character in a string", pointer);
      if (code !== 0x5c) {
        this.position += 1;
        continue;
      }
      result += text.slice(start, this.position);
      const escape = text[this.position + 1];
      if (escape === "u") {
        const hex = text.slice(this.position + 2, this.position + 6);
        if (!HEX4.test(hex)) throw this.syntax("invalid \\uXXXX escape", pointer);
        // Lone surrogate escapes are kept, as Python keeps them; adjacent
        // escapes forming a pair become one astral character in both.
        result += String.fromCharCode(Number.parseInt(hex, 16));
        this.position += 6;
      } else {
        const decoded = escape === undefined ? undefined : SIMPLE_ESCAPES[escape];
        if (decoded === undefined) throw this.syntax("invalid escape", pointer);
        result += decoded;
        this.position += 2;
      }
      start = this.position;
    }
  }

  private number(pointer: string): JsonValue {
    NUMBER.lastIndex = this.position;
    const match = NUMBER.exec(this.text);
    if (match === null) throw this.syntax("expecting a value", pointer);
    const lexeme = match[0];
    const token = jsonNumber(lexeme);
    if (token.integer) {
      const digits = lexeme.startsWith("-") ? lexeme.length - 1 : lexeme.length;
      if (digits > MAX_INTEGER_DIGITS) {
        throw this.syntax(`an integer of ${digits} digits exceeds the ${MAX_INTEGER_DIGITS}-digit limit`, pointer);
      }
    } else if (!Number.isFinite(Number(lexeme))) {
      throw new LoaderFailure("nonfinite-number", pointer, `${lexeme} overflows a double at ${this.where()}`);
    }
    this.position += lexeme.length;
    return token;
  }

  private literal(word: string, value: JsonValue, pointer: string): JsonValue {
    if (!this.text.startsWith(word, this.position)) throw this.syntax("expecting a value", pointer);
    this.position += word.length;
    return value;
  }

  private constant(word: string, pointer: string): never {
    if (!this.text.startsWith(word, this.position)) throw this.syntax("expecting a value", pointer);
    throw new LoaderFailure("nonfinite-number", pointer, `${word} is not a JSON number`);
  }

  private checkDepth(pointer: string, depth: number): void {
    if (depth > MAX_DEPTH) throw this.syntax(`the document nests deeper than ${MAX_DEPTH} levels`, pointer);
  }

  private skipWhitespace(): void {
    const text = this.text;
    for (;;) {
      const char = text[this.position];
      if (char !== " " && char !== "\t" && char !== "\n" && char !== "\r") return;
      this.position += 1;
    }
  }

  private where(): string {
    const before = this.text.slice(0, this.position);
    const line = before.split("\n").length;
    const column = this.position - before.lastIndexOf("\n");
    return `line ${line} column ${column}`;
  }

  private syntax(message: string, pointer: string): LoaderFailure {
    return new LoaderFailure("syntax", pointer, `${message} at ${this.where()}`);
  }
}

/** See `StrictLoader`. Every rejection carries one located problem. */
export const loadStrict: StrictLoader = (bytes: Uint8Array): ContractResult<JsonValue> => {
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    return { ok: false, problems: [{ code: "invalid-utf8", location: "", message: "the document is not valid UTF-8" }] };
  }
  if (text.startsWith("\ufeff")) {
    return {
      ok: false,
      problems: [{ code: "byte-order-mark", location: "", message: "the document starts with a UTF-8 byte order mark" }],
    };
  }
  try {
    return { ok: true, value: new Parser(text).document() };
  } catch (error) {
    if (!(error instanceof LoaderFailure)) throw error;
    return { ok: false, problems: [{ code: error.code, location: error.location, message: error.message }] };
  }
};
