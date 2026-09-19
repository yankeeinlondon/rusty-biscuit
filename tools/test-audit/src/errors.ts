/**
 * Error and violation vocabulary shared by every command.
 *
 * Two error classes carry exit semantics: `UsageError` (exit 2 — the caller
 * asked for something the tool cannot interpret) and `MalformedInput`
 * (exit 1 — the evidence itself cannot be trusted). Everything that is a
 * finding rather than a failure to run is a `Violation`, collected and
 * rendered by the command that produced it.
 */

/** A usage problem: missing argument, unreadable path, unknown command. Exit 2. */
export class UsageError extends Error {}

/** Evidence that cannot be interpreted: malformed XML, truncated log, bad JSON. Exit 1. */
export class MalformedInput extends Error {}

/** A configuration file that fails schema or cross-field validation. Exit 2. */
export class ConfigError extends UsageError {}

export interface Violation<Kind extends string = string> {
  kind: Kind;
  detail: string;
}

/** Exit codes every command shares. */
export const EXIT = {
  clean: 0,
  violations: 1,
  usage: 2,
} as const;

export type ExitCode = (typeof EXIT)[keyof typeof EXIT];
