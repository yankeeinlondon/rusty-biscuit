## Important Standards

### Output Formats

- `--json` and `--plain` are mandatory output flags; add `--csv`, `--yaml`, `--toml` as needed
- The **default** output is terminal-optimized (escape codes for color, bold, dim, italics)
    - Start with the `Prose` component from `biscuit-terminal` for rich terminal output
    - Use `Table`, `TwoColumns`, `BlockQuote`, `OrderedList`, and `UnorderedList` for structure
- `--plain` strips all escape codes — essential for piped output and non-TTY contexts
- Respect the `NO_COLOR` environment variable (see https://no-color.org): when set, disable colors even without `--plain`
- Support `FORCE_COLOR=1` to enable colors even when output is not a TTY (useful for CI logs)

### STDOUT vs STDERR

- **STDOUT** is for data — the primary output the user or a downstream tool consumes
- **STDERR** is for metadata — progress indicators, status messages, warnings, diagnostics
- When `--json` is active, STDOUT **must** be valid JSON; never mix non-JSON text into it
- Error messages always go to STDERR, even in `--json` mode
- Progress bars and spinners (via `indicatif`) should write to STDERR so they don't corrupt piped data

### Exit Codes

- `0` — success
- `1` — general error (command failed)
- `2` — usage error (invalid arguments, missing required flags)
- Use clap's built-in exit code handling for argument parsing errors
- Document any domain-specific exit codes in `--help` output

### Verbosity

- `--verbose` / `-v` for more output; 
    - optionally support stacking (`-vv`, `-vvv`) via `action = ArgAction::Count` for increasing detail levels
    - verbose logging is user facing and should always carefully designed, it is not a replacement for debug logging
- `--quiet` / `-q` for minimal metadata output (data only)
- `--silent` sends nothing to STDOUT; only errors to STDERR
- `--no-output` suppresses both STDOUT and STDERR entirely
- Conflicting flag combinations (e.g., `--json --no-output`) should produce a clear error explaining the conflict

### Help System

- `--help` / `-h` is a globally registered flag
- Help adapts to command depth — shows only relevant details for the current subcommand level
- A `help` subcommand is fine to include but should not appear in help output itself

### Shell Completions

- Shell completions are always required
- Prefer **dynamic completions** — more useful and easier for users to integrate into shell init
- For simple CLIs: use a `--completions <shell>` flag; for subcommand-based CLIs: use a `completions <shell>` subcommand
- Make `<shell>` optional so that `--completions --help` works; show shell-specific installation examples in a "Examples" help section
- Use the `clap_complete` crate with `derive` and `unstable-ext` features on `clap`
- Provide value hints for completable parameters:
    - Enumerated values: supply the enum variants to completion logic
    - File paths: auto-complete to valid file types for the parameter
    - `FileReference` paths (from `biscuit-file`): resolve `@` and `!` based paths
        - `@` resolves from repo root or home directory (configurable)
        - `!` resolves from the current package root in monorepos

### Signal Handling

- Handle `SIGINT` (Ctrl+C) gracefully — clean up temp files, flush logs, release locks
- Handle `SIGTERM` the same way for containerized/daemonized usage
- Use `tokio::signal` or `ctrlc` crate depending on whether async is already in play
- Long-running operations should check for cancellation periodically rather than relying solely on process termination

## How to Structure Code in a Clap-Based CLI

A monolithic `main.rs` quickly becomes a maintenance and testing bottleneck. Use a modular layout:

### Module Breakdown

| Module | Responsibility |
|--------|---------------|
| `args.rs` | Clap structs (`Parser`, `Subcommand`, `Args`), flag attributes, shell completion logic, help strings. Nothing else. |
| `commands.rs` | Execution logic for each subcommand. Glue between CLI args and the core library. For CLIs with many subcommands, use a `commands/` directory with one file per subcommand. |
| `output.rs` | Output formatting — JSON serialization structs, table generation, terminal styling. |
| `main.rs` | Thin wrapper: `color_eyre` setup, tracing init, dynamic completions registration, arg parsing, route to `commands.rs`. |

### Library Interaction

The CLI crate (`./cli`) is a frontend for the library crate (`./lib`).

- **No business logic in CLI** — the CLI handles I/O: parsing input, mapping to library types, formatting output. Core logic lives in the library.
- **Testable by design** — library functions are unit-testable without mocking process arguments or standard streams.

## Effective Testing of a CLI Stack

### Testing Tech Stack

| Crate | Purpose |
|-------|---------|
| `assert_cmd` | Integration testing — spawn the binary, pass args, assert on stdout/stderr/exit codes |
| `insta` | Snapshot testing — capture complex visual output, track regressions with `cargo insta review` |
| `expectrl` | PTY testing — spawn a pseudo-terminal to test TTY-dependent behavior (colors, interactive prompts, terminal dimensions) |
| `proptest` | Property-based testing — fuzz custom parsers for adversarial edge cases |
| `cargo-nextest` | Fast test runner with retries for flaky tests, useful for TTY/timeout issues in CI |

### Testing Types

#### Unit Tests

Test internal helpers, serialization, and input parsers directly in `args.rs`, `commands.rs`, and `output.rs` with `#[test]` blocks. No binary spawning needed.

#### Integration Tests

Use `assert_cmd::Command` in `tests/` to test end-to-end command execution: exit codes, flag combinations, missing-argument errors, and help text output.

#### Snapshot Tests

Pipe CLI output to `insta::assert_snapshot!` for visual outputs, tables, and ANSI-styled content. **Always set `NO_COLOR=1` in test environments** to guarantee deterministic results across machines. Use `FORCE_COLOR=1` when specifically testing color output.

#### PTY Tests

Use `expectrl` to spawn the binary in a pseudo-terminal for testing TTY-dependent code paths (terminal width queries, interactive prompts, `is_terminal()` conditionals) that are skipped in standard integration tests.

### Error Output

- Use `color_eyre` for CLI error reporting
- Format user-facing errors with styled output (bold "Error:" prefix, deduped cause chain)
- Never show raw backtraces or internal error chains to users by default
