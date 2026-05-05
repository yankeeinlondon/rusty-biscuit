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

- macOS: [cliclick](./cliclick.md) (e.g. `cliclick kd:ctrl t:r ku:ctrl` for a chord; `kd:ctrl,sleep,ku:ctrl` for a hold)
- Linux: `xdotool`

This is the **only** level that can verify "what bytes does the terminal actually emit when key X is pressed?" — required for any spec line of the form "*when the user holds/presses key X, Y happens*."

**Caveats:**
- macOS focus is a shared global resource; gate Level 3 behind `RUN_LEVEL3=1` so non-deterministic focus thrash doesn't break `cargo test`.
- Before injection, focus the spawned pane explicitly (`wezterm cli activate-pane --pane-id N`) AND raise the app (`osascript -e 'tell application "WezTerm" to activate'`).
- Provide a parallel chord-injection test alongside the modifier-hold test — the chord variant proves the cliclick→terminal→binary chain works end-to-end, isolating "did the press arrive" from "did the terminal encode it correctly."
- **Capture during the hold, not after.** Splitting `kd:` and `ku:` with the capture in between is critical when the binary clears its display state on modifier release. A `hold_modifier(800ms)` helper that internally sequences press → sleep → release leaves nothing observable by the time `capture()` runs.
- **`--test-threads=1` is mandatory** for a Level-3 suite where each test spawns its own GUI window. Parallel runs guarantee one spawned window steals focus from the other before cliclick finishes injecting.
- **Add a "plain key" diagnostic test** alongside the modifier ones (e.g., `kp:arrow-down` + assert active marker moved). When a Level-3 test fails, run the diagnostic first — if it passes, the harness is sound and the failure is in chord/modifier-specific paths; if it fails, focus delivery is broken upstream.
- **Use `cliclick -m verbose`** in test harness wrappers and pipe stdout+stderr to test stderr. Verbose mode prints what cliclick is dispatching and surfaces Accessibility-permission warnings; without it, silent failures look identical to focus failures.
- **Test setup gotcha — option hotkey assignment.** A spec line like *"Ctrl+R selects the Red option"* requires the test to spawn the binary with explicit hotkey prefixes (`[CTRL+r] Red` not `Red`). If the binary's spec says plain options have no hotkey, sending `Ctrl+R` against `Red` does nothing — the chord arrives but the binary has nothing bound to it. Failures present as "click didn't transfer focus" diagnostics that send you down the wrong rabbit hole.

##### Multi-window WezTerm targeting on macOS

When the developer has many WezTerm windows already open (a common reality, not an edge case), naive activation breaks:

- **`tell application "WezTerm" to activate` is window-ambiguous** when the app already owns multiple windows. macOS resolves "front window of activated app" to whichever NSWindow was most recently `keyWindow`, which is rarely the test's freshly spawned window.
- **`wezterm cli activate-pane --pane-id N` only handles intra-WezTerm pane focus.** It does not move OS-level keyWindow between sibling WezTerm windows.
- **`set frontmost of <process> to true` via System Events is unreliable across applications** on modern macOS — modifying another app's frontmost state typically requires Apple Events permission, and even when granted the activation can lose to whichever app the user typed in most recently.

**The reliable pattern is title-stamped AXRaise + click:**

1. Stamp a unique title on the spawned window via `wezterm cli set-tab-title --pane-id N <unique>`. **Use `set-tab-title`, not `set-window-title`** — most users' `wezterm.lua` defines a `format-window-title` event that derives the OS-level NSWindow title from the active tab's title and silently overrides direct `set-window-title` calls.
2. Resolve the WezTerm process by name pattern (`processes whose name contains "wezterm"` — case-insensitive substring match catches `WezTerm`, `wezterm-gui`, etc.).
3. AppleScript / System Events: find the window whose title matches the stamp and call `perform action "AXRaise"`. Wrap the whole script in `with timeout of 5 seconds` so a wedged AX query fails fast.
4. Get window position+size from System Events (`position of targetWin`, `size of targetWin`).
5. **Click into the window via cliclick (`c:X,Y`).** AXRaise alone often does NOT transfer keyWindow when the test runner lives in a different app — only a real OS click reliably forces keyWindow assignment across applications.
6. Combine the click with the actual key injection in **one batched cliclick invocation** (`cliclick -w 100 c:X,Y kd:ctrl`). Splitting into separate processes leaves a focus-drift window wide enough on a busy desktop for events to route to the wrong window.

##### Restoring focus after a test spawns a GUI window (macOS)

A common harness pattern: snapshot the frontmost app *before* spawning the test window, then re-activate that app afterwards so the developer keeps working without animations or accidental keystrokes into the test window.

The naive implementation is a trap:

```applescript
tell application "System Events" to set prevApp to name of first process whose frontmost is true
-- ... spawn test window ...
tell application prevApp to activate    -- ⚠ pops "Choose Application" dialog
```

`tell application "<name>"` resolves `<name>` through LaunchServices **by `.app` bundle name**. But `name of process` from System Events returns the **executable / process name**, which diverges from the bundle name whenever `CFBundleExecutable != CFBundleName`. When the strings differ, LaunchServices cannot find a matching bundle and pops a *modal* "Choose Application — Where is X?" dialog that blocks the test until a human dismisses it. In CI it deadlocks, on a developer machine it interrupts every run.

