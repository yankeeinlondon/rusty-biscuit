import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  CLASS_A_ACTIONS,
  forecast,
  measureAccountUsage,
  percentile,
  percentiles,
  runInventory,
  walkNotesRef,
  type Measurements,
  type NotesSnapshot,
  type SourceResult,
  type Sources,
} from "../../cli/inventory.ts";
import { CAPACITY } from "../../src/contracts/api.ts";

const NOW = Date.UTC(2026, 10, 11) / 1000;
const DAY = 86_400;

function observation(id: string, pkg: string, outcome: "pass" | "fail", reportBytes = 1000): object {
  return {
    observation_id: id,
    producer: { kind: "hook", host: { os: "Linux", kernel: "6", report_dir: "/nowhere" } },
    base: "0".repeat(40),
    scope_identity: "s",
    completion: "complete",
    cell: { package: pkg, gate: "L1", outcome, report: `L1/${pkg}.xml` },
    reports: [{ path: `L1/${pkg}.xml`, sha256: "a", bytes: reportBytes, store: "host-dir" }],
  };
}

function collection(...observations: object[]): string {
  return JSON.stringify({ schema_version: 3, environment: "ubuntu-latest", head: "h", tree: "t", observations });
}

function snapshot(notes: Array<{ annotated: string; text: string; at: number }>): NotesSnapshot {
  const blobs: Record<string, string> = {};
  const current: Record<string, string> = {};
  const versions = notes.map((note, index) => {
    const blob = `b${index}`;
    blobs[blob] = note.text;
    current[note.annotated] = blob;
    return { annotated: note.annotated, blob, written_at: note.at };
  });
  return { refs: [{ ref: "refs/notes/ci-local/ubuntu-latest", tip: "tip", versions, current }], blobs };
}

const usage = { r2_bytes: 0, r2_class_a_per_month: 0, d1_bytes: 0, d1_rows_written_per_day: 0 };

function sources(notes: NotesSnapshot, overrides: Partial<Sources> = {}): Sources {
  return {
    notes: async () => ({ ok: true, value: notes }),
    fileSize: async () => null,
    github: async () => ({ ok: true, value: [{ id: 1, created_at: "", artifacts: [{ name: "a", size_in_bytes: 10, expired: false }] }] }),
    accountUsage: async () => ({ ok: true, value: usage }),
    now: () => NOW,
    ...overrides,
  };
}

const options = { days: 30, runs: 30 };

describe("percentiles", () => {
  it("uses nearest rank on known inputs", () => {
    const values = Array.from({ length: 100 }, (_, i) => 100 - i);
    expect(percentile(values, 0.5)).toBe(50);
    expect(percentile(values, 0.95)).toBe(95);
    expect(percentiles([3, 1, 2])).toEqual({ count: 3, p50: 2, p95: 3, max: 3 });
    expect(percentiles([])).toEqual({ count: 0, p50: null, p95: null, max: null });
  });
});

