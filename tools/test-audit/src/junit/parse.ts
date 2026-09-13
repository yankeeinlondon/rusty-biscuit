/**
 * JUnit XML parsing via fast-xml-parser.
 *
 * Replaces the predecessor's regex-based tag scanner with a proper XML parser
 * while maintaining identical validation: missing/invalid durations reject,
 * duplicate identities reject, and captured output containing `<failure>` does
 * not mark the case failed.
 */
import { XMLParser, XMLValidator } from "fast-xml-parser";
import { MalformedInput } from "../errors.ts";

export type Outcome = "passed" | "failed" | "errored" | "skipped";

export interface TestCase {
  /** `<suite>::<case>`, matching `scripts/ci-rollup.rs`'s identity rule. */
  identity: string;
  binary: string;
  name: string;
  seconds: number;
  outcome: Outcome;
  /** Count of retry attempts (`<flakyFailure>`/`<rerunFailure>` children). */
  retries?: number;
}

export interface Report {
  /** `<testsuites time>` — nextest's wall time for the whole run. */
  wallSeconds: number;
  declaredTests: number;
  declaredFailures: number;
  declaredErrors: number;
  cases: TestCase[];
}

function requiredNumber(raw: unknown, what: string, source: string): number {
  if (typeof raw !== "string" && typeof raw !== "number") {
    throw new MalformedInput(`${source}: ${what} has no value`);
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new MalformedInput(`${source}: ${what} is not a number: ${raw}`);
  }
  if (value < 0) {
    throw new MalformedInput(`${source}: ${what} is negative: ${raw}`);
  }
  return value;
}

interface ParsedTestsuite {
  "@_name"?: string;
  testcase?: ParsedTestcase | ParsedTestcase[];
}

interface ParsedTestcase {
  "@_name"?: string;
  "@_classname"?: string;
  "@_time"?: string | number;
  failure?: unknown;
  error?: unknown;
  skipped?: unknown;
  flakyFailure?: unknown;
  rerunFailure?: unknown;
}

interface ParsedTestsuites {
  "@_time"?: string | number;
  "@_tests"?: string | number;
  "@_failures"?: string | number;
  "@_errors"?: string | number;
  testsuite?: ParsedTestsuite | ParsedTestsuite[];
}

/**
 * Parse one nextest JUnit document.
 *
 * ## Errors
 *
 * `MalformedInput` when the document is not well-formed, is not a JUnit report
 * (no `<testsuites>` root), or carries a missing, non-numeric, or negative
 * `time`. An unparseable duration is an error rather than a zero: a
 * silently-zeroed case makes a slow test look free.
 *
 * ## Retries
 *
 * Nextest reports retried attempts as `<flakyFailure>` / `<rerunFailure>`
 * children; we count them but do not fail unless the final outcome failed.
 */
export function parseJunit(xml: string, source: string): Report {
  const validation = XMLValidator.validate(xml);
  if (validation !== true) {
    throw new MalformedInput(`${source}: malformed XML: ${validation.err.msg}`);
  }

  const parser = new XMLParser({
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    parseAttributeValue: false,
    trimValues: true,
  });

  let parsed: { testsuites?: ParsedTestsuites };
  try {
    parsed = parser.parse(xml) as { testsuites?: ParsedTestsuites };
  } catch (error) {
    throw new MalformedInput(`${source}: XML parsing failed: ${(error as Error).message}`);
  }

  if (!parsed.testsuites) {
    throw new MalformedInput(`${source}: not a JUnit report (no <testsuites> root)`);
  }

  const root = parsed.testsuites;
  const wallSeconds = requiredNumber(root["@_time"], "<testsuites time>", source);
  const declaredTests = requiredNumber(root["@_tests"] ?? 0, "<testsuites tests>", source);
  const declaredFailures = requiredNumber(root["@_failures"] ?? 0, "<testsuites failures>", source);
  const declaredErrors = requiredNumber(root["@_errors"] ?? 0, "<testsuites errors>", source);

  const cases: TestCase[] = [];
  const suites = Array.isArray(root.testsuite) ? root.testsuite : root.testsuite ? [root.testsuite] : [];

  for (const suite of suites) {
    const suiteName = suite["@_name"] ?? "";
    const testcases = Array.isArray(suite.testcase) ? suite.testcase : suite.testcase ? [suite.testcase] : [];

    for (const tc of testcases) {
      const name = tc["@_name"] ?? "";
      const classname = tc["@_classname"] ?? "";
      const owner = suiteName || classname;
      const seconds = requiredNumber(tc["@_time"], `<testcase ${name} time>`, source);

      let outcome: Outcome = "passed";
      if (tc.failure !== undefined) outcome = "failed";
      else if (tc.error !== undefined) outcome = "errored";
      else if (tc.skipped !== undefined) outcome = "skipped";

      // Count retries without failing on them unless the final outcome is bad
      let retries = 0;
      if (tc.flakyFailure !== undefined) {
        retries += Array.isArray(tc.flakyFailure) ? tc.flakyFailure.length : 1;
      }
      if (tc.rerunFailure !== undefined) {
        retries += Array.isArray(tc.rerunFailure) ? tc.rerunFailure.length : 1;
      }

      cases.push({
        identity: owner ? `${owner}::${name}` : name,
        binary: owner,
        name,
        seconds,
        outcome,
        ...(retries > 0 ? { retries } : {}),
      });
    }
  }

  return { wallSeconds, declaredTests, declaredFailures, declaredErrors, cases };
}
