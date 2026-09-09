/**
 * Tests for the reconciliation command: family matching, inventory cross-check,
 * source-runner diff, and exclusion validation.
 */
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, writeFileSync, rmSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { MalformedInput } from "../src/errors.ts";
import { parseCapture, buildUniverse, parseMetadata, type Identity, type Universe } from "../src/reconcile/listings.ts";
import { parseFamilyFile, familiesMatching, reconcileFamilies, type Family } from "../src/reconcile/families.ts";
import { parseInventoryFamilyIndex, checkInventory } from "../src/reconcile/inventory.ts";
import { diffSourceAgainstRunner, checkExclusions } from "../src/reconcile/gate.ts";
import type { SourceTest } from "../src/reconcile/sources.ts";

function tempDir(): string {
  return mkdtempSync(join(tmpdir(), "reconcile-test-"));
}

function suite(binaryId: string, pkg: string, kind: string, names: string[]): Record<string, unknown> {
  return {
    "package-name": pkg,
    "binary-id": binaryId,
    kind,
    testcases: Object.fromEntries(names.map((n) => [n, { ignored: false }])),
  };
}

function listing(...suites: Record<string, unknown>[]): string {
  return JSON.stringify({
    "rust-suites": Object.fromEntries(suites.map((s) => [s["binary-id"] as string, s])),
  });
}

const MANIFEST = (labels: string[]) =>
  JSON.stringify({
    revision: "deadbeef",
    toolchain: "rustc 1.97.1",
    nextest: "cargo-nextest 0.9.136",
    platform: "aarch64-apple-darwin",
    captures: labels.map((label) => ({ label, packages: ["p"], features: [], command: `cargo nextest list ${label}` })),
  });

function identity(binaryId: string, name: string, pkg = "test-pkg"): Identity {
  return { id: `${binaryId} :: ${name}`, pkg, binaryId, kind: "lib", name, captures: ["c"] };
}

function universeOf(identities: Identity[]): Universe {
  return { identities: new Map(identities.map((i) => [i.id, i])), emptySuites: [], violations: [] };
}

const FAM = (id: string, match: Family["match"], exclude?: Family["exclude"]): Family => ({
  id,
  package: "test-pkg",
  match,
  exclude,
});

const src = (pkg: string, name: string, file = "f.rs"): SourceTest => ({
  modulePath: [],
  identity: name,
  kind: "lib",
  attributes: ["test"],
  macroExpanded: false,
  pkg,
  file,
  line: 1,
  name,
  cfg: [],
  ignored: false,
  fromMacro: false,
});

describe("Manifest and captures", () => {
  it("rejects a manifest that is not json", () => {
    expect(() => parseMetadata("{not json")).toThrow(MalformedInput);
  });

  it("rejects a manifest with no captures", () => {
    expect(() =>
      parseMetadata(JSON.stringify({ revision: "a", toolchain: "b", nextest: "c", platform: "d", captures: [] })),
    ).toThrow(MalformedInput);
  });

  it("rejects a capture that is not a nextest listing", () => {
    expect(() => parseCapture("c", JSON.stringify({ suites: {} }))).toThrow(MalformedInput);
  });

  it("rejects a capture that is not json", () => {
    expect(() => parseCapture("c", "<html>")).toThrow(MalformedInput);
  });

  it("rejects a suite with no testcases object", () => {
    const text = JSON.stringify({ "rust-suites": { a: { "package-name": "p", "binary-id": "a", kind: "lib" } } });
    expect(() => parseCapture("c", text)).toThrow(MalformedInput);
  });

  it("records a build target that lists no test instead of dropping it", () => {
    const parsed = parseCapture("c", listing(suite("p::bin/p", "p", "bin", [])));
    expect(parsed.identities).toEqual([]);
    expect(parsed.empty.map((e) => e.binaryId)).toEqual(["p::bin/p"]);
  });

  it("a union across captures counts each identity once", () => {
    const dir = tempDir();
    writeFileSync(join(dir, "captures.json"), MANIFEST(["bare", "featured"]));
    writeFileSync(join(dir, "bare.json"), listing(suite("p", "p", "lib", ["a", "b"])));
    writeFileSync(join(dir, "featured.json"), listing(suite("p", "p", "lib", ["a", "b", "c"])));
    const universe = buildUniverse(dir, ["bare", "featured"]);
    expect(universe.identities.size).toBe(3);
    expect(universe.identities.get("p :: a")!.captures).toEqual(["bare", "featured"]);
    expect(universe.identities.get("p :: c")!.captures).toEqual(["featured"]);
    expect(universe.violations).toEqual([]);
    rmSync(dir, { recursive: true });
  });

  it("rejects a declared capture whose file is missing", () => {
    const dir = tempDir();
    writeFileSync(join(dir, "captures.json"), MANIFEST(["present", "absent"]));
    writeFileSync(join(dir, "present.json"), listing(suite("p", "p", "lib", ["a"])));
    const universe = buildUniverse(dir, ["present", "absent"]);
    expect(universe.violations.map((v) => v.kind)).toEqual(["missing-capture"]);
    rmSync(dir, { recursive: true });
  });

  it("rejects a capture file that no config declares", () => {
    const dir = tempDir();
    writeFileSync(join(dir, "captures.json"), MANIFEST(["declared"]));
    writeFileSync(join(dir, "declared.json"), listing(suite("p", "p", "lib", ["a"])));
    writeFileSync(join(dir, "smuggled.json"), listing(suite("q", "q", "lib", ["z"])));
    const universe = buildUniverse(dir, ["declared"]);
    expect(universe.violations.map((v) => v.kind)).toEqual(["malformed-capture"]);
    expect(universe.identities.has("q :: z")).toBe(false);
    rmSync(dir, { recursive: true });
  });

  it("rejects one listing that names an identity twice", () => {
    const text =
      '{"rust-suites":{"one":{"package-name":"p","binary-id":"p","kind":"lib","testcases":{"a":{}}},' +
      '"two":{"package-name":"p","binary-id":"p","kind":"lib","testcases":{"a":{}}}}}';
    expect(() => parseCapture("c", text)).toThrow(MalformedInput);
  });
});

