//! The submitting host's `ReportReader`: Python's
//! `Path(report_dir).expanduser() / report` on Node. Not Worker-safe.
//!
//! Report paths are joined, never normalized or confined: an absolute report
//! path replaces `report_dir` and `..` walks out of it, exactly as the Python
//! reads them, because a historic receipt's paths name what its host retained.

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";
import type { ReportReadFailure, ReportReader } from "./adapter.ts";

export interface NodeReportReaderOptions {
  /** The home directory `~` expands to; defaults to `os.homedir()` (`HOME`, or `USERPROFILE` on Windows). */
  readonly home?: () => string;
  readonly platform?: NodeJS.Platform;
}

function failure(directory: string, message: string): ReportReadFailure {
  return { kind: "report-read-failure", directory, message };
}

/** `Path.expanduser`: only `~` alone or followed by a separator; `~user` is reported, not guessed. */
function expandUser(directory: string, home: () => string, windows: boolean): string | ReportReadFailure {
  if (!directory.startsWith("~")) return directory;
  const rest = directory.slice(1);
  const separator = rest.startsWith("/") || (windows && rest.startsWith("\\"));
  if (rest !== "" && !separator) return failure(directory, "cannot expand another user's home directory");
  const base = home().replace(windows ? /[\\/]+$/ : /\/+$/, "");
  return base + rest || (windows ? "\\" : "/");
}

const DRIVE = /^(?:[A-Za-z]:|[\\/]{2}[^\\/]+[\\/]+[^\\/]+)/;

/** `PurePath.__truediv__` for one relative-or-absolute component, without normalization. */
function joinPath(directory: string, report: string, windows: boolean): string {
  if (windows) {
    if (DRIVE.test(report)) return report;
    if (report.startsWith("\\") || report.startsWith("/")) return (DRIVE.exec(directory)?.[0] ?? "") + report;
    if (directory === "") return report;
    return /[\\/]$/.test(directory) ? directory + report : `${directory}\\${report}`;
  }
  if (report.startsWith("/") || directory === "") return report;
  return directory.endsWith("/") ? directory + report : `${directory}/${report}`;
}

export function createNodeReportReader(options: NodeReportReaderOptions = {}): ReportReader {
  const home = options.home ?? homedir;
  const windows = (options.platform ?? process.platform) === "win32";
  return (reportDir, reportPath) => {
    const directory = expandUser(reportDir, home, windows);
    if (typeof directory !== "string") return directory;
    try {
      return new Uint8Array(readFileSync(joinPath(directory, reportPath, windows)));
    } catch (error) {
      return failure(directory, error instanceof Error ? error.message : String(error));
    }
  };
}

export const nodeReportReader: ReportReader = createNodeReportReader();
