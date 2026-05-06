# Phase 1 Baseline: Terminal Escape Code Bleed Diagnosis

## Date
2026-05-05

## Environment
- Repository: rusty-biscuit monorepo
- Feature: `2026-05-02-flattened-bridge`
- Phase: 1 of 5 (Diagnosis & Baseline)

## Problem Statement

In non-interactive sessions where a prompt is provided via stdin/args but stdout
remains connected to a TTY/PTY, terminal OSC 11 color query responses bleed into
output as literal characters. The observed pattern is:

```
^[]11;rgb:1a1a/1b1b/2626^[\
```

This appears before each tool call icon and persists once it starts.

## Root Cause Analysis

### Primary Cause

`biscuit_terminal::discovery::detection::color_mode()` sends live OSC 11 queries
to stdout on **every invocation**. There is no caching at any level:

1. `Terminal` struct does not cache `color_mode`
2. `color_mode()` free function queries every time
3. `bg_color()` free function queries every time
4. `query_osc_actual()` sends the OSC sequence every time

### Call Chain (Verified)

```
Status::to_terminal()
  -> Terminal::color_mode()          [biscuit-terminal/lib/src/terminal.rs:500]
    -> color_mode()                  [biscuit-terminal/lib/src/discovery/detection/color.rs:121]
      -> bg_color()                  [biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:87]
        -> query_osc_actual(11, ...) [biscuit-terminal/lib/src/discovery/osc_queries/query.rs:206]
          -> writes "\x1b]11;?\x07" to stdout
```

### Hot Paths Calling Terminal::color_mode()

The static method `Terminal::color_mode()` is called from these locations:

| # | File | Line | Context |
|---|------|------|---------|
| 1 | `biscuit-terminal/lib/src/components/status.rs` | 509 | `Status::to_terminal()` - every status render |
| 2 | `biscuit-terminal/lib/src/components/table/table.rs` | 1321 | `Table::render_optimistic()` - alternate bg color |
| 3 | `biscuit-terminal/lib/src/components/table/table.rs` | 1326 | `Table::render_optimistic()` - alternate text color |
| 4 | `biscuit-terminal/lib/src/components/table/table.rs` | 1344 | `Table::render()` - alternate bg color |
| 5 | `biscuit-terminal/lib/src/components/table/table.rs` | 1349 | `Table::render()` - alternate text color |
| 6 | `biscuit-terminal/lib/src/components/mermaid.rs` | 142 | `MermaidRenderer::for_terminal()` - theme selection |
| 7 | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` | 344 | `HorizontalRule::render_image_tier()` - SVG color |
| 8 | `biscuit-terminal/lib/src/components/graph_expression.rs` | 284 | `GraphExpression::for_terminal_mode()` - theme selection |

### Claudine-Specific Paths

In the claudine CLI wrapper:

```
wrap_terminal() [claudine/cli/src/commands/wrap/mod.rs:190]
  -> crate::log::terminal() [claudine/cli/src/log.rs:50]
    -> Terminal::new()       [biscuit-terminal/lib/src/terminal.rs:35]
      -> new_terminal()       [does NOT cache color_mode]
```

Note: `Terminal::new()` does not currently call `color_mode()`, but every
component render that follows does.

### darkmatter Path

```
TerminalOptions::default() [darkmatter/lib/src/markdown/output/terminal.rs:756]
  -> detect_color_mode()    [darkmatter/lib/src/markdown/highlighting/themes.rs:409]
