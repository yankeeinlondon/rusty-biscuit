import { describe, it, expect } from "vitest";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  checkInvocation,
  decidedResults,
  attribute,
  attributeByBinary,
  deriveBudgets,
  invocationFromJunit,
  parseAndCheckLogs,
  REQUIRED_RUNS_PER_LEG,
  type BudgetInput,
} from "../src/attribute/index.ts";
import { MalformedInput } from "../src/errors.ts";
import { parseJunit } from "../src/junit/parse.ts";
import type { Invocation } from "../src/nextest-log.ts";
import type { Family } from "../src/reconcile/families.ts";

const TWO_FAMILIES: Family[] = [
  {
    id: "alpha",
    package: "demo",
    match: { suites: ["demo::alpha"] },
  },
  {
    id: "beta",
    package: "demo",
    match: { suites: ["demo::beta"] },
  },
];

describe("attribute", () => {
  describe("checkInvocation", () => {
    it("accepts a clean passing run", () => {
      const inv: Invocation = {
        results: [
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test_one",
            status: "PASS",
            durationS: 0.1,
            attempt: 1,
          },
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test_two",
            status: "PASS",
            durationS: 0.2,
            attempt: 1,
          },
        ],
        summary: { run: 2, passed: 2, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 0.3 },
        slowMarks: 0,
      };

      const violations = checkInvocation(inv, "test");
      expect(violations).toHaveLength(0);
    });

    it("rejects count mismatch between results and summary", () => {
      const inv: Invocation = {
        results: [
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test",
            status: "PASS",
            durationS: 0.1,
            attempt: 1,
          },
        ],
        summary: { run: 2, passed: 2, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 0.1 },
        slowMarks: 0,
      };

      const violations = checkInvocation(inv, "test");
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("count-mismatch");
    });

    it("rejects duplicate identity at same attempt", () => {
      const inv: Invocation = {
        results: [
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test",
            status: "PASS",
            durationS: 0.1,
            attempt: 1,
          },
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test",
            status: "PASS",
            durationS: 0.2,
            attempt: 1,
          },
        ],
        summary: { run: 1, passed: 1, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 0.3 },
        slowMarks: 0,
      };

      const violations = checkInvocation(inv, "test");
      expect(violations.some((v) => v.kind === "duplicate-identity")).toBe(true);
    });

    it("rejects runs with failed tests", () => {
      const inv: Invocation = {
        results: [
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "test",
            status: "FAIL",
            durationS: 0.1,
            attempt: 1,
          },
        ],
        summary: { run: 1, passed: 0, failed: 1, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 0.1 },
        slowMarks: 0,
      };

      const violations = checkInvocation(inv, "test");
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("failed-run");
    });
  });

  describe("decidedResults", () => {
    it("returns the final attempt for each identity", () => {
      const inv: Invocation = {
        results: [
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "flaky",
            status: "FAIL",
            durationS: 0.1,
            attempt: 1,
          },
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "flaky",
            status: "FAIL",
            durationS: 0.15,
            attempt: 2,
          },
          {
            binaryId: "demo::alpha",
            pkg: "demo",
            name: "flaky",
            status: "PASS",
            durationS: 0.2,
            attempt: 3,
          },
        ],
        summary: { run: 1, passed: 1, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 1, elapsedS: 0.45 },
        slowMarks: 0,
      };

      const decided = decidedResults(inv);
      expect(decided).toHaveLength(1);
      expect(decided[0]!.attempt).toBe(3);
      expect(decided[0]!.durationS).toBe(0.2);
    });
  });

  describe("attribute", () => {
    it("totals cost per family", () => {
      const invocations: Invocation[] = [
        {
          results: [
            {
              binaryId: "demo::alpha",
              pkg: "demo",
              name: "test_one",
              status: "PASS",
              durationS: 1.0,
              attempt: 1,
            },
            {
              binaryId: "demo::alpha",
              pkg: "demo",
              name: "test_two",
              status: "PASS",
              durationS: 2.0,
              attempt: 1,
            },
            {
              binaryId: "demo::beta",
              pkg: "demo",
              name: "test_three",
              status: "PASS",
              durationS: 0.5,
              attempt: 1,
            },
          ],
          summary: { run: 3, passed: 3, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 3.5 },
          slowMarks: 0,
        },
      ];

      const result = attribute(invocations, TWO_FAMILIES);
      expect(result.families).toHaveLength(2);

      const alpha = result.families.find((f) => f.id === "alpha");
      expect(alpha).toBeDefined();
      expect(alpha!.count).toBe(2);
      expect(alpha!.summed).toBe(3.0);
      expect(alpha!.mean).toBe(1.5);
      expect(alpha!.max).toBe(2.0);

      const beta = result.families.find((f) => f.id === "beta");
      expect(beta).toBeDefined();
      expect(beta!.count).toBe(1);
      expect(beta!.summed).toBe(0.5);
    });

    it("reports unassigned identities as violations", () => {
      const invocations: Invocation[] = [
        {
          results: [
            {
              binaryId: "unknown::test",
              pkg: "unknown",
              name: "orphan",
              status: "PASS",
              durationS: 1.0,
              attempt: 1,
            },
          ],
          summary: { run: 1, passed: 1, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 1.0 },
          slowMarks: 0,
        },
      ];

      const result = attribute(invocations, TWO_FAMILIES);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.kind).toBe("unassigned-identity");
    });
  });

  describe("attributeByBinary", () => {
    it("totals cost per binary", () => {
      const invocations: Invocation[] = [
        {
          results: [
            {
              binaryId: "demo::alpha",
              pkg: "demo",
              name: "test_one",
              status: "PASS",
              durationS: 1.0,
              attempt: 1,
            },
            {
              binaryId: "demo::alpha",
              pkg: "demo",
              name: "test_two",
              status: "PASS",
              durationS: 2.0,
              attempt: 1,
            },
            {
              binaryId: "demo::beta",
              pkg: "demo",
              name: "test",
              status: "PASS",
              durationS: 0.5,
              attempt: 1,
            },
          ],
          summary: { run: 3, passed: 3, failed: 0, timedOut: 0, skipped: 0, slow: 0, leaky: 0, flaky: 0, elapsedS: 3.5 },
          slowMarks: 0,
        },
      ];

      const costs = attributeByBinary(invocations);
      expect(costs).toHaveLength(2);

      const alpha = costs.find((c) => c.binaryId === "demo::alpha");
      expect(alpha).toBeDefined();
      expect(alpha!.count).toBe(2);
      expect(alpha!.summed).toBe(3.0);
      expect(alpha!.mean).toBe(1.5);
    });
  });

  describe("deriveBudgets", () => {
    it("refuses local-derived budgets", () => {
      const input: BudgetInput = {
        provenance: {
          kind: "local",
          legs: ["macos"],
          runsPerLeg: { macos: 3 },
        },
        perLegFamilySummed: {
          macos: {
            alpha: [1.0, 1.1, 1.2],
          },
        },
      };

      const { budgets, violations } = deriveBudgets(input);
      expect(budgets).toHaveLength(0);
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("local-derived-budget");
    });

    it("refuses insufficient runs", () => {
      const input: BudgetInput = {
        provenance: {
          kind: "ci",
          legs: ["macos"],
          runsPerLeg: { macos: 2 },
        },
        perLegFamilySummed: {
          macos: {
            alpha: [1.0, 1.1],
          },
        },
      };

      const { budgets, violations } = deriveBudgets(input);
      expect(budgets).toHaveLength(0);
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("insufficient-runs");
      expect(violations[0]!.detail).toContain(`${REQUIRED_RUNS_PER_LEG} consecutive are required`);
    });

    it("derives budgets from CI runs with headroom", () => {
      const input: BudgetInput = {
        provenance: {
          kind: "ci",
          legs: ["macos"],
          runsPerLeg: { macos: 3 },
        },
        perLegFamilySummed: {
          macos: {
            alpha: [1.0, 1.2, 1.1],
          },
        },
        headroom: 0.25,
      };

      const { budgets, violations } = deriveBudgets(input);
      expect(violations).toHaveLength(0);
      expect(budgets).toHaveLength(1);
      expect(budgets[0]!.leg).toBe("macos");
      expect(budgets[0]!.family).toBe("alpha");
      expect(budgets[0]!.observedMax).toBe(1.2);
      expect(budgets[0]!.budget).toBe(Math.ceil(1.2 * 1.25 * 100) / 100);
    });

    it("rejects invalid headroom", () => {
      const input: BudgetInput = {
        provenance: {
          kind: "ci",
          legs: ["macos"],
          runsPerLeg: { macos: 3 },
        },
        perLegFamilySummed: {
          macos: {
            alpha: [1.0, 1.1, 1.2],
          },
        },
        headroom: -0.1,
      };

      const { budgets, violations } = deriveBudgets(input);
      expect(budgets).toHaveLength(0);
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("invalid-headroom");
    });
  });
});

