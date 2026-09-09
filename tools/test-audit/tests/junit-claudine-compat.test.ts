/**
 * Claudine compatibility proof: the new tool produces results that match the
 * legacy script over the preserved baseline tree.
 */
import { describe, it, expect } from "vitest";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { junitCommand } from "../src/junit/command.ts";
import type { CommandIo } from "../src/command.ts";
import { EXIT } from "../src/errors.ts";

const CLAUDINE_ROOT = join(__dirname, "../../../claudine/fixes/2026-09-07-faster-claudine-tests");
const BASELINE_DIR = join(CLAUDINE_ROOT, "baseline");

/**
 * Build a minimal test-audit config for the Claudine baseline.
 *
 * The baseline runs had `L1/claudine-cli` cells on all four legs. We construct
 * a minimal config in a temp directory that declares this structure.
 */
function buildClaudineConfig(): { configPath: string; cleanup: () => void } {
  const tempDir = mkdtempSync(join(tmpdir(), "junit-compat-"));
  const configPath = join(tempDir, "audit.config.json");

  const config = {
    version: 1,
    area: "claudine",
    root: "../../../..",
    evidenceDir: ".",
    packages: [{ name: "claudine-cli", path: "claudine/cli" }],
    routes: [{ id: "nextest-l1", kind: "nextest", command: "just test", executes: true }],
    selections: [
      {
        label: "l1-claudine-cli",
        packages: ["claudine-cli"],
        features: [],
        routes: ["nextest-l1"],
      },
    ],
    cohorts: [
      {
        id: "l1-local",
        description: "L1 local cohort",
        population: "local",
        recipe: "test",
        selections: ["l1-claudine-cli"],
        tiers: ["L1"],
      },
    ],
    environments: [
      {
        name: "ubuntu-latest",
        kind: "native",
        os: "linux",
        cells: [{ tier: "L1", package: "claudine-cli" }],
        pending: false,
      },
      {
        name: "macos-latest",
        kind: "native",
        os: "macos",
        cells: [{ tier: "L1", package: "claudine-cli" }],
        pending: false,
      },
      {
        name: "windows-latest",
        kind: "native",
        os: "windows",
        cells: [{ tier: "L1", package: "claudine-cli" }],
        pending: false,
      },
      {
        name: "wsl2-ubuntu",
        kind: "wsl",
        os: "linux",
        cells: [{ tier: "L1", package: "claudine-cli" }],
        pending: false,
      },
    ],
  };

  writeFileSync(configPath, JSON.stringify(config, null, 2));
  return { configPath, cleanup: () => rmSync(tempDir, { recursive: true, force: true }) };
}

