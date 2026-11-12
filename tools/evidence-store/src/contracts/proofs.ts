//! Structural checks of the two proof documents an observation cites, run when
//! a verifier finalizes their upload.
//!
//! Ports of `schema.validate_completion_record`, the structural part of
//! `completion.manifest_problems`, and the cell/run binding of
//! `hosted_observations._completion_record` and
//! `local_evidence._proof_problem`. The Python stays authoritative; where the
//! two disagree, this file is the defect. Deliberately not ported: plan
//! selection (`selection_problems`, `declared_problems`), skip approvals, and
//! coverage acceptance. A proof that passes here is stored as intact bytes,
//! never as a coverage claim.
//!
//! Each problem keeps the Python's code. Messages differ from the Python in
//! one way: they never quote a value read from the uploaded document (a proof
//! can be up to 64 MiB of arbitrary text), only field names and the
//! observation's own stored values.

import type { ReferenceKind } from "./api.ts";
import { compareCodePoints } from "./canonical.ts";
import { ENVIRONMENTS, GATES } from "./evidence.ts";
import { isJsonNumber, isJsonObject, memberOf, type JsonNumber, type JsonObject, type JsonValue } from "./json.ts";
import { loadStrict } from "./loader.ts";
import { pythonRepr } from "./validate.ts";

export const COMPLETION_RECORD_SCHEMA_VERSION = 1;
export const EXPECTED_MANIFEST_SCHEMA_VERSION = 2;

type Fields = readonly (readonly [name: string, required: boolean])[];

/** `schema.COMPLETION_RECORD_FIELDS`. */
export const COMPLETION_RECORD_FIELDS: Fields = [
  ["schema_version", true],
  ["package", true],
  ["environment", true],
  ["gate", true],
  ["complete", true],
  ["head", true],
  ["run", true],
  ["attempt", true],
  ["nextest_version", false],
  ["reports", true],
  ["build", false],
  ["gate_inputs", false],
  ["companions", false],
  ["backends", false],
];

/** `schema.EXPECTED_MANIFEST_FIELDS`. */
export const EXPECTED_MANIFEST_FIELDS: Fields = [
  ["schema_version", true],
  ["environment", true],
  ["tier", true],
  ["target", true],
  ["nextest_version", true],
  ["from_archive", true],
  ["selection", true],
  ["packages", true],
  ["backends", false],
  ["companion_suites", false],
];

/** `schema.EXPECTED_PACKAGE_FIELDS`. */
export const EXPECTED_PACKAGE_FIELDS = ["tests", "ignored", "excluded"] as const;

/** `schema.BUILD_GATES`: the test tiers, whose completion record names a nextest version. */
const TEST_TIERS: readonly string[] = ["L1", "L2", "browser"];