Apps that hit this in practice:

| App | Process name | Bundle name |
|-----|--------------|-------------|
| WezTerm | `wezterm-gui` | `WezTerm` |
| VS Code | `Code Helper` (or `Electron`) | `Visual Studio Code` |
| Slack, Discord, Notion, Obsidian, … | `<App> Helper` / `Electron` | `<App>` |

**Fix: re-activate via System Events.** It addresses live processes by process name and never goes through LaunchServices:

```applescript
if prevApp is not "" and prevApp is not "Terminal" then
    try
        tell application "System Events" to set frontmost of (first process whose name is prevApp) to true
    end try
end if
```

Symptom-driven debug recipe: if a test suddenly opens a "Choose Application — Where is X?" dialog, grep the harness for `tell application` calls whose target string came from `name of … process …`. The fix is almost always to switch to the `System Events`-mediated form above.

##### Parent-app vs spawned-app

If the cargo test runs in WezTerm and spawns more WezTerm windows, the parent and child windows belong to the same NSApplication and compete for keyWindow. AXRaise + activate is window-ambiguous in that scenario.

**Fix: relaunch the cargo run inside a different terminal application** (iTerm2 if installed, else Terminal.app) via osascript before invoking cargo test. The justfile recipe should detect a WezTerm parent (`$TERM_PROGRAM == "WezTerm"`) and relaunch automatically.

##### Permissions chain on macOS (TCC)

Three distinct permissions are involved; granting one does not grant the others:

| Permission | Granted to | Required for |
|---|---|---|
| **Accessibility** | The terminal app hosting cargo test (e.g. iTerm.app) | All cliclick events; System Events AX queries; keyboard injection |
| **Apple Events / Automation** | Same | `tell application "WezTerm" to activate` and any cross-app `tell` block |
| **Input Monitoring** | Usually not required for our case | Some lower-level CGEvent taps |

Symptom of missing Accessibility: `cliclick` prints `WARNING: Accessibility privileges not enabled. Many actions may fail.` to stderr (visible only with `-m verbose`).

Symptom of missing Apple Events: `tell application "WezTerm" to activate` silently no-ops; macOS may prompt on first use but if dismissed, subsequent attempts fail without re-prompting until you reset via `tccutil reset AppleEvents`.

##### Diagnostic recipes (paste-friendly)

Keyboard delivery sanity check:

```rust
// After focus_spawned_pane, send a plain key and verify the binary saw it.
cliclick("c:X,Y", "kp:arrow-down");
let frame = harness.capture()?;
let active_moved = frame.plain.lines().any(|l| l.contains("▶") && l.contains("Green"));
assert!(active_moved, "plain key delivery broken — focus issue, not chord/modifier issue");
```

Frontmost app + focused window probe:

```rust
let probe = std::process::Command::new("osascript")
    .args(["-e", r#"tell application "System Events"
        set frontApp to name of first application process whose frontmost is true
        set focusedTitle to "<no AXFocusedWindow>"
        try
            tell first process whose name contains "wezterm"
                set focusedTitle to title of (value of attribute "AXFocusedWindow")
            end tell
        end try
        return frontApp & " | focused: " & focusedTitle
    end tell"#])
    .output()?;
eprintln!("[probe] {}", String::from_utf8_lossy(&probe.stdout).trim());
```

**Differentiate `AXFocusedWindow` vs `AXMain`.** AXFocusedWindow is the actual keyWindow (where keyboard events go); AXMain is for app-level menu actions. They can differ. Always probe AXFocusedWindow when debugging key-routing issues.

**Multi-monitor coordinates aren't off-screen by default.** A 2x 5K display setup is 5120 pixels wide; window positions up to ~4500 are valid. Don't assume large x-coordinates are bugs.

##### Known limitation: cliclick + bare modifier keys on macOS

cliclick uses `CGEventCreateKeyboardEvent`, but macOS routes bare-modifier key state through `flagsChanged` events at the AppKit layer. cliclick's synthetic modifier events do not always reach apps via that path:

- **Chord injection works** (`kd:ctrl t:r ku:ctrl`): the modifier flag rides along with the letter `keyDown`, which IS a normal CGEvent that AppKit delivers correctly. A Level-3 chord test like "Ctrl+R submits the option bound to Ctrl+R" passes reliably.
- **Bare-modifier injection is unreliable**: `kd:ctrl` alone often does not produce a `flagsChanged` event that WezTerm sees. The press is dropped before the terminal's input encoder ever fires. This is true even when the binary's bare-modifier handling is correct.
- **AppleScript System Events `key down control` shares the limitation.** It uses the same CGEvent dispatch path under the hood — the AppleScript layer is just a thin wrapper. Don't reach for it expecting different behaviour.
- **The only known fix requires a custom Rust binary** built on the `core_graphics` crate that constructs a CGEvent with `CGEventType::FlagsChanged` and the relevant flag bit set, then posts it via `kCGHIDEventTap`. Until that exists, bare-modifier Level-3 verification on macOS is structurally blocked. Mark such tests `#[ignore]` with a comment block pointing at the canonical Level-2 raw-bytes test.

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
