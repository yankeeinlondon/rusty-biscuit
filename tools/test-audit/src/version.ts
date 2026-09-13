import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * The tool version stamped into every report header.
 *
 * Reports produced by different tool versions are not silently comparable:
 * a parser correction can move a number without any change in the suite, so
 * the stamp travels with the evidence.
 */
export const TOOL_VERSION: string = (() => {
  const manifest = JSON.parse(readFileSync(join(here, "..", "package.json"), "utf8")) as {
    name: string;
    version: string;
  };
  return `${manifest.name}@${manifest.version}`;
})();

export function versionBanner(): string {
  return `${TOOL_VERSION} (node ${process.version})`;
}
