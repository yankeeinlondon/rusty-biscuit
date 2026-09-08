#!/usr/bin/env -S npx tsx --test
/**
 * Tests for this fix's baseline gate.
 *
 * Run with `npx tsx --test junit-metrics.test.ts` from this directory.
 * `node:test` rather than Vitest because the monorepo ships no JavaScript test
 * runner and this fix is not the place to introduce one; `tsx` is already how
 * the predecessor's script was invoked.
 *
 * The gate's whole value is that it fails, so almost every case here asserts a
 * rejection. `passes_the_real_shipped_report` and the fixtures corpus test are
 * the counterweight: they run genuine nextest output through the normal path,
 * so a detector that rejects everything cannot pass this file.
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  rmSync,
  readdirSync,
  existsSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

import {
  parseJunit,
  parseManifest,
  parseArgs,
  collect,
  renderTable,
  main,
  MalformedReport,
  MalformedManifest,
  DEFAULT_EXPECTATIONS,
  type Expectations,
  type Violation,
  type ViolationKind,
} from "./junit-metrics.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(HERE, "fixtures");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** A minimal well-formed nextest report, in nextest's own attribute order. */
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
    '  <testsuite name="claudine-cli::demo" tests="1" disabled="0" errors="0" failures="0">',
    cases,
    "  </testsuite>",
    "</testsuites>",
  ].join("\n");
}

const passingCase = (name: string, time = "0.100") =>
  `    <testcase name="${name}" classname="claudine-cli::demo" time="${time}"></testcase>`;

interface TreeCell {
  tier?: string;
  package?: string;
  xml?: string | null;
  exit_code?: number;
  duration_s?: number;
  report_present?: boolean;
  manifestXmlName?: string;
}

/** Build a `_stage_junit`-shaped artifact tree in a throwaway directory. */
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
      const pkg = cell.package ?? "claudine-cli";
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

const oneLeg = (expect: Partial<Expectations> = {}): Expectations => ({
  ...DEFAULT_EXPECTATIONS,
  environments: ["ubuntu-latest"],
  ...expect,
});

const kinds = (violations: Violation[]): ViolationKind[] => violations.map((v) => v.kind);

