// Input robustness of the contract port through its public entry points.
// Every mutation starts from a frozen authentic parity fixture and makes one
// edit; observation ids are re-derived after the edit (unless the id itself is
// the subject) so the only problem left is the one the case is about.

import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { afterAll, describe, expect, it } from "vitest";
import {
  adaptReceipt,
  canonical,
  jsonNumber,
  jsonObject,
  loadStrict,
  MAX_DEPTH,
  observationIdentity,
  parseCollection,
  parseReceipt,
  type ContractResult,
  type JsonObject,
  type JsonValue,
  type ReportReader,
} from "../../src/contracts/index.ts";
import { createNodeReportReader, nodeReportReader } from "../../src/contracts/node-report-reader.ts";

const DIRECTORY = new URL("../fixtures/parity/", import.meta.url);
const encode = (text: string): Uint8Array => new TextEncoder().encode(text);
const n = jsonNumber;

function fixture(name: string): JsonObject {
  const result = loadStrict(new Uint8Array(readFileSync(new URL(name, DIRECTORY))));
  if (!result.ok) throw new Error(name);
  return result.value as JsonObject;
}

function walk(document: JsonValue, path: readonly (string | number)[]): JsonObject {
  let cursor: JsonValue | undefined = document;
  for (const step of path) {
    cursor = Array.isArray(cursor) ? cursor[step as number] : (cursor as JsonObject)[step as string];
  }
  return cursor as JsonObject;
}

/** Re-derive every well-formed observation's id and restore id order. */
function rehash(document: JsonObject): void {
  const observations = document["observations"];
  if (!Array.isArray(observations)) return;
  for (const observation of observations) {
    const object = observation as JsonObject;
    object["observation_id"] = observationIdentity(object);
  }
  observations.sort((left, right) =>
    ((left as JsonObject)["observation_id"] as string) < ((right as JsonObject)["observation_id"] as string) ? -1 : 1,
  );
}

type Edit = (document: JsonObject) => void;
const set =
  (path: readonly (string | number)[], key: string, value: JsonValue): Edit =>
  (document) => {
    walk(document, path)[key] = value;
  };
const drop =
  (path: readonly (string | number)[], key: string): Edit =>
  (document) => {
    delete walk(document, path)[key];
  };

function collection(base: string, edit: Edit, options: { rehash?: boolean } = {}): ContractResult<unknown> {
  const document = fixture(base);
  edit(document);
  if (options.rehash !== false) rehash(document);
  return parseCollection(encode(canonical(document)));
}

function receipt(edit: Edit): ContractResult<unknown> {
  const document = fixture("v-receipt-control.json");
  edit(document);
  return parseReceipt(encode(canonical(document)));
}

function located(result: ContractResult<unknown>): [string, string][] {
  if (result.ok) return [];
  return result.problems.map((found) => [found.code, found.location]);
}

const HOOK = "v-control-hook.json";
const OBS = ["observations", 0] as const;
const CELL = [...OBS, "cell"] as const;
const COUNTS = [...CELL, "counts"] as const;
const HOST = [...OBS, "producer", "host"] as const;
const PO = "/observations/0";
const PC = `${PO}/cell`;

