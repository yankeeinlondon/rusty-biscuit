/**
 * Area configuration: the one place a package area tells the engine what it
 * owns. Everything Claudine's first-generation scripts hard-coded — package
 * roots, family declarations, environment matrices, required tests, timeout
 * floors — is declared here and validated on load. The engine never learns an
 * area's product behavior; it learns which packages, selections, routes, and
 * environments exist, and what the area expects each artifact to contain.
 *
 * Paths inside the file are relative to the file itself, so a config can be
 * copied between fix directories without editing.
 */
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, resolve } from "node:path";
import { z } from "zod";

import { ConfigError } from "./errors.ts";

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

export const PackageSchema = z.object({
  /** Cargo package name, as `cargo nextest list` reports it. */
  name: z.string().min(1),
  /** Package directory relative to the repository root. */
  path: z.string().min(1),
  /** Directories the source scan walks; defaults to `[path]`. */
  sourceRoots: z.array(z.string().min(1)).optional(),
  note: z.string().optional(),
});

export const RouteKindSchema = z.enum([
  "nextest",
  "doctest",
  "bench",
  "fuzz",
  "cargo-check",
  "script",
  "manual",
  "absent",
]);

/**
 * An execution route: a `just` recipe, CI step, or documented absence. A route
 * row is a declaration that the route exists, never evidence that it ran;
 * `executes` records whether the route runs tests at all.
 */
export const RouteSchema = z.object({
  id: z.string().min(1),
  kind: RouteKindSchema,
  /** The command an operator runs, e.g. `just test` (from the area directory). */
  command: z.string().min(1),
  /** Where CI executes the route (`_package-ci.yml:519`), if it does. */
  ci: z.string().optional(),
  /** Whether the route executes tests. `manual`/`absent` routes are `false`. */
  executes: z.boolean(),
  note: z.string().optional(),
});

/**
 * One package × feature selection a recipe or CI leg hands nextest. Each
 * selection is captured once per source state as
 * `<enumeration>/<label>.json` (`cargo nextest list --message-format json`).
 */
export const SelectionSchema = z.object({
  label: z.string().regex(/^[a-z0-9][a-z0-9-]*$/, "label must be kebab-case"),
  packages: z.array(z.string().min(1)).min(1),
  features: z.array(z.string()),
  env: z.record(z.string(), z.string()).optional(),
  /** Route ids that use this selection. */
  routes: z.array(z.string().min(1)).min(1),
  note: z.string().optional(),
});

/**
 * A population a recipe actually runs: local-default versus CI-selected L1,
 * the sanity subset, an L2 tier. Cohorts are how "the same recipe selects
 * different tests in CI" is kept as data instead of a footnote.
 */
export const CohortSchema = z.object({
  id: z.string().regex(/^[a-z0-9][a-z0-9-]*$/, "id must be kebab-case"),
  description: z.string().min(1),
  population: z.enum(["local", "ci"]),
  recipe: z.string().min(1),
  env: z.record(z.string(), z.string()).optional(),
  /** Selection labels the recipe builds and runs. */
  selections: z.array(z.string().min(1)).min(1),
  /** nextest filterset the recipe applies, when known. */
  filter: z.string().optional(),
  /** Tiers the cohort covers (`L1`, `L2`, `browser`, ...). */
  tiers: z.array(z.string().min(1)).default(["L1"]),
  note: z.string().optional(),
});

export const CellSchema = z.object({ tier: z.string().min(1), package: z.string().min(1) });

/**
 * A CI environment leg. Legs are not rectangular: each declares its own cells,
 * and a leg may be `pending` (declared but with no compatible artifact yet).
 * Native and WSL legs stay distinct even when they share an OS.
 */
export const EnvironmentSchema = z.object({
  name: z.string().min(1),
  kind: z.enum(["native", "wsl"]),
  os: z.enum(["linux", "macos", "windows"]),
  cells: z.array(CellSchema),
  pending: z.boolean().default(false),
  reason: z.string().optional(),
  workflow: z.string().optional(),
});

export const TimeoutFloorSchema = z.object({
  budget: z.number().nonnegative(),
  tick: z.number().nonnegative(),
});

