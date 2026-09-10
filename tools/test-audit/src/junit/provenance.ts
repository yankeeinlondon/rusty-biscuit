/**
 * CI provenance stamped into staging trees by `just/devops.just` `_stage_junit`.
 *
 * A `provenance.json` file sits beside `manifest.jsonl` when the run has GitHub
 * Actions environment variables. Absent provenance makes the tree usable as
 * `local` evidence and refused as `ci` evidence.
 */
import { MalformedInput } from "../errors.ts";

/** GitHub Actions environment variables captured at staging time. */
export interface CiProvenance {
  /** Source revision SHA (`GITHUB_SHA`). */
  sha: string;
  /** Workflow run ID (`GITHUB_RUN_ID`). */
  runId: string;
  /** Run number within the workflow (`GITHUB_RUN_NUMBER`). */
  runNumber: number;
  /** Run attempt (`GITHUB_RUN_ATTEMPT`). */
  runAttempt: number;
  /** Git ref (`GITHUB_REF`). */
  ref: string;
  /** Event that triggered the workflow (`GITHUB_EVENT_NAME`). */
  event: string;
  /** Workflow name (`GITHUB_WORKFLOW`). */
  workflow: string;
}

export type Provenance = CiProvenance | null;

export function parseProvenance(text: string, source: string): CiProvenance {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    throw new MalformedInput(`${source}: not JSON — ${(error as Error).message}`);
  }

  const obj = value as Partial<CiProvenance>;

  for (const key of ["sha", "runId", "ref", "event", "workflow"] as const) {
    if (typeof obj[key] !== "string") {
      throw new MalformedInput(`${source}: '${key}' is not a string`);
    }
  }

  for (const key of ["runNumber", "runAttempt"] as const) {
    if (typeof obj[key] !== "number" || !Number.isInteger(obj[key]) || (obj[key] as number) < 1) {
      throw new MalformedInput(`${source}: '${key}' is not a positive integer`);
    }
  }

  return obj as CiProvenance;
}