function withTree(root: string, run: () => void): void {
  try {
    run();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

// ---------------------------------------------------------------------------
// Real shipped artifacts — the non-vacuity counterweight
// ---------------------------------------------------------------------------

test("passes_the_real_shipped_report", () => {
  const root = tree({
    "ubuntu-latest": [
      {
        xml: readFixture("nextest-l1-excerpt.xml"),
        duration_s: 40,
      },
    ],
  });
  withTree(root, () => {
    const baseline = collect(root, "excerpt", oneLeg());
    assert.deepEqual(baseline.violations, []);
    assert.equal(baseline.measurements.length, 1);
    const [measurement] = baseline.measurements;
    assert.equal(measurement.tests, 31);
    assert.equal(measurement.failures, 0);
    assert.equal(measurement.runnerSeconds, 9.75);
    // 40 s of invocation minus 9.75 s of run is the compile/setup share.
    assert.equal(Number(measurement.buildSeconds.toFixed(2)), 30.25);
    // Summed duration is the parallel work, so it must not read as elapsed.
    assert.ok(measurement.summedSeconds > 0);
    assert.notEqual(measurement.summedSeconds, measurement.runnerSeconds);
  });
});

test("every_shipped_fixture_parses_and_reconciles_its_own_declared_count", () => {
  const files = readdirSync(FIXTURES).filter((name) => name.endsWith(".xml"));
  assert.ok(files.length > 0, "fixtures corpus is empty; the corpus test proves nothing");
  for (const file of files) {
    const parsed = parseJunit(readFixture(file), file);
    assert.equal(parsed.declaredTests, parsed.cases.length, `${file} count mismatch`);
    for (const testCase of parsed.cases) {
      assert.ok(Number.isFinite(testCase.seconds), `${file}: ${testCase.identity} has no duration`);
      assert.ok(testCase.identity.includes("::"), `${file}: ${testCase.identity} has no binary`);
    }
  }
});

test("reads_the_hosts_own_staged_report_when_one_is_present", () => {
  const staged = join(HERE, "..", "..", "..", "target", "nextest", "ci-reports", "L1", "claudine-cli.xml");
  if (!existsSync(staged)) return; // no local CI-profile run on this host
  const parsed = parseJunit(readFixture(staged, true), staged);
  assert.equal(parsed.declaredTests, parsed.cases.length);
  assert.ok(parsed.wallSeconds > 0);
});

function readFixture(name: string, absolute = false): string {
  return readFileSync(absolute ? name : join(FIXTURES, name), "utf8");
}

// ---------------------------------------------------------------------------
// Representation variants the predecessor's regex could not see
// ---------------------------------------------------------------------------

test("counts_testcases_regardless_of_attribute_order_or_quoting", () => {
  const parsed = parseJunit(
    report(
      [
        `    <testcase time="0.100" classname="claudine-cli::demo" name="ordered_last"></testcase>`,
        `    <testcase name='single_quoted' classname='claudine-cli::demo' time='0.200'/>`,
        `    <testcase classname="claudine-cli::demo" time="0.300" name="self_closing"/>`,
      ].join("\n"),
    ),
    "variants.xml",
  );
  assert.deepEqual(
    parsed.cases.map((c) => c.name).sort(),
    ["ordered_last", "self_closing", "single_quoted"],
  );
  assert.deepEqual(parsed.cases.map((c) => c.seconds).sort(), [0.1, 0.2, 0.3]);
  assert.ok(parsed.cases.every((c) => c.outcome === "passed"));
});

test("decodes_xml_entities_in_test_identities", () => {
  const parsed = parseJunit(
    report(`    <testcase name="a_&lt;b&gt;_&amp;_c" classname="claudine-cli::demo" time="0.1"/>`),
    "entities.xml",
  );
  assert.equal(parsed.cases[0].name, "a_<b>_&_c");
  assert.equal(parsed.cases[0].identity, "claudine-cli::demo::a_<b>_&_c");
});

test("a_failure_element_quoted_inside_captured_output_is_not_a_failure", () => {
  const parsed = parseJunit(
    report(
      [
        `    <testcase name="prints_markup" classname="claudine-cli::demo" time="0.1">`,
        `      <system-out>assertion text mentioning &lt;failure&gt; and a literal <failure/> tag</system-out>`,
        `    </testcase>`,
      ].join("\n"),
    ),
    "quoted.xml",
  );
  assert.equal(parsed.cases.length, 1);
  assert.equal(parsed.cases[0].outcome, "passed");
});

test("a_real_failure_element_marks_its_case", () => {
  const parsed = parseJunit(
    report(
      [
        `    <testcase name="broke" classname="claudine-cli::demo" time="0.1">`,
        `      <failure message="assert">left != right</failure>`,
        `    </testcase>`,
        passingCase("fine"),
      ].join("\n"),
      { failures: 1 },
    ),
    "failing.xml",
  );
  assert.equal(parsed.cases.find((c) => c.name === "broke")?.outcome, "failed");
  assert.equal(parsed.cases.find((c) => c.name === "fine")?.outcome, "passed");
});

// ---------------------------------------------------------------------------
// Rejection class 1 — malformed reports
// ---------------------------------------------------------------------------

test("rejects_a_document_that_is_not_a_junit_report", () => {
  assert.throws(
    () => parseJunit("<html><body>404: Not Found</body></html>", "html"),
    (error: Error) => error instanceof MalformedReport && /not a JUnit report/.test(error.message),
  );
});

test("rejects_an_unterminated_tag", () => {
  assert.throws(
    () => parseJunit('<testsuites time="1.0" tests="0"', "truncated"),
    (error: Error) => error instanceof MalformedReport && /unterminated tag/.test(error.message),
  );
});

test("rejects_a_truncated_download_with_an_unclosed_element", () => {
  const whole = report(passingCase("ok"));
  assert.throws(
    () => parseJunit(whole.slice(0, whole.indexOf("</testsuite>")), "truncated"),
    (error: Error) => error instanceof MalformedReport && /unclosed/.test(error.message),
  );
});

test("rejects_mismatched_close_tags", () => {
  assert.throws(
    () =>
      parseJunit(
        '<testsuites time="1.0" tests="0"><testsuite name="a"></testsuites></testsuite>',
        "mismatched",
      ),
    (error: Error) => error instanceof MalformedReport && /closes </.test(error.message),
  );
});

test("rejects_a_testcase_outside_any_testsuites_root", () => {
  assert.throws(
    () => parseJunit('<testcase name="orphan" time="0.1"/>', "orphan"),
    MalformedReport,
  );
});

// ---------------------------------------------------------------------------
// Rejection class 2 — invalid durations
// ---------------------------------------------------------------------------

test("rejects_a_testcase_with_no_duration_rather_than_scoring_it_zero", () => {
  assert.throws(
    () => parseJunit(report(`    <testcase name="untimed" classname="claudine-cli::demo"/>`), "untimed"),
    (error: Error) => error instanceof MalformedReport && /has no value/.test(error.message),
  );
});

test("rejects_a_non_numeric_duration", () => {
  assert.throws(
    () => parseJunit(report(passingCase("weird", "fast")), "nonnumeric"),
    (error: Error) => error instanceof MalformedReport && /not a number/.test(error.message),
  );
});

test("rejects_a_negative_duration", () => {
  assert.throws(
    () => parseJunit(report(passingCase("negative", "-1.0")), "negative"),
    (error: Error) => error instanceof MalformedReport && /negative/.test(error.message),
  );
});

test("accepts_a_zero_duration_boundary", () => {
  const parsed = parseJunit(report(passingCase("instant", "0.000")), "zero");
  assert.equal(parsed.cases[0].seconds, 0);
});

test("classifies_a_bad_duration_apart_from_other_malformation", () => {
  const duration = tree({ "ubuntu-latest": [{ xml: report(passingCase("x", "soon")) }] });
  withTree(duration, () => {
    assert.deepEqual(kinds(collect(duration, "d", oneLeg()).violations), ["invalid-duration"]);
  });

  // A malformed document that happens to mention "time" must not be
  // reclassified as a duration problem by message text.
  const shape = tree({
    "ubuntu-latest": [{ xml: '<html><body>timed out fetching artifact</body></html>' }],
  });
  withTree(shape, () => {
    assert.deepEqual(kinds(collect(shape, "s", oneLeg()).violations), ["malformed-report"]);
  });
});

test("rejects_a_manifest_duration_below_the_runs_own_elapsed", () => {
  const root = tree({
    "ubuntu-latest": [{ xml: report(passingCase("ok"), { time: "170.5" }), duration_s: 12 }],
  });
  withTree(root, () => {
    const baseline = collect(root, "skewed", oneLeg());
    assert.ok(kinds(baseline.violations).includes("invalid-duration"));
  });
});

test("tolerates_whole_second_manifest_rounding_below_elapsed", () => {
  const root = tree({
    "ubuntu-latest": [{ xml: report(passingCase("ok"), { time: "30.4" }), duration_s: 30 }],
  });
  withTree(root, () => {
    const baseline = collect(root, "rounded", oneLeg());
    assert.deepEqual(baseline.violations, []);
    assert.equal(baseline.measurements[0].buildSeconds, 0);
  });
});

// ---------------------------------------------------------------------------
// Rejection class 3 — duplicate identities
// ---------------------------------------------------------------------------

test("rejects_a_report_listing_one_identity_twice", () => {
  const root = tree({
    "ubuntu-latest": [
      { xml: report([passingCase("twin"), passingCase("twin")].join("\n")) },
    ],
  });
  withTree(root, () => {
    const baseline = collect(root, "dupes", oneLeg());
    assert.ok(kinds(baseline.violations).includes("duplicate-identity"));
    assert.match(
      baseline.violations.find((v) => v.kind === "duplicate-identity")!.detail,
      /claudine-cli::demo::twin/,
    );
  });
});

// ---------------------------------------------------------------------------
// Rejection class 4 — missing artifacts
// ---------------------------------------------------------------------------

test("rejects_a_missing_environment_leg", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")) }] });
  withTree(root, () => {
    const baseline = collect(root, "one-leg", {
      ...DEFAULT_EXPECTATIONS,
      environments: ["ubuntu-latest", "windows-latest"],
    });
    const missing = baseline.violations.filter((v) => v.kind === "missing-artifact");
    assert.equal(missing.length, 1);
    assert.equal(missing[0].environment, "windows-latest");
  });
});

test("reports_a_declared_pending_leg_as_pending_instead_of_missing", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")) }] });
  withTree(root, () => {
    const baseline = collect(root, "pending", {
      ...DEFAULT_EXPECTATIONS,
      environments: ["ubuntu-latest", "wsl2-ubuntu"],
      pendingEnvironments: ["wsl2-ubuntu"],
    });
    assert.deepEqual(baseline.violations, []);
    assert.deepEqual(baseline.pending, ["wsl2-ubuntu"]);
    assert.match(renderTable(baseline), /wsl2-ubuntu.*pending/);
  });
});

