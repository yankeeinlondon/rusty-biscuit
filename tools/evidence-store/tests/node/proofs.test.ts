// The ported proof structure checks (`src/contracts/proofs.ts`). The field
// shape matrix runs once per validator here; the service tests only wire one
// valid and a few invalid documents through finalization.
import { describe, expect, it } from "vitest";
import { loadStrict, type JsonObject, type JsonValue } from "../../src/contracts/index.ts";
import {
  completionBindingProblems,
  expectedManifestProblems,
  proofProblems,
  validateCompletionRecord,
  type ProofCell,
} from "../../src/contracts/proofs.ts";

const encoder = new TextEncoder();

function load(text: string): JsonValue {
  const loaded = loadStrict(encoder.encode(text));
  if (!loaded.ok) throw new Error(`test document does not load: ${text}`);
  return loaded.value;
}

/** A realistic record: the shape `completion.py` writes for a hosted L1 cell. */
const RECORD = {
  schema_version: 1,
  package: "alpha",
  environment: "ubuntu-latest",
  gate: "L1",
  complete: true,
  head: "b".repeat(40),
  run: "100",
  attempt: 1,
  nextest_version: "0.9.99",
  reports: ["L1/alpha.xml"],
  build: "build-key",
};

/** The shape `just/devops.just::_expected_manifest` writes (schema version 2). */
const MANIFEST = {
  schema_version: 2,
  environment: "ubuntu-latest",
  tier: "L1",
  target: "x86_64-unknown-linux-gnu",
  nextest_version: "0.9.99",
  from_archive: false,
  selection: { filter: "not test(/^level2_/)", profile: "ci", test_args: "" },
  packages: { alpha: { tests: ["alpha::fine"], ignored: [], excluded: [] } },
};

const CELL: ProofCell = {
  package: "alpha",
  gate: "L1",
  environment: "ubuntu-latest",
  head: "b".repeat(40),
  target: "x86_64-unknown-linux-gnu",
  producer: load('{"kind":"hosted","run_id":"100","attempt":1,"host":{"os":"Linux","kernel":"6"}}') as JsonObject,
};

/** `base` with `key` replaced by the raw JSON `value`, or removed when `value` is undefined. */
function edited(base: Record<string, unknown>, key: string, value: string | undefined): JsonValue {
  const entries = Object.entries(base)
    .filter(([name]) => name !== key)
    .map(([name, item]) => `${JSON.stringify(name)}:${JSON.stringify(item)}`);
  if (value !== undefined) entries.push(`${JSON.stringify(key)}:${value}`);
  return load(`{${entries.join(",")}}`);
}

const codes = (problems: readonly { code: string }[]) => problems.map((problem) => problem.code);

