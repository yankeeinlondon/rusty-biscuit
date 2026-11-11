//! Ingestion CLI: submit one evidence collection (version 3, or a version-2
//! receipt adapted on this host) and stage the files it references.
//!
//! ```text
//! ingest   INPUT --endpoint URL --repo ID --token-file PATH --state PATH [--proof-dir PATH]
//! resume   STATE
//! finalize STATE --token-file PATH
//! ```
//!
//! Everything that can be checked locally is checked before the first
//! request: flags, the token file, the input document, and every file the
//! `--proof-dir` map names. The resume state (paths and IDs only, never a
//! token) is written atomically before the first request and after every
//! acknowledgment, beside retained copies of the original input and the exact
//! submitted collection, so a lost response or crash resumes without the
//! source bundle and without rerunning a test. A producer credential stages
//! bytes; only `finalize` with a verifier credential publishes them.
//!
//! Exit codes: 0 success; 1 a service, network, or file failure after the
//! state exists (a resume command is printed); 2 a usage or input error,
//! found before any request.

import { createHash } from "node:crypto";
import { existsSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, posix, resolve, win32 } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { CLIENT_RETRY, LIMITS, REFERENCE_KINDS, type ReferenceKind } from "../src/contracts/api.ts";
import {
  adaptReceipt,
  isJsonNumber,
  isJsonObject,
  loadStrict,
  objectKeys,
  parseCollection,
  parseReceipt,
  PROOF_COMPLETION_FILE,
  PROOF_EXPECTED_FILE,
  type ContractProblem,
  type JsonObject,
  type JsonValue,
  type ReportReader,
} from "../src/contracts/index.ts";
import { nodeReportReader } from "../src/contracts/node-report-reader.ts";
import { CliError, readTokenFile } from "./operator.ts";

/** First full-jitter window; the window doubles per attempt up to `CLIENT_RETRY.backoff_cap_ms`. */
const BACKOFF_BASE_MS = 1_000;
const REPOSITORY_ID = /^[1-9][0-9]{0,19}$/;
const OBSERVATION_ID = /^[0-9a-f]{8,64}$/;
const SHA256 = /^[0-9a-f]{64}$/;
const UPLOAD_ID = /^[0-9a-f]{32}$/;
const LOOPBACK_HOSTS = new Set(["127.0.0.1", "localhost", "[::1]"]);
const STATE_FORMAT_VERSION = 1;

/** Everything the commands touch outside the file system, so tests run them in process and instantly. */
export interface IngestIo {
  readonly fetch: typeof fetch;
  readonly sleep: (ms: number) => Promise<void>;
  readonly random: () => number;
  readonly now: () => number;
  readonly stdout: (text: string) => void;
  readonly stderr: (text: string) => void;
  /** Resolves a version-2 receipt's report bytes on this host. */
  readonly readReport: ReportReader;
}

export function defaultIo(): IngestIo {
  return {
    fetch: globalThis.fetch,
    sleep: (ms) => new Promise((done) => setTimeout(done, ms)),
    random: Math.random,
    now: Date.now,
    stdout: (text) => process.stdout.write(text),
    stderr: (text) => process.stderr.write(text),
    readReport: nodeReportReader,
  };
}

export type UploadState = "pending" | "staged" | "finalized";

export interface UploadEntry {
  observation_id: string;
  reference_kind: ReferenceKind;
  /** A report's `sha256` or the proof digest; always the SHA-256 hex of `file` (checked before any request). */
  reference_digest: string;
  /** The reference's original path string (a proof's file name), sent as provenance. */
  path: string;
  /** Absolute native path of the bytes on this host. */
  file: string;
  bytes: number;
  upload_id?: string;
  parts_acknowledged: number;
  state: UploadState;
}

/** Shape of `api/schemas/resume-state.schema.json`. */
export interface ResumeState {
  format_version: 1;
  endpoint: string;
  repository_id: string;
  /** A path only; the token never enters the state. */
  token_file: string;
  input: { file: string; source: string; sha256: string };
  submission: { file: string; source_version: 2 | 3; collection_id: string };
  ingest: { state: "pending" | "acknowledged"; evidence_revision?: number; accepted?: string[]; already_present?: string[] };
  uploads: UploadEntry[];
  updated_at_ms: number;
}

export interface IngestOptions {
  readonly input: string;
  readonly endpoint: string;
  readonly repo: string;
  readonly tokenFile: string;
  readonly state: string;
  readonly proofDir?: string;
}

function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

