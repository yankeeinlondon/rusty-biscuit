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

### Debugging and Tracing

- `--verbose` is for richer human-facing output only; it should NOT map directly to debug reporting
    - Everything that is emitted by "verbose" setting should be made to fit in with a well formed and crafted message to the user. When traces are logged, they provide useful insights to developers who are debugging but look crude in comparison to the rest of the CLI's output
    - we recommend using `--debug <level>` as well as supporting the RUST_LOG env triggers typically used in idiomatic Rust. Both of these methods are fine to use a basic reporter to print traces and metrics.
- Help text and docs must describe `--verbose` and debug logging separately; never imply that `-vv` means "debug"
- Support standard Rust diagnostics through `RUST_LOG`
- If the CLI exposes an explicit debug switch, prefer `--debug <level>` with clear values such as `trace`, `debug`, `info`, `warn`, and `error`
- Prefer no short alias for `--debug` when the CLI wraps or forwards to other CLIs that may already use `-d`
- Precedence should be: `RUST_LOG` first, then `--debug <level>`, then the CLI's default filter
- Avoid custom env vars like `DEBUG` unless there is a strong compatibility reason
- Keep diagnostic logs on STDERR so STDOUT remains clean for data and piping
- Use `tracing` spans for major boundaries: command entry, session/run, external process execution, network calls, retries, validations, and expensive operations
- Include timing at those span boundaries so traces explain both control flow and performance
- When practical, configure the tracing subscriber to emit span-close timing for debug sessions
- Prefer structured trace fields over prose: command name, subcommand, provider, session ID, model, attempt number, exit code, duration, and similar stable identifiers
- Never emit secrets, raw tokens, or full sensitive payloads into traces by default; use redaction, hashes, counts, and short safe summaries instead


### Trace Metrics

- Treat spans as the primary source of performance truth; every major operation should have a start, end, outcome, and duration
- Record latency at useful boundaries: total command runtime, subcommand runtime, external process execution, network requests, retries, validation passes, parsing, rendering, and file I/O
- Include outcome fields on measured spans: success/failure, exit code, retry count, timeout, cancellation, and fallback mode
- Prefer stable numeric fields that support aggregation: `duration_ms`, `items_processed`, `bytes_read`, `bytes_written`, `attempt`, `exit_code`, `status_code`, `cache_hit`, `tool_calls`, `warnings`, `errors`
- For throughput-style work, record both duration and work size so downstream systems can compute rates instead of guessing
- Distinguish queue/wait time from active execution time when both matter
- Emit one metric per boundary, not one metric per log line; avoid noisy event spam that cannot be aggregated cleanly
- Keep metric names and fields consistent across CLIs in the monorepo so dashboards and comparisons work without per-tool translation
- Use low-cardinality dimensions for metrics and summary spans; avoid unbounded labels such as raw file paths, prompts, user input, UUID-heavy labels, or full command lines
- If high-cardinality context is useful for debugging, keep it in traces as redacted fields rather than promoting it to metric labels
- Prefer histograms or duration distributions for latency analysis; counters are better for totals such as retries, failures, warnings, bytes, and items processed
- When timing nested operations, ensure the hierarchy is meaningful so total runtime can be explained by child spans instead of double-counted
- For flaky or retrying workflows, record both per-attempt metrics and final rolled-up outcome metrics
- When external systems are involved, capture enough metadata to isolate bottlenecks: remote service name, operation kind, status code/class, timeout path, and fallback path
- Metrics should be safe by default: never include secrets, auth headers, tokens, prompt bodies, or raw sensitive payloads

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

