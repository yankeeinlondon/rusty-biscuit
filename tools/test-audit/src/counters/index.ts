/**
 * Work-count evidence validation and comparison.
 *
 * Evidence files declare readings (signal, value, revision, environment, etc.)
 * and are validated against a config's signal catalog and compatibility policy.
 * Comparison pairs readings by compatibility keys and rejects incompatible or
 * boundary-spanning pairs.
 */

import { z } from "zod";

import { MalformedInput, type Violation } from "../errors.ts";
import type { CountersConfig } from "../config.ts";

export const ReadingSchema = z.object({
  /** Signal identifier matching config.counters.signals[].id */
  signal: z.string().min(1),
  /** Work count; must be >= 0 for non-pending signals, absent for pending signals */
  value: z.number().nonnegative().optional(),
  /** Revision identifier (e.g. "baseline", "candidate", commit SHA) */
  revision: z.string().min(1),
  /** Environment leg name */
  environment: z.string().min(1),
  /** Platform OS */
  os: z.enum(["linux", "macos", "windows"]),
  /** Platform kind */
  platformKind: z.enum(["native", "wsl"]),
  /** Request shape descriptor */
  requestShape: z.string().min(1),
  /** Counter implementation version */
  counterVersion: z.string().min(1),
  /** Measurement phase */
  phase: z.enum(["acquisition", "execution", "total"]),
  /** Collection metadata */
  collector: z.object({
    /** Whether the counter was propagated from a parent process */
    propagated: z.boolean(),
    /** Number of worker processes/threads, if applicable */
    workers: z.number().int().positive().optional(),
  }),
  /** Evidence source (file path, CI artifact, manual observation) */
  source: z.string().min(1),
  /** Boundary markers for cross-run comparisons */
  boundaries: z.record(z.string(), z.enum(["before", "after"])).optional(),
  /** Free-form note */
  note: z.string().optional(),
});

export const EvidenceFileSchema = z.object({
  /** Tool version that generated this file */
  toolVersion: z.string().optional(),
  /** Package area name */
  area: z.string().min(1),
  /** Array of readings */
  readings: z.array(ReadingSchema).min(1),
});

export type Reading = z.infer<typeof ReadingSchema>;
export type EvidenceFile = z.infer<typeof EvidenceFileSchema>;

export type CountersViolationKind =
  | "unknown-signal"
  | "pending-signal-with-value"
  | "incompatible-comparison"
  | "spans-boundary"
  | "no-baseline"
  | "no-candidate";

export type CountersViolation = Violation<CountersViolationKind>;

/**
 * Validate an evidence file against the configured signal catalog.
 *
 * Rejects:
 * - Unknown signals (not declared in config)
 * - Readings for pending signals that carry a value
 * - Negative or NaN values (schema already rejects these, but we check)
 */
export function validateEvidence(
  evidence: EvidenceFile,
  config: CountersConfig
): CountersViolation[] {
  if (!config) {
    throw new MalformedInput("config.counters is not defined");
  }

  const violations: CountersViolation[] = [];
  const knownSignals = new Map(config.signals.map((s) => [s.id, s]));

  for (const reading of evidence.readings) {
    const signal = knownSignals.get(reading.signal);
    if (!signal) {
      violations.push({
        kind: "unknown-signal",
        detail: `signal '${reading.signal}' is not declared in config.counters.signals`,
      });
      continue;
    }

    if (signal.coverage === "pending" && reading.value !== undefined) {
      violations.push({
        kind: "pending-signal-with-value",
        detail: `signal '${reading.signal}' has coverage=pending but reading carries value=${reading.value}`,
      });
    }

    if (reading.value !== undefined) {
      if (!Number.isFinite(reading.value) || reading.value < 0) {
        violations.push({
          kind: "pending-signal-with-value",
          detail: `signal '${reading.signal}' has invalid value: ${reading.value}`,
        });
      }
    }
  }

  return violations;
}

export interface Comparison {
  signal: string;
  baseline: Reading;
  candidate: Reading;
  delta: number;
  ratio: number;
}

export interface ComparisonResult {
  comparisons: Comparison[];
  violations: CountersViolation[];
  pendingSignals: string[];
}

/**
 * Compare baseline and candidate evidence files.
 *
 * Pairs readings by compatibilityKeys. Rejects:
 * - Pairs with mismatched compatibility key values (incompatible-comparison)
 * - Pairs that span a declared boundary (spans-boundary)
 * - Signals with no baseline or no candidate reading
 */
