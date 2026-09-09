/**
 * Manifest parsing for `_stage_junit` directories.
 *
 * Coupled to `just/devops.just` `_stage_junit`: each environment directory is
 * one uploaded `target/nextest/ci-reports` tree, holding `<tier>/<package>.xml`
 * per nextest invocation plus an appended `manifest.jsonl` whose records carry
 * `{tier, package, xml, exit_code, environment, duration_s, report_present}`.
 */
import { MalformedInput } from "../errors.ts";

export interface ManifestRecord {
  tier: string;
  package: string;
  xml: string;
  exit_code: number;
  environment: string;
  duration_s: number;
  report_present: boolean;
}

export function parseManifest(text: string, source: string): ManifestRecord[] {
  const records: ManifestRecord[] = [];
  text.split("\n").forEach((line, index) => {
    if (line.trim() === "") return;
    let value: unknown;
    try {
      value = JSON.parse(line);
    } catch (error) {
      throw new MalformedInput(`${source}:${index + 1}: not JSON — ${(error as Error).message}`);
    }
    const record = value as Partial<ManifestRecord>;
    for (const key of ["tier", "package", "xml", "environment"] as const) {
      if (typeof record[key] !== "string") {
        throw new MalformedInput(`${source}:${index + 1}: '${key}' is not a string`);
      }
    }
    for (const key of ["exit_code", "duration_s"] as const) {
      if (typeof record[key] !== "number" || !Number.isFinite(record[key])) {
        throw new MalformedInput(`${source}:${index + 1}: '${key}' is not a finite number`);
      }
    }
    if (typeof record.report_present !== "boolean") {
      throw new MalformedInput(`${source}:${index + 1}: 'report_present' is not a boolean`);
    }
    if ((record.duration_s as number) < 0) {
      throw new MalformedInput(`${source}:${index + 1}: 'duration_s' is negative`);
    }
    records.push(record as ManifestRecord);
  });
  return records;
}
