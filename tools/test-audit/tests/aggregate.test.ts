/**
 * `attribute aggregate` — the JUnit → family join that fills
 * `perLegFamilySummed`. One named test per requirement clause: the shape
 * `deriveBudgets` consumes, and each failure class that must disqualify a leg
 * rather than be averaged away.
 */
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { aggregate, renderAggregateTable } from "../src/attribute/aggregate.ts";
import { deriveBudgets } from "../src/attribute/index.ts";
import { attributeCommand } from "../src/attribute/command.ts";
import { parseFamilyFile, type Family } from "../src/reconcile/families.ts";
import { EXIT, MalformedInput, UsageError } from "../src/errors.ts";
import type { CommandIo } from "../src/command.ts";
import type { EnvironmentConfig } from "../src/config.ts";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FAMILIES: Family[] = parseFamilyFile(
  JSON.stringify({
    families: [
      { id: "alpha", package: "demo-pkg", match: { suites: ["demo-pkg::alpha"] } },
      { id: "beta", package: "demo-pkg", match: { suites: ["demo-pkg::beta"] } },
    ],
  })
).families;

function env(
  name: string,
  overrides: Partial<EnvironmentConfig> = {}
): EnvironmentConfig {
  return {
    name,
    kind: "native",
    os: "linux",
    cells: [{ tier: "L1", package: "demo-pkg" }],
    pending: false,
    ...overrides,
  };
}

interface CaseSpec {
  suite: string;
  name: string;
  seconds: number;
  failed?: boolean;
}

function xmlFor(cases: CaseSpec[], overrides: { tests?: number; failures?: number } = {}): string {
  const bySuite = new Map<string, CaseSpec[]>();
  for (const c of cases) bySuite.set(c.suite, [...(bySuite.get(c.suite) ?? []), c]);
  const wall = cases.reduce((total, c) => total + c.seconds, 0);
  const failures = overrides.failures ?? cases.filter((c) => c.failed).length;
  const suites = [...bySuite.entries()]
    .map(([suite, list]) => {
      const rows = list
        .map((c) =>
          c.failed
            ? `<testcase name="${c.name}" classname="${suite}" time="${c.seconds}"><failure/></testcase>`
            : `<testcase name="${c.name}" classname="${suite}" time="${c.seconds}"/>`
        )
        .join("\n");
      return `<testsuite name="${suite}" tests="${list.length}" skipped="0" errors="0" failures="${list.filter((c) => c.failed).length}">\n${rows}\n</testsuite>`;
    })
    .join("\n");
  return [
    `<?xml version="1.0" encoding="UTF-8"?>`,
    `<testsuites name="nextest-run" tests="${overrides.tests ?? cases.length}" skipped="0" failures="${failures}" errors="0" time="${wall.toFixed(3)}">`,
    suites,
    `</testsuites>`,
  ].join("\n");
}

interface CellSpec {
  tier?: string;
  package?: string;
  cases?: CaseSpec[];
  xml?: string;
  exit_code?: number;
  report_present?: boolean;
  environment?: string;
  /** Skip writing the XML file while still naming it in the manifest. */
  omitFile?: boolean;
}

interface ProvenanceSpec {
  sha: string;
  runId: string;
  runNumber: number;
  runAttempt: number;
  ref: string;
  event: string;
  workflow: string;
}

