//! Dispatch the hosted ingestion demonstration (`evidence-store-demo.yml`) on
//! the origin repository's `main`.
//!
//! ```text
//! demo-hosted --run-id ID --observation NAME [--repo owner/name]
//! ```
//!
//! NAME is one nightly cell's `observation-<package>-<gate>-<environment>`
//! artifact in run ID; the workflow fetches that cell's `junit-` and
//! `completion-` artifacts beside it and assembles the collection itself
//! (`hosted-collection.ts`).
//!
//! The workflow authenticates to the service with its GitHub OIDC token, and
//! the service trusts that token only from the workflow committed on `main`.
//! So this command first confirms the workflow file is on `main` and, when it
//! is not, stops as a failed external prerequisite. It never pushes, merges,
//! or creates a branch to make the workflow appear.
//!
//! Exit codes: 0 dispatched; 1 a GitHub request failed; 2 usage error, missing
//! token, or the workflow is not on `main`.

import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { CliError, defaultIo, type OperatorIo } from "./operator.ts";

export const DEMO_WORKFLOW = "evidence-store-demo.yml";
const DEMO_REF = "main";
const SLUG = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const GITHUB_REMOTE = /^(?:https:\/\/github\.com\/|git@github\.com:|ssh:\/\/git@github\.com\/)([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+?)(?:\.git)?\/?$/;
const REQUEST_TIMEOUT_MS = 60_000;

/** `owner/name` from a GitHub remote URL (HTTPS, scp-style SSH, or ssh://); null for anything else. */
export function slugFromRemote(url: string): string | null {
  return GITHUB_REMOTE.exec(url.trim())?.[1] ?? null;
}

/** The nightly's per-cell observation artifact (`scripts/ci/hosted_observations.py`). */
const OBSERVATION_ARTIFACT = /^observation-[A-Za-z0-9_.-]+$/;

export interface DemoOptions {
  readonly runId: string;
  /** `observation-<package>-<gate>-<environment>`. */
  readonly observation: string;
  /** `owner/name`; read from `git remote get-url origin` when absent. */
  readonly repo?: string;
}

export interface DispatchRecord {
  readonly repository: string;
  readonly workflow: string;
  readonly ref: string;
  readonly inputs: Readonly<Record<string, string>>;
}

function githubHeaders(token: string): Record<string, string> {
  return {
    authorization: `Bearer ${token}`,
    accept: "application/vnd.github+json",
    "x-github-api-version": "2022-11-28",
    "user-agent": "evidence-store-demo",
  };
}

async function request(io: OperatorIo, url: string, init: RequestInit, what: string): Promise<Response> {
  try {
    return await io.fetch(url, { ...init, signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
  } catch (error) {
    throw new CliError(1, `${what} failed: ${error instanceof Error ? error.message : "network error"}`);
  }
}

function originSlug(io: OperatorIo): string {
  const result = io.spawn("git", ["remote", "get-url", "origin"]);
  const slug = result.status === 0 ? slugFromRemote(result.stdout) : null;
  if (slug === null) throw new CliError(2, "could not read a GitHub owner/name from `git remote get-url origin`; pass --repo owner/name");
  return slug;
}

export async function dispatchDemo(options: DemoOptions, io: OperatorIo): Promise<DispatchRecord> {
  const token = io.env["GITHUB_TOKEN"] || io.env["GH_TOKEN"];
  if (!token) throw new CliError(2, "demo-hosted dispatches a GitHub workflow; set GITHUB_TOKEN or GH_TOKEN (no interactive login is attempted). Nothing was dispatched.");
  if (!/^[1-9][0-9]*$/.test(options.runId)) throw new CliError(2, "--run-id must be a GitHub Actions run ID");
  if (!OBSERVATION_ARTIFACT.test(options.observation)) {
    throw new CliError(2, "--observation must name a nightly cell's observation-<package>-<gate>-<environment> artifact");
  }
  const inputs: Record<string, string> = { run_id: options.runId, observation: options.observation };
  if (options.repo !== undefined && !SLUG.test(options.repo)) throw new CliError(2, "--repo must be owner/name");
  const slug = options.repo ?? originSlug(io);

  const api = `https://api.github.com/repos/${slug}`;
  const contents = await request(
    io,
    `${api}/contents/.github/workflows/${DEMO_WORKFLOW}?ref=${DEMO_REF}`,
    { headers: githubHeaders(token) },
    `GitHub lookup of ${DEMO_WORKFLOW} on ${slug}`,
  );
  if (contents.status === 404) {
    throw new CliError(
      2,
      `external prerequisite unavailable: the workflow is not on ${DEMO_REF} of ${slug} (.github/workflows/${DEMO_WORKFLOW}). ` +
        "It must be merged through review first; this command never pushes or merges. Nothing was dispatched.",
    );
  }
  if (contents.status === 401) throw new CliError(2, `GitHub rejected the token (401) while looking up ${DEMO_WORKFLOW} on ${slug}. Nothing was dispatched.`);
  if (!contents.ok) throw new CliError(1, `GitHub lookup of ${DEMO_WORKFLOW} on ${slug} answered ${contents.status}. Nothing was dispatched.`);

  const dispatch = await request(
    io,
    `${api}/actions/workflows/${DEMO_WORKFLOW}/dispatches`,
    { method: "POST", headers: { ...githubHeaders(token), "content-type": "application/json" }, body: JSON.stringify({ ref: DEMO_REF, inputs }) },
    `dispatch of ${DEMO_WORKFLOW} on ${slug}`,
  );
  if (!dispatch.ok) {
    const detail = (await dispatch.text()).slice(0, 500);
    throw new CliError(1, `dispatch of ${DEMO_WORKFLOW} on ${slug} answered ${dispatch.status}: ${detail}`);
  }
  const record: DispatchRecord = { repository: slug, workflow: DEMO_WORKFLOW, ref: DEMO_REF, inputs };
  io.stdout(`${JSON.stringify(record, null, 2)}\n`);
  io.stderr(`demo-hosted: dispatched ${DEMO_WORKFLOW} on ${slug}@${DEMO_REF}; follow it under the repository's Actions tab\n`);
  return record;
}

const USAGE = "usage:\n  demo-hosted --run-id ID --observation NAME [--repo owner/name]\n";

/** Runs the command; answers the exit code and reports failures on `io.stderr`. */
export async function run(argv: readonly string[], io: OperatorIo = defaultIo()): Promise<number> {
  try {
    let v: Record<string, string | undefined>;
    try {
      v = parseArgs({
        args: [...argv],
        options: Object.fromEntries(["run-id", "observation", "repo"].map((name) => [name, { type: "string" as const }])),
        strict: true,
        allowPositionals: false,
      }).values as Record<string, string | undefined>;
    } catch (error) {
      throw new CliError(2, (error as Error).message);
    }
    const absent = ["run-id", "observation"].filter((name) => v[name] === undefined);
    if (absent.length > 0) throw new CliError(2, `missing ${absent.map((name) => `--${name}`).join(", ")}`);
    await dispatchDemo(
      {
        runId: v["run-id"]!,
        observation: v["observation"]!,
        ...(v["repo"] === undefined ? {} : { repo: v["repo"] }),
      },
      io,
    );
    return 0;
  } catch (error) {
    if (!(error instanceof CliError)) throw error;
    io.stderr(`demo-hosted: ${error.message}\n`);
    if (error.exitCode === 2 && /^(missing --|Unknown option|Option )/.test(error.message)) io.stderr(USAGE);
    return error.exitCode;
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await run(process.argv.slice(2));
}