describe("forecast", () => {
  const base: Measurements = {
    baseline_observations: 1000,
    daily_mean: 100,
    daily_max: 400,
    observation_p95_bytes: 2000,
    report_p95_bytes: 10_000,
    reports_per_observation: 1,
    proof_p95_bytes: 1000,
    proofs_per_observation: 2,
    observed_cells: 50,
    unresolved_cells: 5,
    unresolved_growth_per_day: 0.1,
  };
  const r2PerObservation = 10_000 + 2 * 1000;

  it("doubles daily volume and accumulates D1 at p95 without pruning", () => {
    const result = forecast(base);
    expect(result.admission_daily_observations).toBe(200);
    expect(result.admission_peak_daily_observations).toBe(800);
    const twelve = 1000 + 200 * 30.44 * 12;
    expect(result.months[11]?.d1_bytes).toBe(Math.round(twelve * 2000 * 2));
    expect(result.months[11]?.r2_bytes_unpruned).toBe(Math.round(twelve * r2PerObservation));
    const rows = result.checks.find((c) => c.limit === "d1_rows_written_per_day");
    expect(rows?.projected).toBe(800 * (3 + 1) * 2);
    // Every object fits in one part: 1 part + create + complete, for 3 objects.
    const classA = result.checks.find((c) => c.limit === "r2_class_a_per_month");
    expect(classA?.projected).toBe(Math.round(200 * 30.44 * 3 * 3));
    expect(result.checks.every((c) => c.breach_month === null)).toBe(true);
  });

  it("projects R2 as retained cells plus the grace period's superseded bytes", () => {
    const result = forecast(base);
    // Doubled (cells + unresolved + growth) plus 14 days of doubled daily volume.
    const retained = 2 * (50 + 5 + 0.1 * 30.44 * 12) + 200 * 14;
    expect(result.months[11]?.r2_bytes).toBe(Math.round(retained * r2PerObservation));
    const r2 = result.checks.find((c) => c.limit === "r2_live_plus_staging_bytes");
    expect(r2?.projected).toBe(Math.round(retained * r2PerObservation));
  });

  it("reports the never-pruned R2 projection without making it a check", () => {
    // 2 × 50k/day × 12 KB a month passes 8 GB within months; retention stays far below.
    const result = forecast({ ...base, daily_mean: 50_000, daily_max: 50_000 });
    expect(result.unpruned_r2.breach_month).not.toBeNull();
    expect(result.unpruned_r2.constraint).toContain("retention verifier");
    expect(result.checks.map((c) => c.limit)).not.toContain(result.unpruned_r2.limit);
  });

  it("adds account usage to every month of the matching projection", () => {
    const plain = forecast(base);
    const offset = forecast(base, { r2_live_plus_staging_bytes: 1_000_000, d1_rows_written_per_day: 7 });
    expect(offset.months[0]!.r2_bytes - plain.months[0]!.r2_bytes).toBe(1_000_000);
    expect(offset.months[0]!.d1_bytes).toBe(plain.months[0]!.d1_bytes);
    const rows = (f: typeof plain) => f.checks.find((c) => c.limit === "d1_rows_written_per_day")!.projected;
    expect(rows(offset) - rows(plain)).toBe(7);
  });

  it("reports the first month crossing the refuse ratio", () => {
    // 2 × 1000/day × 30.44 × 2 KiB × 2 per month; 80% of 500 MB is crossed in month 2.
    const result = forecast({ ...base, baseline_observations: 0, daily_mean: 1000, observation_p95_bytes: 2048 });
    const d1 = result.checks.find((c) => c.limit === "d1_database_bytes");
    const perMonth = 2000 * 30.44 * 2048 * 2;
    expect(d1?.breach_month).toBe(Math.ceil((CAPACITY.d1_database_bytes * CAPACITY.refuse_ratio) / perMonth));
    expect(d1?.breach_month).toBe(2);
    expect(d1?.alert_month).toBe(Math.ceil((CAPACITY.d1_database_bytes * CAPACITY.alert_ratio) / perMonth));
  });
});

describe("runInventory", () => {
  const notes = snapshot([
    { annotated: "c1", text: collection(observation("o1", "a", "pass"), observation("o2", "b", "fail")), at: NOW - 2 * DAY },
    { annotated: "c2", text: collection(observation("o3", "a", "fail", 5000)), at: NOW - DAY },
  ]);

  it("is feasible when every source answers and nothing breaches", async () => {
    const report = await runInventory(options, sources(notes));
    expect(report.failures).toEqual([]);
    expect(report.feasible).toBe(true);
    expect(report.notes.observations).toBe(3);
    expect(report.notes.observed_cells).toBe(2);
    expect(report.history.unresolved_cells_now).toBe(2);
    expect(report.reports.recorded_bytes.max).toBe(5000);
    // Two observations of package `a` name the same retained path; it is stat-ed once.
    expect(report.reports.local.missing).toBe(2);
  });

  it("names a missing GitHub source and is infeasible", async () => {
    const github = async (): Promise<SourceResult<never>> => ({
      ok: false,
      source: "github-actions",
      error: "GITHUB_TOKEN and GH_TOKEN are unset and `gh auth status` exited 1",
    });
    const report = await runInventory(options, sources(notes, { github }));
    expect(report.feasible).toBe(false);
    expect(report.failures).toEqual([
      "missing required source github-actions: GITHUB_TOKEN and GH_TOKEN are unset and `gh auth status` exited 1",
    ]);
    expect(report.notes.observations).toBe(3);
    expect(report.forecast).not.toBeNull();
  });

  it("counts a malformed note as unparseable without dropping the rest", async () => {
    const withBad = snapshot([
      { annotated: "c1", text: collection(observation("o1", "a", "pass")), at: NOW - DAY },
      { annotated: "c2", text: "{not json", at: NOW - DAY },
    ]);
    const report = await runInventory(options, sources(withBad));
    expect(report.notes.unparseable_notes).toBe(1);
    expect(report.notes.refs[0]?.notes).toBe(2);
    expect(report.notes.observations).toBe(1);
    expect(report.notes.note_blob_bytes.count).toBe(2);
  });

  it("treats an empty scan as a missing baseline, not a zero estimate", async () => {
    const report = await runInventory(options, sources(snapshot([])));
    expect(report.feasible).toBe(false);
    expect(report.forecast).toBeNull();
    expect(report.failures.some((f) => f.startsWith("missing required source git-notes:") && f.includes("empty scan"))).toBe(true);
  });
});