/** Write one `<run>/<leg>/` staging tree the way `_stage_junit` builds it. */
function writeLeg(
  runDir: string,
  leg: string,
  cells: CellSpec[],
  options: { manifest?: string | null; provenance?: ProvenanceSpec | null } = {}
): void {
  const legDir = join(runDir, leg);
  mkdirSync(legDir, { recursive: true });
  const records: string[] = [];
  for (const cell of cells) {
    const tier = cell.tier ?? "L1";
    const pkg = cell.package ?? "demo-pkg";
    const relative = `${tier}/${pkg}.xml`;
    if (cell.report_present !== false && !cell.omitFile) {
      mkdirSync(join(legDir, tier), { recursive: true });
      writeFileSync(join(legDir, relative), cell.xml ?? xmlFor(cell.cases ?? []), "utf8");
    }
    records.push(
      JSON.stringify({
        tier,
        package: pkg,
        xml: relative,
        exit_code: cell.exit_code ?? 0,
        environment: cell.environment ?? leg,
        duration_s: 100,
        report_present: cell.report_present ?? true,
      })
    );
  }
  if (options.manifest === null) return;
  writeFileSync(join(legDir, "manifest.jsonl"), options.manifest ?? `${records.join("\n")}\n`, "utf8");

  // Write provenance.json if provided
  if (options.provenance !== undefined && options.provenance !== null) {
    writeFileSync(join(legDir, "provenance.json"), JSON.stringify(options.provenance, null, 2), "utf8");
  }
}

const GREEN: CaseSpec[] = [
  { suite: "demo-pkg::alpha", name: "one", seconds: 1.5 },
  { suite: "demo-pkg::alpha", name: "two", seconds: 0.5 },
  { suite: "demo-pkg::beta", name: "three", seconds: 4 },
];

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "test-audit-aggregate-"));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

function run(...names: string[]): string[] {
  return names.map((name) => join(root, name));
}

function capture(argv: string[]): { code: number; out: string; err: string } {
  const out: string[] = [];
  const err: string[] = [];
  const io: CommandIo = { out: (l) => out.push(l), err: (l) => err.push(l) };
  const code = attributeCommand.run(argv, io) as number;
  return { code, out: out.join("\n"), err: err.join("\n") };
}

// ---------------------------------------------------------------------------
// The shape `deriveBudgets` consumes
// ---------------------------------------------------------------------------

describe("aggregate emits the budget input shape", () => {
  it("sums per family per leg and records one sample per run", () => {
    for (const id of ["r1", "r2"]) writeLeg(join(root, id), "ubuntu-latest", [{ cases: GREEN }]);

    const result = aggregate({
      runDirs: run("r1", "r2"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });

    expect(result.violations).toEqual([]);
    expect(result.budgetInput.perLegFamilySummed).toEqual({
      "ubuntu-latest": { beta: [4, 4], alpha: [2, 2] },
    });
    expect(result.budgetInput.provenance.runsPerLeg).toEqual({ "ubuntu-latest": 2 });
    expect(result.budgetInput.provenance.runs).toEqual(["r1", "r2"]);
  });

  it("feeds deriveBudgets directly and produces a budget at three green runs", () => {
    const ciProv = (runNumber: number): ProvenanceSpec => ({
      sha: "abc123",
      runId: "12345678",
      runNumber,
      runAttempt: 1,
      ref: "refs/heads/main",
      event: "push",
      workflow: "CI",
    });
    for (const id of ["r1", "r2", "r3"]) {
      const scale = id === "r3" ? 2 : 1;
      writeLeg(join(root, id), "ubuntu-latest", [
        { cases: GREEN.map((c) => ({ ...c, seconds: c.seconds * scale })) },
      ], { provenance: ciProv(Number(id.slice(1))) });
    }

    const result = aggregate({
      runDirs: run("r1", "r2", "r3"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);

    const { budgets, violations } = deriveBudgets(result.budgetInput);
    expect(violations).toEqual([]);
    // worst of three (`beta` 8 s in r3) times the 1.25 headroom
    expect(budgets).toEqual([
      { leg: "ubuntu-latest", family: "beta", observedMax: 8, budget: 10 },
      { leg: "ubuntu-latest", family: "alpha", observedMax: 4, budget: 5 },
    ]);
  });

  it("carries the default 0.25 headroom into the output and honours an override", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const base = { runDirs: run("r1"), environments: [env("ubuntu-latest")], families: FAMILIES };
    expect(aggregate(base).budgetInput.headroom).toBe(0.25);
    expect(aggregate({ ...base, headroom: 0.1 }).budgetInput.headroom).toBe(0.1);
  });

  it("keeps one run at one sample so deriveBudgets still refuses", () => {
    const ciProv = { sha: "abc123", runId: "12345678", runNumber: 1, runAttempt: 1, ref: "refs/heads/main", event: "push", workflow: "CI" };
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv });
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    const { budgets, violations } = deriveBudgets(result.budgetInput);
    expect(budgets).toEqual([]);
    expect(violations.map((v) => v.kind)).toEqual(["insufficient-runs"]);
  });

  it("leaves a family absent from one run short of a sample", () => {
    const ciProv = (n: number) => ({ sha: "abc123", runId: "12345678", runNumber: n, runAttempt: 1, ref: "refs/heads/main", event: "push", workflow: "CI" });
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(1) });
    for (const id of ["r2", "r3"]) {
      writeLeg(join(root, id), "ubuntu-latest", [
        { cases: GREEN.filter((c) => c.suite === "demo-pkg::alpha") },
      ], { provenance: ciProv(Number(id.slice(1))) });
    }
    const result = aggregate({
      runDirs: run("r1", "r2", "r3"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.perLegFamilySummed["ubuntu-latest"]!.beta).toEqual([4]);

    const { budgets, violations } = deriveBudgets(result.budgetInput);
    expect(budgets).toEqual([]);
    expect(violations).toEqual([
      { kind: "insufficient-runs", detail: "ubuntu-latest/beta: 1 sample(s); 3 are required" },
    ]);
  });

  it("derives local provenance when no provenance.json exists so deriveBudgets refuses", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.budgetInput.provenance.kind).toBe("local");
    expect(deriveBudgets(result.budgetInput).violations.map((v) => v.kind)).toEqual([
      "local-derived-budget",
    ]);
  });
});

