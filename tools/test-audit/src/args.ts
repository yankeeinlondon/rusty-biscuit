import { UsageError } from "./errors.ts";

export interface ParsedArgv {
  positional: string[];
  flags: Map<string, string | true>;
}

/**
 * Minimal `--flag value` / `--flag` / positional parser.
 *
 * `--flag=value` and `--flag value` are equivalent. A flag listed in
 * `booleans` never consumes the following token. Repeated flags keep the last
 * value; commands that need multi-valued flags read `flagAll`.
 */
export function parseArgv(argv: readonly string[], booleans: readonly string[] = []): ParsedArgv {
  const positional: string[] = [];
  const flags = new Map<string, string | true>();
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]!;
    if (token === "--") {
      positional.push(...argv.slice(i + 1));
      break;
    }
    if (!token.startsWith("--")) {
      positional.push(token);
      continue;
    }
    const eq = token.indexOf("=");
    if (eq >= 0) {
      flags.set(token.slice(2, eq), token.slice(eq + 1));
      continue;
    }
    const name = token.slice(2);
    if (booleans.includes(name)) {
      flags.set(name, true);
      continue;
    }
    const next = argv[i + 1];
    if (next === undefined || next.startsWith("--")) {
      flags.set(name, true);
      continue;
    }
    flags.set(name, next);
    i += 1;
  }
  return { positional, flags };
}

export function flagAll(argv: readonly string[], name: string): string[] {
  const values: string[] = [];
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]!;
    if (token === `--${name}`) {
      const next = argv[i + 1];
      if (next !== undefined && !next.startsWith("--")) {
        values.push(next);
        i += 1;
      }
    } else if (token.startsWith(`--${name}=`)) {
      values.push(token.slice(name.length + 3));
    }
  }
  return values;
}

export function requireString(parsed: ParsedArgv, name: string, usage: string): string {
  const value = parsed.flags.get(name);
  if (typeof value !== "string" || value.length === 0) {
    throw new UsageError(`--${name} is required\n${usage}`);
  }
  return value;
}

export function optionalString(parsed: ParsedArgv, name: string): string | undefined {
  const value = parsed.flags.get(name);
  return typeof value === "string" ? value : undefined;
}

export function hasFlag(parsed: ParsedArgv, name: string): boolean {
  return parsed.flags.has(name);
}
