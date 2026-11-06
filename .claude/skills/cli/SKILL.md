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

**Root command:** a CLI that has subcommands never has a separate root command. A root alias is fine for ergonomics, but it must be an exact alias of one named subcommand.

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

### The `--feature` convention

Named opt-ins share one flag across this repository's CLIs:

- `--feature <NAME>` is repeatable and comma-separated (`--feature a --feature
  b` == `--feature a,b`); names are case-insensitive; repeating one is harmless.
- Each CLI declares its names in a `ValueEnum` (kebab-case). An unknown or
  empty name (`--feature a,,b`, `--feature ""`) is a usage error (exit 2)
  listing the valid names, and completion offers them.
- When two named features overlap, the more restrictive one wins.
- clap gives all of this from `#[arg(long = "feature", value_enum,
  value_delimiter = ',', ignore_case = true, action = ArgAction::Append)]` on a
  `Vec<Feature>`; clap rejects empty elements itself. Lower the list onto the
  options in one helper so adding a name is one variant plus one match arm.
  Reference: `md` (`darkmatter/cli/src/args/enums.rs` `Feature`,
  `render::apply_features`; docs in `darkmatter/docs/cli/render.md#features`).
- Names that several CLIs share live once, in the library that owns their
  meaning, as a `ValueEnum` behind its `clap` feature, with the fold beside
  it. The Mermaid text-fallback names (`no-mermaid-fallbacks`,
  `no-mermaid-partial-fallbacks`, `no-mermaid-partial-warning`) are
  `biscuit_visualized::components::MermaidFallbackFeature`, folded by
  `MermaidFallbackPolicy::from_features`. A CLI whose only features are shared
  uses that enum directly (`bt`: a flattened `MermaidFeatureArgs` in
  `biscuit-terminal/cli/src/args.rs`); a CLI with its own names adds variants
  to its `Feature` enum and maps them onto the shared one (`md`:
  `Feature::mermaid_fallback()`).
- A flag beats the environment: lower each feature onto an *explicit* library
  setting, and leave the setting unset when no feature names it, so an
  environment variable (here `MERMAID_FALLBACK`) still applies.
- A feature that only matters in some mode is accepted, with no effect, outside
  it (`md`'s fallback features without `--mermaid`; `bt git-graph`, whose
  diagram has no text drawing), and docs say so. Names a CLI does not declare
  are usage errors (`bt --feature no-mermaid-rendering`).
- Document each CLI's names in its CLI reference page (`md`:
  `darkmatter/docs/cli/render.md#features`; `bt`:
  `biscuit-terminal/docs/cli.md#--feature`).

## Output Standards

See [cli-best-practices.md](cli-best-practices.md) for the full standards reference.

### Output Formats

Every output comes from one of two sources on the result: implement `renderable::CommandOutput` ([trait](../../../renderable/docs/command-output.md)) and print with `biscuit_terminal::output::write_output` ([dispatch](../../../biscuit-terminal/docs/output.md)), never a hand-written renderer per output.

| Family | `--output` values | Source |
|--------|-------------------|--------|
| formatted | `terminal`, `plain`, `markdown`, `markdown-plus`, `html` (alias `browser`) | `as_prose()` |
| data | `json`, `json5`, `yaml`, `toml` | `as_data()` |

- `--output <target>` is the **one** output selector. Do not add `--json`, `--plain`, `--format`, or per-format flags
- A command accepts whole families (both, or data only) plus any command-specific extras its `--help` lists; never part of a family
- Every command declares its default in `--help`, and there is no `auto`. An explicit `--output` always wins
    - a command whose deliverable is a Markdown document defaults to `markdown` and **always** writes Markdown, whatever stdout is
    - every other command defaults to `terminal`, which degrades to the same bytes as `plain` when stdout is not a terminal or the terminal has no color
- `plain` removes styling, not meaning: no escape byte ever, but code keeps its backticks and fences and tables keep their borders
- Respect `NO_COLOR` env var; support `FORCE_COLOR=1` for CI

### `--width`

- `--width <N | N% | auto | source | unlimited>` is the CLI-wide width option; every command has it, with no effect where no prose prints
- `auto` is `OUTPUT_COLUMNS`, else the terminal's columns, else unlimited, never a guessed 80; a Markdown-document command defaults to `source`
- Resolve it with `biscuit_terminal::output::{parse_width_request, resolve_width}`. `--max-width` is **not** a general pattern

### STDOUT vs STDERR

- **STDOUT** holds the deliverable, in the requested format
- **Data output**: STDOUT is exactly one value, on success and on failure, and STDERR is **empty**. Warnings, timings, and problems are fields in the value, or are not said
- **Formatted output**: anything that is not the deliverable goes to STDERR, as `terminal` with the `plain` fallback, judged by STDERR's own TTY and color detection
- When a report is what the command was run to produce, it is the deliverable and goes to STDOUT
- STDERR has its own width (`resolve_stderr_width`): its terminal's columns, or unlimited. `--width` and `OUTPUT_COLUMNS` never apply to it

### Typed data outputs

Every command's data output has a schema file enforced by tests (per-command conformance, a guard that fails on a command with no schema, format parity), so a new command cannot ship an untyped data output. Reference: `darkmatter/cli/schemas/output/`.

### Verbosity

| Flag | Behavior |
|------|----------|
| `-v` / `--verbose` | More output; optionally stackable (`-vv`, `-vvv`) |
| `-q` / `--quiet` | Minimal metadata, data only |
| `--silent` | Nothing to STDOUT; errors to STDERR only |
| `--no-output` | Suppresses both STDOUT and STDERR |

Conflicting combinations (e.g. `--output json --no-output`) should produce a clear error. `-v` only affects formatted outputs.

### Exit Codes

- `0` = the command ran and succeeded
- `1` = the command ran and failed; a data output still holds the one value, with its failure outcome on STDOUT
- `2` = a usage error, and nothing else: the arguments could not be used, no command ran, the message is on STDERR, and STDOUT is empty. This is the one exception to an empty STDERR in data mode
- A script decides what happened from the exit status, never from STDERR. A check that finds a problem exits 1, not 2
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
| PTY (Level 1) | `expectrl` | TTY-dependent paths with manufactured input bytes |
| Real-terminal (Level 2) | `wezterm cli`, `kitty @`, `tmux` | Render correctness in an actual terminal/multiplexer; capture pane text |
| Real-keyboard (Level 3) | `cliclick` (macOS), `xdotool` (Linux) | OS-level key injection so the terminal's input encoder fires |
| Property | `proptest` | Fuzz custom parsers for edge cases |
| Runner | `cargo-nextest` | Fast execution with retries for flaky TTY/timeout tests |

**Test rigor matters as much as test count.** A user-observable requirement of the form "when the user holds modifier X, behaviour Y happens" cannot be covered by Level 1 tests alone — the terminal's encoder is never exercised. See [`cli-best-practices.md` §Test Rigor](cli-best-practices.md) for the terminal Level 1 / 2 / 3 vocabulary; the `rust-testing` skill's `verification-fidelity.md` generalizes it to F1/F2/F3 rungs per boundary. In the rusty-biscuit monorepo, the Level 2/3 harness implementations live in the shared `biscuit-test-harness` crate — its `biscuit-test-harness/README.md` documents the harness variants, when to use each, and the environment each requires.

## Detailed References

- **[clap.md](clap.md)** — Full clap reference: Derive API, Builder API, subcommands, validation, env vars, completions, ecosystem crates
- **[cli-best-practices.md](cli-best-practices.md)** — Complete standards: output formats, STDOUT/STDERR, exit codes, verbosity, signals, testing, error output