// ---------------------------------------------------------------------------
// Failure classes — each disqualifies the leg for that run
// ---------------------------------------------------------------------------

describe("aggregate refuses evidence it cannot trust", () => {
  it("reports an identity no family claims and counts the leg for nothing", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [
      { cases: [...GREEN, { suite: "demo-pkg::gamma", name: "orphan", seconds: 3 }] },
    ]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([
      { kind: "unassigned-identity", detail: "demo-pkg::gamma orphan matches no family" },
    ]);
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
    expect(result.budgetInput.provenance.runsPerLeg["ubuntu-latest"]).toBe(0);
  });

  it("counts an identity two families claim by neither of them", () => {
    const overlapping = parseFamilyFile(
      JSON.stringify({
        families: [
          { id: "alpha", package: "demo-pkg", match: { suites: ["demo-pkg::alpha"] } },
          { id: "alpha-too", package: "demo-pkg", match: { suites: ["demo-pkg::alpha"] } },
          { id: "beta", package: "demo-pkg", match: { suites: ["demo-pkg::beta"] } },
        ],
      })
    ).families;
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);

    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: overlapping,
    });
    expect(result.violations.map((v) => v.kind)).toEqual([
      "double-assigned-identity",
      "double-assigned-identity",
    ]);
    expect(result.violations[0]!.detail).toContain("alpha, alpha-too");
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
  });

  it("rejects a leg without a manifest as not a staging tree", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { manifest: null });
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toHaveLength(1);
    expect(result.violations[0]!.kind).toBe("missing-artifact");
    expect(result.violations[0]!.detail).toContain("manifest.jsonl absent");
  });

  it("rejects a malformed manifest", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { manifest: "{not json\n" });
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["malformed-report"]);
  });

  it("rejects malformed XML in a cell", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ xml: "<testsuites><oops" }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["malformed-report"]);
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
  });

  it("rejects a report whose declared test count disagrees with its cases", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ xml: xmlFor(GREEN, { tests: 99 }) }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["malformed-report"]);
    expect(result.violations[0]!.detail).toContain("declares 99 case(s)");
  });

  it("reports a declared leg absent from the tree as a missing leg", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest"), env("macos-latest", { os: "macos" })],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([
      { kind: "missing-leg", detail: "r1/macos-latest: declared but absent from the staging tree" },
    ]);
    expect(Object.keys(result.budgetInput.perLegFamilySummed)).toEqual(["ubuntu-latest"]);
    expect(result.budgetInput.provenance.legs).toEqual(["ubuntu-latest", "macos-latest"]);
  });

  it("reports a leg declaring no cells rather than counting it as measured", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest", { cells: [] })],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["missing-leg"]);
    expect(result.violations[0]!.detail).toContain("declares no cells");
  });

  it("reports a pending leg as pending and never counts it", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [
        env("ubuntu-latest"),
        env("wsl2-ubuntu", { kind: "wsl", pending: true, reason: "no runner yet" }),
      ],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.pending).toEqual(["r1/wsl2-ubuntu"]);
    expect(result.budgetInput.provenance.legs).toEqual(["ubuntu-latest"]);
    expect(result.budgetInput.provenance.runsPerLeg).toEqual({ "ubuntu-latest": 1 });
  });

  it("treats a non-zero manifest exit code as a red run that measures nothing", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN, exit_code: 100 }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["failed-run"]);
    expect(result.violations[0]!.detail).toContain("a red run measures nothing");
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
  });

  it("treats a failed test case as a red run that measures nothing", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [
      { cases: GREEN.map((c, i) => (i === 0 ? { ...c, failed: true } : c)) },
    ]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["failed-run"]);
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
  });

  it("rejects a manifest record that names another environment", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN, environment: "macos-latest" }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["provenance-mismatch"]);
    expect(result.budgetInput.perLegFamilySummed).toEqual({});
  });

  it("rejects a cell the manifest names but the tree does not carry", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN, omitFile: true }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["missing-artifact"]);
    expect(result.violations[0]!.detail).toContain("absent from the tree");
  });

  it("rejects a cell the manifest records as producing no report", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN, report_present: false }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["missing-artifact"]);
    expect(result.violations[0]!.detail).toContain("report_present=false");
  });

  it("rejects a declared cell with no manifest record", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ tier: "L2", package: "demo-pkg" }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["unexpected-manifest-cell", "missing-artifact"]);
    expect(result.violations[0]!.detail).toContain("L2/demo-pkg");
    expect(result.violations[1]!.detail).toContain("no manifest record for L1/demo-pkg.xml");
  });

  it("reports a run directory that does not exist", () => {
    const result = aggregate({
      runDirs: run("nope"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["missing-artifact"]);
    expect(result.violations[0]!.detail).toContain("no such staging tree");
  });

  it("refuses the same run supplied twice", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    expect(() =>
      aggregate({
        runDirs: run("r1", "r1"),
        environments: [env("ubuntu-latest")],
        families: FAMILIES,
      })
    ).toThrow(MalformedInput);
  });

  it("counts a clean leg even when a sibling leg in the same run is red", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    writeLeg(join(root, "r1"), "macos-latest", [{ cases: GREEN, exit_code: 101 }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest"), env("macos-latest", { os: "macos" })],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["failed-run"]);
    expect(result.budgetInput.provenance.runsPerLeg).toEqual({
      "ubuntu-latest": 1,
      "macos-latest": 0,
    });
  });
});

