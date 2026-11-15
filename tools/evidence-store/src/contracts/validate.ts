//! Ports of `schema.validate_collection`, `validate_observation`,
//! `receipt_cell_problems`, and `validate_receipt` over lossless `JsonValue`s.
//!
//! Each problem's `message` is the exact Python problem string, including its
//! `repr` renderings, and its `code` is the text before the first `:`. The
//! `location` is this port's addition. Every early return of the Python is kept,
//! so a document never yields more problems here than there. Where the Python
//! raises instead of reporting (a non-numeric count part, an unhashable value
//! used as a dictionary key) this port reports a located problem or skips the
//! check that would raise; `docs/contract-inventory.md` lists each case.

import { compareCodePoints, canonical, formatPythonFloat, formatPythonNumber, observationIdentity } from "./canonical.ts";
import {
  childPointer,
  isJsonNumber,
  isJsonObject,
  memberOf,
  objectKeys,
  type ContractProblem,
  type JsonNumber,
  type JsonObject,
  type JsonValue,
} from "./json.ts";
import {
  COLLECTION_SCHEMA_VERSION,
  COMPLETIONS,
  ENVIRONMENTS,
  FAILURE_DETAIL_LIMIT,
  GATES,
  LEGACY_RECEIPT_SCHEMA_VERSION,
  OUTCOMES,
  PRODUCER_KINDS,
  RECEIPT_SCHEMA_VERSION,
  REPORT_STORES,
} from "./evidence.ts";

type Fields = readonly (readonly [name: string, required: boolean])[];

const RECEIPT_FIELDS: Fields = [
  ["schema_version", true],
  ["environment", true],
  ["base", true],
  ["head", true],
  ["tree", true],
  ["scope_identity", true],
  ["completion", true],
  ["host", true],
  ["cells", true],
];
const RECEIPT_CELL_FIELDS: Fields = [
  ["package", true],
  ["gate", true],
  ["outcome", true],
  ["exit_code", true],
  ["completion", true],
  ["counts", true],
  ["duration_s", true],
  ["gate_input_identity", true],
  ["backends", true],
  ["report", true],
  ["failed_tests", false],
  ["failure_detail_truncated", false],
  ["build", false],
  ["test_filter", false],
];
const RECEIPT_BUILD_FIELDS: Fields = [
  ["key", true],
  ["digest", true],
];
const HOST_FIELDS: Fields = [
  ["os", true],
  ["kernel", true],
  ["report_dir", true],
];
const COLLECTION_FIELDS: Fields = [
  ["schema_version", true],
  ["environment", true],
  ["head", true],
  ["tree", true],
  ["observations", true],
];
const OBSERVATION_FIELDS: Fields = [
  ["observation_id", true],
  ["retry_of", false],
  ["producer", true],
  ["base", true],
  ["scope_identity", true],
  ["completion", true],
  ["cell", true],
  ["proof", false],
  ["reports", true],
  ["features", false],
];
const OBSERVATION_HOST_FIELDS: Fields = [
  ["os", true],
  ["kernel", true],
  ["label", false],
  ["report_dir", false],
];
const OBSERVATION_PRODUCER_FIELDS: Fields = [
  ["kind", true],
  ["host", true],
  ["workflow", false],
  ["run_id", false],
  ["attempt", false],
  ["seed_id", false],
];
const OBSERVATION_PROOF_FIELDS: Fields = [
  ["nextest_version", true],
  ["target", true],
  ["expected_listing_digest", true],
  ["completion_record_digest", false],
];
const OBSERVATION_REPORT_FIELDS: Fields = [
  ["path", true],
  ["sha256", true],
  ["bytes", true],
  ["store", true],
];
const COUNT_FIELDS = ["total", "passed", "failed", "skipped", "errored"] as const;
const COUNT_PARTS = ["passed", "failed", "skipped", "errored"] as const;

const SHA = /^[0-9a-f]{40}$/;
const IDENTITY = /^[0-9a-f]{8,64}$/;
const REPORT_DIGEST = /^[0-9a-f]{64}$/;

// --- Python value semantics -------------------------------------------------