test("rejects_a_tree_with_no_staging_manifest", () => {
  const root = tree(
    { "ubuntu-latest": [{ xml: report(passingCase("ok")) }] },
    { omitManifest: ["ubuntu-latest"] },
  );
  withTree(root, () => {
    const baseline = collect(root, "no-manifest", oneLeg());
    assert.ok(
      baseline.violations.some((v) => /manifest\.jsonl absent/.test(v.detail)),
      "a tree without a manifest is not a staging directory",
    );
  });
});

test("rejects_a_manifest_record_whose_report_was_never_emitted", () => {
  const root = tree({ "ubuntu-latest": [{ xml: null, report_present: false }] });
  withTree(root, () => {
    const baseline = collect(root, "no-report", oneLeg());
    assert.ok(baseline.violations.some((v) => /report_present=false/.test(v.detail)));
  });
});

test("rejects_a_manifest_naming_an_xml_that_is_absent_from_the_artifact", () => {
  const root = tree({ "ubuntu-latest": [{ xml: null, report_present: true }] });
  withTree(root, () => {
    const baseline = collect(root, "dangling", oneLeg());
    assert.ok(baseline.violations.some((v) => /is absent from the artifact/.test(v.detail)));
  });
});

test("rejects_an_expected_cell_with_no_manifest_record", () => {
  const root = tree({ "ubuntu-latest": [{ tier: "L2", xml: report(passingCase("ok")) }] });
  withTree(root, () => {
    const baseline = collect(root, "wrong-tier", oneLeg());
    assert.ok(baseline.violations.some((v) => /no manifest record for L1/.test(v.detail)));
  });
});

