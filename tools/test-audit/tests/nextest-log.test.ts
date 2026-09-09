import { describe, it, expect } from "vitest";
import { parseLog, packageOf } from "../src/nextest-log.ts";
import { MalformedInput } from "../src/errors.ts";

describe("nextest-log", () => {
  describe("packageOf", () => {
    it("extracts package from binary id with double colon", () => {
      expect(packageOf("claudine-cli::context_command")).toBe("claudine-cli");
      expect(packageOf("demo::alpha")).toBe("demo");
    });

    it("returns the whole string for bare suite names", () => {
      expect(packageOf("claudine")).toBe("claudine");
    });
  });

  describe("parseLog", () => {
    it("parses a simple passing run", () => {
      const log = [
        "    Starting 2 tests across 1 binary",
        "        PASS [   0.123s] (   1/2) demo::alpha test_one",
        "        PASS [   0.456s] (   2/2) demo::alpha test_two",
        "     Summary [   0.600s] 2 tests run: 2 passed",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      expect(invocations).toHaveLength(1);

      const inv = invocations[0]!;
      expect(inv.declared).toEqual({ tests: 2, binaries: 1, skipped: 0 });
      expect(inv.results).toHaveLength(2);
      expect(inv.results[0]).toMatchObject({
        binaryId: "demo::alpha",
        name: "test_one",
        status: "PASS",
        durationS: 0.123,
        attempt: 1,
      });
      expect(inv.summary.run).toBe(2);
      expect(inv.summary.passed).toBe(2);
      expect(inv.summary.elapsedS).toBe(0.6);
    });

    it("handles TRY retry lines correctly", () => {
      const log = [
        "    Starting 1 tests across 1 binary",
        "    TRY 1     FAIL [   0.100s] (   1/1) demo::flaky test",
        "    TRY 2     FAIL [   0.110s] (   1/1) demo::flaky test",
        "        PASS [   0.120s] (   1/1) demo::flaky test",
        "     Summary [   0.330s] 1 tests run: 1 passed (1 flaky)",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      expect(invocations).toHaveLength(1);

      const results = invocations[0]!.results;
      expect(results).toHaveLength(1);
      expect(results[0]!.attempt).toBe(3);
      expect(results[0]!.status).toBe("PASS");
      expect(results[0]!.durationS).toBe(0.12);
    });

    it("throws MalformedInput when log has no summary", () => {
      const log = [
        "    Starting 1 tests across 1 binary",
        "        PASS [   0.123s] (   1/1) demo::alpha test",
      ].join("\n");

      expect(() => parseLog(log, "test.log")).toThrow(MalformedInput);
      expect(() => parseLog(log, "test.log")).toThrow(/no nextest summary line/);
    });

    it("throws MalformedInput when summary appears without Starting", () => {
      const log = [
        "        PASS [   0.123s] (   1/1) demo::alpha test",
        "     Summary [   0.123s] 1 tests run: 1 passed",
        "    Starting 1 tests across 1 binary",
      ].join("\n");

      expect(() => parseLog(log, "test.log")).toThrow(MalformedInput);
    });

    it("parses multiple invocations in one log", () => {
      const log = [
        "    Starting 1 tests across 1 binary",
        "        PASS [   0.100s] (   1/1) pkg1::bin test",
        "     Summary [   0.100s] 1 tests run: 1 passed",
        "    Starting 1 tests across 1 binary",
        "        PASS [   0.200s] (   1/1) pkg2::bin test",
        "     Summary [   0.200s] 1 tests run: 1 passed",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      expect(invocations).toHaveLength(2);
      expect(invocations[0]!.results[0]!.binaryId).toBe("pkg1::bin");
      expect(invocations[1]!.results[0]!.binaryId).toBe("pkg2::bin");
    });

    it("parses summary with failures and timeouts", () => {
      const log = [
        "    Starting 3 tests across 1 binary",
        "        PASS [   0.100s] (   1/3) demo::alpha test_pass",
        "        FAIL [   0.200s] (   2/3) demo::alpha test_fail",
        "     TIMEOUT [   5.000s] (   3/3) demo::alpha test_timeout",
        "     Summary [   5.300s] 3 tests run: 1 passed, 1 failed, 1 timed out",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      const summary = invocations[0]!.summary;
      expect(summary.run).toBe(3);
      expect(summary.passed).toBe(1);
      expect(summary.failed).toBe(1);
      expect(summary.timedOut).toBe(1);
    });

    it("ignores SLOW informational lines", () => {
      const log = [
        "    Starting 2 tests across 1 binary",
        "        SLOW [>  10.000s] (   1/2) demo::alpha slow_test",
        "        PASS [  12.000s] (   1/2) demo::alpha slow_test",
        "        PASS [   0.100s] (   2/2) demo::alpha fast_test",
        "     Summary [  12.100s] 2 tests run: 2 passed (1 slow)",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      expect(invocations[0]!.results).toHaveLength(2);
      expect(invocations[0]!.slowMarks).toBe(1);
    });

    it("strips ANSI codes before parsing", () => {
      const log = [
        "\x1b[32m    Starting 1 tests across 1 binary\x1b[0m",
        "\x1b[32m        PASS\x1b[0m [   0.123s] (   1/1) demo::alpha test",
        "\x1b[32m     Summary\x1b[0m [   0.123s] 1 tests run: 1 passed",
      ].join("\n");

      const invocations = parseLog(log, "test.log");
      expect(invocations).toHaveLength(1);
      expect(invocations[0]!.results[0]!.status).toBe("PASS");
    });
  });
});