export function compareEvidence(
  baseline: EvidenceFile,
  candidate: EvidenceFile,
  config: CountersConfig
): ComparisonResult {
  if (!config) {
    throw new MalformedInput("config.counters is not defined");
  }

  const violations: CountersViolation[] = [];
  const comparisons: Comparison[] = [];
  const pendingSignals = new Set<string>();

  const compatibilityKeys = config.compatibilityKeys;
  const boundaryKeys = new Set(config.boundaries.map((b) => b.key));

  function makeKey(reading: Reading): string {
    const parts: string[] = [];
    for (const key of compatibilityKeys) {
      const value = reading[key as keyof Reading];
      if (value === undefined || value === null) {
        parts.push("(none)");
      } else if (typeof value === "object") {
        parts.push(JSON.stringify(value));
      } else {
        parts.push(String(value));
      }
    }
    return parts.join("|");
  }

  const baselineByKey = new Map<string, Reading>();
  const candidateByKey = new Map<string, Reading>();

  for (const reading of baseline.readings) {
    const key = makeKey(reading);
    if (baselineByKey.has(key)) {
      violations.push({
        kind: "incompatible-comparison",
        detail: `baseline has duplicate readings for key: ${key}`,
      });
    }
    baselineByKey.set(key, reading);
  }

  for (const reading of candidate.readings) {
    const key = makeKey(reading);
    if (candidateByKey.has(key)) {
      violations.push({
        kind: "incompatible-comparison",
        detail: `candidate has duplicate readings for key: ${key}`,
      });
    }
    candidateByKey.set(key, reading);
  }

  const allSignals = new Set([
    ...baseline.readings.map((r) => r.signal),
    ...candidate.readings.map((r) => r.signal),
  ]);

  for (const signal of allSignals) {
    const baselineReadings = baseline.readings.filter((r) => r.signal === signal);
    const candidateReadings = candidate.readings.filter((r) => r.signal === signal);

    if (baselineReadings.length === 0) {
      pendingSignals.add(signal);
      violations.push({
        kind: "no-baseline",
        detail: `signal '${signal}' has no baseline reading`,
      });
      continue;
    }

    if (candidateReadings.length === 0) {
      pendingSignals.add(signal);
      violations.push({
        kind: "no-candidate",
        detail: `signal '${signal}' has no candidate reading`,
      });
      continue;
    }

    for (const baselineReading of baselineReadings) {
      const key = makeKey(baselineReading);
      const candidateReading = candidateByKey.get(key);

      if (!candidateReading) {
        pendingSignals.add(signal);
        continue;
      }

      if (candidateReading.signal !== baselineReading.signal) {
        violations.push({
          kind: "incompatible-comparison",
          detail: `key ${key}: signal mismatch (${baselineReading.signal} vs ${candidateReading.signal})`,
        });
        continue;
      }

      for (const key of compatibilityKeys) {
        const bVal = baselineReading[key as keyof Reading];
        const cVal = candidateReading[key as keyof Reading];
        const bStr = typeof bVal === "object" ? JSON.stringify(bVal) : String(bVal);
        const cStr = typeof cVal === "object" ? JSON.stringify(cVal) : String(cVal);
        if (bStr !== cStr) {
          violations.push({
            kind: "incompatible-comparison",
            detail: `signal '${signal}': ${key} mismatch (${bStr} vs ${cStr})`,
          });
        }
      }

      for (const boundaryKey of boundaryKeys) {
        if (!baselineReading.boundaries || !candidateReading.boundaries) continue;
        const bBoundary = baselineReading.boundaries[boundaryKey] as "before" | "after" | undefined;
        const cBoundary = candidateReading.boundaries[boundaryKey] as "before" | "after" | undefined;
        if (
          bBoundary !== undefined &&
          cBoundary !== undefined &&
          bBoundary !== cBoundary
        ) {
          violations.push({
            kind: "spans-boundary",
            detail: `signal '${signal}' spans boundary '${boundaryKey}' (${bBoundary} → ${cBoundary})`,
          });
        }
      }

      if (
        baselineReading.value !== undefined &&
        candidateReading.value !== undefined
      ) {
        const delta = candidateReading.value - baselineReading.value;
        const ratio =
          baselineReading.value === 0
            ? candidateReading.value === 0
              ? 1
              : Infinity
            : candidateReading.value / baselineReading.value;
        comparisons.push({
          signal,
          baseline: baselineReading,
          candidate: candidateReading,
          delta,
          ratio,
        });
      }
    }
  }

  return {
    comparisons,
    violations,
    pendingSignals: [...pendingSignals].sort(),
  };
}

export function renderComparisonTable(result: ComparisonResult): string {
  const lines = [
    "| Signal | Environment | Baseline | Candidate | Delta | Ratio |",
    "|---|---|---:|---:|---:|---:|",
  ];

  for (const c of result.comparisons) {
    const delta =
      c.delta >= 0 ? `+${c.delta.toFixed(0)}` : c.delta.toFixed(0);
    const ratio = Number.isFinite(c.ratio) ? c.ratio.toFixed(3) : "∞";
    lines.push(
      `| \`${c.signal}\` | ${c.baseline.environment} | ${c.baseline.value ?? "—"} | ${c.candidate.value ?? "—"} | ${delta} | ${ratio} |`
    );
  }

  if (result.pendingSignals.length > 0) {
    lines.push("");
    lines.push("**Pending signals (no paired readings):**");
    lines.push("");
    for (const signal of result.pendingSignals) {
      lines.push(`- \`${signal}\``);
    }
  }

  return lines.join("\n");
}
