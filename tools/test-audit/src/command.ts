/** The contract every top-level command implements; `cli.ts` only dispatches. */
export interface CommandIo {
  out: (line: string) => void;
  err: (line: string) => void;
}

export interface Command {
  name: string;
  summary: string;
  usage: string;
  /** Returns the process exit code. Throws `UsageError` for argument problems. */
  run(argv: string[], io: CommandIo): number | Promise<number>;
}

export const stdio: CommandIo = {
  out: (line) => process.stdout.write(`${line}\n`),
  err: (line) => process.stderr.write(`${line}\n`),
};