describe("validateCompletionRecord", () => {
  it("accepts the control record, with and without optional fields", () => {
    expect(validateCompletionRecord(load(JSON.stringify(RECORD)))).toEqual([]);
    expect(validateCompletionRecord(edited(RECORD, "build", undefined))).toEqual([]);
    expect(validateCompletionRecord(edited(RECORD, "nextest_version", undefined))).toEqual([]);
  });

  it("reads the version before the field set: only the integer 1 is version 1", () => {
    for (const version of ["1.0", "true", '"1"', "2", "null"]) {
      const problems = validateCompletionRecord(edited({ ...RECORD, unknown: 1 }, "schema_version", version));
      expect(codes(problems), version).toEqual(["completion-manifest-schema"]);
    }
    expect(codes(validateCompletionRecord(edited(RECORD, "schema_version", undefined)))).toEqual(["completion-plan-unreadable"]);
  });

  it("refuses a non-object document and reports every missing and unknown field", () => {
    for (const document of ["[]", "null", '"record"', "1"]) {
      expect(codes(validateCompletionRecord(load(document))), document).toEqual(["completion-plan-unreadable"]);
    }
    const problems = validateCompletionRecord(load('{"schema_version":1,"zeta":1,"alpha":2}'));
    expect(problems.filter((problem) => problem.message.includes("missing"))).toHaveLength(8);
    expect(problems.filter((problem) => problem.message.includes("unknown")).map((problem) => problem.message)).toEqual([
      "completion-plan-unreadable: completion record has unknown field 'alpha'",
      "completion-plan-unreadable: completion record has unknown field 'zeta'",
    ]);
  });

  // field, invalid raw values, expected code
  const matrix: [string, string[], string][] = [
    ["package", ["null", '""', "1", "[]"], "completion-plan-unreadable"],
    ["run", ["null", '""', "100", "{}"], "completion-plan-unreadable"],
    ["attempt", ["null", "0", "-1", "1.0", "true", '"1"'], "completion-plan-unreadable"],
    ["environment", ["null", '""', '"linux"', "1"], "completion-cell-unknown"],
    ["gate", ["null", '""', '"L3"', "[]"], "completion-cell-unknown"],
    ["head", ["null", '""', '"B'.padEnd(41, "b") + '"', '"abc"', "1"], "completion-plan-unreadable"],
    ["complete", ["null", "false", "1", '"true"'], "completion-plan-unreadable"],
    ["reports", ["null", '""', "{}", "[1]", '["a",null]'], "completion-report-missing"],
  ];
  for (const [field, values, code] of matrix) {
    it(`refuses every invalid ${field} with ${code}`, () => {
      for (const value of values) {
        expect(codes(validateCompletionRecord(edited(RECORD, field, value))), `${field}=${value}`).toEqual([code]);
      }
      expect(codes(validateCompletionRecord(edited(RECORD, field, undefined))), `${field} absent`).toEqual(["completion-plan-unreadable"]);
    });
  }

  it("accepts an empty reports list (a structural check only)", () => {
    expect(validateCompletionRecord(edited(RECORD, "reports", "[]"))).toEqual([]);
  });
});

describe("completionBindingProblems", () => {
  const record = load(JSON.stringify(RECORD)) as JsonObject;

  it("binds the control record to its cell and hosted run", () => {
    expect(completionBindingProblems(record, CELL)).toEqual([]);
  });

  it("refuses another package, environment, gate, head, run, or attempt", () => {
    for (const [field, value] of [
      ["package", '"beta"'],
      ["environment", '"macos-latest"'],
      ["gate", '"L2"'],
      ["head", JSON.stringify("c".repeat(40))],
      ["run", '"101"'],
      ["attempt", "2"],
    ] as const) {
      expect(codes(completionBindingProblems(edited(RECORD, field, value) as JsonObject, CELL)), field).toEqual(["completion-cell-unknown"]);
    }
  });

  it("binds run and attempt only for a hosted producer that records them", () => {
    const rig = { ...CELL, producer: load('{"kind":"rig","run_id":"run7","host":{"os":"Darwin","kernel":"27"}}') as JsonObject };
    expect(completionBindingProblems(edited(RECORD, "run", '"elsewhere"') as JsonObject, rig)).toEqual([]);
  });

  it("requires a nextest version for a test tier only", () => {
    const bare = edited(RECORD, "nextest_version", undefined) as JsonObject;
    expect(codes(completionBindingProblems(bare, CELL))).toEqual(["completion-plan-unreadable"]);
    expect(codes(completionBindingProblems(edited(RECORD, "nextest_version", '""') as JsonObject, CELL))).toEqual(["completion-plan-unreadable"]);
    const check = edited({ ...RECORD, gate: "check" }, "nextest_version", undefined) as JsonObject;
    expect(completionBindingProblems(check, { ...CELL, gate: "check" })).toEqual([]);
  });

  it("never quotes a value read from the record", () => {
    const secret = "do-not-echo-".repeat(4);
    const problems = [
      ...validateCompletionRecord(edited(RECORD, "environment", JSON.stringify(secret))),
      ...completionBindingProblems(edited(RECORD, "run", JSON.stringify(secret)) as JsonObject, CELL),
    ];
    expect(problems).toHaveLength(2);
    for (const problem of problems) expect(problem.message).not.toContain("do-not-echo");
  });
});

