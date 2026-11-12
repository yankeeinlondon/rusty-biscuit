// Drives every frozen Python answer in `tests/fixtures/parity/manifest.json`
// through the TypeScript port. Departures from the Python are asserted as
// departures, by name, so a fixture change cannot silently flip them.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  adaptReceipt,
  canonical,
  identity,
  loadStrict,
  parseCollection,
  validateCollection,
  validateReceipt,
  type ContractProblem,
  type JsonValue,
} from "../../src/contracts/index.ts";
import { createNodeReportReader } from "../../src/contracts/node-report-reader.ts";

type CaseKind = "canonical" | "collection" | "receipt-adapter" | "loader" | "validator";

interface ParityCase {
  readonly name: string;
  readonly kind: CaseKind;
  readonly input: string;
  readonly canonical?: string;
  readonly identity?: string;
  readonly observation_ids?: readonly string[];
  readonly problems?: readonly string[] | null;
  readonly codes?: readonly string[] | null;
  readonly valid?: boolean;
  readonly exception?: string;
  readonly accepts?: boolean;
  readonly stage?: "decode" | "parse";
  readonly error?: string;
  readonly canonical_text?: string;
  readonly target?: "collection" | "receipt";
  readonly home?: string;
  readonly receipt_problems?: readonly string[];
}

const DIRECTORY = new URL("../fixtures/parity/", import.meta.url);
const MANIFEST = JSON.parse(readFileSync(new URL("manifest.json", DIRECTORY), "utf-8")) as { cases: ParityCase[] };

/** Cases whose Python answer the port departs from on purpose. */
const DEPARTURES = new Set(["float-overflow", "v-counts-part-string", "v-counts-part-null"]);
/** The count part each crashing validator case makes non-numeric. */
const NON_NUMERIC_PART: Readonly<Record<string, string>> = {
  "v-counts-part-string": "passed",
  "v-counts-part-null": "skipped",
};

const fixture = (name: string): Uint8Array => new Uint8Array(readFileSync(new URL(name, DIRECTORY)));
const text = (name: string): string => readFileSync(new URL(name, DIRECTORY), "utf-8");
const ofKind = (kind: CaseKind): ParityCase[] => MANIFEST.cases.filter((entry) => entry.kind === kind);

function loaded(name: string): JsonValue {
  const result = loadStrict(fixture(name));
  if (!result.ok) throw new Error(`${name}: ${JSON.stringify(result.problems)}`);
  return result.value;
}

function messages(problems: readonly ContractProblem[]): string[] {
  return problems.map((found) => found.message);
}

/** The loader code each recorded Python failure maps to. */
function expectedLoaderCode(entry: ParityCase): string {
  const error = entry.error ?? "";
  if (entry.stage === "decode") return "invalid-utf8";
  if (error.startsWith("Unexpected UTF-8 BOM")) return "byte-order-mark";
  if (error.startsWith("duplicate key")) return "duplicate-key";
  if (error.endsWith("is not a JSON number")) return "nonfinite-number";
  if (error.startsWith("Extra data")) return "trailing-content";
  if (error.startsWith("Expecting value") && /^[ \t\r\n]*$/.test(text(entry.input))) return "empty-document";
  return "syntax";
}

const exercised = new Map<CaseKind, number>();
const count = (kind: CaseKind): void => {
  exercised.set(kind, (exercised.get(kind) ?? 0) + 1);
};

