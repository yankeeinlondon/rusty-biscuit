import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";

import { loadConfig } from "../src/config.ts";
import { readManifest, buildReport, type CohortSpec, type TargetSpec } from "../src/measure/index.ts";

const FIX_DIR = "/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/claudine/fixes/2026-09-07-faster-claudine-tests";
const CONFIG_PATH = join(FIX_DIR, "audit.config.json");
const MEASUREMENT_DIR = join(FIX_DIR, "measurement");

describe("Claudine measurement compatibility", () => {
  it("reproduces the recorded run counts and spread medians", () => {
    loadConfig(CONFIG_PATH);

    const manifestPath = join(MEASUREMENT_DIR, "runs.jsonl");
    const cohortsPath = join(MEASUREMENT_DIR, "cohorts.json");
    const targetsPath = join(MEASUREMENT_DIR, "targets.json");

    const manifest = readManifest(manifestPath);
    const cohorts: CohortSpec[] = JSON.parse(readFileSync(cohortsPath, "utf8")).cohorts;
    const targets: TargetSpec[] = JSON.parse(readFileSync(targetsPath, "utf8")).targets;

    const report = buildReport(manifest, cohorts, targets, dirname(resolve(manifestPath)));

    const recordedMd = readFileSync(join(MEASUREMENT_DIR, "report.md"), "utf8");

    const justTestBaselineMatch = recordedMd.match(/\|\s*`just test`\s*\|\s*baseline\s*\|\s*(\d+)\s*\|[^|]*\|[^|]*\|[^|]*\|[^|]*\|\s*(\d+)\s*\|/);
    if (justTestBaselineMatch) {
      const recordedRuns = Number(justTestBaselineMatch[1]);
      const recordedIdentities = Number(justTestBaselineMatch[2]);

      const baselineRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "baseline" && r.run.role === "alternating"
      );
      expect(baselineRuns.length).toBe(recordedRuns);
      expect(baselineRuns[0]!.identities.size).toBe(recordedIdentities);
    }

    const justTestCandidateMatch = recordedMd.match(/\|\s*`just test`\s*\|\s*candidate\s*\|\s*(\d+)\s*\|[^|]*\|[^|]*\|[^|]*\|[^|]*\|\s*(\d+)\s*\|/);
    if (justTestCandidateMatch) {
      const recordedRuns = Number(justTestCandidateMatch[1]);
      const recordedIdentities = Number(justTestCandidateMatch[2]);

      const candidateRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "candidate" && r.run.role === "alternating"
      );
      expect(candidateRuns.length).toBe(recordedRuns);
      expect(candidateRuns[0]!.identities.size).toBe(recordedIdentities);
    }
  });

  it("reproduces spread medians for baseline and candidate", () => {
    loadConfig(CONFIG_PATH);

    const manifestPath = join(MEASUREMENT_DIR, "runs.jsonl");
    const manifest = readManifest(manifestPath);
    const report = buildReport(manifest, [], [], dirname(resolve(manifestPath)));

    const recordedMd = readFileSync(join(MEASUREMENT_DIR, "report.md"), "utf8");

    const baselineSpreadMatch = recordedMd.match(
      /\|\s*`just test`\s*\|\s*baseline\s*\|[^|]*\|[^|]*\|[^|]*\|\s*([\d.]+)\s*\/\s*([\d.]+)\s*\/\s*([\d.]+)\s*s\s*\|[^|]*\|\s*([\d.]+)\s*s\s*\|/
    );
    if (baselineSpreadMatch) {
      const recordedSummedMedian = Number(baselineSpreadMatch[4]);

      const baselineRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "baseline" && r.run.role === "alternating"
      );
      const summedValues = baselineRuns.map(r => r.summedS);
      const sortedSummed = [...summedValues].sort((a, b) => a - b);
      const median = sortedSummed[Math.floor(sortedSummed.length / 2)]!;

      expect(median).toBeCloseTo(recordedSummedMedian, 1);
    }

    const candidateSpreadMatch = recordedMd.match(
      /\|\s*`just test`\s*\|\s*candidate\s*\|[^|]*\|[^|]*\|[^|]*\|\s*([\d.]+)\s*\/\s*([\d.]+)\s*\/\s*([\d.]+)\s*s\s*\|[^|]*\|\s*([\d.]+)\s*s\s*\|/
    );
    if (candidateSpreadMatch) {
      const recordedSummedMedian = Number(candidateSpreadMatch[4]);

      const candidateRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "candidate" && r.run.role === "alternating"
      );
      const summedValues = candidateRuns.map(r => r.summedS);
      const sortedSummed = [...summedValues].sort((a, b) => a - b);
      const median = sortedSummed[Math.floor(sortedSummed.length / 2)]!;

      expect(median).toBeCloseTo(recordedSummedMedian, 1);
    }
  });

  it("gate exit matches the recorded result", () => {
    loadConfig(CONFIG_PATH);

    const manifestPath = join(MEASUREMENT_DIR, "runs.jsonl");
    const cohortsPath = join(MEASUREMENT_DIR, "cohorts.json");
    const targetsPath = join(MEASUREMENT_DIR, "targets.json");

    const manifest = readManifest(manifestPath);
    const cohorts: CohortSpec[] = JSON.parse(readFileSync(cohortsPath, "utf8")).cohorts;
    const targets: TargetSpec[] = JSON.parse(readFileSync(targetsPath, "utf8")).targets;

    const report = buildReport(manifest, cohorts, targets, dirname(resolve(manifestPath)));

    const recordedGate = readFileSync(join(MEASUREMENT_DIR, "report.gate.txt"), "utf8").trim();
    const expectedExitCode = recordedGate.includes("EXIT=0") ? 0 : 1;

    const actualExitCode = report.violations.length === 0 ? 0 : 1;
    expect(actualExitCode).toBe(expectedExitCode);
  });

  it("total run count matches recorded output", () => {
    loadConfig(CONFIG_PATH);

    const manifestPath = join(MEASUREMENT_DIR, "runs.jsonl");
    const manifest = readManifest(manifestPath);
    const report = buildReport(manifest, [], [], dirname(resolve(manifestPath)));

    const recordedManifest = readManifest(manifestPath);
    expect(report.runs.length).toBe(recordedManifest.length);
  });

  it("identity changes are detected correctly", () => {
    loadConfig(CONFIG_PATH);

    const manifestPath = join(MEASUREMENT_DIR, "runs.jsonl");
    const manifest = readManifest(manifestPath);
    const report = buildReport(manifest, [], [], dirname(resolve(manifestPath)));

    const recordedMd = readFileSync(join(MEASUREMENT_DIR, "report.md"), "utf8");

    const identityChangeMatch = recordedMd.match(/`just test`\s+—\s+added\s+(\d+),\s+removed\s+(\d+)/);
    if (identityChangeMatch) {
      const recordedAdded = Number(identityChangeMatch[1]);
      const recordedRemoved = Number(identityChangeMatch[2]);

      const baselineRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "baseline" && r.run.role === "alternating"
      );
      const candidateRuns = report.runs.filter(
        r => r.run.suite === "just test" && r.run.revision === "candidate" && r.run.role === "alternating"
      );

      if (baselineRuns.length > 0 && candidateRuns.length > 0) {
        const baselineIds = baselineRuns[0]!.identities;
        const candidateIds = candidateRuns[0]!.identities;

        const added = [...candidateIds].filter(id => !baselineIds.has(id));
        const removed = [...baselineIds].filter(id => !candidateIds.has(id));

        expect(added.length).toBe(recordedAdded);
        expect(removed.length).toBe(recordedRemoved);
      }
    }
  });
});
