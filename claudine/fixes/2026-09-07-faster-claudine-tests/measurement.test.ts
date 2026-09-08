/**
 * Tests for `measurement.ts`.
 *
 * Run with `npx tsx --test measurement.test.ts` from this directory.
 * `node:test` rather than Vitest, matching the other gates in this directory.
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  parseLog,
  readLog,
  metricsForRun,
  spread,
  established,
  matchesCohort,
  matchesTarget,
  buildReport,
  readManifest,
  type RunRecord,
} from "./measurement.ts";

const PASSING_LOG = `
Testing [1mclaudine[0m in one local invocation

    Starting 4 tests across 2 binaries (1 test skipped)
        PASS [   0.018s] (   1/4) claudine actions::hook_action::tests::bash_type_labels
        SLOW [>  5.000s] (─────────) claudine composition::sequence::task::tests::slow_one
        PASS [   5.250s] (   2/4) claudine composition::sequence::task::tests::slow_one
        PASS [   0.300s] (   3/4) claudine-cli::context_command context_default_exits_zero
        LEAK [   0.100s] (   4/4) claudine-cli::context_command leaky_but_passing
     Summary [   6.000s] 4 tests run: 4 passed (1 slow, 1 leaky), 1 skipped
`;

const FAILING_LOG = `
    Starting 3 tests across 1 binary
        PASS [   0.010s] (   1/3) claudine-cli::loop_cli a
        TRY 1 FAIL [   0.020s] (   2/3) claudine-cli::loop_cli b
        PASS [   0.030s] (   2/3) claudine-cli::loop_cli b
     TIMEOUT [  30.000s] (   3/3) claudine-cli::loop_cli c
     Summary [  31.000s] 3 tests run: 2 passed (1 flaky), 1 timed out, 0 skipped

        FAIL [   0.020s] claudine-cli::loop_cli b
     TIMEOUT [  30.000s] claudine-cli::loop_cli c
`;

const MULTI_LOG = `
Testing rendezvous-core package
    Starting 2 tests across 1 binary
        PASS [   0.010s] (   1/2) rendezvous-core::a x
        PASS [   0.020s] (   2/2) rendezvous-core::a y
     Summary [   0.050s] 2 tests run: 2 passed, 0 skipped
Testing rendezvous-daemon package
    Starting 1 test across 1 binary (2 tests skipped)
        PASS [   1.000s] (   1/1) rendezvous-daemon::pairing_and_sync endpoints_are_stable_per_fixture_and_distinct_across_fixtures
     Summary [   1.100s] 1 test run: 1 passed, 2 skipped
`;

test("parseLog reads statuses, durations, slow marks and the summary", () => {
  const [inv] = parseLog(PASSING_LOG, "passing");
  assert.equal(inv.declared.tests, 4);
  assert.equal(inv.declared.binaries, 2);
  assert.equal(inv.declared.skipped, 1);
  assert.equal(inv.results.length, 4);
  assert.deepEqual(
    inv.results.map((r) => [r.binary, r.status, r.durationS]),
    [
      ["claudine", "PASS", 0.018],
      ["claudine", "PASS", 5.25],
      ["claudine-cli::context_command", "PASS", 0.3],
      ["claudine-cli::context_command", "LEAK", 0.1],
    ],
  );
  assert.equal(inv.slowMarks, 1);
  assert.deepEqual(inv.summary, {
    elapsedS: 6,
    run: 4,
    passed: 4,
    failed: 0,
    timedOut: 0,
    skipped: 1,
    slow: 1,
    leaky: 1,
    flaky: 0,
  });
});

test("parseLog counts retries once and ignores the post-summary recap", () => {
  const [inv] = parseLog(FAILING_LOG, "failing");
  assert.equal(inv.results.length, 3, "the recap after Summary must not be re-counted");
  const b = inv.results.find((r) => r.name === "b");
  assert.equal(b?.retries, 1);
  assert.equal(b?.status, "PASS");
  const c = inv.results.find((r) => r.name === "c");
  assert.equal(c?.status, "TIMEOUT");
  assert.equal(inv.summary.timedOut, 1);
  assert.equal(inv.summary.flaky, 1);
});

test("parseLog splits a multi-package recipe log into invocations", () => {
  const invocations = parseLog(MULTI_LOG, "multi");
  assert.equal(invocations.length, 2);
  assert.equal(invocations[1].declared.skipped, 2);
  assert.equal(invocations[1].results[0].binary, "rendezvous-daemon::pairing_and_sync");
});

test("parseLog rejects a truncated log and a log with no invocation", () => {
  assert.throws(() => parseLog("    Starting 1 test across 1 binary\n        PASS [ 0.1s] (1/1) a b\n", "t"), /truncated/);
  assert.throws(() => parseLog("nothing here\n", "empty"), /no nextest invocation/);
});

test("readLog transparently reads gzip", () => {
  const dir = mkdtempSync(join(tmpdir(), "measurement-"));
  try {
    const path = join(dir, "run.log.gz");
    writeFileSync(path, gzipSync(Buffer.from(PASSING_LOG)));
    assert.equal(readLog(path), PASSING_LOG);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

function run(overrides: Partial<RunRecord>): RunRecord {
  return {
    suite: "just test",
    revision: "candidate",
    sha: "abc",
    sequence: 1,
    role: "alternating",
    started: "2026-09-08T00:00:00Z",
    wallS: 10,
    exitCode: 0,
    log: "x.log",
    ...overrides,
  };
}

test("metricsForRun keeps the three costs apart and flags non-passing results", () => {
  const clean = metricsForRun(run({}), parseLog(PASSING_LOG));
  assert.equal(clean.elapsedS, 6);
  assert.equal(clean.buildSetupS, 4);
  assert.equal(Number(clean.summedS.toFixed(3)), 5.668);
  assert.equal(clean.leaks, 1);
  assert.deepEqual(clean.violations, [], "LEAK on a passing test is a count, not a failure");

  const bad = metricsForRun(run({ exitCode: 100 }), parseLog(FAILING_LOG));
  assert.ok(bad.violations.some((v) => v.startsWith("[non-passing]") && v.includes("TIMEOUT")));
  assert.ok(bad.violations.some((v) => v.startsWith("[exit-code]")));
  assert.equal(bad.retries, 1);
  assert.equal(bad.timedOut, 1);
});

test("metricsForRun flags result lines that disagree with the summary", () => {
  const log = PASSING_LOG.replace("4 tests run: 4 passed", "5 tests run: 5 passed");
  const m = metricsForRun(run({}), parseLog(log));
  assert.ok(m.violations.some((v) => v.startsWith("[count-mismatch]")));
});

test("spread and the drift bracket", () => {
  const b = spread([10, 10.5, 11, 10.2, 10.8]);
  assert.equal(b.median, 10.5);
  assert.equal(b.min, 10);
  assert.equal(b.max, 11);
  assert.equal(Number(b.driftRatio.toFixed(4)), Number((1 / 10.5).toFixed(4)));
  const c = spread([8, 8.4, 8.2, 8.1, 8.3]);
  assert.equal(established(b, c), true, "a 2.3 s delta clears a 1 s bracket");
  const noisy = spread([9.6, 10.9, 10.2, 9.8, 10.7]);
  assert.equal(established(b, noisy), false, "a 0.3 s delta inside a 1.3 s bracket is not established");
});

test("cohort and target matching", () => {
  assert.equal(matchesCohort({ name: "c", binaries: ["claudine-cli::loop_cli"] }, "claudine-cli::loop_cli", "x"), true);
  assert.equal(matchesCohort({ name: "c", binaryPrefixes: ["claudine-cli::"] }, "claudine-cli::anything", "x"), true);
  assert.equal(matchesCohort({ name: "c", prefixes: [{ binary: "claudine", prefix: "linking::paths::" }] }, "claudine", "linking::paths::tests::a"), true);
  assert.equal(matchesCohort({ name: "c", prefixes: [{ binary: "claudine", prefix: "linking::paths::" }] }, "claudine-cli", "linking::paths::tests::a"), false);
  assert.equal(matchesTarget({ binary: "claudine", suffix: "::reaps", contract: "" }, "claudine", "a::b::reaps"), true);
  assert.equal(matchesTarget({ binary: "claudine", name: "a", contract: "" }, "claudine", "ab"), false);
  assert.equal(matchesTarget({ binary: "x", contract: "" }, "x", "anything"), true);
});

test("buildReport joins the manifest to its logs, diffs identities, and gates", () => {
  const dir = mkdtempSync(join(tmpdir(), "measurement-report-"));
  try {
    const baselineLog = PASSING_LOG.replace("leaky_but_passing", "old_identity");
    const records: RunRecord[] = [];
    for (let seq = 1; seq <= 2; seq += 1) {
      for (const revision of ["baseline", "candidate"] as const) {
        const name = `${revision}-${seq}.log`;
        writeFileSync(join(dir, name), revision === "baseline" ? baselineLog : PASSING_LOG);
        records.push(run({ revision, sequence: seq, log: name, wallS: revision === "baseline" ? 12 : 9 }));
      }
    }
    const manifestPath = join(dir, "runs.jsonl");
    writeFileSync(manifestPath, records.map((r) => JSON.stringify(r)).join("\n") + "\n");
    const manifest = readManifest(manifestPath);
    assert.equal(manifest.length, 4);

    const report = buildReport(
      manifest,
      [{ name: "context", binaries: ["claudine-cli::context_command"] }],
      [
        { binary: "claudine-cli::context_command", name: "leaky_but_passing", contract: "example" },
        { binary: "claudine-cli::nowhere", contract: "missing" },
      ],
      dir,
    );
    assert.ok(report.markdown.includes("| `just test` | identities | 4 | 4 | +1 / −1 | — |"));
    assert.ok(report.markdown.includes("- added: `claudine-cli::context_command leaky_but_passing`"));
    assert.ok(report.markdown.includes("- removed: `claudine-cli::context_command old_identity`"));
    assert.ok(report.markdown.includes("| `context` | 2 | 0.40 / 0.40 / 0.40 s | 2 |"));
    // Same log at both revisions → every pair's ratio is exactly 1, which is not an improvement.
    assert.ok(report.markdown.includes("| `just test` | elapsed | 2 | 1.000 / 1.000 / 1.000 | **no** |"));
    assert.ok(report.markdown.includes("| `just test` | summed | 2 | 1.000 / 1.000 / 1.000 | **no** |"));
    assert.ok(report.violations.some((v) => v.startsWith("[target-short]")), "two executions are not ten");
    assert.ok(report.violations.some((v) => v.startsWith("[target-unmatched]")));
    assert.ok(!report.violations.some((v) => v.startsWith("[unstable-identities]")));

    // An unstable identity set within one revision is a violation, not noise.
    writeFileSync(join(dir, "candidate-2.log"), PASSING_LOG.replace("leaky_but_passing", "renamed_between_runs"));
    const unstable = buildReport(manifest, [], [], dir);
    assert.ok(unstable.violations.some((v) => v.startsWith("[unstable-identities] just test candidate")));

    // A missing log is a violation rather than a silently shorter table.
    const missing = buildReport([run({ log: "absent.log" })], [], [], dir);
    assert.ok(missing.violations.some((v) => v.startsWith("[missing-log]")));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
