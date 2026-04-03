---
name: cli
description: Expert knowledge for designing and building high-quality Rust CLI applications. Use when creating CLI tools, parsing arguments, structuring output formats, implementing shell completions, handling signals, or following CLI best practices. Covers clap, output formatting, testing strategies, and UNIX conventions.
---

# CLI Design & Development

Build well-behaved, composable command-line tools in Rust. This skill covers argument parsing with clap, output discipline, testing strategies, and the conventions that make CLIs pleasant to use in pipelines and interactive sessions.

## Architecture

CLI packages in this monorepo follow a strict **lib/cli split**:

- `./lib` — core logic, independently testable, no I/O assumptions
- `./cli` — thin frontend: parse args, call library, format output

No business logic lives in the CLI crate. The CLI handles I/O only.

### Module Layout

| Module | Responsibility |
|--------|---------------|
| `args.rs` | Clap structs, flag attributes, completions, help strings |
| `commands.rs` | Execution logic per subcommand (glue between args and library) |
| `output.rs` | JSON serialization, table generation, terminal styling |
| `main.rs` | Setup (`color_eyre`, tracing, completions), arg parse, route to commands |

For CLIs with many subcommands, use a `commands/` directory with one file per subcommand.

## Argument Parsing with clap

Use the **Derive API** (covers 90% of cases). See [clap.md](clap.md) for full reference including Builder API, custom validation, environment variable fallback, enum value selection, subcommand patterns, and shell completion generation.

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    name: String,

    #[arg(short, long, default_value_t = 1)]
    count: u8,
}
```

Key clap patterns:
- `#[arg(short, long)]` for `-n`/`--name` flags
- `#[arg(action = ArgAction::Count)]` for stacking `-vvv`
- `#[command(subcommand)]` for git-style nested commands
- `clap::ValueEnum` for enum flag values
- `clap_complete` with `derive` + `unstable-ext` features for shell completions

## Output Standards

See [cli-best-practices.md](cli-best-practices.md) for the full standards reference.

### Output Formats

- `--json` and `--plain` are **mandatory** output flags
- Default output is terminal-optimized (colors, bold, dim, italics via `biscuit-terminal`)
- `--plain` strips all escape codes for piped/non-TTY contexts
- Respect `NO_COLOR` env var; support `FORCE_COLOR=1` for CI

### STDOUT vs STDERR

- **STDOUT** = data (what downstream tools consume)
- **STDERR** = metadata (progress, status, warnings, diagnostics)
- `--json` mode: STDOUT must be valid JSON only
- Errors always go to STDERR, even in `--json` mode

### Verbosity

| Flag | Behavior |
|------|----------|
| `-v` / `--verbose` | More output; optionally stackable (`-vv`, `-vvv`) |
| `-q` / `--quiet` | Minimal metadata, data only |
| `--silent` | Nothing to STDOUT; errors to STDERR only |
| `--no-output` | Suppresses both STDOUT and STDERR |

Conflicting combinations (e.g. `--json --no-output`) should produce a clear error.

### Exit Codes

- `0` = success, `1` = general error, `2` = usage error (bad args)
- Use clap's built-in exit code handling for parse errors

## Error Handling

- Use `color_eyre` for CLI error reporting
- Format user-facing errors with styled output (bold "Error:" prefix, deduped cause chain)
- Never show raw backtraces or internal error chains by default

## Shell Completions

- **Always required** for CLI packages
- Prefer **dynamic completions** over static scripts
- Simple CLIs: `--completions <shell>` flag; subcommand CLIs: `completions <shell>` subcommand
- Provide value hints: enum variants, file paths, `FileReference` paths (`@`/`!` prefixes)

## Signal Handling

- Handle `SIGINT` (Ctrl+C) and `SIGTERM` gracefully: clean up temps, flush logs, release locks
- Use `tokio::signal` (async) or `ctrlc` crate (sync)
- Long-running operations should check for cancellation periodically

## Help System

- `--help` / `-h` is globally registered
- Help adapts to command depth (only relevant details per subcommand level)
- `help` subcommand is fine but should not appear in help output itself

## Testing Strategy

| Layer | Tool | What to test |
|-------|------|-------------|
| Unit | `#[test]` | Internal helpers, serialization, parsers in args/commands/output |
| Integration | `assert_cmd` | End-to-end: exit codes, flag combos, missing args, help text |
| Snapshot | `insta` | Visual output, tables, ANSI content (set `NO_COLOR=1`) |
| PTY | `expectrl` | TTY-dependent paths: terminal width, interactive prompts, `is_terminal()` |
| Property | `proptest` | Fuzz custom parsers for edge cases |
| Runner | `cargo-nextest` | Fast execution with retries for flaky TTY/timeout tests |

## Detailed References

- **[clap.md](clap.md)** — Full clap reference: Derive API, Builder API, subcommands, validation, env vars, completions, ecosystem crates
- **[cli-best-practices.md](cli-best-practices.md)** — Complete standards: output formats, STDOUT/STDERR, exit codes, verbosity, signals, testing, error output