describe("Family matching", () => {
  it("a suite only family claims the whole suite", () => {
    const hits = familiesMatching(identity("test-pkg", "config::tests::a"), [FAM("all", { suites: ["test-pkg"] })]);
    expect(hits.map((h) => h.id)).toEqual(["all"]);
  });

  it("a suite plus module family claims only that module", () => {
    const families = [FAM("narrow", { suites: ["test-pkg"], modules: ["config::atomic"] })];
    expect(familiesMatching(identity("test-pkg", "config::atomic::tests::a"), families).length).toBe(1);
    expect(familiesMatching(identity("test-pkg", "config::claude::tests::a"), families).length).toBe(0);
  });

  it("a module prefix stops at a path separator", () => {
    const families = [FAM("narrow", { modules: ["config::atomic"] })];
    expect(familiesMatching(identity("test-pkg", "config::atomicity::tests::a"), families).length).toBe(0);
    expect(familiesMatching(identity("test-pkg", "config::atomic"), families).length).toBe(1);
  });

  it("a family never reaches another package", () => {
    const families = [FAM("lib", { suites: ["test-pkg"] })];
    const other: Identity = { ...identity("test-pkg", "a"), pkg: "other-pkg" };
    expect(familiesMatching(other, families).length).toBe(0);
  });

  it("an exclude block removes a carve out from its parent", () => {
    const parent = FAM("parent", { suites: ["test-pkg"] }, { modules: ["config::atomic"] });
    const child = FAM("child", { suites: ["test-pkg"], modules: ["config::atomic"] });
    const id = identity("test-pkg", "config::atomic::tests::a");
    expect(familiesMatching(id, [parent, child]).map((f) => f.id)).toEqual(["child"]);
  });

  it("an exact test entry matches the full identity name", () => {
    const families = [FAM("one", { suites: ["test-pkg"], tests: ["a::b::c"] })];
    expect(familiesMatching(identity("test-pkg", "a::b::c"), families).length).toBe(1);
    expect(familiesMatching(identity("test-pkg", "a::b::c::d"), families).length).toBe(0);
  });

  it("an identity matched by no family is a violation", () => {
    const result = reconcileFamilies(universeOf([identity("test-pkg", "orphan")]), [FAM("other", { suites: ["nothing"] })]);
    expect(result.violations.map((v) => v.kind)).toEqual(["unassigned-identity", "stale-family"]);
  });

  it("an identity matched by two families is a violation and is counted by neither", () => {
    const families = [FAM("a", { suites: ["test-pkg"] }), FAM("b", { modules: ["config"] })];
    const result = reconcileFamilies(universeOf([identity("test-pkg", "config::x")]), families);
    const doubles = result.violations.filter((v) => v.kind === "double-assigned-identity");
    expect(doubles.length).toBe(1);
    expect(doubles[0]!.detail).toMatch(/a, b/);
    expect(result.counts.get("a")).toBe(0);
    expect(result.counts.get("b")).toBe(0);
  });

  it("a family matching nothing is stale unless it declares why", () => {
    const empty = FAM("empty", { suites: ["gone"] });
    const declared: Family = { ...FAM("declared", { suites: ["gone"] }), expectEmpty: "windows-only suite" };
    const withStale = reconcileFamilies(universeOf([identity("test-pkg", "a")]), [FAM("real", { suites: ["test-pkg"] }), empty]);
    expect(withStale.violations.some((v) => v.kind === "stale-family")).toBe(true);
    const withDeclared = reconcileFamilies(universeOf([identity("test-pkg", "a")]), [FAM("real", { suites: ["test-pkg"] }), declared]);
    expect(withDeclared.violations.length).toBe(0);
  });

  it("rejects a family file with duplicate ids", () => {
    const text = JSON.stringify({
      families: [
        { id: "x", package: "p", match: { suites: ["a"] } },
        { id: "x", package: "p", match: { suites: ["b"] } },
      ],
    });
    expect(() => parseFamilyFile(text)).toThrow(MalformedInput);
  });

  it("rejects a family with an empty match block", () => {
    const text = JSON.stringify({ families: [{ id: "x", package: "p", match: {} }] });
    expect(() => parseFamilyFile(text)).toThrow(MalformedInput);
  });
});

