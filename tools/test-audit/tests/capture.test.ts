import { describe, expect, it } from "vitest";

import { countIdentities, selectionCommand } from "../src/capture/command.ts";

describe("capture", () => {
  it("builds the nextest list command from a selection", () => {
    expect(selectionCommand({ label: "x", packages: ["a", "b"], features: [], routes: ["r"] })).toEqual([
      "nextest", "list", "--message-format", "json", "-p", "a", "-p", "b",
    ]);
    expect(selectionCommand({ label: "x", packages: ["a"], features: ["f1", "f2"], routes: ["r"] })).toEqual([
      "nextest", "list", "--message-format", "json", "-p", "a", "--features", "f1,f2",
    ]);
  });

  it("counts identities from nextest's test-count or from the per-suite testcases", () => {
    expect(countIdentities(JSON.stringify({ "test-count": 7, "rust-suites": {} }))).toBe(7);
    expect(
      countIdentities(
        JSON.stringify({ "rust-suites": { a: { testcases: { x: {}, y: {} } }, b: { testcases: { z: {} } } } }),
      ),
    ).toBe(3);
    expect(countIdentities("{}")).toBe(0);
    expect(() => countIdentities("nope")).toThrow();
  });
});