describe("shared loader: document shapes", () => {
  const rejects: [string, string | Uint8Array, string, string][] = [
    ["duplicate key", '{"a":{"b":1,"b":1}}', "duplicate-key", "/a/b"],
    ["trailing garbage", '{"a":1} x', "trailing-content", ""],
    ["concatenated documents", '{"a":1}{"a":2}', "trailing-content", ""],
    ["JSON Lines", '{"a":1}\n{"a":2}\n', "trailing-content", ""],
    ["NaN inside an array", '{"a":[1,NaN]}', "nonfinite-number", "/a/1"],
    ["-Infinity", "-Infinity", "nonfinite-number", ""],
    ["float overflow 1e400 (departure: Python yields inf)", '{"a":[1e400]}', "nonfinite-number", "/a/0"],
    ["integral float overflow", `{"a":1${"0".repeat(400)}.0}`, "nonfinite-number", "/a"],
    ["invalid UTF-8", new Uint8Array([0x7b, 0x22, 0xff, 0x22, 0x3a, 0x31, 0x7d]), "invalid-utf8", ""],
    ["byte order mark", new Uint8Array([0xef, 0xbb, 0xbf, 0x31]), "byte-order-mark", ""],
    ["blank", "", "empty-document", ""],
    ["whitespace only", " \n\t\r", "empty-document", ""],
    ["JSON5 comment", '{"a":1 // c\n}', "syntax", ""],
    ["JSON5 single quotes", "{'a':1}", "syntax", ""],
    ["YAML", "a: 1", "syntax", ""],
    ["raw control character", '{"a":"x\ty"}', "syntax", "/a"],
    ["invalid escape", '{"a":"\\x"}', "syntax", "/a"],
    ["short unicode escape", '{"a":"\\u12"}', "syntax", "/a"],
    ["leading zero", '{"a":01}', "syntax", ""],
    ["missing value", '{"a":}', "syntax", "/a"],
    ["integer past Python's 4300-digit limit", `[${"1".repeat(4301)}]`, "syntax", "/0"],
    ["nesting past MAX_DEPTH", `${"[".repeat(MAX_DEPTH + 1)}${"]".repeat(MAX_DEPTH + 1)}`, "syntax", `/${"0/".repeat(MAX_DEPTH - 1)}0`],
  ];
  it.each(rejects)("rejects %s", (_name, input, code, location) => {
    const result = loadStrict(typeof input === "string" ? encode(input) : input);
    expect(located(result)).toEqual([[code, location]]);
  });

  it("reports an error inside a later member before a duplicate key, as Python's pairs hook does", () => {
    expect(located(loadStrict(encode('{"a":1,"a":NaN}')))).toEqual([["nonfinite-number", "/a"]]);
  });

  it("escapes JSON pointer tokens", () => {
    expect(located(loadStrict(encode('{"a/b~":{"x":1,"x":2}}')))).toEqual([["duplicate-key", "/a~1b~0/x"]]);
  });

  it("accepts integers up to 4300 digits, underflow as -0.0, and nesting at MAX_DEPTH", () => {
    const digits = "9".repeat(4300);
    const big = loadStrict(encode(`[${digits}]`));
    expect(big.ok && canonical(big.value)).toBe(`[${digits}]`);
    const tiny = loadStrict(encode("-1e-400"));
    expect(tiny.ok && canonical(tiny.value)).toBe("-0.0");
    expect(loadStrict(encode(`${"[".repeat(MAX_DEPTH)}${"]".repeat(MAX_DEPTH)}`)).ok).toBe(true);
  });

  it("keeps __proto__ as an ordinary key and never mistakes an object for a number", () => {
    const result = loadStrict(encode('{"__proto__":{"kind":"number","lexeme":"1","integer":true}}'));
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(Object.keys(result.value as JsonObject)).toEqual(["__proto__"]);
    expect(canonical(result.value)).toBe('{"__proto__":{"integer":true,"kind":"number","lexeme":"1"}}');
  });

  it("wires the loader into parseCollection and parseReceipt", () => {
    expect(located(parseCollection(encode('{"schema_version":3,"schema_version":3}')))).toEqual([
      ["duplicate-key", "/schema_version"],
    ]);
    expect(located(parseReceipt(encode('{"schema_version":2} []')))).toEqual([["trailing-content", ""]]);
  });
});