describe("inventory.md cross-check", () => {
  it("reads the family index regardless of column order", () => {
    const md = ["| Disposition | Identities | Family | Package |", "|---|---:|---|---|", "| satisfactory | 12 | `alpha` | `p` |"].join("\n");
    expect(parseInventoryFamilyIndex(md)).toEqual([{ id: "alpha", identities: 12, disposition: "satisfactory" }]);
  });

  it("ignores tables that are not the family index", () => {
    const md = [
      "| Gate | Result |",
      "|---|---|",
      "| lint | green |",
      "",
      "| Family | Identities | Disposition |",
      "|---|---:|---|",
      "| `a` | 1 | satisfactory |",
    ].join("\n");
    expect(parseInventoryFamilyIndex(md).map((r) => r.id)).toEqual(["a"]);
  });

  it("rejects a family row with a non numeric identity count", () => {
    const md = "| Family | Identities | Disposition |\n|---|---|---|\n| `a` | many | satisfactory |";
    expect(() => parseInventoryFamilyIndex(md)).toThrow(MalformedInput);
  });

  it("a family with no inventory row is drift", () => {
    const violations = checkInventory([], [FAM("a", { suites: ["x"] })], new Map([["a", 3]]));
    expect(violations.map((v) => v.kind)).toEqual(["inventory-drift"]);
  });

  it("an inventory row naming no family is drift", () => {
    const violations = checkInventory([{ id: "ghost", identities: 1, disposition: "satisfactory" }], [], new Map());
    expect(violations.map((v) => v.kind)).toEqual(["inventory-drift"]);
  });

  it("an inventory count that disagrees with the captures is drift", () => {
    const violations = checkInventory([{ id: "a", identities: 4, disposition: "satisfactory" }], [FAM("a", { suites: ["x"] })], new Map([["a", 3]]));
    expect(violations.length).toBe(1);
    expect(violations[0]!.detail).toMatch(/inventory says 4 identities, the captures say 3/);
  });

  it("an empty or unrecognized disposition fails", () => {
    const families = [FAM("a", { suites: ["x"] })];
    const counts = new Map([["a", 1]]);
    expect(checkInventory([{ id: "a", identities: 1, disposition: "" }], families, counts).map((v) => v.kind)).toEqual(["missing-disposition"]);
    expect(checkInventory([{ id: "a", identities: 1, disposition: "looks fine to me" }], families, counts).map((v) => v.kind)).toEqual([
      "missing-disposition",
    ]);
    expect(checkInventory([{ id: "a", identities: 1, disposition: "follow-up: linked spec" }], families, counts)).toEqual([]);
  });
});

describe("Source ↔ runner diff and exclusions", () => {
  it("a source test the runner never lists is source only", () => {
    const diff = diffSourceAgainstRunner([src("test-pkg", "a"), src("test-pkg", "windows_only")], universeOf([identity("test-pkg", "mod::tests::a")]));
    expect(diff.sourceOnly.map((s) => s.name)).toEqual(["windows_only"]);
    expect(diff.runnerOnly).toEqual([]);
  });

  it("a per platform pair of the same name shows one source only copy", () => {
    const diff = diffSourceAgainstRunner([src("test-pkg", "both"), src("test-pkg", "both")], universeOf([identity("test-pkg", "mod::tests::both")]));
    expect(diff.sourceOnly.map((s) => [s.name, s.count])).toEqual([["both", 1]]);
  });

  it("a runner identity with no source definition is reported", () => {
    const diff = diffSourceAgainstRunner([], universeOf([identity("test-pkg", "generated")]));
    expect(diff.runnerOnly.map((r) => r.name)).toEqual(["generated"]);
  });

  it("an undeclared exclusion fails the gate", () => {
    const diff = diffSourceAgainstRunner([src("test-pkg", "unlisted")], universeOf([]));
    expect(checkExclusions(diff, []).map((v) => v.kind)).toEqual(["undeclared-exclusion"]);
  });

  it("a declared exclusion that now runs is stale", () => {
    const diff = diffSourceAgainstRunner([src("test-pkg", "a")], universeOf([identity("test-pkg", "mod::a")]));
    const violations = checkExclusions(diff, [{ package: "test-pkg", name: "a", gate: "cfg(windows)", route: "windows-latest" }]);
    expect(violations.map((v) => v.kind)).toEqual(["stale-exclusion"]);
  });

  it("a declared exclusion that still holds passes", () => {
    const diff = diffSourceAgainstRunner([src("test-pkg", "win")], universeOf([]));
    expect(checkExclusions(diff, [{ package: "test-pkg", name: "win", gate: "cfg(windows)", route: "windows-latest" }])).toEqual([]);
  });
});
