#!/usr/bin/env bun
/**
 * Pools the bracketed rounds emitted by `refgraph-setup-fix.sh` into the gate
 * table, recomputed from the retained per-run vectors.
 *
 * Each round samples every pin, so pooling `<case>.r*.json` by arm spreads each
 * pin evenly across the session; a load excursion then shifts all arms together
 * instead of landing inside one. `after_A` and `after_B` are the same binary, so
 * their delta is the drift floor: an effect at or below it is not established,
 * however tight its confidence interval looks.
 *
 * Usage: bun refgraph-setup-fix-report.ts <run-dir> [--json]
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ARMS = ["base", "before", "after_A", "after_B"] as const;
type Arm = (typeof ARMS)[number];

interface HyperfineExport {
  results: { command: string; times: number[] }[];
}

/** Percentile bootstrap CI for the mean; timings are right-skewed, so no
 *  normal-theory interval. Seeded for reproducibility across re-runs. */
function bootstrapMeanCi(
  observations: number[],
  resamples = 10_000,
): [number, number] {
  let seed = 0x5eed_1234;
  const nextUnit = () => {
    // xorshift32: deterministic, so a third party recomputes identical bounds.
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return ((seed >>> 0) % 0xffff_ffff) / 0xffff_ffff;
  };
  const means: number[] = [];
  for (let r = 0; r < resamples; r++) {
    let total = 0;
    for (let i = 0; i < observations.length; i++) {
      total += observations[Math.floor(nextUnit() * observations.length)]!;
    }
    means.push(total / observations.length);
  }
  means.sort((a, b) => a - b);
  return [
    means[Math.floor(0.025 * resamples)]!,
    means[Math.floor(0.975 * resamples)]!,
  ];
}

const mean = (v: number[]) => v.reduce((a, b) => a + b, 0) / v.length;
const stdev = (v: number[]) => {
  const m = mean(v);
  return Math.sqrt(v.reduce((a, b) => a + (b - m) ** 2, 0) / (v.length - 1));
};
const pct = (a: number, b: number) => ((a - b) / b) * 100;

const runDir = process.argv[2];
if (!runDir) {
  console.error("usage: bun refgraph-setup-fix-report.ts <run-dir> [--json]");
  process.exit(2);
}

const files = readdirSync(runDir).filter((f) => /\.r\d+\.json$/.test(f));
const cases = [...new Set(files.map((f) => f.replace(/\.r\d+\.json$/, "")))].sort();

const report = cases.map((caseName) => {
  const pooled: Record<string, number[]> = {};
  for (const file of files.filter((f) => f.startsWith(`${caseName}.r`))) {
    const data: HyperfineExport = JSON.parse(
      readFileSync(join(runDir, file), "utf8"),
    );
    for (const result of data.results) {
      // hyperfine `times` are seconds; the gate is quoted in milliseconds.
      (pooled[result.command] ??= []).push(...result.times.map((t) => t * 1000));
    }
  }
  const arms = Object.fromEntries(
    ARMS.map((arm) => {
      const observations = pooled[arm] ?? [];
      return [
        arm,
        {
          n: observations.length,
          mean: mean(observations),
          stdev: stdev(observations),
          ci95: bootstrapMeanCi(observations),
        },
      ];
    }),
  ) as Record<Arm, { n: number; mean: number; stdev: number; ci95: [number, number] }>;

  const afterMean = (arms.after_A.mean + arms.after_B.mean) / 2;
  return {
    case: caseName,
    arms,
    driftFloorPct: pct(arms.after_B.mean, arms.after_A.mean),
    beforeVsBasePct: pct(arms.before.mean, arms.base.mean),
    afterVsBasePct: pct(afterMean, arms.base.mean),
    afterVsBaseMs: afterMean - arms.base.mean,
  };
});

if (process.argv.includes("--json")) {
  console.log(JSON.stringify(report, null, 2));
} else {
  for (const row of report) {
    console.log(`\n== ${row.case}  (n=${row.arms.base.n} per arm)`);
    for (const arm of ARMS) {
      const a = row.arms[arm];
      console.log(
        `  ${arm.padEnd(8)} ${a.mean.toFixed(3).padStart(7)} ± ${a.stdev
          .toFixed(3)
          .padStart(5)} ms  CI95[${a.ci95[0].toFixed(3)}, ${a.ci95[1].toFixed(
          3,
        )}]  vs base ${pct(a.mean, row.arms.base.mean).toFixed(2).padStart(6)}%`,
      );
    }
    console.log(
      `  drift floor (identical code)  ${row.driftFloorPct.toFixed(2).padStart(6)}%`,
    );
    console.log(
      `  before vs base               ${row.beforeVsBasePct.toFixed(2).padStart(6)}%`,
    );
    console.log(
      `  after  vs base               ${row.afterVsBasePct
        .toFixed(2)
        .padStart(6)}%   (${row.afterVsBaseMs >= 0 ? "+" : ""}${row.afterVsBaseMs.toFixed(3)} ms)`,
    );
    const established = Math.abs(row.afterVsBasePct) > Math.abs(row.driftFloorPct);
    console.log(
      `  after effect vs drift floor  ${established ? "RESOLVABLE" : "within drift — not established"}`,
    );
  }
}
