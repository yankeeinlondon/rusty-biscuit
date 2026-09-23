#!/usr/bin/env bun
/**
 * Recomputes benchmark statistics from retained Criterion and hyperfine sample
 * vectors.
 *
 * Review-1 rejected this feature's benchmark evidence because the retained
 * `criterion-*.json` artifacts were copies of Criterion's `estimates.json` —
 * derived statistics only (`mean`, `median`, `slope`, `std_dev`), with no
 * sample vectors. Nothing downstream could be recomputed or independently
 * checked. Run records now retain `*-sample.json` (Criterion's
 * `<bench>/new/sample.json`: parallel `iters` / `times` vectors), and this
 * script is the tool that turns those vectors back into the reported numbers.
 *
 * Every statistic quoted in a run record's `summary.md` is produced by this
 * script from the committed sample vectors beside it, so a third party can
 * re-derive them without re-running the benchmark.
 *
 * Usage:
 *   bun recompute.ts <run-record-dir> [--json]
 *
 * Two retained-vector formats are read, matching the two runners the
 * benchmark README blesses:
 *
 * - **Criterion** `<bench>-sample.json`: sampling emits, per sample i, a batch
 *   of `iters[i]` iterations taking `times[i]` nanoseconds total, so the
 *   per-iteration observation is `times[i] / iters[i]`.
 * - **hyperfine** `--export-json`: `results[].times` is already a per-run
 *   vector in **seconds**; each entry is reported as `<file stem>/<command>`.
 *
 * Both are normalized to nanoseconds so one table can carry either runner.
 */

import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";

interface CriterionSample {
  sampling_mode: string;
  iters: number[];
  times: number[];
}

interface HyperfineResult {
  command: string;
  times: number[];
}

interface HyperfineExport {
  results: HyperfineResult[];
}

/** A named per-observation vector in nanoseconds, from either runner. */
interface NamedObservations {
  benchmark: string;
  samplingMode: string;
  observations: number[];
}

interface Statistics {
  sampleCount: number;
  mean: number;
  median: number;
  stdDev: number;
  min: number;
  max: number;
  meanCi95: [number, number];
}

/** Per-iteration nanoseconds for each Criterion sample batch. */
function perIterationTimes(sample: CriterionSample): number[] {
  if (sample.iters.length !== sample.times.length) {
    throw new Error("malformed sample: iters/times length mismatch");
  }
  return sample.times.map((total, index) => total / sample.iters[index]);
}

function median(sorted: number[]): number {
  const middle = sorted.length >> 1;
  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle];
}

/**
 * Percentile bootstrap CI for the mean.
 *
 * Criterion itself reports a bootstrap CI rather than a normal-theory one, and
 * benchmark timings are right-skewed, so a bootstrap keeps this script's
 * numbers comparable to Criterion's own rather than subtly tighter.
 *
 * Seeded (mulberry32) so re-running yields byte-identical output — an
 * unseeded CI would make this script's own results irreproducible, which is
 * the exact defect it exists to fix.
 */
function bootstrapMeanCi(
  observations: number[],
  resamples = 100_000,
  seed = 0x5eed,
): [number, number] {
  let state = seed;
  const random = () => {
    state |= 0;
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };

  const count = observations.length;
  const means = new Float64Array(resamples);
  for (let r = 0; r < resamples; r += 1) {
    let sum = 0;
    for (let i = 0; i < count; i += 1) {
      sum += observations[(random() * count) | 0];
    }
    means[r] = sum / count;
  }
  const sorted = Float64Array.prototype.slice.call(means).sort();
  return [
    sorted[Math.floor(0.025 * resamples)],
    sorted[Math.floor(0.975 * resamples)],
  ];
}

function statistics(observations: number[]): Statistics {
  const sorted = [...observations].sort((a, b) => a - b);
  const mean =
    observations.reduce((sum, value) => sum + value, 0) / observations.length;
  const variance =
    observations.reduce((sum, value) => sum + (value - mean) ** 2, 0) /
    (observations.length - 1);

  return {
    sampleCount: observations.length,
    mean,
    median: median(sorted),
    stdDev: Math.sqrt(variance),
    min: sorted[0],
    max: sorted[sorted.length - 1],
    meanCi95: bootstrapMeanCi(observations),
  };
}

/** Formats nanoseconds at the scale a reader can compare by eye. */
function formatNanos(nanos: number): string {
  if (nanos >= 1e6) return `${(nanos / 1e6).toFixed(4)} ms`;
  if (nanos >= 1e3) return `${(nanos / 1e3).toFixed(4)} µs`;
  return `${nanos.toFixed(4)} ns`;
}

const [runRecordDir, ...flags] = process.argv.slice(2);
if (!runRecordDir) {
  console.error("usage: bun recompute.ts <run-record-dir> [--json]");
  process.exit(1);
}

/** Reads either retained-vector format, keyed off its distinguishing shape. */
function loadObservations(dir: string, name: string): NamedObservations[] {
  const parsed = JSON.parse(readFileSync(join(dir, name), "utf8")) as unknown;

  if (name.endsWith("-sample.json")) {
    const sample = parsed as CriterionSample;
    return [
      {
        benchmark: basename(name, "-sample.json"),
        samplingMode: sample.sampling_mode,
        observations: perIterationTimes(sample),
      },
    ];
  }

  const exported = parsed as HyperfineExport;
  if (!Array.isArray(exported.results)) return [];
  const stem = basename(name, ".json");
  return exported.results
    .filter((result) => Array.isArray(result.times))
    .map((result) => ({
      benchmark: `${stem}/${result.command}`,
      samplingMode: "hyperfine",
      // hyperfine exports seconds; Criterion vectors are already nanoseconds.
      observations: result.times.map((seconds) => seconds * 1e9),
    }));
}

const sampleFiles = readdirSync(runRecordDir)
  .filter((name) => name.endsWith(".json"))
  .sort();

const results = sampleFiles
  .flatMap((name) => loadObservations(runRecordDir, name))
  .map(({ benchmark, samplingMode, observations }) => ({
    benchmark,
    samplingMode,
    ...statistics(observations),
  }));

if (results.length === 0) {
  console.error(`no Criterion or hyperfine sample vectors in ${runRecordDir}`);
  process.exit(1);
}

if (flags.includes("--json")) {
  console.log(JSON.stringify(results, null, 2));
} else {
  console.log(`Recomputed from retained sample vectors in ${runRecordDir}\n`);
  console.log(
    "| benchmark | n | mean | 95% CI (mean) | median | std dev | min | max |",
  );
  console.log("|---|---|---|---|---|---|---|---|");
  for (const r of results) {
    console.log(
      `| \`${r.benchmark}\` | ${r.sampleCount} | ${formatNanos(r.mean)} | ` +
        `[${formatNanos(r.meanCi95[0])}, ${formatNanos(r.meanCi95[1])}] | ` +
        `${formatNanos(r.median)} | ${formatNanos(r.stdDev)} | ` +
        `${formatNanos(r.min)} | ${formatNanos(r.max)} |`,
    );
  }
}
