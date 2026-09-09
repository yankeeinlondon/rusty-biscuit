import { describe, expect, it } from "vitest";

import { countIdentities, noteFor, selectionCommand } from "../src/capture/command.ts";

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

  describe("provenance note", () => {
    const prior = { revision: "abc123", note: "taken from the working tree" };

    it("records the note the operator supplied", () => {
      expect(noteFor("why this tree", prior, "abc123")).toBe("why this tree");
      expect(noteFor("why this tree", undefined, "abc123")).toBe("why this tree");
    });

    it("carries the previous note across a partial re-capture of the same revision", () => {
      expect(noteFor(undefined, prior, "abc123")).toBe(prior.note);
    });

    it("drops the previous note once the revision moves", () => {
      expect(noteFor(undefined, prior, "def456")).toBeUndefined();
      expect(noteFor(undefined, { revision: "abc123" }, "abc123")).toBeUndefined();
      expect(noteFor(undefined, undefined, "abc123")).toBeUndefined();
    });
  });
});