const SHA = /^[0-9a-f]{40}$/;
/** Python's `str.isspace` set, which `str.strip()` removes. */
const PYTHON_SPACE = /^[\t\n\v\f\r\x1c-\x1f \x85\xa0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/;

export interface ProofProblem {
  readonly code: string;
  readonly message: string;
}

/**
 * What a proof is bound to: the observation's cell, its collection context
 * (environment and head live on the stored observation row), and its producer.
 */
export interface ProofCell {
  readonly package: string;
  readonly gate: string;
  readonly environment: string;
  readonly head: string;
  /** `proof.target`: the only toolchain identity an observation records. */
  readonly target: string;
  readonly producer: JsonObject;
}

function problem(code: string, message: string): ProofProblem {
  return { code, message: `${code}: ${message}` };
}

/** `schema.is_version`: an integer token, never a boolean or a float. */
function isVersion(value: JsonValue | undefined, expected: number): boolean {
  return isJsonNumber(value) && value.integer && BigInt(value.lexeme) === BigInt(expected);
}

/** `schema._keys` with a caller-chosen code. */
function keys(where: string, value: JsonValue, fields: Fields, code: string): ProofProblem[] {
  if (!isJsonObject(value)) return [problem(code, `${where} must be an object`)];
  const problems = fields
    .filter(([name, required]) => required && !Object.hasOwn(value, name))
    .map(([name]) => problem(code, `${where} is missing required field '${name}'`));
  const known = new Set(fields.map(([name]) => name));
  for (const name of Object.keys(value).sort(compareCodePoints)) {
    if (!known.has(name)) problems.push(problem(code, `${where} has unknown field ${pythonRepr(name)}`));
  }
  return problems;
}

/** `schema.validate_completion_record`: structure only. The version is read before the field set. */
export function validateCompletionRecord(document: JsonValue): ProofProblem[] {
  if (isJsonObject(document) && Object.hasOwn(document, "schema_version") && !isVersion(document["schema_version"], COMPLETION_RECORD_SCHEMA_VERSION)) {
    return [problem("completion-manifest-schema", `completion record is not schema version ${COMPLETION_RECORD_SCHEMA_VERSION}, which this tool reads`)];
  }
  const problems = keys("completion record", document, COMPLETION_RECORD_FIELDS, "completion-plan-unreadable");
  if (problems.length > 0 || !isJsonObject(document)) return problems;
  for (const name of ["run", "package"]) {
    const value = document[name];
    if (typeof value !== "string" || value === "") {
      problems.push(problem("completion-plan-unreadable", `completion record ${name} must be a string`));
    }
  }
  const attempt = document["attempt"];
  if (!isJsonNumber(attempt) || !attempt.integer || BigInt(attempt.lexeme) < 1n) {
    problems.push(problem("completion-plan-unreadable", "completion record attempt must be a positive integer"));
  }
  for (const [name, allowed] of [
    ["environment", ENVIRONMENTS],
    ["gate", GATES],
  ] as const) {
    const value = document[name];
    if (typeof value !== "string" || !(allowed as readonly string[]).includes(value)) {
      problems.push(problem("completion-cell-unknown", `completion record ${name} is not one of ${pythonRepr([...allowed])}`));
    }
  }
  const head = document["head"];
  if (typeof head !== "string" || !SHA.test(head)) {
    problems.push(problem("completion-plan-unreadable", "completion record head is not a full Git object ID"));
  }
  if (document["complete"] !== true) {
    problems.push(
      problem("completion-plan-unreadable", "a completion record exists only when validation succeeded, so `complete` must be true"),
    );
  }
  const reports = document["reports"];
  if (!Array.isArray(reports) || !reports.every((item) => typeof item === "string")) {
    problems.push(problem("completion-report-missing", "completion record reports must be a list of strings"));
  }
  return problems;
}

function sameInteger(left: JsonValue | undefined, right: JsonNumber): boolean {
  return isJsonNumber(left) && left.integer && right.integer && BigInt(left.lexeme) === BigInt(right.lexeme);
}

/**
 * A structurally valid record bound to `cell`. A `hosted` record binds as
 * `_completion_record` binds it: package, environment, gate, head, and the
 * run and attempt it records. Any other producer binds package, environment,
 * and gate only, as `local_evidence.py` checks a rig or seed record: those
 * runs prove each cell on whichever same-tree commit they ran it on, so the
 * record's head need not be the observation's (its tree is, but a record
 * carries no tree to compare).
 */
export function completionBindingProblems(record: JsonObject, cell: ProofCell): ProofProblem[] {
  const problems: ProofProblem[] = [];
  const mismatch = (name: string, value: string) =>
    problem("completion-cell-unknown", `the completion record's ${name} is not this observation's ${pythonRepr(value)}; a record from another cell or run proves nothing here`);
  for (const name of ["package", "environment", "gate"] as const) {
    if (record[name] !== cell[name]) problems.push(mismatch(name, cell[name]));
  }
  if (memberOf(cell.producer, "kind") === "hosted") {
    if (record["head"] !== cell.head) problems.push(mismatch("head", cell.head));
    const runId = memberOf(cell.producer, "run_id");
    if (typeof runId === "string" && record["run"] !== runId) problems.push(mismatch("run", runId));
    const attempt = memberOf(cell.producer, "attempt");
    if (isJsonNumber(attempt) && !sameInteger(record["attempt"], attempt)) problems.push(mismatch("attempt", attempt.lexeme));
  }
  if (TEST_TIERS.includes(cell.gate)) {
    const version = memberOf(record, "nextest_version");
    if (typeof version !== "string" || version === "") {
      problems.push(problem("completion-plan-unreadable", "the completion record names no nextest version"));
    }
  }
  return problems;
}

/**
 * The structural part of `completion.manifest_problems`. `target` binds to
 * the observation's `proof.target`; no other toolchain identity is recorded
 * on an observation, so none is invented here. `nextest_version` must be a
 * nonempty string but is not bound to `proof.nextest_version`, which a hosted
 * producer copies from the completion record rather than the listing.
 */
export function expectedManifestProblems(manifest: JsonValue, cell: ProofCell): ProofProblem[] {
  const provenance = "completion-manifest-provenance";
  if (!isJsonObject(manifest)) return [problem("completion-manifest-schema", "the expected manifest must be an object")];
  if (!isVersion(memberOf(manifest, "schema_version"), EXPECTED_MANIFEST_SCHEMA_VERSION)) {
    return [
      problem(
        "completion-manifest-schema",
        `the expected manifest is not schema_version ${EXPECTED_MANIFEST_SCHEMA_VERSION}, which records the ignored and excluded identities an earlier version dropped`,
      ),
    ];
  }
  const problems = keys("the expected manifest", manifest, EXPECTED_MANIFEST_FIELDS, provenance);
  if (problems.length > 0) return problems;

  if (manifest["target"] !== cell.target) {
    problems.push(
      problem(
        "completion-manifest-target",
        `the expected manifest was not listed on this observation's target ${pythonRepr(cell.target)}; a cfg-compiled test set differs between targets`,
      ),
    );
  }
  if (manifest["environment"] !== cell.environment) {
    problems.push(problem(provenance, `the expected manifest does not name the observation's environment ${pythonRepr(cell.environment)}`));
  }
  if (manifest["tier"] !== cell.gate) {
    problems.push(problem(provenance, `the expected manifest does not name the observation's gate ${pythonRepr(cell.gate)} as its tier`));
  }
  const version = manifest["nextest_version"];
  if (typeof version !== "string" || PYTHON_SPACE.test(version)) {
    problems.push(problem(provenance, "the expected manifest records no nextest_version, so a cross-host listing mismatch could not be seen"));
  }
  const packages = manifest["packages"];
  if (!isJsonObject(packages) || !Object.hasOwn(packages, cell.package)) {
    problems.push(problem(provenance, `the expected manifest lists no identities for ${cell.package}`));
  } else {
    const listing = packages[cell.package];
    for (const name of EXPECTED_PACKAGE_FIELDS) {
      if (!isJsonObject(listing) || !Object.hasOwn(listing, name)) {
        problems.push(problem(provenance, `the expected manifest's ${cell.package} listing is missing '${name}'`));
      }
    }
  }
  return problems;
}

/**
 * Every structural reason `bytes` is not a usable proof of `kind` for
 * `cell`, loaded with the shared strict loader. A report has no structure to
 * check and always answers `[]`.
 */
export function proofProblems(kind: ReferenceKind, bytes: Uint8Array, cell: ProofCell): ProofProblem[] {
  if (kind === "report") return [];
  const loaded = loadStrict(bytes);
  if (!loaded.ok) {
    const first = loaded.problems[0]!;
    return kind === "completion-record"
      ? [problem("completion-plan-unreadable", `the completion record is not a strict JSON document (${first.code})`)]
      : [problem("completion-manifest-schema", `the expected manifest is not a strict JSON document (${first.code})`)];
  }
  if (kind === "expected-listing") return expectedManifestProblems(loaded.value, cell);
  const problems = validateCompletionRecord(loaded.value);
  if (problems.length > 0 || !isJsonObject(loaded.value)) return problems;
  return completionBindingProblems(loaded.value, cell);
}

/** The binding of a stored observation: its loaded payload plus the row's `environment` and `head`. */
export function proofCellOf(observation: JsonObject, context: { readonly environment: string; readonly head: string }): ProofCell {
  // Stored payloads passed `validateObservation`, so these members have their shapes.
  const cell = observation["cell"] as JsonObject;
  const proof = memberOf(observation, "proof");
  return {
    package: cell["package"] as string,
    gate: cell["gate"] as string,
    environment: context.environment,
    head: context.head,
    target: isJsonObject(proof) ? (proof["target"] as string) : "",
    producer: observation["producer"] as JsonObject,
  };
}