```

Note: `darkmatter`'s `detect_color_mode()` does **not** call OSC queries. It
checks `NO_COLOR` and `COLORFGBG` environment variables, then defaults to dark.
However, it is called on every `TerminalOptions` instantiation, which happens
frequently during markdown rendering.

## Code Inspection Details

### query_osc_actual() [biscuit-terminal/lib/src/discovery/osc_queries/query.rs:206]

```rust
pub fn query_osc_actual(code: u8, timeout: Duration) -> Result<RgbValue, OscQueryError> {
    // Pre-flight checks (returns early in non-TTY, CI, or multiplexer)
    if !is_tty() { return Err(OscQueryError::NotTty); }
    if is_ci() { return Err(OscQueryError::CiEnvironment); }
    if let Some(mux) = detect_multiplexer() {
        return Err(OscQueryError::Multiplexer(mux.to_string()));
    }

    // Acquire mutex and raw mode guard
    let _lock = TERMINAL_QUERY_MUTEX.lock()?;
    let _guard = RawModeGuard::stdin()?;

    // SEND OSC QUERY TO STDOUT
    let query = format!("\x1b]{};?\x07", code);
    let mut stdout = std::io::stdout();
    stdout.write_all(query.as_bytes())?;
    stdout.flush()?;

    // Read response from stdin
    // ...
}
```

The query sequence `\x1b]11;?\x07` is:
- `\x1b]` = OSC introducer
- `11` = background color query
- `;?` = request current value
- `\x07` = BEL (string terminator)

### color_mode() [biscuit-terminal/lib/src/discovery/detection/color.rs:121]

```rust
pub fn color_mode() -> ColorMode {
    // Try to get background color and determine from luminance
    if let Some(bg) = crate::discovery::osc_queries::bg_color() {
        let luminance = bg.luminance();
        if luminance > 0.5 {
            return ColorMode::Light;
        } else {
            return ColorMode::Dark;
        }
    }
    // ... fallback to env vars and defaults
}
```

### Terminal::color_mode() [biscuit-terminal/lib/src/terminal.rs:500]

```rust
pub fn color_mode() -> ColorMode {
    color_mode()  // Calls free function every time
}
```

This is a **static method** that does not use any cached instance data.

### Terminal::new() [biscuit-terminal/lib/src/terminal.rs:35]

```rust
fn new_terminal() -> Terminal {
    Terminal {
        // ... many fields initialized via detection functions
        // color_mode is NOT cached here
    }
}
```

The `Terminal` struct [line 166] does not have a `color_mode` field.

## Why the Bleed Happens

In a non-interactive claudine session:

1. User runs: `echo "prompt text" | claudine codex --some-flag`
2. `stdin` is the pipe from `echo` (not the terminal)
3. `stdout` is still the terminal TTY
4. claudine renders live output using `Status` components
5. Each `Status::to_terminal()` calls `Terminal::color_mode()`
6. `color_mode()` calls `bg_color()` -> `query_osc_actual(11, ...)`
7. `query_osc_actual` sends `\x1b]11;?\x07` to **stdout** (the TTY)
8. The terminal emulator receives the query and responds with the background color
9. The response goes to the terminal's input buffer
10. But `stdin` is the pipe, not the terminal, so the response is not consumed
11. The response may be echoed to stdout or mixed with program output
12. Result: literal `^[]11;rgb:...^[` sequences appear in the rendered output

## Baseline Metrics

### Query Frequency

Without caching, the number of OSC 11 queries per session equals the number of
times any component calls `Terminal::color_mode()`. In a typical claudine
session with live output:

- Each tool call renders a `Status` line: ~1 query per tool call
- Each thinking block render: ~1 query
- Each progress update: ~1 query
- **Estimated: 10-50+ queries per minute of active session**

### Affected Components

All components that call `Terminal::color_mode()` are affected:
- `Status` (most frequent in claudine wrapper output)
- `Table` (when alternate row colors enabled)
- `MermaidRenderer` (when rendering diagrams)
- `HorizontalRule` (when rendering image tiers)
- `GraphExpression` (when rendering graphs)

### Environment Factors

The bleed is most visible when:
- **Terminal emulator**: Any that responds to OSC 11 (iTerm2, WezTerm, Kitty, Alacritty, Ghostty)
- **Session type**: Non-interactive (piped stdin) with TTY stdout
- **OS**: macOS and Linux (Windows uses different path)
- **Multiplexers**: `query_osc_actual` returns early for tmux/screen, so no bleed there

## Validation Checkpoint

- [x] Call paths traced and documented
- [x] Reproduction script created (`reproduce.sh`)
- [x] Baseline behavior documented in this file
- [x] All affected files identified
- [x] Query mechanism understood (OSC 11 via stdout)

## Next Phase

Phase 2 will implement the core fix in biscuit-terminal:
1. Add `color_mode` field to `Terminal` struct
2. Cache `color_mode` in `Terminal::new()`
3. Add `color_mode` to `TerminalBuilder`
4. Change `Terminal::color_mode()` from static to instance method
5. Fix all call sites to use instance method
6. Add process-level `OnceLock` cache for `bg_color()`