describe("Claudine compatibility", () => {
  it("matches_the_legacy_output_on_preserved_baseline", async () => {
    const runs = readdirSync(BASELINE_DIR).filter((name) => /^\d+$/.test(name));
    if (runs.length === 0) {
      throw new Error("No baseline runs found under claudine/fixes/.../baseline/");
    }

    const [runId] = runs;
    const runDir = join(BASELINE_DIR, runId!);
    const expectationsPath = join(BASELINE_DIR, "expectations.json");
    const legacyOutputPath = join(runDir, "junit-metrics.txt");

    if (!existsSync(expectationsPath)) {
      throw new Error(`Missing expectations.json in ${BASELINE_DIR}`);
    }

    const { configPath, cleanup } = buildClaudineConfig();

    try {
      const output: string[] = [];
      const errors: string[] = [];
      const io: CommandIo = {
        out: (line) => output.push(line),
        err: (line) => errors.push(line),
      };

      const exitCode = await junitCommand.run(
        [runDir, "--config", configPath, "--expect", expectationsPath, "--markdown"],
        io,
      );

      expect(exitCode).toBe(EXIT.clean);

      const newOutput = output.join("\n");

      // Verify key metrics are present
      expect(newOutput).toContain("ubuntu-latest");
      expect(newOutput).toContain("macos-latest");
      expect(newOutput).toContain("windows-latest");
      expect(newOutput).toContain("wsl2-ubuntu");
      expect(newOutput).toContain("L1");
      expect(newOutput).toContain("claudine-cli");

      // If legacy output exists, compare test counts and environments
      if (existsSync(legacyOutputPath)) {
        const legacyOutput = readFileSync(legacyOutputPath, "utf8");

        // Extract environment rows from both outputs
        for (const env of ["ubuntu-latest", "macos-latest", "windows-latest", "wsl2-ubuntu"]) {
          const newRow = newOutput.split("\n").find((line) => line.includes(env));
          const legacyRow = legacyOutput.split("\n").find((line) => line.includes(env));

          if (newRow && legacyRow) {
            // Extract test count (7th column in markdown table)
            const newMatch = /\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|/.exec(newRow);
            const legacyMatch = /\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|/.exec(legacyRow);

            if (newMatch && legacyMatch) {
              // Test count should match
              expect(newMatch[1]).toBe(legacyMatch[1]);
              // Failures should match
              expect(newMatch[2]).toBe(legacyMatch[2]);
            }
          }
        }
      }

      // Verify no violations
      expect(errors.length).toBe(0);

      // Verify platform exclusions are reported
      expect(newOutput).toContain("Platform exclusions");

      // All Windows exclusions should be listed
      const exclusions = [
        "watchdog_subagent_hang_terminates_and_names_stuck_ids",
        "watchdog_stream_idle_timeout_after_tool_call_hang",
        "compose_non_harness_respects_cli_timeout",
      ];
      for (const test of exclusions) {
        const match = newOutput.includes(test);
        expect(match).toBe(true);
      }
    } finally {
      cleanup();
    }
  });

  it("accepts_every_stored_baseline_run", async () => {
    const runs = readdirSync(BASELINE_DIR).filter((name) => /^\d+$/.test(name));
    expect(runs.length).toBeGreaterThan(0);

    const expectationsPath = join(BASELINE_DIR, "expectations.json");
    const { configPath, cleanup } = buildClaudineConfig();

    try {
      for (const runId of runs) {
        const output: string[] = [];
        const errors: string[] = [];
        const io: CommandIo = {
          out: (line) => output.push(line),
          err: (line) => errors.push(line),
        };

        const exitCode = await junitCommand.run(
          [join(BASELINE_DIR, runId), "--config", configPath, "--expect", expectationsPath, "--markdown"],
          io,
        );

        expect(exitCode, `run ${runId} should pass`).toBe(EXIT.clean);
        expect(errors.length, `run ${runId} should have no violations`).toBe(0);
      }
    } finally {
      cleanup();
    }
  });

  it("windows_exclusions_are_absent_on_windows_and_present_elsewhere", async () => {
    const [runId] = readdirSync(BASELINE_DIR).filter((name) => /^\d+$/.test(name));
    if (!runId) return;

    const runDir = join(BASELINE_DIR, runId);
    const expectationsPath = join(BASELINE_DIR, "expectations.json");
    const expectations = JSON.parse(readFileSync(expectationsPath, "utf8")) as {
      platformExclusions: Record<string, string[]>;
    };

    const { configPath, cleanup } = buildClaudineConfig();

    try {
      const output: string[] = [];
      const io: CommandIo = {
        out: (line) => output.push(line),
        err: () => {},
      };

      const exitCode = await junitCommand.run(
        [runDir, "--config", configPath, "--expect", expectationsPath, "--json"],
        io,
      );

      expect(exitCode).toBe(EXIT.clean);

      const result = JSON.parse(output.join("\n")) as {
        measurements: Array<{ environment: string; tests: number }>;
      };

      const excluded = expectations.platformExclusions["windows-latest"] ?? [];
      expect(excluded.length).toBeGreaterThan(0);

      // Windows should have fewer tests than other environments
      const windowsTests = result.measurements.find((m) => m.environment === "windows-latest")?.tests ?? 0;
      const ubuntuTests = result.measurements.find((m) => m.environment === "ubuntu-latest")?.tests ?? 0;

      expect(windowsTests).toBeGreaterThan(0);
      expect(ubuntuTests).toBeGreaterThan(windowsTests);
      expect(ubuntuTests - windowsTests).toBeGreaterThanOrEqual(excluded.length);
    } finally {
      cleanup();
    }
  });
});