describe("attribute over JUnit reports", () => {
  const junit = (cases: string, tests: number, time = "47.090") =>
    [
      '<?xml version="1.0" encoding="UTF-8"?>',
      `<testsuites name="nextest-run" tests="${tests}" skipped="0" failures="0" errors="0" uuid="u" timestamp="2026-09-06T04:38:49.257+00:00" time="${time}">`,
      '    <testsuite name="demo::alpha" tests="1" skipped="0" errors="0" failures="0">',
      cases,
      "    </testsuite>",
      "</testsuites>",
    ].join("\n");

  it("projects a CI cell onto the console-log invocation shape", () => {
    const xml = junit(
      [
        '<testcase name="test_one" classname="demo::alpha" time="0.250"/>',
        '<testcase name="test_two" classname="demo::alpha" time="1.500"><flakyFailure message="x"/></testcase>',
        '<testcase name="test_three" classname="demo::alpha" time="0.000"><skipped/></testcase>',
      ].join("\n"),
      3
    );
    const inv = invocationFromJunit(parseJunit(xml, "cell.xml"), "cell");

    expect(inv.summary).toEqual({
      run: 2,
      passed: 2,
      failed: 0,
      timedOut: 0,
      skipped: 1,
      slow: 0,
      leaky: 0,
      flaky: 1,
      elapsedS: 47.09,
    });
    expect(inv.declared).toEqual({ tests: 3, binaries: 0, skipped: 1 });
    expect(inv.results.map((r) => [r.binaryId, r.pkg, r.name, r.status, r.durationS, r.attempt])).toEqual([
      ["demo::alpha", "demo", "test_one", "PASS", 0.25, 1],
      ["demo::alpha", "demo", "test_two", "PASS", 1.5, 2],
    ]);
    // The projection feeds the same gate and matcher as a console log.
    expect(checkInvocation(inv, "cell")).toHaveLength(0);
    const result = attribute([inv], TWO_FAMILIES);
    expect(result.violations).toHaveLength(0);
    expect(result.count).toBe(2);
    expect(result.summed).toBeCloseTo(1.75, 6);
    expect(result.elapsed).toBeCloseTo(47.09, 6);
    expect(result.families[0]).toMatchObject({ id: "alpha", count: 2, max: 1.5 });
  });

  it("keeps a red cell from measuring anything", () => {
    const xml = junit(
      [
        '<testcase name="ok" classname="demo::alpha" time="0.1"/>',
        '<testcase name="broken" classname="demo::alpha" time="0.2"><failure message="boom"/></testcase>',
        '<testcase name="crashed" classname="demo::alpha" time="0.3"><error message="abort"/></testcase>',
      ].join("\n"),
      3
    );
    const inv = invocationFromJunit(parseJunit(xml, "cell.xml"), "cell");
    expect(inv.results.map((r) => r.status)).toEqual(["PASS", "FAIL", "ABORT"]);
    const kinds = checkInvocation(inv, "cell").map((v) => v.kind);
    expect(kinds).toEqual(["failed-run", "failed-run"]);
  });

  it("rejects a report whose declared count disagrees with its cases", () => {
    const xml = junit('<testcase name="only" classname="demo::alpha" time="0.1"/>', 5);
    expect(() => invocationFromJunit(parseJunit(xml, "cell.xml"), "cell")).toThrow(MalformedInput);
  });

  it("routes .xml paths through the JUnit adapter and .log paths through the console adapter", () => {
    const dir = mkdtempSync(join(tmpdir(), "test-audit-attr-"));
    try {
      const xmlPath = join(dir, "darkmatter-cli.xml");
      writeFileSync(
        xmlPath,
        junit('<testcase name="test_one" classname="demo::alpha" time="0.250"/>', 1, "0.300")
      );
      const logPath = join(dir, "run.log");
      writeFileSync(
        logPath,
        [
          "        PASS [   0.400s] (1/1) demo::beta test_two",
          "------------",
          "     Summary [   0.500s] 1 test run: 1 passed, 0 skipped",
          "",
        ].join("\n")
      );
      const { invocations, violations } = parseAndCheckLogs([xmlPath, logPath]);
      expect(violations).toHaveLength(0);
      expect(invocations).toHaveLength(2);
      const result = attribute(invocations, TWO_FAMILIES);
      expect(result.violations).toHaveLength(0);
      expect(result.families.map((f) => [f.id, f.summed])).toEqual([
        ["beta", 0.4],
        ["alpha", 0.25],
      ]);
      expect(result.elapsed).toBeCloseTo(0.8, 6);

      const badPath = join(dir, "broken.xml");
      writeFileSync(badPath, "<testsuites><testsuite><testcase name='x'");
      const bad = parseAndCheckLogs([badPath]);
      expect(bad.invocations).toHaveLength(0);
      expect(bad.violations.map((v) => v.kind)).toEqual(["malformed-report"]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