/** Python's `str.isprintable` is false for categories C* and Z* other than the ASCII space. */
const NOT_PRINTABLE = /[\p{C}\p{Z}]/u;
/** Python's `str.isspace` set, which `str.strip()` removes. */
const PYTHON_SPACE = /^[\t\n\v\f\r\x1c-\x1f \x85\xa0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/;

function reprString(text: string): string {
  const quote = text.includes("'") && !text.includes('"') ? '"' : "'";
  let result = quote;
  for (const char of text) {
    const point = char.codePointAt(0) ?? 0;
    if (char === quote || char === "\\") result += `\\${char}`;
    else if (char === "\t") result += "\\t";
    else if (char === "\n") result += "\\n";
    else if (char === "\r") result += "\\r";
    else if (point < 0x20 || point === 0x7f) result += `\\x${point.toString(16).padStart(2, "0")}`;
    else if (point < 0x7f || (char !== " " && !NOT_PRINTABLE.test(char))) result += char;
    else if (point <= 0xff) result += `\\x${point.toString(16).padStart(2, "0")}`;
    else if (point <= 0xffff) result += `\\u${point.toString(16).padStart(4, "0")}`;
    else result += `\\U${point.toString(16).padStart(8, "0")}`;
  }
  return result + quote;
}

/** Python `repr` of a loaded JSON value; an absent member (`dict.get`) is `None`. */
export function pythonRepr(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return "None";
  if (value === true) return "True";
  if (value === false) return "False";
  if (typeof value === "string") return reprString(value);
  if (isJsonNumber(value)) return formatPythonNumber(value, "repr");
  if (Array.isArray(value)) return `[${value.map(pythonRepr).join(", ")}]`;
  const object: JsonObject = value;
  return `{${objectKeys(object)
    .map((key) => `${reprString(key)}: ${pythonRepr(object[key])}`)
    .join(", ")}}`;
}

/** Python `str()` of a loaded JSON value. */
export function pythonStr(value: JsonValue | undefined): string {
  return typeof value === "string" ? value : pythonRepr(value);
}

/** Python `text[:count]`, which slices code points, not UTF-16 units. */
function head(text: string, count: number): string {
  return Array.from(text).slice(0, count).join("");
}

function truthy(value: JsonValue | undefined): boolean {
  if (value === undefined || value === null || value === false) return false;
  if (value === true) return true;
  if (typeof value === "string" || Array.isArray(value)) return value.length > 0;
  if (isJsonNumber(value)) return value.integer ? BigInt(value.lexeme) !== 0n : Number(value.lexeme) !== 0;
  return Object.keys(value).length > 0;
}

/** `type(value) is int`: an integer token, never a boolean or a float. */
function exactInt(value: JsonValue | undefined): value is JsonNumber {
  return isJsonNumber(value) && value.integer;
}

type PythonNumber = { readonly kind: "int"; readonly value: bigint } | { readonly kind: "float"; readonly value: number };

/** The number Python holds for `value` under `isinstance(value, (int, float))`, which includes booleans. */
function pythonNumber(value: JsonValue | undefined): PythonNumber | undefined {
  if (value === true || value === false) return { kind: "int", value: value ? 1n : 0n };
  if (!isJsonNumber(value)) return undefined;
  return value.integer ? { kind: "int", value: BigInt(value.lexeme) } : { kind: "float", value: Number(value.lexeme) };
}

function isNegative(number: PythonNumber): boolean {
  return number.kind === "int" ? number.value < 0n : number.value < 0;
}

/** Python `a == b` between numbers: exact, even between an int and a float. */
function numbersEqual(left: PythonNumber, right: PythonNumber): boolean {
  if (left.kind === "int" && right.kind === "int") return left.value === right.value;
  if (left.kind === "float" && right.kind === "float") return left.value === right.value;
  const [whole, float] = left.kind === "int" ? [left.value, right.value as number] : [right.value as bigint, left.value];
  return Number.isInteger(float) && BigInt(float) === whole;
}

function numberText(number: PythonNumber): string {
  return number.kind === "int" ? number.value.toString() : formatPythonFloat(number.value, "repr");
}

/**
 * A key equal exactly when Python's dict-key equality holds (`1 == 1.0 ==
 * True`), or `undefined` for an unhashable list or object.
 */
function hashKey(value: JsonValue | undefined): string | undefined {
  if (value === undefined || value === null) return "n";
  if (typeof value === "string") return `s${value}`;
  if (Array.isArray(value) || isJsonObject(value)) return undefined;
  const number = pythonNumber(value);
  if (number === undefined) return undefined;
  if (number.kind === "int") return `i${number.value}`;
  return Number.isInteger(number.value) ? `i${BigInt(number.value)}` : `f${formatPythonFloat(number.value, "repr")}`;
}

function isVersion(value: JsonValue | undefined, expected: number): boolean {
  return exactInt(value) && BigInt(value.lexeme) === BigInt(expected);
}

function fullmatch(pattern: RegExp, text: string): boolean {
  return pattern.test(text);
}

// --- Shared checks (`_keys`, `_member`, `_sha`, `_str_list`, `_counts`) -----

function problem(location: string, message: string): ContractProblem {
  const colon = message.indexOf(":");
  return { code: colon === -1 ? message : message.slice(0, colon), location, message };
}

function keys(where: string, value: JsonValue | undefined, fields: Fields, pointer: string): ContractProblem[] {
  if (!isJsonObject(value)) return [problem(pointer, `malformed-receipt: ${where} must be an object`)];
  const problems = fields
    .filter(([name, required]) => required && !Object.hasOwn(value, name))
    .map(([name]) => problem(childPointer(pointer, name), `malformed-receipt: ${where} is missing required field '${name}'`));
  const known = new Set(fields.map(([name]) => name));
  for (const name of Object.keys(value).sort(compareCodePoints)) {
    if (!known.has(name)) {
      problems.push(problem(childPointer(pointer, name), `malformed-receipt: ${where} has unknown field '${name}'`));
    }
  }
  return problems;
}

function member(
  where: string,
  value: JsonValue | undefined,
  allowed: readonly string[],
  code: string,
  pointer: string,
): ContractProblem[] {
  if (typeof value === "string" && allowed.includes(value)) return [];
  return [problem(pointer, `${code}: ${where} is ${pythonRepr(value)}, expected one of ${pythonRepr([...allowed])}`)];
}

function sha(where: string, value: JsonValue | undefined, pointer: string): ContractProblem[] {
  if (typeof value === "string" && fullmatch(SHA, value)) return [];
  return [problem(pointer, `malformed-receipt: ${where} is not a full Git object ID: ${pythonRepr(value)}`)];
}

/** `_str_list` without `allowed`; located at the first non-string element when the value is a list. */
function stringList(where: string, value: JsonValue | undefined, pointer: string): ContractProblem[] {
  if (Array.isArray(value)) {
    const bad = value.findIndex((item) => typeof item !== "string");
    if (bad === -1) return [];
    return [problem(childPointer(pointer, bad), `malformed-receipt: ${where} must be a list of strings`)];
  }
  return [problem(pointer, `malformed-receipt: ${where} must be a list of strings`)];
}

function counts(label: string, value: JsonValue | undefined, pointer: string): ContractProblem[] {
  if (!isJsonObject(value)) return [problem(pointer, `malformed-receipt: ${label} counts must be an object`)];
  const problems = COUNT_FIELDS.filter((field) => !Object.hasOwn(value, field)).map((field) =>
    problem(childPointer(pointer, field), `malformed-receipt: ${label} counts is missing '${field}'`),
  );
  for (const field of COUNT_FIELDS) {
    const number = pythonNumber(memberOf(value, field));
    if (number?.kind === "int" && isNegative(number)) {
      problems.push(problem(childPointer(pointer, field), `malformed-receipt: ${label} counts.${field} must be a non-negative integer`));
    }
  }
  if (problems.length > 0) return problems;

  let sum: PythonNumber = { kind: "int", value: 0n };
  for (const field of COUNT_PARTS) {
    const part = pythonNumber(memberOf(value, field));
    if (part === undefined) {
      // Python's `sum` raises TypeError here; a port must answer, not crash.
      problems.push(problem(childPointer(pointer, field), `malformed-receipt: ${label} counts.${field} must be a number`));
      continue;
    }
    if (problems.length > 0) continue;
    if (sum.kind === "int" && part.kind === "int") {
      sum = { kind: "int", value: sum.value + part.value };
      continue;
    }
    const left: number = sum.kind === "int" ? Number(sum.value) : sum.value;
    const right: number = part.kind === "int" ? Number(part.value) : part.value;
    if (!Number.isFinite(left) || !Number.isFinite(right)) {
      // Python raises OverflowError converting an int past the double range.
      problems.push(problem(childPointer(pointer, field), `malformed-receipt: ${label} counts.${field} is too large to sum with a float`));
      continue;
    }
    sum = { kind: "float", value: left + right };
  }
  if (problems.length > 0) return problems;

  const total = memberOf(value, "total");
  const totalNumber = pythonNumber(total);
  if (totalNumber === undefined || !numbersEqual(sum, totalNumber)) {
    problems.push(
      problem(
        childPointer(pointer, "total"),
        `malformed-receipt: ${label} counts.total is ${pythonStr(total)} but its parts sum to ${numberText(sum)}`,
      ),
    );
  }
  return problems;
}

// --- Receipt cell -----------------------------------------------------------

/**
 * `receipt_cell_problems`. `environment` is the raw value the caller labels
 * with (the receipt's `environment`, or `str()` of the collection's).
 */
export function receiptCellProblems(entry: JsonValue | undefined, environment: JsonValue, pointer: string): ContractProblem[] {
  const problems = keys("receipt cell", entry, RECEIPT_CELL_FIELDS, pointer);
  if (!isJsonObject(entry)) return problems;
  const at = (name: string): string => childPointer(pointer, name);
  const pkg = memberOf(entry, "package");
  const gate = memberOf(entry, "gate");
  if (typeof pkg !== "string" || pkg === "" || typeof gate !== "string") {
    const where = typeof pkg !== "string" || pkg === "" ? at("package") : at("gate");
    return [...problems, problem(where, "malformed-receipt: receipt cell names no package and gate")];
  }
  const label = `receipt cell ${pkg}/${pythonStr(environment)}/${gate}`;
  if (Object.hasOwn(entry, "test_filter")) {
    const filter = entry["test_filter"];
    if (typeof filter !== "string" || PYTHON_SPACE.test(filter) || gate !== "L1") {
      problems.push(
        problem(
          at("test_filter"),
          `malformed-receipt: ${label} carries a test_filter, which only a non-empty L1 narrowing may be`,
        ),
      );
    }
  }
  problems.push(...member(`${label} gate`, gate, GATES, "unknown-gate", at("gate")));
  problems.push(...member(`${label} outcome`, memberOf(entry, "outcome"), OUTCOMES, "malformed-receipt", at("outcome")));
  problems.push(
    ...member(`${label} completion`, memberOf(entry, "completion"), COMPLETIONS, "malformed-receipt", at("completion")),
  );
  problems.push(...counts(label, memberOf(entry, "counts"), at("counts")));
  if (!fullmatch(IDENTITY, pythonStr(memberOf(entry, "gate_input_identity")))) {
    problems.push(problem(at("gate_input_identity"), `malformed-receipt: ${label} has no gate-input identity`));
  }
  if (!exactInt(memberOf(entry, "exit_code"))) {
    problems.push(problem(at("exit_code"), `malformed-receipt: ${label} exit_code must be an integer`));
  }
  const duration = memberOf(entry, "duration_s");
  const durationNumber = typeof duration === "boolean" ? undefined : pythonNumber(duration);
  if (durationNumber === undefined || isNegative(durationNumber)) {
    problems.push(problem(at("duration_s"), `malformed-receipt: ${label} duration_s must be a non-negative number`));
  }
  if (typeof memberOf(entry, "report") !== "string") {
    problems.push(problem(at("report"), `malformed-receipt: ${label} report must be a string`));
  }
  problems.push(...stringList(`${label} backends`, memberOf(entry, "backends"), at("backends")));
  if (Object.hasOwn(entry, "build")) {
    const build = entry["build"];
    problems.push(...keys(`${label} build`, build, RECEIPT_BUILD_FIELDS, at("build")));
    if (isJsonObject(build)) {
      for (const field of ["key", "digest"]) {
        if (!fullmatch(IDENTITY, pythonStr(memberOf(build, field)))) {
          problems.push(problem(childPointer(at("build"), field), `malformed-receipt: ${label} build ${field} is not a digest`));
        }
      }
    }
  }
  // `entry.get("failed_tests", [])`: absent reads as an empty list; present `null` is checked as `None`.
  const failed: JsonValue = Object.hasOwn(entry, "failed_tests") ? (entry["failed_tests"] as JsonValue) : [];
  problems.push(...stringList(`${label} failed_tests`, failed, at("failed_tests")));
  if (Array.isArray(failed) && failed.length > FAILURE_DETAIL_LIMIT) {
    problems.push(
      problem(
        at("failed_tests"),
        `malformed-receipt: ${label} names ${failed.length} failed tests, over the ${FAILURE_DETAIL_LIMIT} the receipt carries; truncate and set failure_detail_truncated`,
      ),
    );
  }
  if (
    memberOf(entry, "outcome") === "fail" &&
    memberOf(entry, "completion") === "complete" &&
    !truthy(failed) &&
    !truthy(memberOf(entry, "failure_detail_truncated"))
  ) {
    problems.push(
      problem(
        pointer,
        `malformed-receipt: ${label} failed but names no failing test; a failure with no detail cannot be acted on`,
      ),
    );
  }
  return problems;
}

// --- Receipt ----------------------------------------------------------------

/** `validate_receipt`: every reason `document` is not a valid version-2 receipt. */
export function validateReceipt(document: JsonValue): ContractProblem[] {
  if (!isJsonObject(document) || !Object.hasOwn(document, "schema_version")) {
    return keys("receipt", document, RECEIPT_FIELDS, "");
  }
  const version = document["schema_version"];
  if (isVersion(version, LEGACY_RECEIPT_SCHEMA_VERSION)) {
    return [
      problem(
        "/schema_version",
        "v1-requires-exact-tree: a version-1 receipt carries no per-cell outcome and is reusable only on exact tree identity, pass-only, whole-environment",
      ),
    ];
  }
  if (!isVersion(version, RECEIPT_SCHEMA_VERSION)) {
    return [
      problem(
        "/schema_version",
        `unknown-schema-version: receipt is version ${pythonRepr(version)}, this tool reads ${RECEIPT_SCHEMA_VERSION} and ${LEGACY_RECEIPT_SCHEMA_VERSION}`,
      ),
    ];
  }
  const problems = keys("receipt", document, RECEIPT_FIELDS, "");
  if (problems.length > 0) return problems;

  const environment = document["environment"] as JsonValue;
  problems.push(...member("receipt environment", environment, ENVIRONMENTS, "unknown-environment", "/environment"));
  for (const field of ["base", "head", "tree"]) {
    problems.push(...sha(`receipt ${field}`, document[field], `/${field}`));
  }
  if (!fullmatch(IDENTITY, pythonStr(document["scope_identity"]))) {
    problems.push(problem("/scope_identity", "malformed-receipt: receipt scope_identity is not a hex identity"));
  }
  problems.push(...member("receipt completion", document["completion"], COMPLETIONS, "malformed-receipt", "/completion"));
  problems.push(...keys("receipt host", document["host"], HOST_FIELDS, "/host"));

  const cells = document["cells"];
  if (!Array.isArray(cells) || cells.length === 0) {
    problems.push(problem("/cells", "malformed-receipt: receipt cells must be a non-empty list"));
    return problems;
  }
  const seen = new Map<string, JsonObject>();
  cells.forEach((entry, index) => {
    const pointer = childPointer("/cells", index);
    problems.push(...receiptCellProblems(entry, environment, pointer));
    if (!isJsonObject(entry) || !Object.hasOwn(entry, "package") || !Object.hasOwn(entry, "gate")) return;
    const pkg = entry["package"] as JsonValue;
    const gate = entry["gate"] as JsonValue;
    const pkgKey = hashKey(pkg);
    const gateKey = hashKey(gate);
    // An unhashable package or gate makes Python's `key in seen` raise; the
    // cell problems above already reject it, so the conflict check is skipped.
    if (pkgKey === undefined || gateKey === undefined) return;
    const key = JSON.stringify([pkgKey, gateKey]);
    const previous = seen.get(key);
    if (previous !== undefined && canonical(previous) !== canonical(entry)) {
      const label = `receipt cell ${pythonStr(pkg)}/${pythonStr(environment)}/${pythonStr(gate)}`;
      problems.push(problem(pointer, `conflicting-evidence: ${label} appears twice with different results`));
    }
    if (previous === undefined) seen.set(key, entry);
  });
  return problems;
}

// --- Observation ------------------------------------------------------------

/** `validate_observation`. `environment` is `str()` of the collection's environment. */
export function validateObservation(observation: JsonValue | undefined, environment: string, pointer: string): ContractProblem[] {
  const problems = keys("observation", observation, OBSERVATION_FIELDS, pointer);
  if (problems.length > 0 || !isJsonObject(observation)) return problems;
  const at = (name: string): string => childPointer(pointer, name);
  const id = observation["observation_id"] as JsonValue;
  const label = `observation ${head(pythonStr(id), 12)}`;
  if (typeof id !== "string" || !fullmatch(IDENTITY, id)) {
    return [problem(at("observation_id"), `malformed-receipt: ${label} has no hex observation_id`)];
  }
  if (id !== observationIdentity(observation)) {
    problems.push(problem(at("observation_id"), `observation-id-mismatch: ${label} is not the identity of its own content`));
  }
  if (Object.hasOwn(observation, "retry_of")) {
    const retry = observation["retry_of"];
    if (typeof retry !== "string" || !fullmatch(IDENTITY, retry)) {
      problems.push(problem(at("retry_of"), `observation-link-invalid: ${label} retry_of must be an observation id`));
    } else if (retry === id) {
      problems.push(problem(at("retry_of"), `observation-link-invalid: ${label} retries itself`));
    }
  }
  problems.push(...sha(`${label} base`, observation["base"], at("base")));
  if (!fullmatch(IDENTITY, pythonStr(observation["scope_identity"]))) {
    problems.push(problem(at("scope_identity"), `malformed-receipt: ${label} scope_identity is not a hex identity`));
  }
  problems.push(...member(`${label} completion`, observation["completion"], COMPLETIONS, "malformed-receipt", at("completion")));
  if (Object.hasOwn(observation, "features") && typeof observation["features"] !== "string") {
    problems.push(problem(at("features"), `malformed-receipt: ${label} features must be a string`));
  }

  const producer = observation["producer"];
  const producerAt = (name: string): string => childPointer(at("producer"), name);
  problems.push(...keys(`${label} producer`, producer, OBSERVATION_PRODUCER_FIELDS, at("producer")));
  let kind: JsonValue | undefined;
  let host: JsonValue | undefined;
  if (isJsonObject(producer)) {
    kind = memberOf(producer, "kind");
    host = memberOf(producer, "host");
    problems.push(...member(`${label} producer kind`, kind, PRODUCER_KINDS, "malformed-receipt", producerAt("kind")));
    problems.push(...keys(`${label} producer host`, host, OBSERVATION_HOST_FIELDS, producerAt("host")));
    if (isJsonObject(host)) {
      const hostAt = (name: string): string => childPointer(producerAt("host"), name);
      for (const field of ["os", "kernel"]) {
        const value = memberOf(host, field);
        if (typeof value !== "string" || value === "") {
          problems.push(problem(hostAt(field), `malformed-receipt: ${label} host ${field} must be a string`));
        }
      }
      for (const field of ["label", "report_dir"]) {
        if (Object.hasOwn(host, field) && typeof host[field] !== "string") {
          problems.push(problem(hostAt(field), `malformed-receipt: ${label} host ${field} must be a string`));
        }
      }
    }
    // Python looks `kind` up in a dict, which raises for an unhashable list
    // or object; such a kind is already reported above and requires nothing.
    const required = kind === "hosted" ? ["workflow", "run_id", "attempt"] : kind === "seed" ? ["seed_id"] : [];
    for (const name of required) {
      if (!Object.hasOwn(producer, name)) {
        problems.push(problem(producerAt(name), `malformed-receipt: ${label} producer ${pythonStr(kind)} needs '${name}'`));
      }
    }
    for (const name of ["workflow", "run_id", "seed_id"]) {
      if (Object.hasOwn(producer, name) && typeof producer[name] !== "string") {
        problems.push(problem(producerAt(name), `malformed-receipt: ${label} producer ${name} must be a string`));
      }
    }
    if (Object.hasOwn(producer, "attempt")) {
      const attempt = producer["attempt"];
      if (!exactInt(attempt) || BigInt(attempt.lexeme) < 1n) {
        problems.push(problem(producerAt("attempt"), `malformed-receipt: ${label} producer attempt must be a positive integer`));
      }
    }
  }

  const cell = observation["cell"];
  if (kind === "hook") {
    if (Object.hasOwn(observation, "proof")) {
      problems.push(
        problem(
          at("proof"),
          `malformed-receipt: ${label} is a hook observation, which proves through its report counts and carries no proof`,
        ),
      );
    }
  } else if (!Object.hasOwn(observation, "proof")) {
    if (!(isJsonObject(cell) && memberOf(cell, "outcome") === "fail")) {
      problems.push(
        problem(at("proof"), `observation-proof-missing: ${label} is a ${pythonRepr(kind)} observation and names no proof`),
      );
    }
  } else {
    const proof = observation["proof"];
    problems.push(...keys(`${label} proof`, proof, OBSERVATION_PROOF_FIELDS, at("proof")));
    if (isJsonObject(proof)) {
      for (const [name] of OBSERVATION_PROOF_FIELDS) {
        if (Object.hasOwn(proof, name) && (typeof proof[name] !== "string" || proof[name] === "")) {
          problems.push(
            problem(childPointer(at("proof"), name), `malformed-receipt: ${label} proof ${name} must be a non-empty string`),
          );
        }
      }
      if (isJsonObject(cell) && memberOf(cell, "outcome") === "pass" && !Object.hasOwn(proof, "completion_record_digest")) {
        problems.push(
          problem(
            childPointer(at("proof"), "completion_record_digest"),
            `observation-proof-missing: ${label} passed and its proof cites no completion record`,
          ),
        );
      }
    }
  }

  problems.push(...receiptCellProblems(cell, environment, at("cell")));

  const reports = observation["reports"];
  if (!Array.isArray(reports)) {
    problems.push(problem(at("reports"), `malformed-receipt: ${label} reports must be a list`));
    return problems;
  }
  reports.forEach((report, index) => {
    const reportPointer = childPointer(at("reports"), index);
    const reportAt = (name: string): string => childPointer(reportPointer, name);
    problems.push(...keys(`${label} report`, report, OBSERVATION_REPORT_FIELDS, reportPointer));
    if (!isJsonObject(report)) return;
    const path = memberOf(report, "path");
    if (typeof path !== "string" || path === "") {
      problems.push(problem(reportAt("path"), `malformed-receipt: ${label} report path must be a string`));
    }
    const digest = memberOf(report, "sha256");
    if (typeof digest !== "string" || !fullmatch(REPORT_DIGEST, digest)) {
      problems.push(problem(reportAt("sha256"), `malformed-receipt: ${label} report sha256 is not a digest`));
    }
    const bytes = memberOf(report, "bytes");
    if (!exactInt(bytes) || BigInt(bytes.lexeme) < 0n) {
      problems.push(problem(reportAt("bytes"), `malformed-receipt: ${label} report bytes must be a count`));
    }
    const store = memberOf(report, "store");
    problems.push(...member(`${label} report store`, store, REPORT_STORES, "malformed-receipt", reportAt("store")));
    if (store === "host-dir" && isJsonObject(host) && !truthy(memberOf(host, "report_dir"))) {
      problems.push(
        problem(
          childPointer(producerAt("host"), "report_dir"),
          `malformed-receipt: ${label} cites a host-dir report and its host names no report_dir`,
        ),
      );
    }
  });
  if (isJsonObject(cell) && memberOf(cell, "completion") === "complete") {
    const named = memberOf(cell, "report");
    if (typeof named === "string" && !reports.some((report) => isJsonObject(report) && memberOf(report, "path") === named)) {
      problems.push(
        problem(
          childPointer(at("cell"), "report"),
          `observation-report-unavailable: ${label} is complete and names report ${pythonRepr(named)}, which none of its references cite`,
        ),
      );
    }
  }
  return problems;
}

// --- Collection -------------------------------------------------------------

function cellKey(cell: JsonObject): string {
  // `cell.get("test_filter", "")`: a cell without a filter is the unfiltered cell.
  return JSON.stringify([cell["package"], cell["gate"], memberOf(cell, "test_filter") ?? ""]);
}

/** `validate_collection`: every reason `document` is not a valid version-3 collection. */
export function validateCollection(document: JsonValue): ContractProblem[] {
  if (!isJsonObject(document) || !Object.hasOwn(document, "schema_version")) {
    return keys("observation collection", document, COLLECTION_FIELDS, "");
  }
  const version = document["schema_version"];
  if (!isVersion(version, COLLECTION_SCHEMA_VERSION)) {
    return [
      problem(
        "/schema_version",
        `unknown-schema-version: collection is version ${pythonRepr(version)}, this tool reads ${COLLECTION_SCHEMA_VERSION}`,
      ),
    ];
  }
  const problems = keys("observation collection", document, COLLECTION_FIELDS, "");
  if (problems.length > 0) return problems;
  problems.push(...member("collection environment", document["environment"], ENVIRONMENTS, "unknown-environment", "/environment"));
  for (const field of ["head", "tree"]) {
    problems.push(...sha(`collection ${field}`, document[field], `/${field}`));
  }
  const observations = document["observations"];
  if (!Array.isArray(observations)) {
    return [...problems, problem("/observations", "malformed-receipt: collection observations must be a list")];
  }
  const environment = pythonStr(document["environment"]);
  const ids = new Map<string, { observation: JsonObject; pointer: string }>();
  observations.forEach((observation, index) => {
    const pointer = childPointer("/observations", index);
    const found = validateObservation(observation, environment, pointer);
    problems.push(...found);
    if (found.length > 0 || !isJsonObject(observation)) return;
    const id = observation["observation_id"] as string;
    if (ids.has(id)) {
      problems.push(problem(childPointer(pointer, "observation_id"), `conflicting-evidence: observation ${head(id, 12)} appears twice`));
    }
    // Python's `ids[id] = observation` keeps the first insertion position and the last value.
    ids.set(id, { observation, pointer });
  });
  if (problems.length > 0) return problems;

  const order = observations.map((observation) => (observation as JsonObject)["observation_id"] as string);
  const sorted = [...ids.keys()].sort(compareCodePoints);
  if (order.length !== sorted.length || order.some((id, index) => id !== sorted[index])) {
    problems.push(problem("/observations", "malformed-receipt: collection observations are not sorted by id"));
  }
  for (const [id, { observation, pointer }] of ids) {
    const target = memberOf(observation, "retry_of");
    if (typeof target !== "string") continue;
    const label = `observation ${head(id, 12)}`;
    const at = childPointer(pointer, "retry_of");
    const previous = ids.get(target);
    if (previous === undefined) {
      problems.push(problem(at, `observation-link-invalid: ${label} retries ${head(target, 12)}, which is absent`));
      continue;
    }
    if (cellKey(previous.observation["cell"] as JsonObject) !== cellKey(observation["cell"] as JsonObject)) {
      problems.push(problem(at, `observation-link-invalid: ${label} retries an observation of another cell`));
    }
    const visited = new Set([id]);
    let cursor: string | undefined = target;
    while (cursor !== undefined) {
      if (visited.has(cursor)) {
        problems.push(problem(at, `observation-link-invalid: ${label} is on a retry cycle`));
        break;
      }
      visited.add(cursor);
      const next = ids.get(cursor);
      const link = next === undefined ? undefined : memberOf(next.observation, "retry_of");
      cursor = typeof link === "string" ? link : undefined;
    }
  }
  return problems;
}