/** What a complete JUnit artifact set must contain, beyond the per-leg cells. */
export const JunitExpectationsSchema = z.object({
  /** Identities (`<suite>::<case>`) or bare names whose absence is a failure. */
  requiredTests: z.array(z.string().min(1)).default([]),
  /** Per environment, tests that leg cannot execute. */
  platformExclusions: z.record(z.string(), z.array(z.string().min(1))).default({}),
  /** Per test, the semantic timeout floor its duration must respect. */
  timeoutFloors: z.record(z.string(), TimeoutFloorSchema).default({}),
  enforceTimeoutFloors: z.boolean().default(false),
});

export const CounterSignalSchema = z.object({
  id: z.string().regex(/^[a-z0-9][a-z0-9-]*$/, "signal id must be kebab-case"),
  description: z.string().min(1),
  /** `instrumented`: a stable product counter; `fixture`: a test-owned count; `pending`: no counter yet. */
  coverage: z.enum(["instrumented", "fixture", "pending"]),
  /** Where the number comes from (feature flag, fixture method, lldb breakpoint). */
  source: z.string().min(1),
  note: z.string().optional(),
});

/**
 * Work-count evidence policy. `compatibilityKeys` are the provenance fields two
 * readings must share before they can be compared; `boundaries` are declared
 * events (a production caching change, a counter rename) that a comparison may
 * not span.
 */
export const CountersSchema = z.object({
  signals: z.array(CounterSignalSchema).min(1),
  compatibilityKeys: z
    .array(z.string().min(1))
    .default(["signal", "environment", "platformKind", "requestShape", "counterVersion", "phase"]),
  boundaries: z
    .array(
      z.object({
        key: z.string().min(1),
        description: z.string().min(1),
      }),
    )
    .default([]),
});

export const AuditConfigSchema = z.object({
  version: z.literal(1),
  area: z.string().min(1),
  /** Repository root, relative to the config file. */
  root: z.string().min(1),
  /** Where captures, families, inventory, and reports live; relative to the config file. */
  evidenceDir: z.string().default("."),
  packages: z.array(PackageSchema).min(1),
  routes: z.array(RouteSchema).min(1),
  selections: z.array(SelectionSchema).min(1),
  cohorts: z.array(CohortSchema).min(1),
  environments: z.array(EnvironmentSchema),
  families: z.string().default("families.json"),
  inventory: z.string().default("inventory.md"),
  enumeration: z.string().default("enumeration"),
  sourceScan: z
    .object({
      /** Path fragments that exclude a file from the source scan (fixture trees, generated code). */
      exclude: z.array(z.string().min(1)).default([]),
    })
    .default({ exclude: [] }),
  junit: JunitExpectationsSchema.default({
    requiredTests: [],
    platformExclusions: {},
    timeoutFloors: {},
    enforceTimeoutFloors: false,
  }),
  counters: CountersSchema.optional(),
  /** Free-form notes an area wants beside its configuration. */
  notes: z.array(z.string()).optional(),
});

export type AuditConfig = z.infer<typeof AuditConfigSchema>;
export type PackageConfig = z.infer<typeof PackageSchema>;
export type RouteConfig = z.infer<typeof RouteSchema>;
export type SelectionConfig = z.infer<typeof SelectionSchema>;
export type CohortConfig = z.infer<typeof CohortSchema>;
export type EnvironmentConfig = z.infer<typeof EnvironmentSchema>;
export type JunitExpectations = z.infer<typeof JunitExpectationsSchema>;
export type CountersConfig = z.infer<typeof CountersSchema>;
export type CounterSignal = z.infer<typeof CounterSignalSchema>;

/** A loaded configuration with every path made absolute. */
export interface ResolvedConfig extends AuditConfig {
  configPath: string;
  repoRoot: string;
  evidenceRoot: string;
  familiesPath: string;
  inventoryPath: string;
  enumerationDir: string;
  /** Absolute source roots per package, in declaration order. */
  sourceRoots: { package: string; roots: string[] }[];
}

// ---------------------------------------------------------------------------
// Cross-field validation
// ---------------------------------------------------------------------------

/**
 * Referential checks the schema cannot express: every selection names declared
 * packages and routes, every cohort names declared selections, every cell and
 * exclusion names a declared package or environment, and ids are unique.
 */