describe("strict object/version", () => {
  it.each([
    ["string document", '"x"'],
    ["number document", "3"],
  ])("rejects a %s as a collection", (_name, text) => {
    expect(located(parseCollection(encode(text)))).toEqual([["malformed-receipt", ""]]);
  });

  it.each([
    ["3.0", n("3.0")],
    ["3e0 (a float, not the integer 3)", n("3e0")],
    ["30e-1", n("30e-1")],
    ["true", true],
    ['"3"', "3"],
    ["2", n("2")],
  ])("collection schema_version %s is an unknown version", (_name, value) => {
    expect(located(collection(HOOK, set([], "schema_version", value), { rehash: false }))).toEqual([
      ["unknown-schema-version", "/schema_version"],
    ]);
  });

  it.each([
    ["2.0", n("2.0"), "unknown-schema-version"],
    ["2e0", n("2e0"), "unknown-schema-version"],
    ["true", true, "unknown-schema-version"],
    ["1", n("1"), "v1-requires-exact-tree"],
  ])("receipt schema_version %s", (_name, value, code) => {
    expect(located(receipt(set([], "schema_version", value)))).toEqual([[code, "/schema_version"]]);
  });

  it.each([
    ["collection", HOOK, [] as (string | number)[], ""],
    ["observation", HOOK, [...OBS], PO],
    ["cell", HOOK, [...CELL], PC],
    ["producer", HOOK, [...OBS, "producer"], `${PO}/producer`],
    ["host", HOOK, [...HOST], `${PO}/producer/host`],
    ["report", HOOK, [...OBS, "reports", 0], `${PO}/reports/0`],
    ["proof", "v-control-rig.json", [...OBS, "proof"], `${PO}/proof`],
  ])("locates an unknown key on the %s", (_name, base, path, pointer) => {
    expect(located(collection(base, set(path, "zz", n("1"))))).toEqual([["malformed-receipt", `${pointer}/zz`]]);
  });

  it("locates an unknown key on a receipt, its host, and its build", () => {
    expect(located(receipt(set([], "zz", n("1"))))).toEqual([["malformed-receipt", "/zz"]]);
    expect(located(receipt(set(["host"], "zz", n("1"))))).toEqual([["malformed-receipt", "/host/zz"]]);
    expect(
      located(receipt(set(["cells", 0], "build", jsonObject([["key", "deadbeef"], ["digest", "deadbeef"], ["zz", n("1")]])))),
    ).toEqual([["malformed-receipt", "/cells/0/build/zz"]]);
  });

  it("reports a build without digest as both missing and not a digest (Python str(None))", () => {
    expect(located(collection(HOOK, set(CELL, "build", jsonObject([["key", "deadbeef"]]))))).toEqual([
      ["malformed-receipt", `${PC}/build/digest`],
      ["malformed-receipt", `${PC}/build/digest`],
    ]);
  });

  it("locates a missing required member at the member it names", () => {
    expect(located(collection(HOOK, drop(CELL, "exit_code")))).toEqual([
      ["malformed-receipt", `${PC}/exit_code`],
      ["malformed-receipt", `${PC}/exit_code`],
    ]);
    expect(located(receipt(drop([], "base")))).toEqual([["malformed-receipt", "/base"]]);
  });
});

describe("string/enum/identity", () => {
  it.each([
    ["environment empty", set([], "environment", ""), "unknown-environment", "/environment"],
    ["environment null", set([], "environment", null), "unknown-environment", "/environment"],
    ["head of 41 characters", set([], "head", "b".repeat(41)), "malformed-receipt", "/head"],
    ["head integer", set([], "head", n("1")), "malformed-receipt", "/head"],
    ["tree empty", set([], "tree", ""), "malformed-receipt", "/tree"],
    ["base uppercase", set(OBS, "base", "A".repeat(40)), "malformed-receipt", `${PO}/base`],
    ["scope_identity integer of 7 digits", set(OBS, "scope_identity", n("1234567")), "malformed-receipt", `${PO}/scope_identity`],
    ["completion empty", set(OBS, "completion", ""), "malformed-receipt", `${PO}/completion`],
    ["package empty", set(CELL, "package", ""), "malformed-receipt", `${PC}/package`],
    ["gate null", set(CELL, "gate", null), "malformed-receipt", `${PC}/gate`],
    ["outcome null", set(CELL, "outcome", null), "malformed-receipt", `${PC}/outcome`],
    ["gate_input_identity short", set(CELL, "gate_input_identity", "abc"), "malformed-receipt", `${PC}/gate_input_identity`],
    ["test_filter integer", set(CELL, "test_filter", n("5")), "malformed-receipt", `${PC}/test_filter`],
    ["test_filter Python-whitespace only", set(CELL, "test_filter", "\u001c\u0085"), "malformed-receipt", `${PC}/test_filter`],
    ["host os null", set(HOST, "os", null), "malformed-receipt", `${PO}/producer/host/os`],
    ["report path null", set([...OBS, "reports", 0], "path", null), "malformed-receipt", `${PO}/reports/0/path`],
    ["report sha256 of 65 characters", set([...OBS, "reports", 0], "sha256", "a".repeat(65)), "malformed-receipt", `${PO}/reports/0/sha256`],
    ["report store empty", set([...OBS, "reports", 0], "store", ""), "malformed-receipt", `${PO}/reports/0/store`],
    ["producer kind with a trailing space", set([...OBS, "producer"], "kind", "hook "), "malformed-receipt", `${PO}/producer/kind`],
  ])("rejects %s", (_name, edit, code, location) => {
    const found = located(collection(HOOK, edit));
    expect(found[0]).toEqual([code, location]);
  });

  it.each([
    ["observation_id of 7 characters", "abcdef0"],
    ["observation_id of 65 characters", "a".repeat(65)],
  ])("rejects an %s, stopping there", (_name, id) => {
    expect(located(collection(HOOK, set(OBS, "observation_id", id), { rehash: false }))).toEqual([
      ["malformed-receipt", `${PO}/observation_id`],
    ]);
  });

  it.each([
    ["host label empty", HOOK, set(HOST, "label", "")],
    ["hosted workflow empty", "v-control-hosted.json", set([...OBS, "producer"], "workflow", "")],
    ["hosted run_id empty", "v-control-hosted.json", set([...OBS, "producer"], "run_id", "")],
    ["test_filter on L1", HOOK, set(CELL, "test_filter", "tests::one")],
  ])("accepts %s", (_name, base, edit) => {
    expect(located(collection(base, edit))).toEqual([]);
  });

  it("rejects an empty completion_record_digest on a passing proof", () => {
    expect(located(collection("v-control-rig.json", set([...OBS, "proof"], "completion_record_digest", "")))).toEqual([
      ["malformed-receipt", `${PO}/proof/completion_record_digest`],
    ]);
  });

  it("rejects a short receipt scope_identity and accepts an empty receipt cell report", () => {
    expect(located(receipt(set([], "scope_identity", "abc")))).toEqual([["malformed-receipt", "/scope_identity"]]);
    expect(located(receipt(set(["cells", 0], "report", "")))).toEqual([]);
  });
});

