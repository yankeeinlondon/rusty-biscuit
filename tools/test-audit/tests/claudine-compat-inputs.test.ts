/**
 * The property that keeps this shared package's suite honest:
 *
 * **A consuming package area renaming one of its own families must not be able
 * to turn `tools/test-audit`'s tests red.**
 *
 * The compatibility replays consume evidence frozen at one revision. If they
 * judge it with a classifier or an expectation read live out of the consuming
 * fix directory, then every legitimate edit there — a family rename, an
 * inventory recount, a retaken capture — lands as a failure here, in a package
 * that did nothing wrong. That is not hypothetical: review 1 of
 * `2026-09-07-faster-claudine-tests` reclassified nineteen PTY tests, renamed
 * `cli-l2-pty` to `cli-l1-pty-interactive`, and three tests here went red on a
 * correct change.
 *
 * So this is a source guard, in the shape the Rust areas use: every
 * `*-claudine-compat.test.ts` must reach its inputs through
 * `claudine-compat-inputs.ts`, and the live documents are named as forbidden
 * rather than merely absent. Live-consumer health belongs to the consumer's own
 * `test-audit reconcile` run, not to this suite.
 */
import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { parseMetadata } from "../src/reconcile/listings.ts";
import {
  ERA_ENUMERATION_DIR,
  ERA_EXPECTATIONS_PATH,
  ERA_FAMILIES_PATH,
  ERA_LOG_PATHS,
  ERA_REVISION,
  expectations,
} from "./claudine-compat-inputs.ts";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * Live consumer documents a replay may not read.
 *
 * Each is matched as a path *segment* so a frozen sibling with a similar name
 * (`fixtures/claudine-compat/families.json`) is not caught. `config.familiesPath`
 * is listed because it resolves to the live file without naming it.
 */
const FORBIDDEN: { pattern: RegExp; why: string }[] = [
  { pattern: /familiesPath/, why: "resolves the consuming area's live families.json" },
  { pattern: /["'`][^"'`]*fixes\/[^"'`]*\/families\.json/, why: "the live families.json" },
  { pattern: /inventory\.md/, why: "the live inventory.md" },
  { pattern: /attribution\.md/, why: "the live attribution.md" },
  { pattern: /results\.md/, why: "the live results.md" },
  { pattern: /["'`]enumeration["'`]|enumeration\/(?!recipes|9)/, why: "the live top-level enumeration/ listings" },
];

function compatTestSources(): { file: string; text: string }[] {
  return readdirSync(here)
    .filter((f) => f.endsWith("-claudine-compat.test.ts"))
    .map((f) => ({ file: f, text: readFileSync(join(here, f), "utf8") }));
}

/** Blank comments so prose about a forbidden document is not a hit. */
function stripComments(text: string): string {
  return text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^[ \t]*\/\/.*$/gm, "");
}

describe("Claudine compatibility replays read only frozen inputs", () => {
  it("finds the replays it is guarding", () => {
    const files = compatTestSources().map((s) => s.file).sort();
    expect(files).toEqual([
      "aggregate-claudine-compat.test.ts",
      "attribute-claudine-compat.test.ts",
      "junit-claudine-compat.test.ts",
      "measure-claudine-compat.test.ts",
      "reconcile-claudine-compat.test.ts",
    ]);
  });

  it("no replay reaches a live consumer document", () => {
    const hits: string[] = [];
    for (const { file, text } of compatTestSources()) {
      const code = stripComments(text);
      for (const { pattern, why } of FORBIDDEN) {
        if (pattern.test(code)) hits.push(`${file}: reads ${why} (/${pattern.source}/)`);
      }
    }
    expect(hits).toEqual([]);
  });

  it("no replay hard-codes an absolute path", () => {
    const hits: string[] = [];
    for (const { file, text } of compatTestSources()) {
      for (const match of stripComments(text).matchAll(/["'`](\/[A-Za-z][^"'`\n]*|[A-Za-z]:\\[^"'`\n]*)["'`]/g)) {
        hits.push(`${file}: ${match[1]}`);
      }
    }
    expect(hits).toEqual([]);
  });

  it("every frozen input the module names is present", () => {
    for (const path of [ERA_ENUMERATION_DIR, ERA_FAMILIES_PATH, ERA_EXPECTATIONS_PATH, ...ERA_LOG_PATHS]) {
      expect(existsSync(path), `missing frozen input ${path}`).toBe(true);
    }
  });

  it("the preserved listings and the snapshotted expectations agree on their revision", () => {
    const metadata = parseMetadata(readFileSync(join(ERA_ENUMERATION_DIR, "captures.json"), "utf8"));
    expect(metadata.revision).toMatch(new RegExp(`^${ERA_REVISION}`));
    expect(expectations().revision).toBe(metadata.revision);
  });
});
