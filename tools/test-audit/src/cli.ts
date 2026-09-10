#!/usr/bin/env -S npx tsx
/**
 * `test-audit` — shared test-suite audit commands for rusty-biscuit package
 * areas. Dispatch only; each command owns its parsing, validation, and
 * rendering. Exit codes: 0 clean, 1 violations or malformed evidence, 2 usage.
 *
 * Commands load lazily so that one module's runtime dependency (the
 * tree-sitter wasm, say) is paid only by the command that needs it.
 */
import { fileURLToPath } from "node:url";

import { stdio, type Command, type CommandIo } from "./command.ts";
import { EXIT, MalformedInput, UsageError } from "./errors.ts";
import { versionBanner } from "./version.ts";

interface CommandEntry {
  name: string;
  summary: string;
  load: () => Promise<Command>;
}

export const COMMANDS: readonly CommandEntry[] = [
  { name: "config", summary: "validate or display an area configuration", load: async () => (await import("./config-command.ts")).configCommand },
  { name: "capture", summary: "capture `cargo nextest list` for every declared selection", load: async () => (await import("./capture/command.ts")).captureCommand },
  { name: "fetch", summary: "assemble a CI run's JUnit staging trees per environment via gh", load: async () => (await import("./fetch/command.ts")).fetchCommand },
  { name: "junit", summary: "gate JUnit staging trees, compare baseline and candidate", load: async () => (await import("./junit/command.ts")).junitCommand },
  { name: "reconcile", summary: "prove the family inventory is total and non-overlapping", load: async () => (await import("./reconcile/command.ts")).reconcileCommand },
  { name: "sources", summary: "scan Rust sources for test attributes (source-side population)", load: async () => (await import("./reconcile/sources-command.ts")).sourcesCommand },
  { name: "attribute", summary: "attribute nextest run cost to families", load: async () => (await import("./attribute/command.ts")).attributeCommand },
  { name: "measure", summary: "local alternating-run measurement report and runner", load: async () => (await import("./measure/command.ts")).measureCommand },
  { name: "counters", summary: "validate and compare work-count evidence", load: async () => (await import("./counters/command.ts")).countersCommand },
];

function usage(): string {
  const width = Math.max(...COMMANDS.map((c) => c.name.length), "version".length);
  const rows = COMMANDS.map((c) => `  ${c.name.padEnd(width)}  ${c.summary}`).join("\n");
  return `usage: test-audit <command> [options]\n\ncommands:\n${rows}\n  ${"version".padEnd(width)}  print the tool version\n\nRun \`test-audit <command> --help\` for a command's options.`;
}

export async function main(argv: string[], io: CommandIo = stdio): Promise<number> {
  const [name, ...rest] = argv;
  if (name === undefined || name === "--help" || name === "-h" || name === "help") {
    io.out(usage());
    return name === undefined ? EXIT.usage : EXIT.clean;
  }
  if (name === "version" || name === "--version") {
    io.out(versionBanner());
    return EXIT.clean;
  }
  const entry = COMMANDS.find((c) => c.name === name);
  if (!entry) {
    io.err(`unknown command: ${name}\n${usage()}`);
    return EXIT.usage;
  }
  const command = await entry.load();
  if (rest.includes("--help") || rest.includes("-h")) {
    io.out(command.usage);
    return EXIT.clean;
  }
  try {
    return await command.run(rest, io);
  } catch (error) {
    if (error instanceof UsageError) {
      io.err(error.message);
      return EXIT.usage;
    }
    if (error instanceof MalformedInput) {
      io.err(`malformed input: ${error.message}`);
      return EXIT.violations;
    }
    throw error;
  }
}

const invokedDirectly =
  process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (invokedDirectly) {
  main(process.argv.slice(2)).then((code) => process.exit(code));
}