describe("expectedManifestProblems", () => {
  it("accepts the control manifest, with and without optional fields", () => {
    expect(expectedManifestProblems(load(JSON.stringify(MANIFEST)), CELL)).toEqual([]);
    expect(expectedManifestProblems(load(JSON.stringify({ ...MANIFEST, backends: ["tmux"], companion_suites: [] })), CELL)).toEqual([]);
  });

  it("refuses a non-object and any version other than the integer 2", () => {
    for (const document of ["[]", "null", '"m"']) {
      expect(codes(expectedManifestProblems(load(document), CELL)), document).toEqual(["completion-manifest-schema"]);
    }
    for (const version of ["2.0", "true", '"2"', "1", "null"]) {
      expect(codes(expectedManifestProblems(edited(MANIFEST, "schema_version", version), CELL)), version).toEqual(["completion-manifest-schema"]);
    }
    expect(codes(expectedManifestProblems(edited(MANIFEST, "schema_version", undefined), CELL))).toEqual(["completion-manifest-schema"]);
  });

  it("reports every missing required field and unknown field", () => {
    for (const field of ["environment", "tier", "target", "nextest_version", "from_archive", "selection", "packages"]) {
      expect(codes(expectedManifestProblems(edited(MANIFEST, field, undefined), CELL)), field).toEqual(["completion-manifest-provenance"]);
    }
    expect(codes(expectedManifestProblems(edited(MANIFEST, "extra", "1"), CELL))).toEqual(["completion-manifest-provenance"]);
  });

  const matrix: [string, string[], string][] = [
    ["target", ["null", '""', "1", '"aarch64-apple-darwin"'], "completion-manifest-target"],
    ["environment", ["null", '""', "1", '"macos-latest"'], "completion-manifest-provenance"],
    ["tier", ["null", '""', "[]", '"L2"'], "completion-manifest-provenance"],
    ["nextest_version", ["null", '""', '"  "', "1", "{}"], "completion-manifest-provenance"],
    ["packages", ["null", "[]", '""', "{}", '{"beta":{}}'], "completion-manifest-provenance"],
  ];
  for (const [field, values, code] of matrix) {
    it(`refuses every invalid ${field} with ${code}`, () => {
      for (const value of values) {
        expect(codes(expectedManifestProblems(edited(MANIFEST, field, value), CELL)), `${field}=${value}`).toEqual([code]);
      }
    });
  }

  it("requires each listing set in the cell's package entry", () => {
    for (const field of ["tests", "ignored", "excluded"]) {
      const listing = Object.fromEntries(Object.entries(MANIFEST.packages.alpha).filter(([name]) => name !== field));
      const problems = expectedManifestProblems(load(JSON.stringify({ ...MANIFEST, packages: { alpha: listing } })), CELL);
      expect(problems.map((problem) => problem.message), field).toEqual([
        `completion-manifest-provenance: the expected manifest's alpha listing is missing '${field}'`,
      ]);
    }
    expect(expectedManifestProblems(edited(MANIFEST, "packages", '{"alpha":[]}'), CELL)).toHaveLength(3);
  });

  it("does not port plan selection: any selection object shape is accepted", () => {
    expect(expectedManifestProblems(edited(MANIFEST, "selection", '{"filter":"anything"}'), CELL)).toEqual([]);
  });
});

describe("proofProblems", () => {
  it("loads proofs with the shared strict loader", () => {
    const duplicate = encoder.encode('{"schema_version":1,"schema_version":1}');
    expect(codes(proofProblems("completion-record", duplicate, CELL))).toEqual(["completion-plan-unreadable"]);
    expect(codes(proofProblems("expected-listing", encoder.encode("{} {}"), CELL))).toEqual(["completion-manifest-schema"]);
    expect(proofProblems("report", encoder.encode("not json"), CELL)).toEqual([]);
  });

  it("checks structure before binding", () => {
    expect(proofProblems("completion-record", encoder.encode(JSON.stringify(RECORD)), CELL)).toEqual([]);
    const unbound = encoder.encode(JSON.stringify({ ...RECORD, package: "beta", attempt: 0 }));
    expect(codes(proofProblems("completion-record", unbound, CELL))).toEqual(["completion-plan-unreadable"]);
    expect(proofProblems("expected-listing", encoder.encode(JSON.stringify(MANIFEST)), CELL)).toEqual([]);
  });
});