export function validateConfig(config: AuditConfig): string[] {
  const problems: string[] = [];
  const dup = (label: string, values: string[]) => {
    const seen = new Set<string>();
    for (const value of values) {
      if (seen.has(value)) problems.push(`duplicate ${label}: ${value}`);
      seen.add(value);
    }
  };
  const packages = new Set(config.packages.map((p) => p.name));
  const routes = new Set(config.routes.map((r) => r.id));
  const selections = new Set(config.selections.map((s) => s.label));
  const environments = new Set(config.environments.map((e) => e.name));

  dup("package", config.packages.map((p) => p.name));
  dup("route id", config.routes.map((r) => r.id));
  dup("selection label", config.selections.map((s) => s.label));
  dup("cohort id", config.cohorts.map((c) => c.id));
  dup("environment", config.environments.map((e) => e.name));

  for (const selection of config.selections) {
    for (const pkg of selection.packages) {
      if (!packages.has(pkg)) problems.push(`selection ${selection.label}: unknown package ${pkg}`);
    }
    for (const route of selection.routes) {
      if (!routes.has(route)) problems.push(`selection ${selection.label}: unknown route ${route}`);
    }
  }
  for (const cohort of config.cohorts) {
    for (const label of cohort.selections) {
      if (!selections.has(label)) problems.push(`cohort ${cohort.id}: unknown selection ${label}`);
    }
  }
  for (const environment of config.environments) {
    for (const cell of environment.cells) {
      if (!packages.has(cell.package)) {
        problems.push(`environment ${environment.name}: cell ${cell.tier}/${cell.package} names an unknown package`);
      }
    }
    if (environment.pending && !environment.reason) {
      problems.push(`environment ${environment.name}: pending legs must state a reason`);
    }
  }
  for (const [environment, tests] of Object.entries(config.junit.platformExclusions)) {
    if (!environments.has(environment)) {
      problems.push(`junit.platformExclusions: unknown environment ${environment}`);
    }
    if (tests.length === 0) problems.push(`junit.platformExclusions[${environment}] is empty`);
  }
  for (const route of config.routes) {
    if ((route.kind === "absent" || route.kind === "manual") && route.executes) {
      problems.push(`route ${route.id}: ${route.kind} routes cannot claim to execute tests`);
    }
  }
  if (config.counters) {
    dup("counter signal", config.counters.signals.map((s) => s.id));
    dup("counter boundary", config.counters.boundaries.map((b) => b.key));
  }
  return problems;
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

function resolveFrom(base: string, path: string): string {
  return isAbsolute(path) ? path : resolve(base, path);
}

/** Parse, validate, and resolve a configuration file. Throws `ConfigError`. */
export function loadConfig(path: string): ResolvedConfig {
  const configPath = resolve(path);
  if (!existsSync(configPath) || !statSync(configPath).isFile()) {
    throw new ConfigError(`config not found: ${configPath}`);
  }
  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(configPath, "utf8"));
  } catch (error) {
    throw new ConfigError(`config is not valid JSON: ${configPath}: ${(error as Error).message}`);
  }
  const parsed = AuditConfigSchema.safeParse(raw);
  if (!parsed.success) {
    const lines = parsed.error.issues.map((issue) => `  ${issue.path.join(".") || "<root>"}: ${issue.message}`);
    throw new ConfigError(`config schema violations in ${configPath}:\n${lines.join("\n")}`);
  }
  const problems = validateConfig(parsed.data);
  if (problems.length > 0) {
    throw new ConfigError(`config validation failed in ${configPath}:\n${problems.map((p) => `  ${p}`).join("\n")}`);
  }
  return resolveConfig(parsed.data, configPath);
}

export function resolveConfig(config: AuditConfig, configPath: string): ResolvedConfig {
  const base = dirname(configPath);
  const repoRoot = resolveFrom(base, config.root);
  const evidenceRoot = resolveFrom(base, config.evidenceDir);
  return {
    ...config,
    configPath,
    repoRoot,
    evidenceRoot,
    familiesPath: resolveFrom(evidenceRoot, config.families),
    inventoryPath: resolveFrom(evidenceRoot, config.inventory),
    enumerationDir: resolveFrom(evidenceRoot, config.enumeration),
    sourceRoots: config.packages.map((pkg) => ({
      package: pkg.name,
      roots: (pkg.sourceRoots ?? [pkg.path]).map((root) => resolveFrom(repoRoot, root)),
    })),
  };
}

/** The environments a gate must see: declared and not pending. */
export function activeEnvironments(config: AuditConfig): EnvironmentConfig[] {
  return config.environments.filter((e) => !e.pending);
}