test("rejects_a_missing_artifact_directory_outright", () => {
  assert.throws(
    () => collect(join(tmpdir(), "junit-gate-does-not-exist"), "gone", oneLeg()),
    /no such artifact directory/,
  );
});

// ---------------------------------------------------------------------------
// Rejection class 5 — missing tests
// ---------------------------------------------------------------------------

test("rejects_a_run_that_silently_stopped_executing_a_required_test", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("still_here")) }] });
  withTree(root, () => {
    const baseline = collect(
      root,
      "shrunk",
      oneLeg({ requiredTests: ["still_here", "quietly_dropped"] }),
    );
    const missing = baseline.violations.filter((v) => v.kind === "missing-test");
    assert.equal(missing.length, 1);
    assert.match(missing[0].detail, /quietly_dropped/);
  });
});

test("matches_a_required_test_given_as_a_full_identity", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("present")) }] });
  withTree(root, () => {
    const ok = collect(root, "id", oneLeg({ requiredTests: ["claudine-cli::demo::present"] }));
    assert.deepEqual(ok.violations, []);
    const bad = collect(root, "id", oneLeg({ requiredTests: ["claudine-cli::other::present"] }));
    assert.equal(bad.violations.filter((v) => v.kind === "missing-test").length, 1);
  });
});

// ---------------------------------------------------------------------------
// Rejection class 6 — failed runs
// ---------------------------------------------------------------------------

test("rejects_a_nonzero_exit_code_even_when_every_case_passed", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")), exit_code: 100 }] });
  withTree(root, () => {
    const baseline = collect(root, "exit", oneLeg());
    assert.ok(kinds(baseline.violations).includes("failed-run"));
    assert.match(baseline.violations[0].detail, /exited 100/);
  });
});