describe("lists and members", () => {
  it.each([
    ["backends: one bad element", set(CELL, "backends", ["tmux", n("1")]), `${PC}/backends/1`],
    ["backends: all bad elements", set(CELL, "backends", [n("1"), null]), `${PC}/backends/0`],
    ["backends: scalar", set(CELL, "backends", "tmux"), `${PC}/backends`],
    ["failed_tests: one bad element", set(CELL, "failed_tests", ["a", null]), `${PC}/failed_tests/1`],
    ["observations: one valid, one null", (document: JsonObject) => {
      (document["observations"] as JsonValue[]).push(null);
    }, "/observations/1"],
    ["reports: one valid, one scalar", (document: JsonObject) => {
      (walk(document, [...OBS, "reports"]) as unknown as JsonValue[]).push("x");
    }, `${PO}/reports/1`],
  ])("locates %s", (_name, edit, location) => {
    expect(located(collection(HOOK, edit, { rehash: !_name.startsWith("observations") }))).toEqual([["malformed-receipt", location]]);
  });

  it("accepts empty backends and failed_tests", () => {
    expect(located(collection(HOOK, set(CELL, "backends", [])))).toEqual([]);
    expect(located(collection(HOOK, set(CELL, "failed_tests", [])))).toEqual([]);
  });

  it("locates every invalid receipt cell", () => {
    expect(located(receipt(set([], "cells", [null, n("1")])))).toEqual([
      ["malformed-receipt", "/cells/0"],
      ["malformed-receipt", "/cells/1"],
    ]);
    expect(located(receipt(set([], "cells", [])))).toEqual([["malformed-receipt", "/cells"]]);
  });
});

