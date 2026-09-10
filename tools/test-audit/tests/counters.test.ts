import { describe, it, expect } from "vitest";
import {
  validateEvidence,
  compareEvidence,
  type EvidenceFile,
  type Reading,
} from "../src/counters/index.ts";
import type { CountersConfig } from "../src/config.ts";

const MOCK_CONFIG: CountersConfig = {
  signals: [
    { id: "parse-calls", description: "Parser invocations", coverage: "instrumented", source: "counter" },
    { id: "cache-hits", description: "Cache hits", coverage: "instrumented", source: "counter" },
    { id: "future-signal", description: "Not yet implemented", coverage: "pending", source: "placeholder" },
  ],
  compatibilityKeys: ["signal", "environment", "platformKind", "requestShape", "counterVersion", "phase"],
  boundaries: [{ key: "cache-strategy", description: "Caching implementation changed" }],
};

function makeReading(overrides: Partial<Reading>): Reading {
  return {
    signal: "parse-calls",
    value: 100,
    revision: "baseline",
    environment: "macos-native",
    os: "macos",
    platformKind: "native",
    requestShape: "default",
    counterVersion: "v1",
    phase: "execution",
    collector: { propagated: false },
    source: "test",
    ...overrides,
  };
}

describe("counters", () => {
  describe("validateEvidence", () => {
    it("accepts valid evidence with known signals", () => {
      const evidence: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "parse-calls", value: 100 })],
      };

      const violations = validateEvidence(evidence, MOCK_CONFIG);
      expect(violations).toHaveLength(0);
    });

    it("rejects unknown signals", () => {
      const evidence: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "unknown-signal", value: 100 })],
      };

      const violations = validateEvidence(evidence, MOCK_CONFIG);
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("unknown-signal");
    });

    it("rejects pending signals with values", () => {
      const evidence: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "future-signal", value: 100 })],
      };

      const violations = validateEvidence(evidence, MOCK_CONFIG);
      expect(violations).toHaveLength(1);
      expect(violations[0]!.kind).toBe("pending-signal-with-value");
    });

    it("accepts pending signals without values", () => {
      const evidence: EvidenceFile = {
        area: "test",
        readings: [
          {
            ...makeReading({ signal: "future-signal" }),
            value: undefined,
          },
        ],
      };

      const violations = validateEvidence(evidence, MOCK_CONFIG);
      expect(violations).toHaveLength(0);
    });
  });

  describe("compareEvidence", () => {
    it("pairs readings by compatibility keys", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "parse-calls", value: 100, revision: "baseline" })],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "parse-calls", value: 80, revision: "candidate" })],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.violations).toHaveLength(0);
      expect(result.comparisons).toHaveLength(1);
      expect(result.comparisons[0]!.delta).toBe(-20);
      expect(result.comparisons[0]!.ratio).toBe(0.8);
    });

    it("rejects incompatible platform kinds", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [makeReading({ platformKind: "native", revision: "baseline" })],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [makeReading({ platformKind: "wsl", revision: "candidate" })],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.comparisons).toHaveLength(0);
      expect(result.pendingSignals).toContain("parse-calls");
    });

    it("rejects incompatible counter versions", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [makeReading({ counterVersion: "v1", revision: "baseline" })],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [makeReading({ counterVersion: "v2", revision: "candidate" })],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.comparisons).toHaveLength(0);
    });

    it("rejects incompatible request shapes", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [makeReading({ requestShape: "minimal", revision: "baseline" })],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [makeReading({ requestShape: "full", revision: "candidate" })],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.comparisons).toHaveLength(0);
    });

    it("detects boundary spanning", () => {
      const baselineReading = makeReading({
        revision: "baseline",
      });
      baselineReading.boundaries = { "cache-strategy": "before" };

      const candidateReading = makeReading({
        revision: "candidate",
      });
      candidateReading.boundaries = { "cache-strategy": "after" };

      const baseline: EvidenceFile = {
        area: "test",
        readings: [baselineReading],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [candidateReading],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.violations.some((v) => v.kind === "spans-boundary")).toBe(true);
    });

    it("reports signals with no baseline reading", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "parse-calls", revision: "candidate" })],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.pendingSignals).toContain("parse-calls");
      expect(result.violations.some((v) => v.kind === "no-baseline")).toBe(true);
    });

    it("reports signals with no candidate reading", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [makeReading({ signal: "parse-calls", revision: "baseline" })],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.pendingSignals).toContain("parse-calls");
      expect(result.violations.some((v) => v.kind === "no-candidate")).toBe(true);
    });

    it("handles multiple readings for the same signal", () => {
      const baseline: EvidenceFile = {
        area: "test",
        readings: [
          makeReading({ environment: "macos-native", value: 100, revision: "baseline" }),
          makeReading({ environment: "linux-native", value: 110, revision: "baseline" }),
        ],
      };

      const candidate: EvidenceFile = {
        area: "test",
        readings: [
          makeReading({ environment: "macos-native", value: 90, revision: "candidate" }),
          makeReading({ environment: "linux-native", value: 100, revision: "candidate" }),
        ],
      };

      const result = compareEvidence(baseline, candidate, MOCK_CONFIG);
      expect(result.violations).toHaveLength(0);
      expect(result.comparisons).toHaveLength(2);
    });
  });
});
