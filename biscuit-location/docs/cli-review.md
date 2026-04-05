# `where` CLI Review

Review date: 2026-04-05

Scope: `biscuit-location/cli` (`where`), reviewed against the local `cli` skill, the repo's CLI conventions, and current runtime behavior.

## Summary

The `where` CLI has a solid base: the crate is thin, command routing is simple, Clap is used idiomatically for the current arguments, and the existing integration tests cover the happy path plus a few invalid-input cases.

The main gaps are not correctness bugs in the current commands. They are missing CLI platform features that the repo treats as standard: machine-readable/plain output modes, shell completions, richer error and tracing behavior, and a broader test matrix. Right now `where` behaves like a functional first pass, not a finished monorepo-grade CLI.

## What Already Follows Best Practices

- The lib/cli split is correct. Core logic lives in `biscuit-location/lib`, and the CLI mostly parses args, calls the library, and formats output.
- The source layout matches the expected shape:
  - `biscuit-location/cli/src/args.rs`
  - `biscuit-location/cli/src/commands.rs`
  - `biscuit-location/cli/src/output.rs`
  - `biscuit-location/cli/src/main.rs`
- Clap derive usage is straightforward and readable in `biscuit-location/cli/src/args.rs`.
- Parse errors use Clap's built-in handling, which correctly exits with code `2` for usage failures such as an invalid IP.
- Data is written to stdout and errors to stderr, which is the right default contract.
- `just test` passes for both the library and CLI packages.

## Recommendations

### 1. Add explicit output modes: `--json` and `--plain`

Priority: High

The local `cli` skill treats `--json` and `--plain` as mandatory output flags. `where` currently exposes neither. `biscuit-location/cli/src/args.rs:10` only defines `--db-path` and `--maps`, and `biscuit-location/cli/src/output.rs:4` only produces human-readable strings.

Why this matters:

- `where ip 8.8.8.8` and `where reverse ...` return multi-line prose that is pleasant for humans but awkward to consume from scripts.
- The current output layer has no stable contract for downstream tooling.
- If rich terminal formatting is added later, there is no escape hatch for pipe-safe output.

Recommended change:

- Add global output flags in `biscuit-location/cli/src/args.rs`:
  - `--json`
  - `--plain`
- Move output selection into `biscuit-location/cli/src/output.rs` behind typed response models instead of returning ad hoc strings.
- For `--json`, emit structured objects for:
  - location results
  - distance results
  - "no GPS fix" results
- Keep stdout data-only in every mode.

Minimum schema to support:

- Location:
  - `coordinates`
  - `place`
  - `source`
  - `accuracy_meters`
  - `maps_url`
- Distance:
  - `value`
  - `unit`
  - optionally `from` and `to` resolved coordinates

### 2. Add shell completions

Priority: High

The `cli` skill says shell completions are always required. The current CLI has no completions flag or subcommand, and `biscuit-location/cli/Cargo.toml` does not depend on `clap_complete`.

Why this matters:

- The command surface is small now, but users still benefit from completions for subcommands, flags, and unit values.
- The `distance` command especially needs discoverability because its input grammar is mixed (`gps`, `ip:<addr>`, `lat,lon`).

Recommended change:

- Add `clap_complete`.
- Expose either:
  - `where completions <shell>`
  - or `where --completions <shell>`
- Document install examples in `biscuit-location/cli/README.md`.
- Add tests that ensure completion generation works for the supported shells.

### 3. Stop bypassing `color_eyre` and define a real error presentation contract

Priority: Medium

`biscuit-location/cli/src/main.rs:21-29` installs `color_eyre` and then immediately reduces every failure to `eprintln!("Error: {err}")`. That discards most of the value of the crate and leaves no room for output-mode-aware error formatting.

Why this matters:

- User-facing errors should be consistently styled and deduplicated.
- Once `--json` exists, errors need an explicit machine-readable representation on stderr or a clearly documented human-only stderr contract.
- Current messages are serviceable but minimal, and they are not centrally controlled.

Recommended change:

- Return a `color_eyre::Result<()>` from `main` or introduce a dedicated error rendering function.
- If keeping custom stderr formatting, format the full chain intentionally instead of collapsing it to a single `{err}` string.
- Define how errors behave in:
  - default terminal mode
  - `--plain`
  - `--json`

### 4. Add verbosity and tracing controls

Priority: Medium

The CLI skill expects a distinction between human-facing verbosity and debug diagnostics. `where` currently has neither user-facing verbosity flags nor tracing initialization.

Relevant files:

- `biscuit-location/cli/src/args.rs`
- `biscuit-location/cli/src/main.rs`

Why this matters:

- `gps` and `reverse` can block on host APIs or network calls.
- `ip` depends on external configuration and common failure cases are operational, not logical.
- Without tracing or debug controls, failures are harder to diagnose in CI and support scenarios.