describe("numeric/boolean/counts", () => {
  it.each([
    ["attempt beyond 2^53", "v-control-hosted.json", set([...OBS, "producer"], "attempt", n("9007199254740993"))],
    ["report bytes beyond 2^64", HOOK, set([...OBS, "reports", 0], "bytes", n("18446744073709551617"))],
    ["duration_s -0.0", HOOK, set(CELL, "duration_s", n("-0.0"))],
    ["total 10.0 against integer parts summing to 10", HOOK, set(COUNTS, "total", n("10.0"))],
    ["partial fail without failure detail", HOOK, (document: JsonObject) => {
      const cell = walk(document, CELL);
      cell["completion"] = "partial";
      cell["outcome"] = "fail";
    }],
  ])("accepts %s", (_name, base, edit) => {
    expect(located(collection(base, edit))).toEqual([]);
  });

  it.each([
    ["attempt null", "v-control-hosted.json", set([...OBS, "producer"], "attempt", null), `${PO}/producer/attempt`],
    ["report bytes absent", HOOK, drop([...OBS, "reports", 0], "bytes"), `${PO}/reports/0/bytes`],
    ["duration_s absent", HOOK, drop(CELL, "duration_s"), `${PC}/duration_s`],
    ["exit_code 1.5", HOOK, set(CELL, "exit_code", n("1.5")), `${PC}/exit_code`],
    ["counts.total null", HOOK, set(COUNTS, "total", null), `${PC}/counts/total`],
  ])("locates %s", (_name, base, edit, location) => {
    expect(located(collection(base, edit)).at(-1)).toEqual(["malformed-receipt", location]);
  });

  it("reports a list count part as a located problem where Python raises TypeError", () => {
    expect(located(collection(HOOK, set(COUNTS, "passed", [n("9")])))).toEqual([["malformed-receipt", `${PC}/counts/passed`]]);
  });

  it("reports an int count too large to add to a float where Python raises OverflowError", () => {
    const edit: Edit = (document) => {
      const counts = walk(document, COUNTS);
      counts["passed"] = n(`1${"0".repeat(400)}`);
      counts["failed"] = n("0.5");
    };
    expect(located(collection(HOOK, edit))).toEqual([["malformed-receipt", `${PC}/counts/failed`]]);
  });
});

describe("graph/context", () => {
  function three(): JsonObject {
    const document = fixture(HOOK);
    const first = walk(document, OBS);
    const observations = ["a", "b", "c"].map((name) => {
      const copy = loadStrict(encode(canonical(first)));
      const observation = (copy.ok ? copy.value : null) as JsonObject;
      (observation["cell"] as JsonObject)["package"] = name;
      return observation;
    });
    document["observations"] = observations;
    rehash(document);
    return document;
  }

  it("rejects a three-member partial sort at /observations", () => {
    const document = three();
    const list = document["observations"] as JsonValue[];
    [list[1], list[2]] = [list[2] as JsonValue, list[1] as JsonValue];
    expect(located(parseCollection(encode(canonical(document))))).toEqual([["malformed-receipt", "/observations"]]);
  });

  it("locates a duplicate id at the second occurrence", () => {
    const document = three();
    const list = document["observations"] as JsonValue[];
    list[1] = list[0] as JsonValue;
    expect(located(parseCollection(encode(canonical(document))))[0]).toEqual(["conflicting-evidence", "/observations/1/observation_id"]);
  });

  it("rejects a retry naming an 8-character prefix of a present id as absent", () => {
    const document = three();
    const list = document["observations"] as JsonObject[];
    const target = (list[0] as JsonObject)["observation_id"] as string;
    const retrying = list[1] as JsonObject;
    retrying["retry_of"] = target.slice(0, 8);
    retrying["producer"] = (list[0] as JsonObject)["producer"] as JsonValue;
    rehash(document);
    const result = parseCollection(encode(canonical(document)));
    expect(located(result)).toHaveLength(1);
    expect(located(result)[0]?.[0]).toBe("observation-link-invalid");
    expect(located(result)[0]?.[1]).toMatch(/^\/observations\/\d\/retry_of$/);
  });
});