describe("walkNotesRef", () => {
  it("reads a note's version history from a bare repository", async () => {
    const dir = mkdtempSync(join(tmpdir(), "inventory-test-"));
    try {
      const env = {
        ...process.env,
        GIT_AUTHOR_NAME: "t",
        GIT_AUTHOR_EMAIL: "t@example.invalid",
        GIT_COMMITTER_NAME: "t",
        GIT_COMMITTER_EMAIL: "t@example.invalid",
        GIT_CONFIG_NOSYSTEM: "1",
        GIT_CONFIG_GLOBAL: "/dev/null",
      };
      const repo = join(dir, "r.git");
      const run = (...args: string[]): string => execFileSync("git", args, { env, encoding: "utf8" }).trim();
      run("init", "--bare", "--quiet", repo);
      const tree = run("-C", repo, "hash-object", "-w", "-t", "tree", "/dev/null");
      const commit = run("-C", repo, "commit-tree", tree, "-m", "c");
      const notePath = join(dir, "note.json");
      writeFileSync(notePath, collection(observation("o1", "a", "pass")));
      run("-C", repo, "notes", "--ref", "refs/notes/ci-local/ubuntu-latest", "add", "-F", notePath, commit);
      const tip = run("-C", repo, "rev-parse", "refs/notes/ci-local/ubuntu-latest");
      const ref = await walkNotesRef(repo, "refs/notes/ci-local/ubuntu-latest", tip);
      expect(ref.versions).toHaveLength(1);
      expect(ref.versions[0]?.annotated).toBe(commit);
      expect(Object.keys(ref.current)).toEqual([commit]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

describe("measureAccountUsage", () => {
  const env = { CLOUDFLARE_API_TOKEN: "cf-secret-token", CLOUDFLARE_ACCOUNT_ID: "acct" };

  function fakeCloudflare(overrides: Record<string, () => Response> = {}) {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });
    const impl = (async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, init });
      for (const [fragment, respond] of Object.entries(overrides)) if (url.includes(fragment)) return respond();
      if (url.endsWith("/d1/database?per_page=100")) return json({ success: true, result: [{ file_size: 100 }, { file_size: 23 }] });
      if (url.endsWith("/r2/buckets")) return json({ success: true, result: { buckets: [{ name: "one" }, { name: "two" }] } });
      if (url.includes("/r2/buckets/one/usage")) return json({ success: true, result: { payloadSize: "1000", metadataSize: "10" } });
      if (url.includes("/r2/buckets/two/usage")) return json({ success: true, result: { payloadSize: "5", metadataSize: "0" } });
      const query = JSON.parse(String(init?.body)).query as string;
      if (query.includes("d1AnalyticsAdaptiveGroups")) {
        const group = (date: string, databaseId: string, rowsWritten: number) => ({ sum: { rowsWritten }, dimensions: { date, databaseId } });
        return json({ data: { viewer: { accounts: [{ d1AnalyticsAdaptiveGroups: [group("2026-11-01", "a", 40), group("2026-11-01", "b", 30), group("2026-11-02", "a", 50)] }] } }, errors: null });
      }
      const op = (actionType: string, requests: number) => ({ sum: { requests }, dimensions: { actionType } });
      return json({ data: { viewer: { accounts: [{ r2OperationsAdaptiveGroups: [op("PutObject", 9), op("UploadPart", 3), op("GetObject", 500)] }] } }, errors: null });
    }) as typeof fetch;
    return { impl, calls };
  }

  it("sums databases and buckets, takes the busiest day's rows, and counts only Class A operations", async () => {
    const cf = fakeCloudflare();
    const result = await measureAccountUsage(env, NOW, cf.impl);
    expect(result).toEqual({ ok: true, value: { d1_bytes: 123, r2_bytes: 1015, d1_rows_written_per_day: 70, r2_class_a_per_month: 12 } });
    expect(CLASS_A_ACTIONS.has("GetObject")).toBe(false);
    for (const call of cf.calls) {
      const headers = call.init?.headers as Record<string, string>;
      expect(headers["user-agent"]).toBeTruthy();
      expect(call.url).not.toContain(env.CLOUDFLARE_API_TOKEN);
    }
  });

  it("names the missing credentials without calling Cloudflare", async () => {
    const cf = fakeCloudflare();
    const result = await measureAccountUsage({ CLOUDFLARE_ACCOUNT_ID: "acct" }, NOW, cf.impl);
    expect(result).toMatchObject({ ok: false, source: "cloudflare-account-usage" });
    expect(!result.ok && result.error).toContain("CLOUDFLARE_API_TOKEN unset");
    expect(cf.calls).toEqual([]);
  });

  it("fails naming the endpoint and status, never the token", async () => {
    const cf = fakeCloudflare({ "/r2/buckets": () => new Response(JSON.stringify({ success: false, errors: [{ message: "Authentication error" }] }), { status: 403 }) });
    const result = await measureAccountUsage(env, NOW, cf.impl);
    expect(result.ok).toBe(false);
    expect(!result.ok && result.error).toBe("accounts/acct/r2/buckets answered 403: Authentication error");
    expect(JSON.stringify(result)).not.toContain(env.CLOUDFLARE_API_TOKEN);
  });
});
