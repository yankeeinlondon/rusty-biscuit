//! Read-only capacity inventory for the evidence store's feasibility checkpoint.
//!
//! Measures the existing evidence (remote Git notes, locally retained report and
//! proof bytes, recent GitHub Actions artifacts) and forecasts 12 months of
//! free-tier consumption with p95 sizes and twice the observed daily volume,
//! assuming no production pruning. Missing required data or a projected breach
//! exits 1; an empty scan is never treated as a volume estimate.
//!
//! Nothing is written to the repository: notes are fetched into a scratch bare
//! repository under the OS temp dir, which is deleted on exit.

import { execFile } from "node:child_process";
import { mkdtemp, rm, stat, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { promisify } from "node:util";

import { CAPACITY, LIMITS } from "../src/contracts/api.ts";
import { PROOF_COMPLETION_FILE, PROOF_EXPECTED_FILE } from "../src/contracts/evidence.ts";

const execFileAsync = promisify(execFile);

const DAY_SECONDS = 86_400;
const DAYS_PER_MONTH = 30.44;
const FORECAST_MONTHS = 12;

/**
 * Assumptions the forecast applies. Each is reported in the JSON output so a
 * reader can rerun the arithmetic.
 */
export const ASSUMPTIONS = {
  volume_multiplier: 2,
  /** D1 bytes per observation = p95 canonical size × this, covering indexes, event history, manifests, and report-location rows. */
  d1_overhead_factor: 2,
  /** D1 rows written per observation before index writes: observation, collection membership, observation event; plus one report-location row per report. */
  d1_base_rows_per_observation: 3,
  /** Each D1 index on a written table counts as an extra written row; doubles the base rows. */
  d1_index_write_factor: 2,
  /** Class A per uploaded object: ceil(p95 bytes / upload_part_bytes) parts + 1 create + 1 complete. */
  r2_class_a_fixed_ops_per_object: 2,
  /** Breach means crossing CAPACITY.refuse_ratio; crossing alert_ratio is reported but does not fail. */
  breach_ratio: "refuse_ratio",
} as const;

// ---------------------------------------------------------------------------
// Source shapes

/** One historical version of one note: the blob a notes commit wrote for an annotated object. */
export interface NoteVersion {
  annotated: string;
  blob: string;
  /** Unix seconds of the notes commit that wrote this blob. */
  written_at: number;
}

export interface NotesRef {
  ref: string;
  tip: string;
  /** Chronological (oldest first). */
  versions: NoteVersion[];
  /** Annotated object → blob id at the ref's tip. */
  current: Record<string, string>;
}

export interface NotesSnapshot {
  refs: NotesRef[];
  /** Blob id → text. Every blob named in `versions` and `current` is present. */
  blobs: Record<string, string>;
}

export interface GithubArtifact {
  name: string;
  size_in_bytes: number;
  expired: boolean;
}

export interface GithubRun {
  id: number;
  created_at: string;
  artifacts: GithubArtifact[];
}

export type SourceResult<T> = { ok: true; value: T } | { ok: false; source: string; error: string };

export interface AccountUsage {
  r2_bytes: number;
  r2_class_a_per_month: number;
  d1_bytes: number;
  d1_rows_written_per_day: number;
}

export interface Sources {
  notes(): Promise<SourceResult<NotesSnapshot>>;
  /** File size in bytes, or null when absent. Never reads contents. */
  fileSize(path: string): Promise<number | null>;
  github(runs: number): Promise<SourceResult<GithubRun[]>>;
  accountUsage(): Promise<SourceResult<AccountUsage>>;
  /** Unix seconds. */
  now(): number;
}

export interface InventoryOptions {
  days: number;
  runs: number;
}

// ---------------------------------------------------------------------------
// Statistics

export interface Percentiles {
  count: number;
  p50: number | null;
  p95: number | null;
  max: number | null;
}

/** Nearest-rank percentile: the smallest value with at least `p` of the samples at or below it. */
export function percentile(values: readonly number[], p: number): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const rank = Math.max(1, Math.ceil(p * sorted.length));
  return sorted[rank - 1] ?? null;
}

export function percentiles(values: readonly number[]): Percentiles {
  return {
    count: values.length,
    p50: percentile(values, 0.5),
    p95: percentile(values, 0.95),
    max: values.length === 0 ? null : Math.max(...values),
  };
}

/** Compact JSON with keys sorted, matching the canonical form's shape (number lexemes aside). */
export function compactSorted(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(compactSorted).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const entries = Object.keys(value as Record<string, unknown>)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${compactSorted((value as Record<string, unknown>)[key])}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
}

function utf8Bytes(text: string): number {
  return Buffer.byteLength(text, "utf8");
}

// ---------------------------------------------------------------------------
// Note parsing (lenient, measurement only)

interface MeasuredObservation {
  id: string;
  environment: string;
  package: string;
  gate: string;
  outcome: string;
  canonical_bytes: number;
  report_dir: string | null;
  /** Reports with recorded sizes; `bytes` null for a v2 cell, which records none. */
  reports: Array<{ path: string; bytes: number | null; store: string }>;
  proof_files: number;
  source_version: 2 | 3;
}

