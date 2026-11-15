//! Turn one hosted CI cell's retained artifacts into an ingestible collection.
//!
//! ```text
//! hosted-collection --observation DIR --junit DIR [--completion DIR] --tree SHA --out DIR
//! ```
//!
//! The nightly retains each finished cell as three artifacts (the conventions
//! are `scripts/ci/hosted_observations.py`'s): `observation-<cell>` holds an
//! envelope `{schema_version, environment, head, observation}`,
//! `junit-<cell>` the reports and `expected-<gate>.json`, and
//! `completion-<cell>` the `completion.json` a passing cell's proof cites.
//! This writes `OUT/collection.json`, a version-3 collection of that one
//! observation with its original ID, and `OUT/files/`, laid out the way
//! `evidence-ingest --proof-dir` reads it. The envelope does not carry the
//! head's tree, so the caller supplies it (`git rev-parse <head>^{tree}`).
//!
//! Nothing here checks file digests: ingestion does, before anything is sent.
//!
//! Exit codes: 0 written; 1 a file could not be read or written; 2 usage
//! error or an envelope or artifact that cannot form a valid collection.

import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { canonical, COLLECTION_SCHEMA_VERSION, isJsonNumber, isJsonObject, loadStrict, parseCollection, PROOF_COMPLETION_FILE, PROOF_EXPECTED_FILE, type JsonObject, type JsonValue } from "../src/contracts/index.ts";
import { CliError, defaultIo, type OperatorIo } from "./operator.ts";

const ENVELOPE_FILE = "observation.json";
const COMPLETION_FILE = "completion.json";
const ENVELOPE_SCHEMA_VERSION = "1";
const SHA1 = /^[0-9a-f]{40}$/;

export interface HostedInputs {
  readonly observation: string;
  readonly junit: string;
  readonly completion?: string;
  readonly tree: string;
  readonly out: string;
}

/** What was written, for the summary and for tests. */
export interface HostedCollection {
  readonly collection: string;
  readonly collection_id: string;
  readonly observation_id: string;
  readonly files: readonly string[];
}

function read(path: string): Buffer {
  try {
    return readFileSync(path);
  } catch (error) {
    throw new CliError(1, `could not read ${path} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
}

/** `root/relative`, refusing a member path that would leave `root`. */
function inside(root: string, relative: string): string {
  const target = resolve(root, relative);
  if (!target.startsWith(resolve(root) + sep)) throw new CliError(2, `the artifact path ${relative} leaves its directory`);
  return target;
}

/** The version as the strict loader represents a number, which `canonical` relies on. */
function collectionVersion(): JsonValue {
  const loaded = loadStrict(new TextEncoder().encode(String(COLLECTION_SCHEMA_VERSION)));
  if (!loaded.ok) throw new Error("the collection schema version is not a JSON number");
  return loaded.value;
}

export function assembleHostedCollection(inputs: HostedInputs): HostedCollection {
  if (!SHA1.test(inputs.tree)) throw new CliError(2, "--tree must be a 40-character lowercase commit tree ID");
  const loaded = loadStrict(read(join(inputs.observation, ENVELOPE_FILE)));
  if (!loaded.ok) throw new CliError(2, `${ENVELOPE_FILE} is not strict JSON: ${loaded.problems[0]?.message ?? "unreadable"}`);
  const envelope = loaded.value;
  const version = isJsonObject(envelope) ? envelope["schema_version"] : undefined;
  if (!isJsonObject(envelope) || !isJsonNumber(version) || version.lexeme !== ENVELOPE_SCHEMA_VERSION) {
    throw new CliError(2, `${ENVELOPE_FILE} is not a version-${ENVELOPE_SCHEMA_VERSION} hosted observation envelope`);
  }
  const observation = envelope["observation"];
  if (typeof envelope["environment"] !== "string" || typeof envelope["head"] !== "string" || !isJsonObject(observation)) {
    throw new CliError(2, `${ENVELOPE_FILE} lacks its environment, head, or observation`);
  }

  const collection: JsonObject = {
    schema_version: collectionVersion(),
    environment: envelope["environment"],
    head: envelope["head"],
    tree: inputs.tree,
    observations: [observation],
  };
  const text = canonical(collection);
  const parsed = parseCollection(new TextEncoder().encode(text));
  if (!parsed.ok) {
    const problem = parsed.problems[0];
    throw new CliError(2, `the cell does not form a valid collection: ${problem?.location ?? ""} ${problem?.message ?? ""}`.trim());
  }

  const files = join(inputs.out, "files");
  const copies: [string, string][] = [];
  const cell = observation["cell"] as JsonObject;
  const name = (template: string) => template.replace("{package}", cell["package"] as string).replace("{gate}", cell["gate"] as string);
  for (const report of observation["reports"] as JsonObject[]) {
    const path = report["path"] as string;
    copies.push([inside(inputs.junit, path), inside(files, path)]);
  }
  const proof = observation["proof"];
  if (isJsonObject(proof)) {
    copies.push([join(inputs.junit, `expected-${cell["gate"] as string}.json`), join(files, name(PROOF_EXPECTED_FILE))]);
    if (typeof proof["completion_record_digest"] === "string") {
      if (inputs.completion === undefined) throw new CliError(2, "the observation's proof cites a completion record, so --completion is required");
      copies.push([join(inputs.completion, COMPLETION_FILE), join(files, name(PROOF_COMPLETION_FILE))]);
    }
  }
  for (const [from] of copies) if (!existsSync(from)) throw new CliError(2, `the artifacts lack ${from}, which the observation cites`);

  try {
    for (const [from, to] of copies) {
      mkdirSync(dirname(to), { recursive: true });
      copyFileSync(from, to);
    }
    writeFileSync(join(inputs.out, "collection.json"), text);
  } catch (error) {
    throw new CliError(1, `could not write under ${inputs.out} (${(error as NodeJS.ErrnoException).code ?? "error"})`);
  }
  return {
    collection: join(inputs.out, "collection.json"),
    collection_id: parsed.value.collection_id,
    observation_id: observation["observation_id"] as string,
    files: copies.map(([, to]) => to),
  };
}

const USAGE = "usage:\n  hosted-collection --observation DIR --junit DIR [--completion DIR] --tree SHA --out DIR\n";

/** Runs the command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: OperatorIo = defaultIo()): Promise<number> {
  try {
    let v: Record<string, string | undefined>;
    try {
      v = parseArgs({
        args: [...argv],
        options: Object.fromEntries(["observation", "junit", "completion", "tree", "out"].map((flag) => [flag, { type: "string" as const }])),
        strict: true,
        allowPositionals: false,
      }).values as Record<string, string | undefined>;
    } catch (error) {
      throw new CliError(2, (error as Error).message);
    }
    const absent = ["observation", "junit", "tree", "out"].filter((flag) => v[flag] === undefined);
    if (absent.length > 0) throw new CliError(2, `missing ${absent.map((flag) => `--${flag}`).join(", ")}`);
    const result = assembleHostedCollection({
      observation: v["observation"]!,
      junit: v["junit"]!,
      ...(v["completion"] === undefined ? {} : { completion: v["completion"] }),
      tree: v["tree"]!,
      out: v["out"]!,
    });
    io.stdout(`${JSON.stringify(result, null, 2)}\n`);
    return 0;
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`hosted-collection: ${error.message}\n`);
    if (error.exitCode === 2 && /^(missing --|Unknown option|Option )/.test(error.message)) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
