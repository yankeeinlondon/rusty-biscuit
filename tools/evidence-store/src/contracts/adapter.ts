//! The version-2 receipt to version-3 collection adapter: the default-hook path
//! of `observation_notes.receipt_observations` followed by `collection_of`.
//!
//! Pure: report bytes come from the injected `ReportReader`, so the Worker
//! never opens a path; the submitting host supplies `nodeReportReader`.
//! Unlike the Python, which raises with the first problem, a rejection lists
//! every receipt problem (or every unreadable report), and nothing is produced.

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex } from "@noble/hashes/utils.js";
import { canonical, collectionIdentity, compareCodePoints, observationIdentity } from "./canonical.ts";
import { COLLECTION_SCHEMA_VERSION } from "./evidence.ts";
import {
  childPointer,
  isJsonObject,
  jsonNumber,
  jsonObject,
  objectKeys,
  type ContractProblem,
  type ContractResult,
  type JsonObject,
  type JsonValue,
} from "./json.ts";
import { validateCollection, validateReceipt } from "./validate.ts";

/** Why a report could not be read. `directory` is `report_dir` as the reader expanded it. */
export interface ReportReadFailure {
  readonly kind: "report-read-failure";
  readonly directory: string;
  readonly message: string;
}

/**
 * Reads `reportPath` relative to the receipt's unexpanded `report_dir`, the
 * way Python's `Path(report_dir).expanduser() / report_path` names it.
 */
export type ReportReader = (reportDir: string, reportPath: string) => Uint8Array | ReportReadFailure;

export interface AdaptedReceipt {
  readonly source_version: 2;
  readonly collection: JsonObject;
  readonly canonical: string;
  readonly collection_id: string;
  readonly observation_ids: readonly string[];
}

function isFailure(value: Uint8Array | ReportReadFailure): value is ReportReadFailure {
  return !(value instanceof Uint8Array);
}

/** A collection pointer re-expressed against the receipt value the observation copied (its cell otherwise). */
function toReceiptPointer(pointer: string, origins: readonly number[]): string {
  if (pointer === "/observations") return "/cells";
  const match = /^\/observations\/(\d+)(\/.*)?$/.exec(pointer);
  const origin = match === null ? undefined : origins[Number(match[1])];
  if (match === null || origin === undefined) return "";
  const cell = childPointer("/cells", origin);
  const rest = match[2] ?? "";
  const within = (prefix: string): boolean => rest === prefix || rest.startsWith(`${prefix}/`);
  if (within("/cell")) return cell + rest.slice("/cell".length);
  if (within("/producer/host")) return `/host${rest.slice("/producer/host".length)}`;
  for (const copied of ["/base", "/scope_identity", "/completion"]) if (within(copied)) return copied;
  return cell;
}

export function adaptReceipt(receipt: JsonValue, readReport: ReportReader): ContractResult<AdaptedReceipt> {
  const invalid = validateReceipt(receipt);
  if (invalid.length > 0) return { ok: false, problems: invalid };
  const document = receipt as JsonObject;
  const host = document["host"] as JsonObject;
  const reportDir = host["report_dir"];
  if (typeof reportDir !== "string") {
    // `validate_receipt` never types host values; Python's `Path()` then raises TypeError.
    return {
      ok: false,
      problems: [
        {
          code: "malformed-receipt",
          location: "/host/report_dir",
          message: "malformed-receipt: receipt host report_dir must be a string to locate its reports",
        },
      ],
    };
  }

  const cells = document["cells"] as JsonObject[];
  const unreadable: ContractProblem[] = [];
  const built: { observation: JsonObject; origin: number }[] = [];
  cells.forEach((cell, index) => {
    const reports: JsonValue[] = [];
    if (cell["completion"] === "complete") {
      const path = cell["report"] as string;
      const content = readReport(reportDir, path);
      if (isFailure(content)) {
        unreadable.push({
          code: "report-unavailable",
          location: childPointer(childPointer("/cells", index), "report"),
          message: `${content.directory} does not retain ${path}: ${content.message}`,
        });
        return;
      }
      reports.push(
        jsonObject([
          ["path", path],
          ["sha256", bytesToHex(sha256(content))],
          ["bytes", jsonNumber(String(content.byteLength))],
          ["store", "host-dir"],
        ]),
      );
    }
    const body = jsonObject([
      [
        "producer",
        jsonObject([
          ["kind", "hook"],
          ["host", jsonObject(objectKeys(host).map((key) => [key, host[key] as JsonValue]))],
        ]),
      ],
      ["base", document["base"] as JsonValue],
      ["scope_identity", document["scope_identity"] as JsonValue],
      ["completion", document["completion"] as JsonValue],
      ["cell", cell],
      ["reports", reports],
    ]);
    const observation = jsonObject([
      ["observation_id", observationIdentity(body)],
      ...objectKeys(body).map((key): [string, JsonValue] => [key, body[key] as JsonValue]),
    ]);
    built.push({ observation, origin: index });
  });
  if (unreadable.length > 0) return { ok: false, problems: unreadable };

  built.sort((left, right) =>
    compareCodePoints(left.observation["observation_id"] as string, right.observation["observation_id"] as string),
  );
  const collection = jsonObject([
    ["schema_version", jsonNumber(String(COLLECTION_SCHEMA_VERSION))],
    ["environment", document["environment"] as JsonValue],
    ["head", document["head"] as JsonValue],
    ["tree", document["tree"] as JsonValue],
    ["observations", built.map(({ observation }) => observation)],
  ]);
  const problems = validateCollection(collection);
  if (problems.length > 0) {
    const origins = built.map(({ origin }) => origin);
    return {
      ok: false,
      problems: problems.map((found) => ({ ...found, location: toReceiptPointer(found.location, origins) })),
    };
  }
  const text = canonical(collection);
  return {
    ok: true,
    value: {
      source_version: 2,
      collection,
      canonical: text,
      collection_id: collectionIdentity(collection),
      observation_ids: built.map(({ observation }) => observation["observation_id"] as string),
    },
  };
}
