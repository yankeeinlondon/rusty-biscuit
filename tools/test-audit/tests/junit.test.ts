/**
 * JUnit gate tests: parsing, validation, and comparison.
 */
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { parseJunit } from "../src/junit/parse.ts";
import { parseManifest } from "../src/junit/manifest.ts";
import { collect, type CollectionInput, type ViolationKind } from "../src/junit/collect.ts";
import { compare } from "../src/junit/compare.ts";
import { MalformedInput } from "../src/errors.ts";
import type { EnvironmentConfig } from "../src/config.ts";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function report(
  cases: string,
  overrides: { tests?: number; failures?: number; errors?: number; time?: string } = {},
): string {
  const tests = overrides.tests ?? (cases.match(/<testcase\b/g) ?? []).length;
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<testsuites name="nextest-run" tests="${tests}" failures="${overrides.failures ?? 0}" ` +
      `errors="${overrides.errors ?? 0}" uuid="u" timestamp="2026-09-07T00:00:00Z" ` +
      `time="${overrides.time ?? "1.000"}">`,
    '  <testsuite name="test-pkg::demo" tests="1" disabled="0" errors="0" failures="0">',
    cases,
    "  </testsuite>",
    "</testsuites>",
  ].join("\n");
}

const passingCase = (name: string, time = "0.100") =>
  `    <testcase name="${name}" classname="test-pkg::demo" time="${time}"></testcase>`;

interface TreeCell {
  tier?: string;
  package?: string;
  xml?: string | null;
  exit_code?: number;
  duration_s?: number;
  report_present?: boolean;
  manifestXmlName?: string;
}

function tree(
  environments: Record<string, TreeCell[]>,
  options: { omitManifest?: string[]; rawManifest?: Record<string, string> } = {},
): string {
  const root = mkdtempSync(join(tmpdir(), "junit-gate-"));
  for (const [environment, cells] of Object.entries(environments)) {
    const envRoot = join(root, environment);
    mkdirSync(envRoot, { recursive: true });
    const lines: string[] = [];
    for (const cell of cells) {
      const tier = cell.tier ?? "L1";
      const pkg = cell.package ?? "test-pkg";
      const relative = cell.manifestXmlName ?? `${tier}/${pkg}.xml`;
      if (cell.xml !== null && cell.xml !== undefined) {
        mkdirSync(join(envRoot, tier), { recursive: true });
        writeFileSync(join(envRoot, `${tier}/${pkg}.xml`), cell.xml);
      }
      lines.push(
        JSON.stringify({
          tier,
          package: pkg,
          xml: relative,
          exit_code: cell.exit_code ?? 0,
          environment,
          duration_s: cell.duration_s ?? 30,
          report_present: cell.report_present ?? true,
        }),
      );
    }
    if (options.rawManifest?.[environment] !== undefined) {
      writeFileSync(join(envRoot, "manifest.jsonl"), options.rawManifest[environment]);
    } else if (!options.omitManifest?.includes(environment)) {
      writeFileSync(join(envRoot, "manifest.jsonl"), `${lines.join("\n")}\n`);
    }
  }
  return root;
}

const oneLeg = (overrides: Partial<EnvironmentConfig> = {}): EnvironmentConfig[] => [
  {
    name: "ubuntu-latest",
    kind: "native",
    os: "linux",
    cells: [{ tier: "L1", package: "test-pkg" }],
    pending: false,
    ...overrides,
  },
];

let tempDirs: string[] = [];
afterEach(() => {
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true });
  tempDirs = [];
});

function withTree(root: string, run: () => void): void {
  tempDirs.push(root);
  run();
}

// ---------------------------------------------------------------------------
// Real fixture — non-vacuity counterweight
// ---------------------------------------------------------------------------

describe("parseJunit", () => {
  it("passes_the_real_shipped_report", () => {
    const xml = readFileSync(join(__dirname, "../fixtures/junit/nextest-l1-excerpt.xml"), "utf8");
    const parsed = parseJunit(xml, "excerpt");
    expect(parsed.declaredTests).toBe(31);
    expect(parsed.cases.length).toBe(31);
    expect(parsed.wallSeconds).toBe(9.75);
    expect(parsed.cases.every((c) => c.outcome === "passed")).toBe(true);
  });

  it("counts_testcases_regardless_of_attribute_order", () => {
    const parsed = parseJunit(
      report(
        [
          `    <testcase time="0.100" classname="test-pkg::demo" name="ordered_last"></testcase>`,
          `    <testcase name="self_closing" classname="test-pkg::demo" time="0.200"/>`,
        ].join("\n"),
      ),
      "variants.xml",
    );
    expect(parsed.cases.map((c) => c.name).sort()).toEqual(["ordered_last", "self_closing"]);
    expect(parsed.cases.map((c) => c.seconds)).toEqual([0.1, 0.2]);
  });

  it("a_real_failure_element_marks_its_case", () => {
    const parsed = parseJunit(
      report(
        [
          `    <testcase name="broke" classname="test-pkg::demo" time="0.1">`,
          `      <failure message="assert">left != right</failure>`,
          `    </testcase>`,
          passingCase("fine"),
        ].join("\n"),
        { failures: 1 },
      ),
      "failing.xml",
    );
    expect(parsed.cases.find((c) => c.name === "broke")?.outcome).toBe("failed");
    expect(parsed.cases.find((c) => c.name === "fine")?.outcome).toBe("passed");
  });

  it("counts_retries_without_failing", () => {
    const parsed = parseJunit(
      report(
        [
          `    <testcase name="flaky" classname="test-pkg::demo" time="0.1">`,
          `      <flakyFailure/>`,
          `      <rerunFailure/>`,
          `    </testcase>`,
        ].join("\n"),
      ),
      "retries.xml",
    );
    expect(parsed.cases[0]?.outcome).toBe("passed");
    expect(parsed.cases[0]?.retries).toBe(2);
  });

  // ---------------------------------------------------------------------------
  // Rejection class 1 — malformed reports
  // ---------------------------------------------------------------------------

  it("rejects_a_document_that_is_not_a_junit_report", () => {
    expect(() => parseJunit("<html><body>404: Not Found</body></html>", "html")).toThrow(MalformedInput);
  });

  it("rejects_an_unterminated_tag", () => {
    expect(() => parseJunit('<testsuites time="1.0" tests="0"', "truncated")).toThrow(MalformedInput);
  });

  it("rejects_mismatched_close_tags", () => {
    expect(() =>
      parseJunit('<testsuites time="1.0" tests="0"><testsuite name="a"></testsuites></testsuite>', "mismatched"),
    ).toThrow(MalformedInput);
  });

  // ---------------------------------------------------------------------------
  // Rejection class 2 — invalid durations
  // ---------------------------------------------------------------------------

  it("rejects_a_testcase_with_no_duration", () => {
    expect(() => parseJunit(report(`    <testcase name="untimed" classname="test-pkg::demo"/>`), "untimed")).toThrow(
      MalformedInput,
    );
  });

  it("rejects_a_non_numeric_duration", () => {
    expect(() => parseJunit(report(passingCase("weird", "fast")), "nonnumeric")).toThrow(MalformedInput);
  });

  it("rejects_a_negative_duration", () => {
    expect(() => parseJunit(report(passingCase("negative", "-1.0")), "negative")).toThrow(MalformedInput);
  });

  it("accepts_a_zero_duration_boundary", () => {
    const parsed = parseJunit(report(passingCase("instant", "0.000")), "zero");
    expect(parsed.cases[0]?.seconds).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Manifest parsing
// ---------------------------------------------------------------------------

describe("parseManifest", () => {
  it("parses_a_stage_junit_manifest_record", () => {
    const [record] = parseManifest(
      '{"tier":"L1","package":"test-pkg","xml":"L1/test-pkg.xml","exit_code":0,' +
        '"environment":"ubuntu-latest","duration_s":41,"report_present":true}\n',
      "manifest",
    );
    expect(record?.tier).toBe("L1");
    expect(record?.duration_s).toBe(41);
    expect(record?.report_present).toBe(true);
  });

  it("ignores_blank_manifest_lines", () => {
    const records = parseManifest(
      '\n{"tier":"L1","package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":1,"report_present":true}\n\n',
      "manifest",
    );
    expect(records.length).toBe(1);
  });

  it("rejects_manifest_lines_that_are_not_json", () => {
    expect(() => parseManifest("not json\n", "manifest")).toThrow(MalformedInput);
  });

  it("rejects_manifest_records_with_wrong_field_types", () => {
    expect(() =>
      parseManifest(
        '{"tier":1,"package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":1,"report_present":true}\n',
        "manifest",
      ),
    ).toThrow(MalformedInput);
  });
});

// ---------------------------------------------------------------------------
// Collection and validation
// ---------------------------------------------------------------------------

describe("collect", () => {
  it("passes_a_complete_tree", () => {
    const root = tree({
      "ubuntu-latest": [
        {
          xml: readFileSync(join(__dirname, "../fixtures/junit/nextest-l1-excerpt.xml"), "utf8"),
          duration_s: 40,
        },
      ],
    });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "excerpt",
        environments: oneLeg(),
        expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
      };
      const result = collect(input);
      expect(result.violations).toEqual([]);
      expect(result.measurements.length).toBe(1);
      expect(result.measurements[0]?.tests).toBe(31);
      expect(result.measurements[0]?.runnerSeconds).toBe(9.75);
    });
  });

  it("rejects_a_missing_environment_leg", () => {
    const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")) }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "one-leg",
        environments: [
          ...oneLeg(),
          { name: "windows-latest", kind: "native", os: "windows", cells: [{ tier: "L1", package: "test-pkg" }], pending: false },
        ],
        expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
      };
      const result = collect(input);
      const missing = result.violations.filter((v) => v.kind === "missing-artifact");
      expect(missing.length).toBe(1);
      expect(missing[0]?.environment).toBe("windows-latest");
    });
  });

  it("reports_a_declared_pending_leg_as_pending", () => {
    const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")) }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "pending",
        environments: [
          ...oneLeg(),
          { name: "wsl2-ubuntu", kind: "wsl", os: "linux", cells: [{ tier: "L1", package: "test-pkg" }], pending: true },
        ],
        expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
      };
      const result = collect(input);
      expect(result.violations).toEqual([]);
      expect(result.pending).toEqual(["wsl2-ubuntu"]);
    });
  });

  it("rejects_a_nonzero_exit_code", () => {
    const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")), exit_code: 100 }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "exit",
        environments: oneLeg(),
        expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
      };
      const result = collect(input);
      expect(result.violations.some((v) => v.kind === "failed-run" && v.detail.includes("exited 100"))).toBe(true);
    });
  });

  it("rejects_duplicate_identities", () => {
    const root = tree({
      "ubuntu-latest": [{ xml: report([passingCase("twin"), passingCase("twin")].join("\n")) }],
    });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "dupes",
        environments: oneLeg(),
        expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
      };
      const result = collect(input);
      expect(result.violations.some((v) => v.kind === "duplicate-identity")).toBe(true);
    });
  });

  it("rejects_a_missing_required_test", () => {
    const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("still_here")) }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "shrunk",
        environments: oneLeg(),
        expectations: {
          requiredTests: ["still_here", "quietly_dropped"],
          platformExclusions: {},
          timeoutFloors: {},
          enforceTimeoutFloors: false,
        },
      };
      const result = collect(input);
      const missing = result.violations.filter((v) => v.kind === "missing-test");
      expect(missing.length).toBe(1);
      expect(missing[0]?.detail).toContain("quietly_dropped");
    });
  });

  it("platform_exclusion_excuses_absence", () => {
    const root = tree({ "windows-latest": [{ xml: report(passingCase("portable")) }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "win",
        environments: [
          { name: "windows-latest", kind: "native", os: "windows", cells: [{ tier: "L1", package: "test-pkg" }], pending: false },
        ],
        expectations: {
          requiredTests: ["portable", "unix_only"],
          platformExclusions: { "windows-latest": ["unix_only"] },
          timeoutFloors: {},
          enforceTimeoutFloors: false,
        },
      };
      const result = collect(input);
      expect(result.violations).toEqual([]);
      expect(result.exclusions).toEqual([{ environment: "windows-latest", test: "unix_only" }]);
    });
  });

  it("stale_exclusion_is_a_violation", () => {
    const root = tree({ "windows-latest": [{ xml: report(passingCase("now_portable")) }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "stale",
        environments: [
          { name: "windows-latest", kind: "native", os: "windows", cells: [{ tier: "L1", package: "test-pkg" }], pending: false },
        ],
        expectations: {
          requiredTests: [],
          platformExclusions: { "windows-latest": ["now_portable"] },
          timeoutFloors: {},
          enforceTimeoutFloors: false,
        },
      };
      const result = collect(input);
      expect(result.violations.some((v) => v.kind === "stale-exclusion")).toBe(true);
    });
  });

  it("timeout_floor_miss_only_fails_when_enforced", () => {
    const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("slow_test", "9.000")) }] });
    withTree(root, () => {
      const lenient: CollectionInput = {
        dir: root,
        label: "lenient",
        environments: oneLeg(),
        expectations: {
          requiredTests: [],
          platformExclusions: {},
          timeoutFloors: { slow_test: { budget: 0.5, tick: 0.1 } },
          enforceTimeoutFloors: false,
        },
      };
      expect(collect(lenient).violations).toEqual([]);

      const strict: CollectionInput = { ...lenient, label: "strict", expectations: { ...lenient.expectations, enforceTimeoutFloors: true } };
      expect(collect(strict).violations.some((v) => v.kind === "timeout-floor-miss")).toBe(true);
    });
  });

  it("wsl2_gets_one_extra_second_on_timeout_floors", () => {
    const within = report(passingCase("test", "2.500"));
    const root = tree({ "ubuntu-latest": [{ xml: within }], "wsl2-ubuntu": [{ xml: within }] });
    withTree(root, () => {
      const input: CollectionInput = {
        dir: root,
        label: "legs",
        environments: [
          ...oneLeg(),
          { name: "wsl2-ubuntu", kind: "wsl", os: "linux", cells: [{ tier: "L1", package: "test-pkg" }], pending: false },
        ],
        expectations: {
          requiredTests: [],
          platformExclusions: {},
          timeoutFloors: { test: { budget: 0.5, tick: 0.1 } },
          enforceTimeoutFloors: true,
        },
      };
      const result = collect(input);
      // 2.5 s misses the 1.6 s native bound but clears the 2.6 s WSL2 one.
      expect(result.violations.map((v) => v.environment)).toEqual(["ubuntu-latest"]);
    });
  });
});

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

describe("compare", () => {
  it("matches_identities_within_one_environment_only", () => {
    const before = tree({
      "ubuntu-latest": [{ xml: report(passingCase("a", "1.000") + "\n" + passingCase("b", "2.000")) }],
    });
    const after = tree({
      "ubuntu-latest": [{ xml: report(passingCase("a", "0.500") + "\n" + passingCase("c", "4.000")) }],
      "windows-latest": [{ xml: report(passingCase("b", "9.000")) }],
    });
    withTree(before, () =>
      withTree(after, () => {
        const mkInput = (dir: string, envs: EnvironmentConfig[]): CollectionInput => ({
          dir,
          label: dir,
          environments: envs,
          expectations: { requiredTests: [], platformExclusions: {}, timeoutFloors: {}, enforceTimeoutFloors: false },
        });
        const baselineEnvs = [
          ...oneLeg(),
          {
            name: "windows-latest",
            kind: "native" as const,
            os: "windows" as const,
            cells: [{ tier: "L1", package: "test-pkg" }],
            pending: true,
          },
        ];
        const candidateEnvs = [
          ...oneLeg(),
          { name: "windows-latest", kind: "native" as const, os: "windows" as const, cells: [{ tier: "L1", package: "test-pkg" }], pending: false },
        ];
        const comparisons = compare(collect(mkInput(before, baselineEnvs)), collect(mkInput(after, candidateEnvs)));
        expect(comparisons.length).toBe(1);
        const [ubuntu] = comparisons;
        expect(ubuntu?.environment).toBe("ubuntu-latest");
        expect(ubuntu?.matched).toBe(1);
        expect(ubuntu?.baselineMatchedSeconds).toBe(1);
        expect(ubuntu?.candidateMatchedSeconds).toBe(0.5);
        expect(ubuntu?.added).toEqual(["test-pkg::demo::c"]);
        expect(ubuntu?.addedSeconds).toBe(4);
        expect(ubuntu?.removed).toEqual(["test-pkg::demo::b"]);
        expect(ubuntu?.removedSeconds).toBe(2);
      }),
    );
  });
});