test("rejects_an_enumerated_failure", () => {
  const root = tree({
    "ubuntu-latest": [
      {
        xml: report(
          [
            `    <testcase name="broke" classname="claudine-cli::demo" time="0.1">`,
            `      <failure message="assert">boom</failure>`,
            `    </testcase>`,
          ].join("\n"),
          { failures: 1 },
        ),
      },
    ],
  });
  withTree(root, () => {
    const baseline = collect(root, "failure", oneLeg());
    assert.ok(baseline.violations.some((v) => /broke failed/.test(v.detail)));
  });
});

test("rejects_a_report_declaring_more_failures_than_it_enumerates", () => {
  const root = tree({
    "ubuntu-latest": [{ xml: report(passingCase("ok"), { failures: 3 }) }],
  });
  withTree(root, () => {
    const baseline = collect(root, "under-listed", oneLeg());
    assert.ok(baseline.violations.some((v) => /declares 3 failures/.test(v.detail)));
  });
});

test("rejects_a_report_whose_declared_test_count_disagrees_with_its_body", () => {
  const root = tree({
    "ubuntu-latest": [{ xml: report(passingCase("ok"), { tests: 2397 }) }],
  });
  withTree(root, () => {
    const baseline = collect(root, "truncated-body", oneLeg());
    assert.ok(baseline.violations.some((v) => /declares 2397 tests but enumerates 1/.test(v.detail)));
  });
});

test("counts_a_skipped_case_as_a_skip_and_not_a_failure", () => {
  const root = tree({
    "ubuntu-latest": [
      {
        xml: report(
          [
            `    <testcase name="not_run" classname="claudine-cli::demo" time="0.0">`,
            `      <skipped/>`,
            `    </testcase>`,
            passingCase("ran"),
          ].join("\n"),
        ),
      },
    ],
  });
  withTree(root, () => {
    const baseline = collect(root, "skips", oneLeg());
    assert.deepEqual(baseline.violations, []);
    assert.equal(baseline.measurements[0].skips, 1);
    assert.equal(baseline.measurements[0].failures, 0);
    assert.equal(baseline.measurements[0].tests, 2);
  });
});

// ---------------------------------------------------------------------------
// Timeout floors: reported by default, enforceable on demand
// ---------------------------------------------------------------------------

const OVER_FLOOR = "sequence_per_step_step_timeout_override";

test("a_timeout_floor_miss_is_only_printed_until_enforcement_is_asked_for", () => {
  // 0.5 s budget + 0.1 s tick + 1 s native allowance = 1.6 s bound.
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase(OVER_FLOOR, "9.000")) }] });
  withTree(root, () => {
    const lenient = collect(root, "lenient", oneLeg());
    assert.deepEqual(lenient.violations, []);

    const strict = collect(root, "strict", oneLeg({ enforceTimeoutFloors: true }));
    assert.deepEqual(kinds(strict.violations), ["timeout-floor-miss"]);
    assert.match(strict.violations[0].detail, /9\.0 s against a 1\.6 s bound/);
  });
});

test("wsl2_gets_one_extra_second_on_every_timeout_floor", () => {
  const within = report(passingCase(OVER_FLOOR, "2.500"));
  const root = tree({ "ubuntu-latest": [{ xml: within }], "wsl2-ubuntu": [{ xml: within }] });
  withTree(root, () => {
    const baseline = collect(root, "legs", {
      ...DEFAULT_EXPECTATIONS,
      environments: ["ubuntu-latest", "wsl2-ubuntu"],
      enforceTimeoutFloors: true,
    });
    // 2.5 s misses the 1.6 s native bound but clears the 2.6 s WSL2 one.
    assert.deepEqual(
      baseline.violations.map((v) => v.environment),
      ["ubuntu-latest"],
    );
  });
});

test("a_test_exactly_on_its_bound_is_not_a_miss", () => {
  const root = tree({ "ubuntu-latest": [{ xml: report(passingCase(OVER_FLOOR, "1.600")) }] });
  withTree(root, () => {
    assert.deepEqual(collect(root, "edge", oneLeg({ enforceTimeoutFloors: true })).violations, []);
  });
});

// ---------------------------------------------------------------------------
// Manifest parsing
// ---------------------------------------------------------------------------