// ---------------------------------------------------------------------------
// Provenance validation — derived kind and consistency checks
// ---------------------------------------------------------------------------

describe("aggregate derives provenance and validates consistency", () => {
  const ciProv = (runNumber: number, sha = "abc123"): ProvenanceSpec => ({
    sha,
    runId: "12345678",
    runNumber,
    runAttempt: 1,
    ref: "refs/heads/main",
    event: "push",
    workflow: "CI",
  });

  it("derives 'local' provenance when no staging tree has provenance.json", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.kind).toBe("local");
  });

  it("derives 'ci' provenance when all counted samples have provenance.json", () => {
    for (const id of ["r1", "r2"]) {
      writeLeg(join(root, id), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(Number(id.slice(1))) });
    }
    const result = aggregate({
      runDirs: run("r1", "r2"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.kind).toBe("ci");
  });

  it("derives 'local' when some samples have CI provenance and others do not", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(1) });
    writeLeg(join(root, "r2"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1", "r2"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.kind).toBe("local");
  });

  it("rejects samples spanning two source revisions", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(1, "abc123") });
    writeLeg(join(root, "r2"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(2, "def456") });
    const result = aggregate({
      runDirs: run("r1", "r2"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["mixed-source-revisions"]);
    expect(result.violations[0]!.detail).toContain("2 source revisions");
    expect(result.violations[0]!.detail).toContain("abc123");
    expect(result.violations[0]!.detail).toContain("def456");
  });

  it("rejects non-consecutive run numbers", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(10) });
    writeLeg(join(root, "r2"), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(12) });
    const result = aggregate({
      runDirs: run("r1", "r2"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["non-consecutive-runs"]);
    expect(result.violations[0]!.detail).toContain("10 → 12 are not consecutive");
  });

  it("accepts consecutive run numbers from one source revision", () => {
    for (const [id, num] of [["r1", 10], ["r2", 11], ["r3", 12]] as const) {
      writeLeg(join(root, id), "ubuntu-latest", [{ cases: GREEN }], { provenance: ciProv(num) });
    }
    const result = aggregate({
      runDirs: run("r1", "r2", "r3"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.kind).toBe("ci");
    expect(result.budgetInput.provenance.runsPerLeg["ubuntu-latest"]).toBe(3);
  });

  it("rejects a malformed provenance.json", () => {
    const legDir = join(root, "r1", "ubuntu-latest");
    mkdirSync(legDir, { recursive: true });
    writeFileSync(join(legDir, "provenance.json"), "{not json\n", "utf8");
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["malformed-report"]);
  });
});