Recommended change:

- Add `-v`/`--verbose` and `-q`/`--quiet`.
- Support standard Rust diagnostics through `RUST_LOG`.
- Initialize `tracing_subscriber` in `main.rs`.
- Keep logs on stderr only.

Nice-to-have:

- Add `--debug <level>` later if this CLI starts accumulating more provider and transport behavior.

### 5. Tighten Clap ergonomics and discoverability

Priority: Medium

The argument layer works, but it leaves useful Clap features on the table.

Specific issues:

- `biscuit-location/cli/src/args.rs:53` uses a manual parser for distance units instead of `ValueEnum`, so help output and completions cannot enumerate valid values cleanly.
- `biscuit-location/cli/src/args.rs:47-55` accepts `from` and `to` as raw `String`, which prevents better hints and targeted validation messages at the Clap layer.
- `where --help` currently shows the generated `help` subcommand. The local CLI guidance says a help subcommand may exist, but it should not appear in help output itself.

Recommended change:

- Convert `DistanceUnit` or a CLI wrapper enum into `clap::ValueEnum`.
- Add clearer `value_name`s for `from` and `to`, such as `LOCATION`.
- Consider a dedicated parser type for `LocationInput` so syntax errors are normalized before command execution.
- Disable the visible help subcommand if keeping only `-h`/`--help` is sufficient.

### 6. Expand the test matrix beyond basic integration cases

Priority: Medium

The current test file at `biscuit-location/cli/tests/cli_tests.rs` is useful, but it is still light compared with the monorepo's stated CLI standard.

Current coverage gaps:

- No unit tests for `output.rs`
- No unit tests for argument parsing behavior in `args.rs`
- No snapshot tests for stable output formatting
- No tests for stdout vs stderr separation beyond one missing-subcommand case
- No assertions for exact exit codes
- No tests for future output modes or completions
- No PTY tests for any TTY-sensitive behavior

Recommended change:

- Add unit tests for formatting helpers in `biscuit-location/cli/src/output.rs`.
- Add integration assertions for:
  - exit code `2` on usage errors
  - stderr-only errors
  - stdout-only successful data output
- Add snapshot coverage once output modes exist.
- Add completion tests once completion support is added.

### 7. Document prerequisites and operational behavior more clearly

Priority: Medium

The CLI README is too terse for a user-facing command that depends on both local assets and networked services.

Relevant docs:

- `biscuit-location/cli/README.md`
- `biscuit-location/lib/README.md`

Specific gaps:

- `where ip <addr>` requires a MaxMind database, but the CLI README does not explain that.
- `where reverse <lat> <lon>` depends on reverse geocoding over the network, but the CLI README does not mention that operational dependency.
- The README does not define stdout/stderr behavior, exit code expectations, or any future output-mode contract.
- There is no installation guidance for completions because completions do not exist yet.

Recommended change:

- Expand `biscuit-location/cli/README.md` to include:
  - prerequisites for `ip`
  - network behavior for `reverse`
  - output modes
  - exit code semantics
  - completion installation

### 8. Consider graceful signal handling for long or blocking operations

Priority: Low

The CLI skill recommends graceful handling for `SIGINT` and `SIGTERM`. `where` does not currently do anything explicit here.

This is not urgent because the current commands are short-lived, but it becomes more important for:

- GPS waits with timeout
- reverse geocoding under slow network conditions

Recommended change:

- If the CLI grows more network or device interaction, add cancellation wiring with `tokio::signal`.
- Ensure partially written output is never emitted on cancellation.

## Suggested Implementation Order

1. Add output mode flags and typed output serialization.
2. Add shell completions.
3. Improve error rendering so it is compatible with the output contract.
4. Add tracing and verbosity controls.
5. Upgrade Clap typing and help ergonomics.
6. Expand tests and README coverage in the same change set.

## Validation Performed

- Read the local `cli` skill and `cli-best-practices.md`
- Reviewed:
  - `biscuit-location/cli/src/main.rs`
  - `biscuit-location/cli/src/args.rs`
  - `biscuit-location/cli/src/commands.rs`
  - `biscuit-location/cli/src/output.rs`
  - `biscuit-location/cli/tests/cli_tests.rs`
  - `biscuit-location/cli/README.md`
  - `biscuit-location/lib/src/error.rs`
  - `biscuit-location/lib/README.md`
- Ran `just test` in `biscuit-location/`
- Ran spot checks for:
  - `cargo run -q -p biscuit-location-cli -- --help`
  - `cargo run -q -p biscuit-location-cli -- ip not-an-ip`
  - `cargo run -q -p biscuit-location-cli -- reverse 999 0`
  - `cargo run -q -p biscuit-location-cli -- distance 34.0522,-118.2437 40.7128,-74.0060`
