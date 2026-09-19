/**
 * Family declaration parsing and identity matching.
 *
 * A family is a declarative pattern over the runner universe: `suites` alone
 * claims whole suites; `suites` plus `modules`/`tests` narrows within them;
 * `modules`/`tests` alone reach every suite of the package.
 */
import { MalformedInput, type Violation } from "../errors.ts";
import type { Identity, Universe } from "./listings.ts";

export interface FamilyMatch {
  suites?: string[];
  modules?: string[];
  tests?: string[];
}

export interface Family {
  id: string;
  package: string;
  match: FamilyMatch;
  exclude?: FamilyMatch | undefined;
  expectEmpty?: string | undefined;
}

export interface Exclusion {
  package: string;
  name: string;
  gate: string;
  route: string;
}

export interface FamilyFile {
  families: Family[];
  exclusions: Exclusion[];
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

export function parseFamilyFile(text: string): FamilyFile {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (error) {
    throw new MalformedInput(`families.json is not JSON: ${(error as Error).message}`);
  }
  const obj = raw as Record<string, unknown>;
  const parseMatch = (value: unknown, what: string): FamilyMatch | undefined => {
    if (value === undefined) return undefined;
    const m = value as Record<string, unknown>;
    const out: FamilyMatch = {};
    for (const key of ["suites", "modules", "tests"] as const) {
      if (m[key] === undefined) continue;
      out[key] = requireArray(m[key], `${what}.${key}`).map((v, i) => requireString(v, `${what}.${key}[${i}]`));
    }
    if (Object.keys(out).length === 0) throw new MalformedInput(`${what}: declares no suites, modules or tests`);
    return out;
  };
  const families = requireArray(obj.families, "families").map((entry, index) => {
    const f = entry as Record<string, unknown>;
    const id = requireString(f.id, `families[${index}].id`);
    const match = parseMatch(f.match, `families[${index}].match`);
    if (!match) throw new MalformedInput(`families[${index}] (${id}): no match block`);
    return {
      id,
      package: requireString(f.package, `families[${index}].package`),
      match,
      exclude: parseMatch(f.exclude, `families[${index}].exclude`),
      expectEmpty: f.expectEmpty === undefined ? undefined : requireString(f.expectEmpty, `families[${index}].expectEmpty`),
    };
  });
  const ids = new Set<string>();
  for (const f of families) {
    if (ids.has(f.id)) throw new MalformedInput(`families: duplicate id ${f.id}`);
    ids.add(f.id);
  }
  const exclusions = requireArray(obj.exclusions ?? [], "exclusions").map((entry, index) => {
    const e = entry as Record<string, unknown>;
    return {
      package: requireString(e.package, `exclusions[${index}].package`),
      name: requireString(e.name, `exclusions[${index}].name`),
      gate: requireString(e.gate, `exclusions[${index}].gate`),
      route: requireString(e.route, `exclusions[${index}].route`),
    };
  });
  return { families, exclusions };
}

function matchesBlock(identity: Identity, block: FamilyMatch | undefined): boolean {
  if (!block) return false;
  if (block.suites && !block.suites.includes(identity.binaryId)) return false;
  if (!block.modules && !block.tests) return block.suites !== undefined;
  if (block.tests?.includes(identity.name)) return true;
  if (block.modules?.some((m) => identity.name === m || identity.name.startsWith(`${m}::`))) return true;
  return false;
}

export function familiesMatching(identity: Identity, families: Family[]): Family[] {
  return families.filter((family) => {
    if (family.package !== identity.pkg) return false;
    if (!matchesBlock(identity, family.match)) return false;
    if (family.exclude && matchesBlock(identity, family.exclude)) return false;
    return true;
  });
}

export interface ReconcileResult {
  violations: Violation[];
  counts: Map<string, number>;
}

export function reconcileFamilies(universe: Universe, families: Family[]): ReconcileResult {
  const violations: Violation[] = [];
  const counts = new Map<string, number>(families.map((f) => [f.id, 0]));
  for (const identity of universe.identities.values()) {
    const hits = familiesMatching(identity, families);
    if (hits.length === 0) {
      violations.push({ kind: "unassigned-identity", detail: `${identity.id} matches no family` });
      continue;
    }
    if (hits.length > 1) {
      violations.push({
        kind: "double-assigned-identity",
        detail: `${identity.id} matches ${hits.length} families: ${hits.map((h) => h.id).join(", ")}`,
      });
      continue;
    }
    const family = hits[0];
    if (family) {
      counts.set(family.id, (counts.get(family.id) ?? 0) + 1);
    }
  }
  for (const family of families) {
    const count = counts.get(family.id);
    if ((count ?? 0) === 0 && family.expectEmpty === undefined) {
      violations.push({
        kind: "stale-family",
        detail: `family ${family.id} matches no identity and declares no expectEmpty reason`,
      });
    }
  }
  return { violations, counts };
}