/** A path as a copy-pasteable command argument; double quotes read the same in POSIX shells, cmd, and PowerShell. */
function argument(path: string): string {
  return /^[A-Za-z0-9_./:\\-]+$/.test(path) ? path : JSON.stringify(path);
}

export function resumeCommand(statePath: string): string {
  return `just evidence-resume ${argument(statePath)}`;
}

function finalizeCommand(statePath: string, tokenFile = "<verifier token path>"): string {
  return `just evidence-finalize ${argument(statePath)} --token-file ${tokenFile === "<verifier token path>" ? tokenFile : argument(tokenFile)}`;
}

function retainedPaths(statePath: string): { input: string; collection: string } {
  return { input: `${statePath}.input.json`, collection: `${statePath}.collection.json` };
}

/** Writes `bytes` to a sibling temporary file, then renames it over `path`, so a crash leaves the old or the new file whole. */
function atomicWrite(path: string, bytes: string | Uint8Array): void {
  const temporary = join(dirname(path), `.${path.slice(dirname(path).length + 1)}.${process.pid}.tmp`);
  try {
    writeFileSync(temporary, bytes, { mode: 0o600 });
    renameSync(temporary, path);
  } catch (error) {
    rmSync(temporary, { force: true });
    throw new CliError(1, `could not write ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
}

function saveState(statePath: string, state: ResumeState, io: IngestIo): void {
  state.updated_at_ms = io.now();
  atomicWrite(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

/** `https://host[:port][/]`, or `http://` only for a loopback host (local `wrangler dev`). Answers the origin. */
export function checkEndpoint(endpoint: string): string {
  const usage = "--endpoint must be https://host[:port] (http:// only for 127.0.0.1, localhost, or [::1]), without a path";
  if (!/^https?:\/\/[^/?#@\s]+\/?$/.test(endpoint)) throw new CliError(2, usage);
  let url: URL;
  try {
    url = new URL(endpoint);
  } catch {
    throw new CliError(2, usage);
  }
  if (url.protocol === "http:" && !LOOPBACK_HOSTS.has(url.hostname)) throw new CliError(2, usage);
  return url.origin;
}

function checkRepo(repo: string): string {
  if (!REPOSITORY_ID.test(repo)) throw new CliError(2, "--repo must be the repository's numeric GitHub ID");
  return repo;
}

/** The token, with a missing or empty file reported as a configuration error (exit 2) before any request. */
function readToken(path: string): string {
  try {
    return readTokenFile(path);
  } catch (error) {
    if (error instanceof CliError) throw new CliError(2, error.message);
    throw error;
  }
}

function readInput(path: string, what: string): Uint8Array {
  try {
    return new Uint8Array(readFileSync(path));
  } catch (error) {
    throw new CliError(2, `could not read ${what} ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
}

function located(file: string, problems: readonly ContractProblem[]): CliError {
  return new CliError(2, problems.map((problem) => `${file} is invalid at "${problem.location}": ${problem.message}`).join("\n"));
}

interface Adapted {
  readonly sourceVersion: 2 | 3;
  readonly collection: JsonObject;
  readonly canonical: string;
  readonly collectionId: string;
}

/** A version-2 receipt is adapted here, reading its reports on this host; anything else must be a version-3 collection. */
function adapt(path: string, bytes: Uint8Array, io: IngestIo): Adapted {
  const loaded = loadStrict(bytes);
  if (!loaded.ok) throw located(path, loaded.problems);
  const version = isJsonObject(loaded.value) ? loaded.value["schema_version"] : undefined;
  if (isJsonNumber(version) && version.integer && version.lexeme === "2") {
    const receipt = parseReceipt(bytes);
    if (!receipt.ok) throw located(path, receipt.problems);
    const adapted = adaptReceipt(receipt.value, io.readReport);
    if (!adapted.ok) throw located(path, adapted.problems);
    return { sourceVersion: 2, collection: adapted.value.collection, canonical: adapted.value.canonical, collectionId: adapted.value.collection_id };
  }
  const parsed = parseCollection(bytes);
  if (!parsed.ok) throw located(path, parsed.problems);
  return {
    sourceVersion: 3,
    collection: parsed.value.collection as JsonObject,
    canonical: parsed.value.canonical,
    collectionId: parsed.value.collection_id,
  };
}

/** A report path a proof directory can hold: relative, with no `..` segment, on any OS's reading. */
function confinable(path: string): boolean {
  return !posix.isAbsolute(path) && !win32.isAbsolute(path) && !path.split(/[\\/]/).includes("..");
}

function proofFile(template: string, cell: JsonObject): string {
  return template.replace("{package}", cell["package"] as string).replace("{gate}", cell["gate"] as string);
}

/**
 * Checks `file` against a reference: it must exist, hash to `digest`, and,
 * for a report, have its recorded length. Answers the byte length.
 */
function checkFile(file: string, digest: string, expectedBytes: number | undefined, where: string): number {
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(file);
  } catch (error) {
    throw new CliError(2, `the file for ${where} is missing: ${file} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  if (expectedBytes !== undefined && bytes.byteLength !== expectedBytes) {
    throw new CliError(2, `the file for ${where} has ${bytes.byteLength} bytes, but the reference records ${expectedBytes}: ${file}`);
  }
  if (sha256Hex(bytes) !== digest) throw new CliError(2, `the file for ${where} does not match its digest: ${file}`);
  if (bytes.byteLength > LIMITS.upload_object_max_bytes) {
    throw new CliError(2, `the file for ${where} exceeds the ${LIMITS.upload_object_max_bytes}-byte object limit: ${file}`);
  }
  return bytes.byteLength;
}

/**
 * The explicit upload map: every report of every observation, and each
 * proof's expected listing and completion record, mapped into `proofDir`.
 * Locations are JSON pointers into the submitted collection. Identical
 * references collapse to one upload.
 */
export function planUploads(collection: JsonObject, proofDir: string): UploadEntry[] {
  const uploads = new Map<string, UploadEntry>();
  const add = (entry: Omit<UploadEntry, "parts_acknowledged" | "state" | "bytes">, expectedBytes: number | undefined, where: string) => {
    const key = JSON.stringify([entry.observation_id, entry.reference_kind, entry.reference_digest, entry.path]);
    if (uploads.has(key)) return;
    const bytes = checkFile(entry.file, entry.reference_digest, expectedBytes, `"${where}"`);
    uploads.set(key, { ...entry, bytes, parts_acknowledged: 0, state: "pending" });
  };
  const observations = collection["observations"] as JsonObject[];
  observations.forEach((observation, index) => {
    const id = observation["observation_id"] as string;
    const base = `/observations/${index}`;
    (observation["reports"] as JsonObject[]).forEach((report, position) => {
      const where = `${base}/reports/${position}`;
      const path = report["path"] as string;
      if (!confinable(path)) throw new CliError(2, `the report path at "${where}" is absolute or contains "..", so --proof-dir cannot hold it`);
      const bytes = report["bytes"];
      add(
        { observation_id: id, reference_kind: "report", reference_digest: report["sha256"] as string, path, file: resolve(proofDir, path) },
        isJsonNumber(bytes) ? Number(bytes.lexeme) : undefined,
        where,
      );
    });
    const proof = observation["proof"];
    if (!isJsonObject(proof)) return;
    const cell = observation["cell"] as JsonObject;
    const proofs: [ReferenceKind, string, string][] = [["expected-listing", "expected_listing_digest", PROOF_EXPECTED_FILE]];
    if (typeof proof["completion_record_digest"] === "string") proofs.push(["completion-record", "completion_record_digest", PROOF_COMPLETION_FILE]);
    for (const [kind, field, template] of proofs) {
      const name = proofFile(template, cell);
      add(
        { observation_id: id, reference_kind: kind, reference_digest: proof[field] as string, path: name, file: resolve(proofDir, name) },
        undefined,
        `${base}/proof/${field}`,
      );
    }
  });
  return [...uploads.values()];
}

// ---- Network -------------------------------------------------------------

/** A non-retryable (or retry-exhausted) service answer; `status` lets callers tell a part conflict apart. */
class ServiceError extends CliError {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(1, message);
  }
}

interface ErrorFields {
  code?: unknown;
  message?: unknown;
  location?: unknown;
  retry_after_ms?: unknown;
}

function errorFields(text: string): { error: ErrorFields; requestId: unknown } {
  try {
    const parsed = JSON.parse(text) as { error?: ErrorFields; request_id?: unknown };
    return { error: parsed.error ?? {}, requestId: parsed.request_id };
  } catch {
    return { error: {}, requestId: undefined };
  }
}

function describeFailure(label: string, status: number, text: string): string {
  const { error, requestId } = errorFields(text);
  const field = (value: unknown) => (value === undefined || value === null ? "-" : String(value));
  return (
    `${label} answered ${status}\n` +
    `  code: ${field(error.code)}\n  message: ${field(error.message)}\n` +
    `  location: ${field(error.location)}\n  request_id: ${field(requestId)}`
  );
}

/** The server's wait hint in ms (`Retry-After` seconds, or the body's `retry_after_ms`), capped at `retry_after_cap_ms`. */
function retryAfter(response: Response, text: string): number {
  const header = response.headers.get("retry-after");
  const fromHeader = header !== null && /^[0-9]+$/.test(header.trim()) ? Number(header.trim()) * 1000 : 0;
  const body = errorFields(text).error.retry_after_ms;
  const fromBody = typeof body === "number" && Number.isFinite(body) && body > 0 ? body : 0;
  return Math.min(Math.max(fromHeader, fromBody), CLIENT_RETRY.retry_after_cap_ms);
}

interface Request {
  readonly method: "POST" | "PUT";
  readonly path: string;
  readonly body: string | Uint8Array;
  readonly contentType: string;
}

/**
 * Sends one request under the client retry policy: only 429, 503, and
 * network or timeout failures retry, at most `max_attempts` times, sleeping
 * full jitter or the server's larger hint. Any other non-2xx stops at once.
 */
async function send(io: IngestIo, endpoint: string, token: string, request: Request): Promise<{ status: number; body: unknown }> {
  const url = `${endpoint}${request.path}`;
  const label = `${request.method} ${request.path}`;
  let last = "";
  for (let attempt = 0; attempt < CLIENT_RETRY.max_attempts; attempt += 1) {
    let hint = 0;
    try {
      const response = await io.fetch(url, {
        method: request.method,
        headers: { authorization: `Bearer ${token}`, "content-type": request.contentType },
        body: request.body,
        signal: AbortSignal.timeout(CLIENT_RETRY.request_timeout_ms),
      });
      const text = await response.text();
      if (response.ok) {
        try {
          return { status: response.status, body: JSON.parse(text) as unknown };
        } catch {
          throw new CliError(1, `${label} answered ${response.status} with a body that is not JSON`);
        }
      }
      if (response.status !== 429 && response.status !== 503) throw new ServiceError(response.status, describeFailure(label, response.status, text));
      last = describeFailure(label, response.status, text);
      hint = retryAfter(response, text);
    } catch (error) {
      if (error instanceof CliError) throw error;
      last = `${label} failed: ${error instanceof Error ? error.message : "network error"}`;
    }
    if (attempt + 1 < CLIENT_RETRY.max_attempts) {
      const window = Math.min(CLIENT_RETRY.backoff_cap_ms, BACKOFF_BASE_MS * 2 ** attempt);
      await io.sleep(Math.max(io.random() * window, hint));
    }
  }
  throw new CliError(1, `gave up after ${CLIENT_RETRY.max_attempts} attempts; last: ${last}`);
}

function malformed(label: string): CliError {
  return new CliError(1, `${label} answered with an unexpected body`);
}

function isCount(value: unknown, max: number): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0 && (value as number) <= max;
}

function isIdList(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string" && OBSERVATION_ID.test(item));
}

function repoPath(state: ResumeState, rest: string): string {
  return `/v1/repos/${state.repository_id}${rest}`;
}

async function submitCollection(state: ResumeState, statePath: string, token: string, io: IngestIo): Promise<void> {
  const canonicalText = readFileSync(state.submission.file, "utf8");
  const body = `{"source_version":${state.submission.source_version},"collection":${canonicalText}}`;
  const label = `POST ${repoPath(state, "/collections")}`;
  const { body: result } = await send(io, state.endpoint, token, { method: "POST", path: repoPath(state, "/collections"), body, contentType: "application/json" });
  const r = result as Record<string, unknown>;
  if (!isIdList(r["accepted"]) || !isIdList(r["already_present"]) || !isCount(r["evidence_revision"], Number.MAX_SAFE_INTEGER)) throw malformed(label);
  if (r["collection_id"] !== state.submission.collection_id) {
    throw new CliError(1, `${label} acknowledged collection ${String(r["collection_id"])}, not the submitted ${state.submission.collection_id}`);
  }
  state.ingest = { state: "acknowledged", evidence_revision: r["evidence_revision"], accepted: r["accepted"], already_present: r["already_present"] };
  saveState(statePath, state, io);
}

function partCount(bytes: number): number {
  return Math.ceil(bytes / LIMITS.upload_part_bytes);
}

/** Reads `upload.file` again and refuses bytes that no longer match the reference. */
function uploadBytes(upload: UploadEntry): Uint8Array {
  let bytes: Uint8Array;
  try {
    bytes = new Uint8Array(readFileSync(upload.file));
  } catch (error) {
    throw new CliError(1, `could not read ${upload.file} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  if (bytes.byteLength !== upload.bytes || sha256Hex(bytes) !== upload.reference_digest) {
    throw new CliError(1, `${upload.file} changed since it was checked; it no longer matches its ${upload.reference_kind} reference`);
  }
  return bytes;
}

/** Opens (or reopens) the upload, then sends every part the server has not received. */
async function stage(state: ResumeState, statePath: string, upload: UploadEntry, token: string, io: IngestIo): Promise<void> {
  const bytes = uploadBytes(upload);
  const openLabel = `POST ${repoPath(state, "/uploads")}`;
  const { body: opened } = await send(io, state.endpoint, token, {
    method: "POST",
    path: repoPath(state, "/uploads"),
    body: JSON.stringify({
      observation_id: upload.observation_id,
      reference_kind: upload.reference_kind,
      reference_digest: upload.reference_digest,
      bytes: upload.bytes,
      path: upload.path,
    }),
    contentType: "application/json",
  });
  const o = opened as Record<string, unknown>;
  const parts = partCount(upload.bytes);
  if (typeof o["upload_id"] !== "string" || !UPLOAD_ID.test(o["upload_id"]) || !isCount(o["parts_received"], parts) || o["part_count"] !== parts) {
    throw malformed(openLabel);
  }
  if (o["state"] === "rejected" || o["state"] === "expired") {
    throw new CliError(1, `upload ${o["upload_id"]} for ${upload.file} is ${o["state"]}; the service will not accept its bytes`);
  }
  upload.upload_id = o["upload_id"];
  upload.parts_acknowledged = o["parts_received"];
  if (o["state"] === "finalized") {
    upload.state = "finalized";
    saveState(statePath, state, io);
    return;
  }
  saveState(statePath, state, io);
  for (let index = upload.parts_acknowledged; index < parts; index += 1) {
    const path = repoPath(state, `/uploads/${upload.upload_id}/parts/${index}`);
    const chunk = bytes.slice(index * LIMITS.upload_part_bytes, Math.min((index + 1) * LIMITS.upload_part_bytes, bytes.byteLength));
    let result: unknown;
    try {
      ({ body: result } = await send(io, state.endpoint, token, { method: "PUT", path, body: chunk, contentType: "application/octet-stream" }));
    } catch (error) {
      if (error instanceof ServiceError && error.status === 409) {
        throw new CliError(1, `the service already holds different bytes for part ${index} of ${upload.file}; the local file changed after it was staged\n${error.message}`);
      }
      throw error;
    }
    const p = result as Record<string, unknown>;
    if (!isCount(p["parts_received"], parts) || (p["parts_received"] as number) < index + 1) throw malformed(`PUT ${path}`);
    upload.parts_acknowledged = p["parts_received"] as number;
    saveState(statePath, state, io);
  }
  upload.state = "staged";
  saveState(statePath, state, io);
}

export interface IngestSummary {
  readonly collection_id: string;
  readonly source_version: 2 | 3;
  readonly accepted: number;
  readonly already_present: number;
  readonly staged: number;
  readonly finalized: number;
}

function summary(state: ResumeState): IngestSummary {
  return {
    collection_id: state.submission.collection_id,
    source_version: state.submission.source_version,
    accepted: state.ingest.accepted?.length ?? 0,
    already_present: state.ingest.already_present?.length ?? 0,
    staged: state.uploads.filter((upload) => upload.state === "staged").length,
    finalized: state.uploads.filter((upload) => upload.state === "finalized").length,
  };
}

/** Drives the state to every requested producer acknowledgment; any stop names the resume command. */
async function drive(state: ResumeState, statePath: string, token: string, io: IngestIo): Promise<IngestSummary> {
  try {
    if (state.ingest.state === "pending") await submitCollection(state, statePath, token, io);
    for (const upload of state.uploads) {
      if (upload.state === "pending") await stage(state, statePath, upload, token, io);
    }
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    throw new CliError(1, `${error.message}\nprogress is saved in ${statePath}; resume with: ${resumeCommand(statePath)}`);
  }
  const result = summary(state);
  io.stdout(
    `collection ${result.collection_id} (source version ${result.source_version}): ` +
      `${result.accepted} accepted, ${result.already_present} already present; evidence revision ${state.ingest.evidence_revision}\n`,
  );
  if (result.staged > 0) {
    io.stdout(`${result.staged} upload(s) staged; pending verifier finalization: ${finalizeCommand(statePath)}\n`);
  } else if (state.uploads.length === 0) {
    io.stdout("no uploads requested\n");
  }
  if (result.finalized > 0) io.stdout(`${result.finalized} upload(s) already finalized\n`);
  return result;
}

/** `ingest`: validate everything locally, persist the bundle and state, then submit and stage. */
export async function ingest(options: IngestOptions, io: IngestIo): Promise<IngestSummary> {
  const endpoint = checkEndpoint(options.endpoint);
  const repositoryId = checkRepo(options.repo);
  const tokenFile = resolve(options.tokenFile);
  const token = readToken(tokenFile);
  const statePath = resolve(options.state);
  if (existsSync(statePath)) throw new CliError(2, `${statePath} already exists; continue it with: ${resumeCommand(statePath)}`);
  const source = resolve(options.input);
  const inputBytes = readInput(source, "the input");
  const adapted = adapt(source, inputBytes, io);
  const body = `{"source_version":${adapted.sourceVersion},"collection":${adapted.canonical}}`;
  if (Buffer.byteLength(body) > LIMITS.request_json_bytes) {
    throw new CliError(2, `${source} is invalid at "": the submission exceeds the ${LIMITS.request_json_bytes}-byte request limit`);
  }
  const uploads = options.proofDir === undefined ? [] : planUploads(adapted.collection, resolve(options.proofDir));

  const retained = retainedPaths(statePath);
  atomicWrite(retained.input, inputBytes);
  atomicWrite(retained.collection, adapted.canonical);
  const state: ResumeState = {
    format_version: STATE_FORMAT_VERSION,
    endpoint,
    repository_id: repositoryId,
    token_file: tokenFile,
    input: { file: retained.input, source, sha256: sha256Hex(inputBytes) },
    submission: { file: retained.collection, source_version: adapted.sourceVersion, collection_id: adapted.collectionId },
    ingest: { state: "pending" },
    uploads,
    updated_at_ms: 0,
  };
  saveState(statePath, state, io);
  return drive(state, statePath, token, io);
}

// ---- Resume state --------------------------------------------------------

/** Validates a strictly loaded resume state field by field, each failure located by JSON pointer. */
class StateReader {
  constructor(private readonly path: string) {}

  fail(where: string, message: string): never {
    throw new CliError(2, `the resume state ${this.path} is invalid at "${where}": ${message}`);
  }

  object(value: JsonValue | undefined, where: string, required: readonly string[], optional: readonly string[] = []): JsonObject {
    if (!isJsonObject(value)) this.fail(where, `must be an object, not ${describe(value)}`);
    for (const key of objectKeys(value)) {
      if (!required.includes(key) && !optional.includes(key)) this.fail(`${where}/${key}`, "is not a known field");
    }
    for (const key of required) if (!(key in value)) this.fail(`${where}/${key}`, "is required");
    return value;
  }

  string(value: JsonValue | undefined, where: string, pattern?: RegExp): string {
    if (typeof value !== "string") this.fail(where, `must be a string, not ${describe(value)}`);
    if (value.length === 0) this.fail(where, "must not be empty");
    if (pattern !== undefined && !pattern.test(value)) this.fail(where, `does not match ${pattern.source}`);
    return value;
  }

  integer(value: JsonValue | undefined, where: string, max: number): number {
    if (!isJsonNumber(value) || !value.integer) this.fail(where, `must be an integer, not ${describe(value)}`);
    const number = Number(value.lexeme);
    if (!Number.isSafeInteger(number) || number < 0 || number > max) this.fail(where, `must be an integer from 0 to ${max}`);
    return number;
  }

  oneOf<T extends string | number>(value: T, where: string, allowed: readonly T[]): T {
    if (!allowed.includes(value)) this.fail(where, `must be one of ${allowed.join(", ")}`);
    return value;
  }

  ids(value: JsonValue | undefined, where: string): string[] {
    if (!Array.isArray(value)) this.fail(where, `must be an array, not ${describe(value)}`);
    return value.map((item, index) => this.string(item, `${where}/${index}`, OBSERVATION_ID));
  }
}

function describe(value: JsonValue | undefined): string {
  if (value === undefined) return "absent";
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  if (isJsonNumber(value)) return "a number";
  return typeof value === "object" ? "an object" : `a ${typeof value}`;
}

/** Loads `path` with the shared strict loader and validates every field of the resume state. */
export function loadState(path: string): ResumeState {
  const loaded = loadStrict(readInput(path, "the resume state"));
  if (!loaded.ok) throw located(path, loaded.problems);
  // Explicitly typed so `r.fail(...)` (returning `never`) narrows like a throw.
  const r: StateReader = new StateReader(path);
  const root = r.object(loaded.value, "", ["format_version", "endpoint", "repository_id", "token_file", "input", "submission", "ingest", "uploads", "updated_at_ms"], ["$schema"]);
  r.oneOf(r.integer(root["format_version"], "/format_version", Number.MAX_SAFE_INTEGER), "/format_version", [STATE_FORMAT_VERSION]);
  const endpointText = r.string(root["endpoint"], "/endpoint");
  let endpoint: string;
  try {
    endpoint = checkEndpoint(endpointText);
  } catch {
    r.fail("/endpoint", "must be an https origin, or an http loopback origin");
  }
  const input = r.object(root["input"], "/input", ["file", "source", "sha256"]);
  const submission = r.object(root["submission"], "/submission", ["file", "source_version", "collection_id"]);
  const ingestValue = r.object(root["ingest"], "/ingest", ["state"], ["evidence_revision", "accepted", "already_present"]);
  const ingestState = r.oneOf(r.string(ingestValue["state"], "/ingest/state") as ResumeState["ingest"]["state"], "/ingest/state", ["pending", "acknowledged"]);
  const ingest: ResumeState["ingest"] = { state: ingestState };
  if (ingestState === "acknowledged") {
    r.object(ingestValue, "/ingest", ["state", "evidence_revision", "accepted", "already_present"]);
    ingest.evidence_revision = r.integer(ingestValue["evidence_revision"], "/ingest/evidence_revision", Number.MAX_SAFE_INTEGER);
    ingest.accepted = r.ids(ingestValue["accepted"], "/ingest/accepted");
    ingest.already_present = r.ids(ingestValue["already_present"], "/ingest/already_present");
  } else {
    r.object(ingestValue, "/ingest", ["state"]);
  }
  const uploadsValue = root["uploads"];
  if (!Array.isArray(uploadsValue)) r.fail("/uploads", `must be an array, not ${describe(uploadsValue)}`);
  const uploads = uploadsValue.map((value, index): UploadEntry => {
    const where = `/uploads/${index}`;
    const u = r.object(value, where, ["observation_id", "reference_kind", "reference_digest", "path", "file", "bytes", "parts_acknowledged", "state"], ["upload_id"]);
    const bytes = r.integer(u["bytes"], `${where}/bytes`, LIMITS.upload_object_max_bytes);
    const entry: UploadEntry = {
      observation_id: r.string(u["observation_id"], `${where}/observation_id`, OBSERVATION_ID),
      reference_kind: r.oneOf(r.string(u["reference_kind"], `${where}/reference_kind`) as ReferenceKind, `${where}/reference_kind`, REFERENCE_KINDS),
      reference_digest: r.string(u["reference_digest"], `${where}/reference_digest`, SHA256),
      path: r.string(u["path"], `${where}/path`),
      file: r.string(u["file"], `${where}/file`),
      bytes,
      parts_acknowledged: r.integer(u["parts_acknowledged"], `${where}/parts_acknowledged`, partCount(bytes)),
      state: r.oneOf(r.string(u["state"], `${where}/state`) as UploadState, `${where}/state`, ["pending", "staged", "finalized"]),
    };
    if ("upload_id" in u) entry.upload_id = r.string(u["upload_id"], `${where}/upload_id`, UPLOAD_ID);
    return entry;
  });
  return {
    format_version: STATE_FORMAT_VERSION,
    endpoint,
    repository_id: r.string(root["repository_id"], "/repository_id", REPOSITORY_ID),
    token_file: r.string(root["token_file"], "/token_file"),
    input: {
      file: r.string(input["file"], "/input/file"),
      source: r.string(input["source"], "/input/source"),
      sha256: r.string(input["sha256"], "/input/sha256", SHA256),
    },
    submission: {
      file: r.string(submission["file"], "/submission/file"),
      source_version: r.oneOf(r.integer(submission["source_version"], "/submission/source_version", 3) as 2 | 3, "/submission/source_version", [2, 3]),
      collection_id: r.string(submission["collection_id"], "/submission/collection_id", SHA256),
    },
    ingest,
    uploads,
    updated_at_ms: r.integer(root["updated_at_ms"], "/updated_at_ms", Number.MAX_SAFE_INTEGER),
  };
}

/** Refuses, before any request, retained files or upload sources that no longer hash to what the state recorded. */
function verifyRetained(state: ResumeState, statePath: string): void {
  if (sha256Hex(readInput(state.input.file, "the retained input")) !== state.input.sha256) {
    throw new CliError(2, `the retained input ${state.input.file} no longer matches the sha256 recorded in ${statePath}`);
  }
  if (sha256Hex(readInput(state.submission.file, "the retained collection")) !== state.submission.collection_id) {
    throw new CliError(2, `the retained collection ${state.submission.file} no longer matches collection ${state.submission.collection_id} recorded in ${statePath}`);
  }
  state.uploads.forEach((upload, index) => {
    if (upload.state === "pending") checkFile(upload.file, upload.reference_digest, upload.bytes, `"/uploads/${index}" in ${statePath}`);
  });
}

/** `resume`: continue an interrupted ingestion from its state; endpoint, repository, and token path come from the state. */
export async function resume(statePathArgument: string, io: IngestIo): Promise<IngestSummary> {
  const statePath = resolve(statePathArgument);
  const state = loadState(statePath);
  verifyRetained(state, statePath);
  const token = readToken(state.token_file);
  return drive(state, statePath, token, io);
}

/** `finalize`: with a verifier credential, finalize every staged upload in the state. */
export async function finalize(statePathArgument: string, tokenFileArgument: string, io: IngestIo): Promise<IngestSummary> {
  const statePath = resolve(statePathArgument);
  const state = loadState(statePath);
  const tokenFile = resolve(tokenFileArgument);
  const token = readToken(tokenFile);
  const pending = state.uploads.filter((upload) => upload.state === "pending").length;
  if (state.ingest.state !== "acknowledged" || pending > 0) {
    throw new CliError(2, `${statePath} has unfinished producer work (${pending} upload(s) pending); first run: ${resumeCommand(statePath)}`);
  }
  try {
    for (const upload of state.uploads) {
      if (upload.state !== "staged") continue;
      const path = repoPath(state, `/uploads/${upload.upload_id}/finalize`);
      const { body } = await send(io, state.endpoint, token, { method: "POST", path, body: "{}", contentType: "application/json" });
      const result = body as { upload?: { state?: unknown } };
      if (result.upload?.state !== "finalized") throw malformed(`POST ${path}`);
      upload.state = "finalized";
      saveState(statePath, state, io);
    }
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    throw new CliError(1, `${error.message}\nprogress is saved in ${statePath}; retry with: ${finalizeCommand(statePath, tokenFile)}`);
  }
  const result = summary(state);
  io.stdout(`collection ${result.collection_id}: ${result.finalized} upload(s) finalized\n`);
  return result;
}

// ---- Command line --------------------------------------------------------

const USAGE = `usage:
  ingest   INPUT --endpoint URL --repo ID --token-file PATH --state PATH [--proof-dir PATH]
  resume   STATE
  finalize STATE --token-file PATH
`;

function parse(argv: readonly string[], names: readonly string[], required: readonly string[]): { values: Record<string, string | undefined>; positional: string } {
  let parsed;
  try {
    parsed = parseArgs({
      args: [...argv],
      options: Object.fromEntries(names.map((name) => [name, { type: "string" as const }])),
      strict: true,
      allowPositionals: true,
    });
  } catch (error) {
    throw new CliError(2, (error as Error).message);
  }
  if (parsed.positionals.length !== 1) throw new CliError(2, `expected exactly one path argument, got ${parsed.positionals.length}`);
  const values = parsed.values as Record<string, string | undefined>;
  const absent = required.filter((name) => values[name] === undefined);
  if (absent.length > 0) throw new CliError(2, `missing ${absent.map((name) => `--${name}`).join(", ")}`);
  return { values, positional: parsed.positionals[0]! };
}

/** Runs one command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: IngestIo = defaultIo()): Promise<number> {
  const [command, ...rest] = argv;
  try {
    switch (command) {
      case "ingest": {
        const { values: v, positional } = parse(rest, ["endpoint", "repo", "token-file", "state", "proof-dir"], ["endpoint", "repo", "token-file", "state"]);
        const options: IngestOptions = { input: positional, endpoint: v["endpoint"]!, repo: v["repo"]!, tokenFile: v["token-file"]!, state: v["state"]! };
        await ingest(v["proof-dir"] === undefined ? options : { ...options, proofDir: v["proof-dir"] }, io);
        return 0;
      }
      case "resume": {
        const { positional } = parse(rest, [], []);
        await resume(positional, io);
        return 0;
      }
      case "finalize": {
        const { values: v, positional } = parse(rest, ["token-file"], ["token-file"]);
        await finalize(positional, v["token-file"]!, io);
        return 0;
      }
      default:
        io.stderr(USAGE);
        return 2;
    }
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`ingest: ${error.message}\n`);
    if (error.exitCode === 2 && /^(missing --|expected exactly|Unknown option|Option )/.test(error.message)) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