| Crate / Tool | Purpose |
|-------|---------|
| `assert_cmd` | Integration testing — spawn the binary, pass args, assert on stdout/stderr/exit codes |
| `insta` | Snapshot testing — capture complex visual output, track regressions with `cargo insta review` |
| `expectrl` | PTY testing — spawn a pseudo-terminal to test TTY-dependent behavior (colors, interactive prompts, terminal dimensions) |
| `proptest` | Property-based testing — fuzz custom parsers for adversarial edge cases |
| `cargo-nextest` | Fast test runner with retries for flaky tests, useful for TTY/timeout issues in CI |
| `wezterm cli`, `kitty @`, `tmux` | Real-terminal IPC for Level 2 tests — spawn the binary inside an actual terminal/multiplexer and capture rendered pane text |
| `cliclick` (macOS), `xdotool` (Linux) | OS-level keyboard injection for Level 3 tests — emit real `CGEventCreateKeyboardEvent` / X11 events so the terminal's input encoder fires |

### Testing Types

#### Unit Tests

Test internal helpers, serialization, and input parsers directly in `args.rs`, `commands.rs`, and `output.rs` with `#[test]` blocks. No binary spawning needed.

#### Integration Tests

Use `assert_cmd::Command` in `tests/` to test end-to-end command execution: exit codes, flag combinations, missing-argument errors, and help text output.

#### Snapshot Tests

Pipe CLI output to `insta::assert_snapshot!` for visual outputs, tables, and ANSI-styled content. **Always set `NO_COLOR=1` in test environments** to guarantee deterministic results across machines. Use `FORCE_COLOR=1` when specifically testing color output.

#### PTY Tests

Use `expectrl` to spawn the binary in a pseudo-terminal for testing TTY-dependent code paths (terminal width queries, interactive prompts, `is_terminal()` conditionals) that are skipped in standard integration tests.

### Test Rigor: Level 1 / Level 2 / Level 3

**Test count is not test rigor.** A feature may have hundreds of unit and integration tests and still ship with a glaring user-visible bug if none of those tests exercise the right layer. Classify every user-observable requirement against these three levels:

#### Level 1 — In-Process / PTY

Unit tests, plus tests that spawn the binary in a pseudo-TTY (`expectrl`) and feed manufactured input bytes. Verifies internal state transitions, byte-level parsing, and rendering logic.

**What it cannot catch:** anything that depends on the real terminal's input encoder. Example: "WezTerm did not emit bare-modifier press events because we forgot to push `REPORT_ALL_KEYS_AS_ESCAPE_CODES`" — Level 1 cannot see this because *the test* generates the bytes the binary parses; the terminal is never involved.

#### Level 2 — Run-In-Real-Terminal with IPC

Spawn the binary inside an actual terminal emulator (WezTerm / Kitty) or multiplexer (tmux) and capture the rendered pane text via the terminal's CLI:

- WezTerm: `wezterm cli spawn --new-window`, `wezterm cli send-text`, `wezterm cli get-text --escapes`
- Kitty: `kitty @ launch`, `kitty @ send-text`, `kitty @ get-text --ansi`
- tmux: `tmux new-session -d`, `tmux send-keys`, `tmux capture-pane -p -e`

Verifies that glyphs, widths, SGR styling, scroll/overflow, and cursor positioning render correctly through a real terminal — none of which a PTY test can prove. Input is still injected as bytes via the terminal's CLI, so the *input encoder* is not exercised.

**Skip semantics:** the harness's `available()` probe should test for the binary on `$PATH` plus any required env (`WEZTERM_UNIX_SOCKET`, `KITTY_LISTEN_ON`). If the host lacks the tooling, print `skipping: requires <X>` to stderr and return. No `#[ignore]` markers.

#### Level 3 — OS Keyboard Injection

Inject real OS keyboard events into the spawned terminal window — the terminal's input encoder fires, then encodes/forwards bytes to the binary just as if a human had pressed the key:

- macOS: `cliclick` (e.g. `cliclick kd:ctrl t:r ku:ctrl` for a chord; `kd:ctrl,sleep,ku:ctrl` for a hold)
- Linux: `xdotool`

This is the **only** level that can verify "what bytes does the terminal actually emit when key X is pressed?" — required for any spec line of the form "*when the user holds/presses key X, Y happens*."