describe("contract port parity", () => {
  describe("canonical", () => {
    for (const entry of ofKind("canonical")) {
      count("canonical");
      if (entry.name === "float-overflow") {
        it("float-overflow: DEPARTURE, the loader refuses 1e400 instead of canonicalizing it to Infinity", () => {
          const result = loadStrict(fixture(entry.input));
          expect(result.ok).toBe(false);
          if (result.ok) return;
          expect(result.problems).toEqual([expect.objectContaining({ code: "nonfinite-number", location: "/a" })]);
          // The Python answer stays recorded: canonical itself is still total.
          expect(text(entry.canonical ?? "")).toBe('{"a":Infinity,"b":-Infinity}');
        });
        continue;
      }
      it(`${entry.name}: canonical bytes and identity`, () => {
        const output = canonical(loaded(entry.input));
        expect(output).toBe(text(entry.canonical ?? ""));
        expect(identity(output)).toBe(entry.identity);
      });
    }
  });

  describe("loader", () => {
    for (const entry of ofKind("loader")) {
      count("loader");
      it(`${entry.name}: ${entry.accepts === true ? "accepts" : "rejects"}`, () => {
        const result = loadStrict(fixture(entry.input));
        if (entry.accepts === true) {
          expect(result.ok, JSON.stringify(result)).toBe(true);
          if (result.ok) expect(canonical(result.value)).toBe(entry.canonical_text);
          return;
        }
        expect(result.ok).toBe(false);
        if (result.ok) return;
        expect(result.problems).toHaveLength(1);
        expect(result.problems[0]?.code).toBe(expectedLoaderCode(entry));
      });
    }
  });

  describe("collection", () => {
    for (const entry of ofKind("collection")) {
      count("collection");
      it(`${entry.name}: canonical, identity, and observation ids`, () => {
        const result = parseCollection(fixture(entry.input));
        expect(result.ok, JSON.stringify(result)).toBe(true);
        if (!result.ok) return;
        expect(result.value.canonical).toBe(text(entry.canonical ?? ""));
        expect(result.value.collection_id).toBe(entry.identity);
        expect(result.value.observation_ids).toEqual(entry.observation_ids);
        expect(messages(validateCollection(result.value.collection))).toEqual(entry.problems);
      });
    }
  });

  describe("receipt-adapter", () => {
    for (const entry of ofKind("receipt-adapter")) {
      count("receipt-adapter");
      it(`${entry.name}: adapts as observation_notes does`, () => {
        const home = fileURLToPath(new URL(entry.home ?? "", DIRECTORY)).replace(/[\\/]$/, "");
        const receipt = loaded(entry.input);
        expect(messages(validateReceipt(receipt))).toEqual(entry.receipt_problems);
        const result = adaptReceipt(receipt, createNodeReportReader({ home: () => home }));
        if (entry.error === undefined) {
          expect(result.ok, JSON.stringify(result)).toBe(true);
          if (!result.ok) return;
          expect(result.value.source_version).toBe(2);
          expect(result.value.canonical).toBe(text(entry.canonical ?? ""));
          expect(result.value.collection_id).toBe(entry.identity);
          expect(result.value.observation_ids).toEqual(entry.observation_ids);
          return;
        }
        expect(result.ok).toBe(false);
        if (result.ok) return;
        expect("value" in result).toBe(false);
        const first = result.problems[0];
        expect(first).toBeDefined();
        if (first?.code === "report-unavailable") {
          // OS error text differs by platform; the sentence before it is the contract.
          expect(first.location).toBe("/cells/0/report");
          const rendered = first.message.startsWith(home) ? `$HOME${first.message.slice(home.length)}` : first.message;
          expect(rendered.split(": ")[0]).toBe(entry.error.split(": ")[0]);
          return;
        }
        const prefix = (entry.receipt_problems ?? []).length > 0 ? "the receipt is invalid" : "the collection would be invalid";
        expect(`${prefix}: ${first?.message ?? ""}`).toBe(entry.error);
      });
    }
  });

  describe("validator", () => {
    for (const entry of ofKind("validator")) {
      count("validator");
      const validate = entry.target === "receipt" ? validateReceipt : validateCollection;
      if (entry.exception !== undefined) {
        it(`${entry.name}: DEPARTURE, Python raises ${entry.exception.split(":")[0]}; the port reports a located problem`, () => {
          expect(DEPARTURES.has(entry.name)).toBe(true);
          const part = NON_NUMERIC_PART[entry.name];
          expect(part).toBeDefined();
          const problems = validate(loaded(entry.input));
          expect(problems).toEqual([
            {
              code: "malformed-receipt",
              location: `/observations/0/cell/counts/${part}`,
              message: `malformed-receipt: receipt cell claudine/macos-latest/L1 counts.${part} must be a number`,
            },
          ]);
        });
        continue;
      }
      it(`${entry.name}: problems and codes`, () => {
        const problems = validate(loaded(entry.input));
        expect(messages(problems)).toEqual(entry.problems);
        expect(problems.map((found) => found.code)).toEqual(entry.codes);
        for (const found of problems) expect(found.location).toMatch(/^(?:|\/.*)$/);
      });
    }
  });

  it("exercises every manifest case exactly once per kind", () => {
    const expected = new Map<CaseKind, number>();
    for (const entry of MANIFEST.cases) expected.set(entry.kind, (expected.get(entry.kind) ?? 0) + 1);
    expect(exercised).toEqual(expected);
    expect([...exercised.values()].reduce((sum, value) => sum + value, 0)).toBe(MANIFEST.cases.length);
  });

  it("names only departures that exist in the manifest", () => {
    const names = new Set(MANIFEST.cases.map((entry) => entry.name));
    for (const name of DEPARTURES) expect(names.has(name), name).toBe(true);
  });
});