type ParsedNote =
  | { kind: "collection" | "receipt"; observations: MeasuredObservation[] }
  | { kind: "scope" }
  | { kind: "unparseable"; reason: string };

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function str(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

export function parseNote(text: string, environment: string, annotated: string): ParsedNote {
  let document: unknown;
  try {
    document = JSON.parse(text);
  } catch (error) {
    return { kind: "unparseable", reason: `invalid JSON: ${(error as Error).message}` };
  }
  if (!isObject(document)) return { kind: "unparseable", reason: "not an object" };
  // `refs/notes/ci-local/scope` holds scope receipts, not evidence.
  if (environment === "scope") return { kind: "scope" };
  const version = document["schema_version"];
  if (version === 3 && Array.isArray(document["observations"])) {
    const observations: MeasuredObservation[] = [];
    for (const raw of document["observations"]) {
      if (!isObject(raw) || !isObject(raw["cell"])) return { kind: "unparseable", reason: "observation without a cell" };
      const cell = raw["cell"];
      const producer = isObject(raw["producer"]) ? raw["producer"] : {};
      const host = isObject(producer["host"]) ? producer["host"] : {};
      const reports = Array.isArray(raw["reports"]) ? raw["reports"].filter(isObject) : [];
      const proof = isObject(raw["proof"]) ? raw["proof"] : null;
      observations.push({
        id: str(raw["observation_id"]) ?? `unnamed:${annotated}:${observations.length}`,
        environment: str(document["environment"]) ?? environment,
        package: str(cell["package"]) ?? "?",
        gate: str(cell["gate"]) ?? "?",
        outcome: str(cell["outcome"]) ?? "?",
        canonical_bytes: utf8Bytes(compactSorted(raw)),
        report_dir: str(host["report_dir"]),
        reports: reports.map((report) => ({
          path: str(report["path"]) ?? "",
          bytes: typeof report["bytes"] === "number" ? report["bytes"] : null,
          store: str(report["store"]) ?? "?",
        })),
        proof_files: proof === null ? 0 : proof["completion_record_digest"] === undefined ? 1 : 2,
        source_version: 3,
      });
    }
    return { kind: "collection", observations };
  }
  if (version === 2 && Array.isArray(document["cells"])) {
    // A v2 receipt has no observation ids; a cell becomes one observation, sized
    // as the cell plus the receipt context the v2 adapter copies onto it.
    const host = isObject(document["host"]) ? document["host"] : {};
    const observations: MeasuredObservation[] = [];
    for (const cell of document["cells"]) {
      if (!isObject(cell)) return { kind: "unparseable", reason: "receipt cell is not an object" };
      const shaped = {
        producer: { kind: "hook", host },
        base: document["base"],
        scope_identity: document["scope_identity"],
        completion: document["completion"],
        cell,
        reports: [{ path: cell["report"], sha256: "0".repeat(64), bytes: 0, store: "host-dir" }],
      };
      observations.push({
        id: `v2:${annotated}:${str(cell["package"])}:${str(cell["gate"])}:${str(cell["gate_input_identity"])}`,
        environment: str(document["environment"]) ?? environment,
        package: str(cell["package"]) ?? "?",
        gate: str(cell["gate"]) ?? "?",
        outcome: str(cell["outcome"]) ?? "?",
        canonical_bytes: utf8Bytes(compactSorted({ observation_id: "0".repeat(64), ...shaped })),
        report_dir: str(host["report_dir"]),
        reports: cell["completion"] === "complete" ? [{ path: str(cell["report"]) ?? "", bytes: null, store: "host-dir" }] : [],
        proof_files: 0,
        source_version: 2,
      });
    }
    return { kind: "receipt", observations };
  }
  return { kind: "unparseable", reason: `unsupported schema_version ${JSON.stringify(version)}` };
}

// ---------------------------------------------------------------------------
// Inventory

export interface Measurements {
  baseline_observations: number;
  daily_mean: number;
  daily_max: number;
  observation_p95_bytes: number;
  report_p95_bytes: number;
  reports_per_observation: number;
  proof_p95_bytes: number;
  proofs_per_observation: number;
}

export interface MonthProjection {
  month: number;
  d1_bytes: number;
  r2_bytes: number;
}

export interface LimitCheck {
  limit: string;
  allowance: number;
  projected: number;
  alert_at: number;
  refuse_at: number;
  /** First month (1–12) the projection crosses each threshold, or null. */
  alert_month: number | null;
  breach_month: number | null;
  formula: string;
}

export interface Forecast {
  admission_daily_observations: number;
  admission_peak_daily_observations: number;
  months: MonthProjection[];
  checks: LimitCheck[];
}

function firstMonth(projected: (month: number) => number, threshold: number): number | null {
  for (let month = 1; month <= FORECAST_MONTHS; month += 1) {
    if (projected(month) >= threshold) return month;
  }
  return null;
}

function check(limit: string, allowance: number, projected: (month: number) => number, formula: string): LimitCheck {
  const alertAt = allowance * CAPACITY.alert_ratio;
  const refuseAt = allowance * CAPACITY.refuse_ratio;
  return {
    limit,
    allowance,
    projected: Math.round(projected(FORECAST_MONTHS)),
    alert_at: Math.round(alertAt),
    refuse_at: Math.round(refuseAt),
    alert_month: firstMonth(projected, alertAt),
    breach_month: firstMonth(projected, refuseAt),
    formula,
  };
}

/** 12-month projection assuming no pruning: everything accumulates linearly from the existing baseline. */
export function forecast(m: Measurements): Forecast {
  const daily = m.daily_mean * ASSUMPTIONS.volume_multiplier;
  const peak = m.daily_max * ASSUMPTIONS.volume_multiplier;
  const observationsAt = (month: number): number => m.baseline_observations + daily * DAYS_PER_MONTH * month;
  const d1BytesPerObservation = m.observation_p95_bytes * ASSUMPTIONS.d1_overhead_factor;
  const r2BytesPerObservation = m.report_p95_bytes * m.reports_per_observation + m.proof_p95_bytes * m.proofs_per_observation;
  const rowsPerObservation =
    (ASSUMPTIONS.d1_base_rows_per_observation + m.reports_per_observation) * ASSUMPTIONS.d1_index_write_factor;
  const opsPerObject = (bytes: number): number =>
    Math.max(1, Math.ceil(bytes / LIMITS.upload_part_bytes)) + ASSUMPTIONS.r2_class_a_fixed_ops_per_object;
  const classAPerObservation =
    m.reports_per_observation * opsPerObject(m.report_p95_bytes) + m.proofs_per_observation * opsPerObject(m.proof_p95_bytes);

  const d1Bytes = (month: number): number => observationsAt(month) * d1BytesPerObservation;
  const r2Bytes = (month: number): number => observationsAt(month) * r2BytesPerObservation;
  const months: MonthProjection[] = [];
  for (let month = 1; month <= FORECAST_MONTHS; month += 1) {
    months.push({ month, d1_bytes: Math.round(d1Bytes(month)), r2_bytes: Math.round(r2Bytes(month)) });
  }
  const checks = [
    check("d1_database_bytes", CAPACITY.d1_database_bytes, d1Bytes, "(baseline + 2×daily_mean×30.44×month) × observation_p95 × d1_overhead_factor"),
    check(
      "d1_rows_written_per_day",
      CAPACITY.d1_rows_written_per_day,
      () => peak * rowsPerObservation,
      "2×daily_max × (3 + reports_per_observation) × index_write_factor",
    ),
    check("r2_live_plus_staging_bytes", CAPACITY.r2_live_plus_staging_bytes, r2Bytes, "(baseline + 2×daily_mean×30.44×month) × (report_p95×reports_per_obs + proof_p95×proofs_per_obs)"),
    check(
      "r2_class_a_per_month",
      CAPACITY.r2_class_a_per_month,
      () => daily * DAYS_PER_MONTH * classAPerObservation,
      "2×daily_mean×30.44 × Σ objects_per_obs × (ceil(p95/upload_part_bytes) + create + complete)",
    ),
  ];
  return { admission_daily_observations: daily, admission_peak_daily_observations: peak, months, checks };
}

export interface InventoryReport {
  generated_at: string;
  window_days: number;
  feasible: boolean;
  failures: string[];
  assumptions: typeof ASSUMPTIONS & { daily_date_source: string; v2_sizing: string; unresolved_ordering: string };
  notes: {
    refs: Array<{
      ref: string;
      environment: string;
      notes: number;
      observations: number;
      unparseable_notes: number;
      versions: number;
    }>;
    observations: number;
    observations_by_environment: Record<string, number>;
    observations_by_source_version: Record<string, number>;
    unparseable_notes: number;
    unparseable_reasons: string[];
    scope_notes: number;
    observed_cells: number;
    cells_by_environment: Record<string, number>;
    observation_canonical_bytes: Percentiles;
    collection_canonical_bytes: Percentiles;
    note_blob_bytes: Percentiles;
  };
  volume: {
    window_days_effective: number;
    first_observation_at: string | null;
    observations_in_window: number;
    daily_mean: number;
    daily_max: number;
    per_day: Record<string, number>;
  };
  history: {
    observations_at_window_start: number;
    observations_now: number;
    growth_per_day: number;
    failures_total: number;
    unresolved_cells_at_window_start: number;
    unresolved_cells_now: number;
    unresolved_growth_per_day: number;
    unresolved_by_environment: Record<string, number>;
  };
  reports: {
    recorded_bytes: Percentiles;
    recorded_by_store: Record<string, number>;
    without_recorded_size: number;
    local: { present: number; present_bytes: number; missing: number; missing_recorded_bytes: number };
  };
  proofs: {
    referenced_files: number;
    local_present: number;
    local_missing: number;
    local_bytes: Percentiles;
  };
  github_actions: null | {
    runs: number;
    artifacts: number;
    expired: number;
    artifact_bytes: Percentiles;
    live_bytes: number;
    expired_bytes: number;
  };
  account_usage: AccountUsage | null;
  measurements: Measurements | null;
  forecast: Forecast | null;
}

function dayKey(seconds: number): string {
  return new Date(seconds * 1000).toISOString().slice(0, 10);
}

function expandHome(path: string): string {
  return path === "~" || path.startsWith("~/") ? join(homedir(), path.slice(1)) : path;
}

function increment(record: Record<string, number>, key: string, by = 1): void {
  record[key] = (record[key] ?? 0) + by;
}

interface Timed {
  observation: MeasuredObservation;
  first_seen: number;
}

function unresolvedAt(timed: readonly Timed[], until: number): Map<string, MeasuredObservation> {
  const latest = new Map<string, Timed>();
  for (const entry of timed) {
    if (entry.first_seen > until) continue;
    const o = entry.observation;
    const key = `${o.package}\u0000${o.environment}\u0000${o.gate}`;
    const prior = latest.get(key);
    if (prior === undefined || entry.first_seen >= prior.first_seen) latest.set(key, entry);
  }
  const unresolved = new Map<string, MeasuredObservation>();
  for (const [key, entry] of latest) if (entry.observation.outcome === "fail") unresolved.set(key, entry.observation);
  return unresolved;
}

function round(value: number, digits = 2): number {
  const scale = 10 ** digits;
  return Math.round(value * scale) / scale;
}

export async function runInventory(options: InventoryOptions, sources: Sources): Promise<InventoryReport> {
  const failures: string[] = [];
  const now = sources.now();
  const windowStart = now - options.days * DAY_SECONDS;

  const report: InventoryReport = {
    generated_at: new Date(now * 1000).toISOString(),
    window_days: options.days,
    feasible: false,
    failures,
    assumptions: {
      ...ASSUMPTIONS,
      daily_date_source:
        "an observation is dated by the committer time of the earliest notes commit whose blob contains it (notes history, not annotated-commit time, which is not fetched)",
      v2_sizing:
        "a v2 receipt cell counts as one observation sized as the cell plus copied receipt context and one host-dir report; its report bytes are known only when retained locally",
      unresolved_ordering:
        "latest observation per {package, environment, gate} by first-seen time (approximates the store's ancestry ordering)",
    },
    notes: {
      refs: [],
      observations: 0,
      observations_by_environment: {},
      observations_by_source_version: {},
      unparseable_notes: 0,
      unparseable_reasons: [],
      scope_notes: 0,
      observed_cells: 0,
      cells_by_environment: {},
      observation_canonical_bytes: percentiles([]),
      collection_canonical_bytes: percentiles([]),
      note_blob_bytes: percentiles([]),
    },
    volume: { window_days_effective: 0, first_observation_at: null, observations_in_window: 0, daily_mean: 0, daily_max: 0, per_day: {} },
    history: {
      observations_at_window_start: 0,
      observations_now: 0,
      growth_per_day: 0,
      failures_total: 0,
      unresolved_cells_at_window_start: 0,
      unresolved_cells_now: 0,
      unresolved_growth_per_day: 0,
      unresolved_by_environment: {},
    },
    reports: {
      recorded_bytes: percentiles([]),
      recorded_by_store: {},
      without_recorded_size: 0,
      local: { present: 0, present_bytes: 0, missing: 0, missing_recorded_bytes: 0 },
    },
    proofs: { referenced_files: 0, local_present: 0, local_missing: 0, local_bytes: percentiles([]) },
    github_actions: null,
    account_usage: null,
    measurements: null,
    forecast: null,
  };

  // --- Git notes -----------------------------------------------------------
  const notesResult = await sources.notes();
  const timed: Timed[] = [];
  const observationSizes: number[] = [];
  const collectionSizes: number[] = [];
  const blobSizes: number[] = [];
  if (!notesResult.ok) {
    failures.push(`missing required source ${notesResult.source}: ${notesResult.error}`);
  } else {
    const { refs, blobs } = notesResult.value;
    const parsedCache = new Map<string, ParsedNote>();
    const parse = (blob: string, environment: string, annotated: string): ParsedNote => {
      const key = `${environment}\u0000${blob}`;
      let parsed = parsedCache.get(key);
      if (parsed === undefined) {
        parsed = parseNote(blobs[blob] ?? "", environment, annotated);
        parsedCache.set(key, parsed);
      }
      return parsed;
    };
    for (const ref of refs) {
      const environment = ref.ref.split("/").pop() ?? ref.ref;
      const firstSeen = new Map<string, number>();
      for (const version of ref.versions) {
        const parsed = parse(version.blob, environment, version.annotated);
        if (parsed.kind !== "collection" && parsed.kind !== "receipt") continue;
        for (const o of parsed.observations) if (!firstSeen.has(o.id)) firstSeen.set(o.id, version.written_at);
      }
      const tipTime = ref.versions.at(-1)?.written_at ?? now;
      let observations = 0;
      let unparseable = 0;
      for (const [annotated, blob] of Object.entries(ref.current)) {
        const text = blobs[blob] ?? "";
        blobSizes.push(utf8Bytes(text));
        const parsed = parse(blob, environment, annotated);
        if (parsed.kind === "scope") {
          report.notes.scope_notes += 1;
          continue;
        }
        if (parsed.kind === "unparseable") {
          unparseable += 1;
          report.notes.unparseable_reasons.push(`${ref.ref} ${annotated.slice(0, 9)}: ${parsed.reason}`);
          continue;
        }
        if (parsed.kind === "collection") {
          try {
            collectionSizes.push(utf8Bytes(compactSorted(JSON.parse(text))));
          } catch {
            // parseNote already accepted it; unreachable
          }
        }
        for (const o of parsed.observations) {
          observations += 1;
          observationSizes.push(o.canonical_bytes);
          increment(report.notes.observations_by_environment, o.environment);
          increment(report.notes.observations_by_source_version, `v${o.source_version}`);
          timed.push({ observation: o, first_seen: firstSeen.get(o.id) ?? tipTime });
        }
      }
      report.notes.refs.push({
        ref: ref.ref,
        environment,
        notes: Object.keys(ref.current).length,
        observations,
        unparseable_notes: unparseable,
        versions: ref.versions.length,
      });
      report.notes.observations += observations;
      report.notes.unparseable_notes += unparseable;
    }
    report.notes.observation_canonical_bytes = percentiles(observationSizes);
    report.notes.collection_canonical_bytes = percentiles(collectionSizes);
    report.notes.note_blob_bytes = percentiles(blobSizes);
    const cells = new Set<string>();
    for (const { observation: o } of timed) {
      const key = `${o.package}\u0000${o.environment}\u0000${o.gate}`;
      if (!cells.has(key)) increment(report.notes.cells_by_environment, o.environment);
      cells.add(key);
    }
    report.notes.observed_cells = cells.size;
    if (timed.length === 0) {
      failures.push(
        `missing required source git-notes: the scan found no observations under ${refs.map((r) => r.ref).join(", ") || "refs/notes/*"}; an empty scan is not a volume estimate`,
      );
    }
  }

  // --- Volume and history --------------------------------------------------
  if (timed.length > 0) {
    const first = Math.min(...timed.map((t) => t.first_seen));
    const effective = Math.max(1, Math.min(options.days, (now - first) / DAY_SECONDS));
    const perDay: Record<string, number> = {};
    let inWindow = 0;
    for (const t of timed) {
      if (t.first_seen < windowStart) continue;
      inWindow += 1;
      increment(perDay, dayKey(t.first_seen));
    }
    report.volume = {
      window_days_effective: round(effective),
      first_observation_at: new Date(first * 1000).toISOString(),
      observations_in_window: inWindow,
      daily_mean: round(inWindow / effective),
      daily_max: Math.max(0, ...Object.values(perDay)),
      per_day: Object.fromEntries(Object.entries(perDay).sort()),
    };
    const historyStart = Math.max(windowStart, first);
    const atStart = timed.filter((t) => t.first_seen < historyStart).length;
    const unresolvedStart = unresolvedAt(timed, historyStart - 1);
    const unresolvedNow = unresolvedAt(timed, now);
    const unresolvedByEnvironment: Record<string, number> = {};
    for (const o of unresolvedNow.values()) increment(unresolvedByEnvironment, o.environment);
    report.history = {
      observations_at_window_start: atStart,
      observations_now: timed.length,
      growth_per_day: round((timed.length - atStart) / effective),
      failures_total: timed.filter((t) => t.observation.outcome === "fail").length,
      unresolved_cells_at_window_start: unresolvedStart.size,
      unresolved_cells_now: unresolvedNow.size,
      unresolved_growth_per_day: round((unresolvedNow.size - unresolvedStart.size) / effective),
      unresolved_by_environment: unresolvedByEnvironment,
    };
    if (inWindow === 0) {
      failures.push(`missing required source git-notes: no observations were written in the last ${options.days} days`);
    }
  }

  // --- Report and proof bytes ----------------------------------------------
  const recorded: number[] = [];
  const proofSizes: number[] = [];
  const seenPaths = new Set<string>();
  let reportsTotal = 0;
  let proofsTotal = 0;
  for (const { observation: o } of timed) {
    proofsTotal += o.proof_files;
    for (const r of o.reports) {
      reportsTotal += 1;
      if (r.bytes === null) report.reports.without_recorded_size += 1;
      else {
        recorded.push(r.bytes);
        increment(report.reports.recorded_by_store, r.store);
      }
      if (o.report_dir === null) continue;
      const path = join(expandHome(o.report_dir), r.path);
      if (seenPaths.has(path)) continue;
      seenPaths.add(path);
      const size = await sources.fileSize(path);
      if (size === null) {
        report.reports.local.missing += 1;
        report.reports.local.missing_recorded_bytes += r.bytes ?? 0;
      } else {
        report.reports.local.present += 1;
        report.reports.local.present_bytes += size;
        if (r.bytes === null) recorded.push(size);
      }
    }
    if (o.proof_files > 0 && o.report_dir !== null) {
      const names = [PROOF_EXPECTED_FILE, PROOF_COMPLETION_FILE].slice(0, o.proof_files);
      for (const name of names) {
        const path = join(expandHome(o.report_dir), name.replace("{package}", o.package).replace("{gate}", o.gate));
        if (seenPaths.has(path)) continue;
        seenPaths.add(path);
        report.proofs.referenced_files += 1;
        const size = await sources.fileSize(path);
        if (size === null) report.proofs.local_missing += 1;
        else {
          report.proofs.local_present += 1;
          proofSizes.push(size);
        }
      }
    }
  }
  report.reports.recorded_bytes = percentiles(recorded);
  report.proofs.local_bytes = percentiles(proofSizes);
  if (timed.length > 0 && recorded.length === 0) {
    failures.push("missing required source report-sizes: no observation records report bytes and none are retained locally");
  }
  if (proofsTotal > 0 && proofSizes.length === 0) {
    failures.push(
      `missing required source proof-sizes: ${proofsTotal} proof documents are cited but none is retained on this host (hosted proofs live only in Actions artifacts)`,
    );
  }

  // --- GitHub Actions --------------------------------------------------------
  const github = await sources.github(options.runs);
  if (!github.ok) {
    failures.push(`missing required source ${github.source}: ${github.error}`);
  } else {
    const artifacts = github.value.flatMap((run) => run.artifacts);
    const sum = (list: GithubArtifact[]): number => list.reduce((total, a) => total + a.size_in_bytes, 0);
    report.github_actions = {
      runs: github.value.length,
      artifacts: artifacts.length,
      expired: artifacts.filter((a) => a.expired).length,
      artifact_bytes: percentiles(artifacts.map((a) => a.size_in_bytes)),
      live_bytes: sum(artifacts.filter((a) => !a.expired)),
      expired_bytes: sum(artifacts.filter((a) => a.expired)),
    };
  }

  // --- Account-wide consumption ---------------------------------------------
  const account = await sources.accountUsage();
  if (!account.ok) failures.push(`missing required source ${account.source}: ${account.error}`);
  else report.account_usage = account.value;

  // --- Forecast ---------------------------------------------------------------
  const observationP95 = report.notes.observation_canonical_bytes.p95;
  const reportP95 = report.reports.recorded_bytes.p95;
  if (timed.length > 0 && observationP95 !== null && reportP95 !== null) {
    const measurements: Measurements = {
      baseline_observations: timed.length,
      daily_mean: report.volume.daily_mean,
      daily_max: report.volume.daily_max,
      observation_p95_bytes: observationP95,
      report_p95_bytes: reportP95,
      reports_per_observation: round(reportsTotal / timed.length, 4),
      proof_p95_bytes: report.proofs.local_bytes.p95 ?? 0,
      proofs_per_observation: round(proofsTotal / timed.length, 4),
    };
    report.measurements = measurements;
    const projected = forecast(measurements);
    if (account.ok) {
      const usage: Record<string, number> = {
        d1_database_bytes: account.value.d1_bytes,
        d1_rows_written_per_day: account.value.d1_rows_written_per_day,
        r2_live_plus_staging_bytes: account.value.r2_bytes,
        r2_class_a_per_month: account.value.r2_class_a_per_month,
      };
      for (const c of projected.checks) c.projected += usage[c.limit] ?? 0;
    }
    report.forecast = projected;
    for (const c of projected.checks) {
      if (c.breach_month !== null) {
        failures.push(`projected breach of ${c.limit}: crosses ${CAPACITY.refuse_ratio * 100}% of ${c.allowance} in month ${c.breach_month}`);
      }
    }
  }

  report.feasible = failures.length === 0;
  return report;
}

// ---------------------------------------------------------------------------
// Human summary

function kib(bytes: number | null): string {
  return bytes === null ? "n/a" : `${(bytes / 1024).toFixed(1)} KiB`;
}

export function summarize(report: InventoryReport): string {
  const lines: string[] = [];
  const p = (label: string, value: Percentiles): string =>
    `${label}: n=${value.count} p50=${kib(value.p50)} p95=${kib(value.p95)} max=${kib(value.max)}`;
  lines.push(`evidence inventory (${report.generated_at}, window ${report.window_days} days)`);
  lines.push(
    `observed cells: ${report.notes.observed_cells}; observations: ${report.notes.observations} ${JSON.stringify(report.notes.observations_by_environment)}; unparseable notes: ${report.notes.unparseable_notes}`,
  );
  lines.push(p("observation size", report.notes.observation_canonical_bytes));
  lines.push(p("note blob size", report.notes.note_blob_bytes));
  lines.push(p("report size", report.reports.recorded_bytes));
  lines.push(p("proof size (local)", report.proofs.local_bytes));
  lines.push(`daily attempts: mean ${report.volume.daily_mean}, max ${report.volume.daily_max} (effective window ${report.volume.window_days_effective} days)`);
  lines.push(
    `history: ${report.history.observations_at_window_start} -> ${report.history.observations_now} observations; unresolved cells ${report.history.unresolved_cells_at_window_start} -> ${report.history.unresolved_cells_now}`,
  );
  if (report.forecast !== null) {
    for (const c of report.forecast.checks) {
      lines.push(
        `forecast ${c.limit}: ${c.projected} of ${c.allowance} (${((c.projected / c.allowance) * 100).toFixed(2)}%); alert month ${c.alert_month ?? "none"}, breach month ${c.breach_month ?? "none"}`,
      );
    }
  }
  lines.push(`feasible: ${report.feasible}`);
  for (const failure of report.failures) lines.push(`  failure: ${failure}`);
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Real sources

const GIT_ENV = { ...process.env, GIT_TERMINAL_PROMPT: "0", GIT_SSH_COMMAND: "ssh -o BatchMode=yes" };

async function git(args: string[], cwd?: string): Promise<string> {
  const { stdout } = await execFileAsync("git", cwd === undefined ? args : ["-C", cwd, ...args], {
    env: GIT_ENV,
    maxBuffer: 1024 * 1024 * 1024,
  });
  return stdout;
}

/** Every blob's text, read through one `git cat-file --batch`. */
async function readBlobs(repo: string, ids: readonly string[]): Promise<Record<string, string>> {
  const { spawn } = await import("node:child_process");
  const child = spawn("git", ["-C", repo, "cat-file", "--batch"], { env: GIT_ENV });
  const chunks: Buffer[] = [];
  child.stdout.on("data", (chunk: Buffer) => chunks.push(chunk));
  const done = new Promise<void>((resolve, reject) => {
    child.on("error", reject);
    child.on("close", (code) => (code === 0 ? resolve() : reject(new Error(`git cat-file --batch exited ${code}`))));
  });
  child.stdin.end(ids.join("\n") + "\n");
  await done;
  const buffer = Buffer.concat(chunks);
  const blobs: Record<string, string> = {};
  let offset = 0;
  for (const id of ids) {
    const newline = buffer.indexOf(0x0a, offset);
    const header = buffer.subarray(offset, newline).toString("utf8").split(" ");
    const size = Number(header[2]);
    if (header[1] === "missing" || Number.isNaN(size)) throw new Error(`blob ${id} is missing from the scratch repository`);
    blobs[id] = buffer.subarray(newline + 1, newline + 1 + size).toString("utf8");
    offset = newline + 1 + size + 1;
  }
  return blobs;
}

/** Walks one notes ref's first-parent history: which blob each notes commit wrote for which annotated object. */
export async function walkNotesRef(repo: string, ref: string, tip: string): Promise<NotesRef> {
  const log = await git(["log", "--reverse", "--first-parent", "-m", "--raw", "--no-abbrev", "--no-renames", "--format=@%ct", tip], repo);
  const versions: NoteVersion[] = [];
  let time = 0;
  for (const line of log.split("\n")) {
    if (line.startsWith("@")) time = Number(line.slice(1));
    else if (line.startsWith(":")) {
      // :old_mode new_mode old_sha new_sha status\tpath
      const [meta, path] = line.split("\t");
      const fields = (meta ?? "").split(" ");
      if (fields[4] === "D" || path === undefined) continue;
      versions.push({ annotated: path.replaceAll("/", ""), blob: fields[3] ?? "", written_at: time });
    }
  }
  const current: Record<string, string> = {};
  for (const line of (await git(["ls-tree", "-r", tip], repo)).split("\n")) {
    const match = /^\d+ blob ([0-9a-f]+)\t(.+)$/.exec(line);
    if (match?.[1] !== undefined && match[2] !== undefined) current[match[2].replaceAll("/", "")] = match[1];
  }
  return { ref, tip, versions, current };
}

/** Notes refs as published on `remoteUrl`, fetched into `scratch` (a bare repository). */
export async function fetchNotes(repoRoot: string, remote: string, scratch: string): Promise<SourceResult<NotesSnapshot>> {
  let url: string;
  try {
    url = (await git(["remote", "get-url", remote], repoRoot)).trim();
  } catch (error) {
    return { ok: false, source: "git-notes", error: `git remote get-url ${remote} failed: ${(error as Error).message.split("\n")[0]}` };
  }
  try {
    const listing = await git(["ls-remote", url, "refs/notes/*"]);
    const advertised = listing
      .split("\n")
      .map((line) => line.split("\t"))
      .filter((parts): parts is [string, string] => parts.length === 2)
      .map(([sha, ref]) => ({ sha, ref }));
    if (advertised.length === 0) return { ok: false, source: "git-notes", error: `git ls-remote ${remote} advertised no refs/notes/*` };
    await git(["init", "--bare", "--quiet", scratch]);
    await git(["fetch", "--quiet", "--no-tags", url, "+refs/notes/*:refs/notes/*"], scratch);
    const refs: NotesRef[] = [];
    for (const { sha, ref } of advertised) refs.push(await walkNotesRef(scratch, ref, sha));
    const ids = new Set<string>();
    for (const ref of refs) {
      for (const v of ref.versions) ids.add(v.blob);
      for (const blob of Object.values(ref.current)) ids.add(blob);
    }
    return { ok: true, value: { refs, blobs: await readBlobs(scratch, [...ids]) } };
  } catch (error) {
    return { ok: false, source: "git-notes", error: (error as Error).message.split("\n")[0] ?? "unknown git failure" };
  }
}

function githubRepository(url: string): string | null {
  const match = /github\.com[:/]([^/]+)\/([^/]+?)(?:\.git)?$/.exec(url);
  return match === null ? null : `${match[1]}/${match[2]}`;
}

async function githubSource(repoRoot: string, remote: string, runs: number): Promise<SourceResult<GithubRun[]>> {
  const source = "github-actions";
  const repository = githubRepository((await git(["remote", "get-url", remote], repoRoot).catch(() => "")).trim());
  if (repository === null) return { ok: false, source, error: `remote ${remote} is not a github.com repository` };
  const token = process.env["GITHUB_TOKEN"] || process.env["GH_TOKEN"];
  let get: (path: string) => Promise<unknown>;
  if (token) {
    const variable = process.env["GITHUB_TOKEN"] ? "GITHUB_TOKEN" : "GH_TOKEN";
    get = async (path) => {
      const response = await fetch(`https://api.github.com/${path}`, {
        headers: { authorization: `Bearer ${token}`, accept: "application/vnd.github+json", "user-agent": "evidence-inventory" },
      });
      if (!response.ok) throw new Error(`GET /${path.split("?")[0]} with ${variable} answered HTTP ${response.status}`);
      return response.json();
    };
  } else {
    try {
      await execFileAsync("gh", ["auth", "status"], { env: { ...process.env, GH_PROMPT_DISABLED: "1" } });
    } catch (error) {
      const failure = error as { code?: unknown; stdout?: string; stderr?: string };
      const output = `${failure.stdout ?? ""}\n${failure.stderr ?? ""}`;
      const reason = /token in \S+ is invalid|not logged in[^\n]*/i.exec(output)?.[0];
      return {
        ok: false,
        source,
        error: `GITHUB_TOKEN and GH_TOKEN are unset and \`gh auth status\` exited ${String(failure.code ?? "nonzero")}${reason ? ` (${reason})` : ""}`,
      };
    }
    get = async (path) => {
      const { stdout } = await execFileAsync("gh", ["api", path], { maxBuffer: 64 * 1024 * 1024 });
      return JSON.parse(stdout) as unknown;
    };
  }
  try {
    const listing = (await get(`repos/${repository}/actions/workflows/ci.yml/runs?status=completed&per_page=${runs}`)) as {
      workflow_runs: Array<{ id: number; created_at: string }>;
    };
    const result: GithubRun[] = [];
    for (const run of listing.workflow_runs.slice(0, runs)) {
      const artifacts = (await get(`repos/${repository}/actions/runs/${run.id}/artifacts?per_page=100`)) as {
        artifacts: GithubArtifact[];
      };
      result.push({
        id: run.id,
        created_at: run.created_at,
        artifacts: artifacts.artifacts.map((a) => ({ name: a.name, size_in_bytes: a.size_in_bytes, expired: a.expired })),
      });
    }
    return { ok: true, value: result };
  } catch (error) {
    return { ok: false, source, error: (error as Error).message.split("\n")[0] ?? "GitHub API failure" };
  }
}

async function accountUsageSource(): Promise<SourceResult<AccountUsage>> {
  const missing = ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"].filter((name) => !process.env[name]);
  if (missing.length > 0) {
    return { ok: false, source: "cloudflare-account-usage", error: `account-wide D1/R2 consumption is unmeasurable: ${missing.join(" and ")} unset` };
  }
  return {
    ok: false,
    source: "cloudflare-account-usage",
    error: "account-wide D1/R2 consumption measurement is not implemented in this stage; credentials are present but unused",
  };
}

// ---------------------------------------------------------------------------
// CLI

interface CliOptions extends InventoryOptions {
  repoRoot: string;
  remote: string;
  out: string | null;
}

function parseArgs(argv: readonly string[]): CliOptions {
  const options: CliOptions = { repoRoot: process.cwd(), remote: "origin", out: null, days: 30, runs: 30 };
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    const value = argv[i + 1];
    if (value === undefined) throw new Error(`${flag} needs a value`);
    i += 1;
    if (flag === "--repo-root") options.repoRoot = value;
    else if (flag === "--remote") options.remote = value;
    else if (flag === "--out") options.out = value;
    else if (flag === "--days" || flag === "--runs") {
      const n = Number(value);
      if (!Number.isInteger(n) || n < 1) throw new Error(`${flag} must be a positive integer`);
      options[flag === "--days" ? "days" : "runs"] = n;
    } else throw new Error(`unknown argument ${flag}`);
  }
  return options;
}

export async function main(argv: readonly string[]): Promise<number> {
  let options: CliOptions;
  try {
    options = parseArgs(argv);
  } catch (error) {
    process.stderr.write(`inventory: ${(error as Error).message}\n`);
    return 2;
  }
  const repoRoot = (await git(["rev-parse", "--show-toplevel"], options.repoRoot)).trim();
  const scratchParent = await mkdtemp(join(tmpdir(), "evidence-inventory-"));
  const cleanup = (): Promise<void> => rm(scratchParent, { recursive: true, force: true });
  const onSignal = (): void => {
    void cleanup().finally(() => process.exit(130));
  };
  process.once("SIGINT", onSignal);
  process.once("SIGTERM", onSignal);
  try {
    const sources: Sources = {
      notes: () => fetchNotes(repoRoot, options.remote, join(scratchParent, "notes.git")),
      fileSize: async (path) => {
        try {
          const info = await stat(path);
          return info.isFile() ? info.size : null;
        } catch {
          return null;
        }
      },
      github: (runs) => githubSource(repoRoot, options.remote, runs),
      accountUsage: accountUsageSource,
      now: () => Math.floor(Date.now() / 1000),
    };
    const report = await runInventory(options, sources);
    const json = `${JSON.stringify(report, null, 2)}\n`;
    if (options.out === null) process.stdout.write(json);
    else await writeFile(options.out, json);
    process.stderr.write(`${summarize(report)}\n`);
    return report.feasible ? 0 : 1;
  } finally {
    await cleanup();
  }
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (error: unknown) => {
      process.stderr.write(`inventory: ${(error as Error).message}\n`);
      process.exit(2);
    },
  );
}