test("parses_a_stage_junit_manifest_record", () => {
  const [record] = parseManifest(
    '{"tier":"L1","package":"claudine-cli","xml":"L1/claudine-cli.xml","exit_code":0,' +
      '"environment":"ubuntu-latest","duration_s":41,"report_present":true}\n',
    "manifest",
  );
  assert.equal(record.tier, "L1");
  assert.equal(record.duration_s, 41);
  assert.equal(record.report_present, true);
});

test("ignores_blank_manifest_lines", () => {
  const records = parseManifest(
    '\n{"tier":"L1","package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":1,"report_present":true}\n\n',
    "manifest",
  );
  assert.equal(records.length, 1);
});

test("rejects_manifest_lines_that_are_not_json", () => {
  assert.throws(() => parseManifest("not json\n", "manifest"), MalformedManifest);
});

test("rejects_manifest_records_with_the_wrong_field_types", () => {
  for (const line of [
    '{"tier":1,"package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":1,"report_present":true}',
    '{"tier":"L1","package":"p","xml":"x","exit_code":"0","environment":"e","duration_s":1,"report_present":true}',
    '{"tier":"L1","package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":1,"report_present":"yes"}',
    '{"tier":"L1","package":"p","xml":"x","exit_code":0,"environment":"e","duration_s":-5,"report_present":true}',
  ]) {
    assert.throws(() => parseManifest(`${line}\n`, "manifest"), MalformedManifest, line);
  }
});

test("a_malformed_manifest_is_a_violation_not_a_crash", () => {
  const root = tree({ "ubuntu-latest": [] }, { rawManifest: { "ubuntu-latest": "{oops\n" } });
  withTree(root, () => {
    const baseline = collect(root, "bad-manifest", oneLeg());
    assert.ok(kinds(baseline.violations).includes("malformed-report"));
  });
});

// ---------------------------------------------------------------------------
// Determinism and the process contract
// ---------------------------------------------------------------------------

test("repeated_reads_of_one_tree_produce_an_identical_table", () => {
  const root = tree({
    "ubuntu-latest": [{ xml: readFixture("nextest-l1-excerpt.xml"), duration_s: 40 }],
    "macos-latest": [{ xml: readFixture("nextest-l1-excerpt.xml"), duration_s: 35 }],
  });
  withTree(root, () => {
    const expectations = {
      ...DEFAULT_EXPECTATIONS,
      environments: ["ubuntu-latest", "macos-latest"],
    };
    const first = renderTable(collect(root, "run", expectations));
    const second = renderTable(collect(root, "run", expectations));
    assert.equal(first, second);
    // Legs render in the declared order, not the filesystem's.
    assert.ok(first.indexOf("ubuntu-latest") < first.indexOf("macos-latest"));
  });
});

test("main_exits_zero_on_a_clean_tree_and_one_on_a_violation", () => {
  const clean = tree({ "ubuntu-latest": [{ xml: readFixture("nextest-l1-excerpt.xml"), duration_s: 40 }] });
  const expectPath = join(clean, "expect.json");
  writeFileSync(expectPath, JSON.stringify({ environments: ["ubuntu-latest"] }));
  withTree(clean, () => {
    assert.equal(main([clean, "--expect", expectPath, "--markdown"]), 0);
  });

  const dirty = tree({ "ubuntu-latest": [{ xml: report(passingCase("ok")), exit_code: 1 }] });
  const dirtyExpect = join(dirty, "expect.json");
  writeFileSync(dirtyExpect, JSON.stringify({ environments: ["ubuntu-latest"] }));
  withTree(dirty, () => {
    assert.equal(main([dirty, "--expect", dirtyExpect, "--markdown"]), 1);
  });
});

test("main_exits_two_on_a_usage_error", () => {
  assert.equal(main([]), 2);
  assert.equal(main(["--label"]), 2);
  assert.equal(main(["dir", "--nonsense"]), 2);
  assert.equal(main(["dir", "extra"]), 2);
});

test("parses_flags_independently_of_their_order", () => {
  assert.deepEqual(parseArgs(["--markdown", "d", "--label", "run 1"]), {
    dir: "d",
    label: "run 1",
    markdown: true,
    json: false,
  });
  assert.deepEqual(parseArgs(["d", "--json"]), { dir: "d", markdown: false, json: true });
});
