import { describe, it, expect } from "vitest";
import {
  median,
  spread,
  established,
  matchesCohort,
  matchesTarget,
  type CohortSpec,
  type TargetSpec,
} from "../src/measure/index.ts";

describe("measure", () => {
  describe("median", () => {
    it("returns the middle value for odd-length arrays", () => {
      expect(median([1, 2, 3])).toBe(2);
      expect(median([5, 1, 3, 2, 4])).toBe(3);
    });

    it("returns the average of two middle values for even-length arrays", () => {
      expect(median([1, 2, 3, 4])).toBe(2.5);
      expect(median([10, 20])).toBe(15);
    });

    it("returns NaN for empty arrays", () => {
      expect(median([])).toBe(NaN);
    });
  });

  describe("spread", () => {
    it("calculates statistics for a set of values", () => {
      const s = spread([1.0, 1.2, 1.1, 1.3]);
      expect(s.n).toBe(4);
      expect(s.min).toBe(1.0);
      expect(s.max).toBe(1.3);
      expect(s.median).toBe(1.15);
      expect(s.driftRatio).toBeCloseTo((1.3 - 1.0) / 1.15, 3);
    });

    it("handles zero median correctly", () => {
      const s = spread([0, 0, 0]);
      expect(s.median).toBe(0);
      expect(s.driftRatio).toBe(0);
    });
  });

  describe("established", () => {
    it("returns true when delta exceeds both drift brackets", () => {
      const baseline = spread([1.0, 1.1, 1.2]);
      const candidate = spread([0.5, 0.6, 0.7]);
      expect(established(baseline, candidate)).toBe(true);
    });

    it("returns false when delta is inside drift bracket", () => {
      const baseline = spread([1.0, 1.1, 1.2]);
      const candidate = spread([1.15, 1.25, 1.35]);
      expect(established(baseline, candidate)).toBe(false);
    });

    it("handles identical distributions", () => {
      const baseline = spread([1.0, 1.0, 1.0]);
      const candidate = spread([1.0, 1.0, 1.0]);
      expect(established(baseline, candidate)).toBe(false);
    });
  });

  describe("matchesCohort", () => {
    const cohort: CohortSpec = {
      name: "test-cohort",
      binaries: ["exact::binary"],
      binaryPrefixes: ["prefix::"],
      prefixes: [{ binary: "special::bin", prefix: "module::" }],
    };

    it("matches exact binary", () => {
      expect(matchesCohort(cohort, "exact::binary", "any_test")).toBe(true);
    });

    it("matches binary prefix", () => {
      expect(matchesCohort(cohort, "prefix::anything", "test")).toBe(true);
    });

    it("matches name prefix for specific binary", () => {
      expect(matchesCohort(cohort, "special::bin", "module::test")).toBe(true);
    });

    it("does not match unrelated binary", () => {
      expect(matchesCohort(cohort, "other::binary", "test")).toBe(false);
    });

    it("does not match wrong name prefix", () => {
      expect(matchesCohort(cohort, "special::bin", "other::test")).toBe(false);
    });
  });

  describe("matchesTarget", () => {
    it("matches exact name when specified", () => {
      const target: TargetSpec = {
        binary: "demo::alpha",
        name: "specific_test",
        contract: "fast",
      };
      expect(matchesTarget(target, "demo::alpha", "specific_test")).toBe(true);
      expect(matchesTarget(target, "demo::alpha", "other_test")).toBe(false);
    });

    it("matches name prefix when specified", () => {
      const target: TargetSpec = {
        binary: "demo::alpha",
        prefix: "integration::",
        contract: "slow",
      };
      expect(matchesTarget(target, "demo::alpha", "integration::test_one")).toBe(true);
      expect(matchesTarget(target, "demo::alpha", "unit::test")).toBe(false);
    });

    it("matches name suffix when specified", () => {
      const target: TargetSpec = {
        binary: "demo::alpha",
        suffix: "::slow",
        contract: "heavy",
      };
      expect(matchesTarget(target, "demo::alpha", "module::test::slow")).toBe(true);
      expect(matchesTarget(target, "demo::alpha", "module::test::fast")).toBe(false);
    });

    it("matches whole binary when no name constraint", () => {
      const target: TargetSpec = {
        binary: "demo::alpha",
        contract: "any",
      };
      expect(matchesTarget(target, "demo::alpha", "any_test_name")).toBe(true);
      expect(matchesTarget(target, "demo::beta", "test")).toBe(false);
    });
  });
});
