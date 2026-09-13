/**
 * Source ↔ runner diff and exclusion checking.
 *
 * Compares the source-side population against the runner universe by
 * `(package, leaf name)` multiset — the shape both sides can produce without
 * resolving Cargo target layout — and requires every source-only test to be
 * a declared exclusion. A `macroExpanded` source test (an `#[rstest]` with
 * cases, say) has no leaf of its own in the runner: the runner lists derived
 * names beneath it (`name::case_1`). Those are matched by prefix and reported
 * separately so neither side reads as missing.
 */
import type { Violation } from "../errors.ts";
import type { Universe } from "./listings.ts";
import type { Exclusion } from "./families.ts";
import type { SourceTest } from "./sources.ts";

export interface SourceDiff {
  sourceOnly: { pkg: string; name: string; count: number; files: string[] }[];
  runnerOnly: { pkg: string; name: string; count: number }[];
  /** Parameterized source tests and the runner identities they account for. */
  macroExpanded: { pkg: string; name: string; file: string; runnerIdentities: number }[];
}

export function diffSourceAgainstRunner(source: SourceTest[], universe: Universe): SourceDiff {
  const expanded = new Map<string, { pkg: string; name: string; file: string; runnerIdentities: number }>();
  for (const test of source) {
    if (test.macroExpanded) {
      expanded.set(`${test.pkg} ${test.name}`, { pkg: test.pkg, name: test.name, file: `${test.file}:${test.line}`, runnerIdentities: 0 });
    }
  }
  const runner = new Map<string, number>();
  for (const identity of universe.identities.values()) {
    const segments = identity.name.split("::");
    const leaf = segments[segments.length - 1]!;
    const parent = segments.length > 1 ? segments[segments.length - 2] : undefined;
    const owner = parent !== undefined ? expanded.get(`${identity.pkg} ${parent}`) : undefined;
    if (owner) {
      owner.runnerIdentities += 1;
      continue;
    }
    const key = `${identity.pkg} ${leaf}`;
    runner.set(key, (runner.get(key) ?? 0) + 1);
  }
  const srcCounts = new Map<string, { pkg: string; name: string; count: number; files: string[] }>();
  for (const test of source) {
    const key = `${test.pkg} ${test.name}`;
    const owner = expanded.get(key);
    if (owner && owner.runnerIdentities > 0) continue;
    const entry = srcCounts.get(key) ?? { pkg: test.pkg, name: test.name, count: 0, files: [] };
    entry.count += 1;
    entry.files.push(`${test.file}:${test.line}`);
    srcCounts.set(key, entry);
  }
  const sourceOnly: SourceDiff["sourceOnly"] = [];
  for (const [key, entry] of srcCounts) {
    const run = runner.get(key) ?? 0;
    if (entry.count > run) sourceOnly.push({ ...entry, count: entry.count - run });
  }
  const runnerOnly: SourceDiff["runnerOnly"] = [];
  for (const [key, count] of runner) {
    const [pkg, name] = key.split(" ");
    const src = srcCounts.get(key)?.count ?? 0;
    if (count > src) runnerOnly.push({ pkg: pkg!, name: name!, count: count - src });
  }
  sourceOnly.sort((a, b) => a.pkg.localeCompare(b.pkg) || a.name.localeCompare(b.name));
  runnerOnly.sort((a, b) => a.pkg.localeCompare(b.pkg) || a.name.localeCompare(b.name));
  const macroExpanded = [...expanded.values()].filter((e) => e.runnerIdentities > 0).sort((a, b) => a.pkg.localeCompare(b.pkg) || a.name.localeCompare(b.name));
  return { sourceOnly, runnerOnly, macroExpanded };
}

export function checkExclusions(diff: SourceDiff, exclusions: Exclusion[]): Violation[] {
  const violations: Violation[] = [];
  const declared = new Map(exclusions.map((e) => [`${e.package} ${e.name}`, e]));
  const observed = new Set(diff.sourceOnly.map((s) => `${s.pkg} ${s.name}`));
  for (const entry of diff.sourceOnly) {
    const key = `${entry.pkg} ${entry.name}`;
    if (!declared.has(key)) {
      violations.push({
        kind: "undeclared-exclusion",
        detail: `${entry.pkg} :: ${entry.name} is defined in source (${entry.files.join(", ")}) but never listed by the runner, and no exclusion declares why`,
      });
    }
  }
  for (const [key, entry] of declared) {
    if (!observed.has(key)) {
      violations.push({
        kind: "stale-exclusion",
        detail: `exclusion ${entry.package} :: ${entry.name} no longer describes an unlisted test; delete it`,
      });
    }
  }
  return violations;
}