describe("adapter", () => {
  const scratch = mkdtempSync(join(tmpdir(), "evidence-adapter-"));
  afterAll(() => rmSync(scratch, { recursive: true, force: true }));

  function receiptWith(edit: Edit): JsonObject {
    const document = fixture("v-receipt-control.json");
    edit(document);
    return document;
  }

  it("adapts a receipt whose report_dir is an absolute directory, keeping it verbatim", () => {
    const content = encode("<testsuites/>");
    writeFileSync(join(scratch, "claudine-L1.xml"), content);
    const result = adaptReceipt(receiptWith(set(["host"], "report_dir", scratch)), nodeReportReader);
    expect(result.ok, JSON.stringify(result)).toBe(true);
    if (!result.ok) return;
    const observation = walk(result.value.collection, ["observations", 0]);
    expect(Object.keys(observation).sort()).toEqual(["base", "cell", "completion", "observation_id", "producer", "reports", "scope_identity"]);
    expect(walk(observation, ["producer"])["kind"]).toBe("hook");
    expect(walk(observation, ["producer", "host"])["report_dir"]).toBe(scratch);
    const report = walk(observation, ["reports", 0]);
    expect(report["sha256"]).toBe(createHash("sha256").update(content).digest("hex"));
    expect(canonical(report["bytes"] as JsonValue)).toBe(String(content.byteLength));
    expect(report["store"]).toBe("host-dir");
    expect(observation["observation_id"]).toBe(observationIdentity(observation));
    expect(result.value.observation_ids).toEqual([observation["observation_id"]]);
  });

  it("reads an absolute report path in place of report_dir, and a '..' path out of it, as Python does", () => {
    const outside = join(scratch, "outside.xml");
    writeFileSync(outside, "x");
    mkdirSync(join(scratch, "inner"), { recursive: true });
    const absolute = adaptReceipt(receiptWith(set(["cells", 0], "report", outside)), nodeReportReader);
    expect(absolute.ok, JSON.stringify(absolute)).toBe(true);
    const walked = adaptReceipt(
      receiptWith((document) => {
        set(["host"], "report_dir", join(scratch, "inner"))(document);
        set(["cells", 0], "report", "../outside.xml")(document);
      }),
      nodeReportReader,
    );
    expect(walked.ok, JSON.stringify(walked)).toBe(true);
  });

  it("publishes nothing for a malformed receipt and lists every problem (Python raises with the first)", () => {
    const result = adaptReceipt(
      receiptWith((document) => {
        document["zz"] = n("1");
        delete document["tree"];
      }),
      () => {
        throw new Error("no report is read for an invalid receipt");
      },
    );
    expect(result.ok).toBe(false);
    expect("value" in result).toBe(false);
    expect(located(result)).toEqual([
      ["malformed-receipt", "/tree"],
      ["malformed-receipt", "/zz"],
    ]);
  });

  it("locates every unreadable report and publishes nothing", () => {
    const document = receiptWith((receipt) => {
      const cells = receipt["cells"] as JsonValue[];
      const second = loadStrict(encode(canonical(cells[0] as JsonValue)));
      if (second.ok) (second.value as JsonObject)["package"] = "other";
      cells.push(second.ok ? second.value : null);
    });
    const home = join(scratch, "empty-home");
    const result = adaptReceipt(document, createNodeReportReader({ home: () => home }));
    expect(result.ok).toBe(false);
    expect(located(result)).toEqual([
      ["report-unavailable", "/cells/0/report"],
      ["report-unavailable", "/cells/1/report"],
    ]);
    if (!result.ok) expect(result.problems[0]?.message.startsWith(`${home}/ci-evidence/bbbbbbbbb does not retain claudine-L1.xml: `)).toBe(true);
  });

  it("never reads the report of an incomplete cell", () => {
    const calls: string[] = [];
    const reader: ReportReader = (directory, path) => {
      calls.push(`${directory}|${path}`);
      return encode("r");
    };
    const result = adaptReceipt(receiptWith(set(["cells", 0], "completion", "interrupted")), reader);
    expect(result.ok).toBe(true);
    expect(calls).toEqual([]);
  });

  it("refuses another user's home rather than guessing it", () => {
    const result = adaptReceipt(receiptWith(set(["host"], "report_dir", "~nobody/x")), nodeReportReader);
    expect(located(result)).toEqual([["report-unavailable", "/cells/0/report"]]);
  });

  it("locates a non-string report_dir where Python's Path() raises TypeError", () => {
    expect(located(adaptReceipt(receiptWith(set(["host"], "report_dir", n("1"))), nodeReportReader))).toEqual([
      ["malformed-receipt", "/host/report_dir"],
    ]);
  });

  it("maps adapted-collection problems back onto the receipt", () => {
    const reader: ReportReader = () => encode("r");
    expect(located(adaptReceipt(receiptWith(set(["host"], "os", n("1"))), reader))).toEqual([["malformed-receipt", "/host/os"]]);
    const identical = receiptWith((document) => {
      const cells = document["cells"] as JsonValue[];
      cells.push(cells[0] as JsonValue);
    });
    expect(located(adaptReceipt(identical, reader))).toEqual([["conflicting-evidence", "/cells/1"]]);
  });

  it("rejects a version-1 receipt", () => {
    expect(located(adaptReceipt(receiptWith(set([], "schema_version", n("1"))), nodeReportReader))).toEqual([
      ["v1-requires-exact-tree", "/schema_version"],
    ]);
  });
});