// ---------------------------------------------------------------------------
// Manifest cardinality — exact one-to-one matching
// ---------------------------------------------------------------------------

describe("aggregate enforces exact manifest cardinality", () => {
  it("rejects a duplicate (tier, package) manifest record", () => {
    const records = [
      JSON.stringify({ tier: "L1", package: "demo-pkg", xml: "L1/demo-pkg.xml", exit_code: 0, environment: "ubuntu-latest", duration_s: 100, report_present: true }),
      JSON.stringify({ tier: "L1", package: "demo-pkg", xml: "L1/demo-pkg.xml", exit_code: 0, environment: "ubuntu-latest", duration_s: 100, report_present: true }),
    ].join("\n") + "\n";
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { manifest: records });
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["duplicate-manifest-cell"]);
    expect(result.violations[0]!.detail).toContain("L1/demo-pkg");
    expect(result.violations[0]!.detail).toContain("2 times");
  });

  it("rejects an unexpected manifest cell not declared by the environment", () => {
    const records = [
      JSON.stringify({ tier: "L1", package: "demo-pkg", xml: "L1/demo-pkg.xml", exit_code: 0, environment: "ubuntu-latest", duration_s: 100, report_present: true }),
      JSON.stringify({ tier: "L2", package: "other-pkg", xml: "L2/other-pkg.xml", exit_code: 0, environment: "ubuntu-latest", duration_s: 100, report_present: true }),
    ].join("\n") + "\n";
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }], { manifest: records });
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations.map((v) => v.kind)).toEqual(["unexpected-manifest-cell"]);
    expect(result.violations[0]!.detail).toContain("L2/other-pkg");
    expect(result.violations[0]!.detail).toContain("declares no such cell");
  });

  it("accepts a clean manifest with exact one-to-one matching", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [env("ubuntu-latest")],
      families: FAMILIES,
    });
    expect(result.violations).toEqual([]);
    expect(result.budgetInput.provenance.runsPerLeg["ubuntu-latest"]).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Rendering and command surface