**Caveats:**
- macOS focus is a shared global resource; gate Level 3 behind `RUN_LEVEL3=1` so non-deterministic focus thrash doesn't break `cargo test`.
- Before injection, focus the spawned pane explicitly (`wezterm cli activate-pane --pane-id N`) AND raise the app (`osascript -e 'tell application "WezTerm" to activate'`).
- Provide a parallel chord-injection test alongside the modifier-hold test — the chord variant proves the cliclick→terminal→binary chain works end-to-end, isolating "did the press arrive" from "did the terminal encode it correctly."
- **Capture during the hold, not after.** Splitting `kd:` and `ku:` with the capture in between is critical when the binary clears its display state on modifier release. A `hold_modifier(800ms)` helper that internally sequences press → sleep → release leaves nothing observable by the time `capture()` runs.

##### Known limitation: cliclick + bare modifier keys on macOS

cliclick uses `CGEventCreateKeyboardEvent`, but macOS routes bare-modifier key state through `flagsChanged` events at the AppKit layer. cliclick's synthetic modifier events do not always reach apps via that path:

- **Chord injection works** (`kd:ctrl t:r ku:ctrl`): the modifier flag rides along with the letter `keyDown`, which IS a normal CGEvent that AppKit delivers correctly. A Level-3 chord test like "Ctrl+R submits the option bound to Ctrl+R" passes reliably.
- **Bare-modifier injection is unreliable**: `kd:ctrl` alone often does not produce a `flagsChanged` event that WezTerm sees. The press is dropped before the terminal's input encoder ever fires. This is true even when the binary's bare-modifier handling is correct.

**Workaround — Level 2 with raw kitty bytes.** When you need to verify "the binary correctly handles bare-modifier kitty bytes inside a real terminal," send the literal escape sequence through the terminal's CLI instead of using OS keyboard injection:

```rust
// Pipe `\e[57442;1u` (kitty: bare LeftControl press) into a real
// WezTerm pane and assert badges render.
harness.send_text(b"\x1b[57442;1u")?;
std::thread::sleep(Duration::from_millis(200));
let frame = harness.capture()?;
let _ = harness.send_text(b"\x1b[57442;1:3u"); // release
assert!(frame.plain.contains("^R"));
```

This does NOT prove the terminal's emitter works (a real keyboard would still be needed for that), but it does verify the *binary's* handling end-to-end through real terminal rendering. Combined with Level-3 chord tests, it covers the cliclick gap.

#### Choosing the Right Level

For each user-observable requirement, ask: *what is the lowest-fidelity test that could lie about this?*

| Requirement shape | Minimum level |
|---|---|
| Internal state transition | Level 1 |
| Argument parsing / output formatting | Level 1 |
| Terminal-rendered glyph / width / colour | Level 2 |
| `--json` output is valid JSON | Level 1 |
| "When user presses X, the badge appears" | Level 2 (kitty bytes via `wezterm cli send-text`) **or** Level 3 |
| "When user holds modifier, behaviour Y" | Level 2 with kitty bytes — Level 3 cliclick has known macOS limitations for bare-modifier events |
| Hotkey chord triggers binding | Level 1 manufactured bytes + Level 3 chord injection (cliclick chords work reliably) |
| Scrolling / overflow indicators visible | Level 2 |

A feature MAY be marked production-ready only when each user-observable requirement has at minimum the level appropriate for it. A reviewer who finds e.g. "spec requires modifier-press to surface badges, only Level 1 tests exist" must flag it as a high-severity gap, not a minor follow-up.

> **Why this matters in practice.** A real-world incident in this repo: 11 reviews of a feature called the test coverage "substantial" while a modifier-press requirement had only Level-1 PTY tests using manufactured kitty bytes. The bug — `REPORT_ALL_KEYS_AS_ESCAPE_CODES` was never pushed, so WezTerm emitted no bare-modifier events — was structurally invisible to every existing test. A Level-2 test piping the kitty press bytes through `wezterm cli send-text` (or a Level-3 cliclick chord test) would have caught it on first run.

### Error Output

- Use `color_eyre` for CLI error reporting
- Format user-facing errors with styled output (bold "Error:" prefix, deduped cause chain)
- Never show raw backtraces or internal error chains to users by default
