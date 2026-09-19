/**
 * Nextest capture parsing and universe building.
 *
 * Reads `cargo nextest list --message-format json` outputs and builds the
 * runner identity universe, tracking which captures list each identity.
 */
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

import { MalformedInput, type Violation } from "../errors.ts";

export interface Identity {
  /** `<binaryId> :: <name>` */
  id: string;
  pkg: string;
  binaryId: string;
  kind: string;
  name: string;
  /** Capture labels that listed this identity. */
  captures: string[];
}

export interface Universe {
  identities: Map<string, Identity>;
  emptySuites: { pkg: string; binaryId: string; kind: string }[];
  violations: Violation[];
}

export interface CaptureMetadata {
  revision?: string | undefined;
  toolchain?: string | undefined;
  nextest?: string | undefined;
  platform?: string | undefined;
  captures: { label: string; packages: string[]; features: string[]; command: string }[];
}

function requireString(value: unknown, what: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new MalformedInput(`${what}: expected a non-empty string, got ${JSON.stringify(value)}`);
  }
  return value;
}

function requireArray(value: unknown, what: string): unknown[] {
  if (!Array.isArray(value)) throw new MalformedInput(`${what}: expected an array`);
  return value;
}

export function parseCapture(label: string, text: string): { identities: Omit<Identity, "captures">[]; empty: Universe["emptySuites"] } {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (error) {
    throw new MalformedInput(`${label}: not JSON (${(error as Error).message})`);
  }
  const suites = (raw as Record<string, unknown>)["rust-suites"];
  if (suites === undefined || suites === null || typeof suites !== "object") {
    throw new MalformedInput(`${label}: no "rust-suites" object; this is not a nextest listing`);
  }
  const identities: Omit<Identity, "captures">[] = [];
  const empty: Universe["emptySuites"] = [];
  const seen = new Set<string>();
  for (const suite of Object.values(suites as Record<string, unknown>)) {
    const s = suite as Record<string, unknown>;
    const pkg = requireString(s["package-name"], `${label}: package-name`);
    const binaryId = requireString(s["binary-id"], `${label}: binary-id`);
    const kind = requireString(s.kind, `${label}: kind`);
    const cases = s.testcases;
    if (cases === undefined || cases === null || typeof cases !== "object") {
      throw new MalformedInput(`${label}: suite ${binaryId} has no testcases object`);
    }
    const names = Object.keys(cases as Record<string, unknown>);
    if (names.length === 0) {
      empty.push({ pkg, binaryId, kind });
      continue;
    }
    for (const name of names) {
      const id = `${binaryId} :: ${name}`;
      if (seen.has(id)) {
        throw new MalformedInput(`${label}: duplicate identity ${id} within one listing`);
      }
      seen.add(id);
      identities.push({ id, pkg, binaryId, kind, name });
    }
  }
  return { identities, empty };
}

export function parseMetadata(text: string): CaptureMetadata {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (error) {
    throw new MalformedInput(`captures.json is not JSON: ${(error as Error).message}`);
  }
  const obj = raw as Record<string, unknown>;
  const captures = requireArray(obj.captures, "captures").map((entry, index) => {
    const c = entry as Record<string, unknown>;
    return {
      label: requireString(c.label, `captures[${index}].label`),
      packages: requireArray(c.packages, `captures[${index}].packages`).map((p, j) =>
        requireString(p, `captures[${index}].packages[${j}]`),
      ),
      features: requireArray(c.features ?? [], `captures[${index}].features`).map((f, j) =>
        requireString(f, `captures[${index}].features[${j}]`),
      ),
      command: requireString(c.command, `captures[${index}].command`),
    };
  });
  if (captures.length === 0) throw new MalformedInput("captures.json declares no captures");
  return {
    revision: typeof obj.revision === "string" ? obj.revision : undefined,
    toolchain: typeof obj.toolchain === "string" ? obj.toolchain : undefined,
    nextest: typeof obj.nextest === "string" ? obj.nextest : undefined,
    platform: typeof obj.platform === "string" ? obj.platform : undefined,
    captures,
  };
}

export function buildUniverse(dir: string, labels: string[]): Universe {
  const identities = new Map<string, Identity>();
  const emptySuites = new Map<string, Universe["emptySuites"][number]>();
  const violations: Violation[] = [];

  const declared = new Set(labels.map((l) => `${l}.json`));
  const onDisk = new Set(readdirSync(dir).filter((f) => f.endsWith(".json") && f !== "captures.json"));

  for (const file of onDisk) {
    if (!declared.has(file)) {
      violations.push({
        kind: "malformed-capture",
        detail: `${file} is present in the enumeration directory but not declared in the config`,
      });
    }
  }

  for (const label of labels) {
    const path = join(dir, `${label}.json`);
    if (!existsSync(path)) {
      violations.push({ kind: "missing-capture", detail: `declared capture ${label} has no ${label}.json` });
      continue;
    }
    const parsed = parseCapture(label, readFileSync(path, "utf8"));
    for (const entry of parsed.identities) {
      const existing = identities.get(entry.id);
      if (existing) existing.captures.push(label);
      else identities.set(entry.id, { ...entry, captures: [label] });
    }
    for (const e of parsed.empty) emptySuites.set(e.binaryId, e);
  }
  return { identities, emptySuites: [...emptySuites.values()], violations };
}