// ---------------------------------------------------------------------------

describe("aggregate command surface", () => {
  function writeConfig(): string {
    const familiesPath = join(root, "families.json");
    writeFileSync(
      familiesPath,
      JSON.stringify({
        families: [
          { id: "alpha", package: "demo-pkg", match: { suites: ["demo-pkg::alpha"] } },
          { id: "beta", package: "demo-pkg", match: { suites: ["demo-pkg::beta"] } },
        ],
      }),
      "utf8"
    );
    const configPath = join(root, "audit.config.json");
    writeFileSync(
      configPath,
      JSON.stringify({
        version: 1,
        area: "demo",
        root: ".",
        evidenceDir: ".",
        packages: [{ name: "demo-pkg", path: "demo" }],
        routes: [{ id: "test", kind: "nextest", command: "just test", executes: true }],
        selections: [{ label: "demo", packages: ["demo-pkg"], features: [], routes: ["test"] }],
        cohorts: [
          {
            id: "l1",
            description: "demo",
            population: "ci",
            recipe: "just test",
            selections: ["demo"],
          },
        ],
        environments: [
          {
            name: "ubuntu-latest",
            kind: "native",
            os: "linux",
            cells: [{ tier: "L1", package: "demo-pkg" }],
          },
        ],
        families: "families.json",
      }),
      "utf8"
    );
    return configPath;
  }

  it("exits 0 on a clean tree and 1 on a violation", () => {
    const configPath = writeConfig();
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const clean = capture(["aggregate", join(root, "r1"), "--config", configPath]);
    expect(clean.code).toBe(EXIT.clean);
    expect(clean.out).toContain("GATE EXIT=0");

    writeLeg(join(root, "r2"), "ubuntu-latest", [{ cases: GREEN, exit_code: 100 }]);
    const red = capture(["aggregate", join(root, "r2"), "--config", configPath]);
    expect(red.code).toBe(EXIT.violations);
    expect(red.err).toContain("[failed-run]");
    expect(red.err).toContain("GATE EXIT=1");
  });

  it("writes the budget input to --out and prints it with --json", () => {
    const configPath = writeConfig();
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const outPath = join(root, "budgets.json");
    const result = capture([
      "aggregate",
      join(root, "r1"),
      "--config",
      configPath,
      "--out",
      outPath,
      "--json",
    ]);
    expect(result.code).toBe(EXIT.clean);
    const written = JSON.parse(result.out.replace(/\nGATE EXIT=0$/, ""));
    expect(written.perLegFamilySummed).toEqual({ "ubuntu-latest": { beta: [4], alpha: [2] } });
    expect(JSON.parse(readFileSync(outPath, "utf8"))).toEqual(written);
  });

  it("is a usage error without run directories or with invalid headroom", () => {
    const configPath = writeConfig();
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    expect(() => capture(["aggregate", "--config", configPath])).toThrow(UsageError);
    expect(() =>
      capture(["aggregate", join(root, "r1"), "--config", configPath, "--headroom", "-1"])
    ).toThrow(UsageError);
  });

  it("renders per-cell and per-leg tables including pending legs", () => {
    writeLeg(join(root, "r1"), "ubuntu-latest", [{ cases: GREEN }]);
    const result = aggregate({
      runDirs: run("r1"),
      environments: [
        env("ubuntu-latest"),
        env("wsl2-ubuntu", { kind: "wsl", pending: true, reason: "no runner yet" }),
      ],
      families: FAMILIES,
    });
    const table = renderAggregateTable(result);
    expect(table).toContain("| r1 | ubuntu-latest | `L1/demo-pkg` | 3 | 6.00 s |");
    expect(table).toContain("| ubuntu-latest | 1 | 2 | 1 |");
    expect(table).toContain("pending legs (declared pending, not measured): r1/wsl2-ubuntu");
  });
});
