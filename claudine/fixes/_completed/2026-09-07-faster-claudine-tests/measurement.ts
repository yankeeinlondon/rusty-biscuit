#!/usr/bin/env -S npx tsx
/**
 * Compatibility wrapper (2026-09-08): the implementation moved to the shared
 * `test-audit` package at `tools/test-audit` (command `measure`).
 * This file keeps the entry point the plan and log records name, forwards
 * every argument unchanged, and supplies this fix's `audit.config.json`
 * unless `--config` is given. No parsing logic lives here.
 */
import { main } from "../../../tools/test-audit/src/cli.ts";

const argv = process.argv.slice(2);
if (!argv.includes("--config") && !argv.includes("--help")) {
  argv.push("--config", new URL("./audit.config.json", import.meta.url).pathname);
}
main(["measure", ...argv]).then((code) => process.exit(code));
