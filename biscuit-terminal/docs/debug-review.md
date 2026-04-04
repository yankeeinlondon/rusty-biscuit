# biscuit-terminal Tracing & Debug Review

**Date:** 2026-04-03
**Scope:** `biscuit-terminal/lib` and `biscuit-terminal/cli`
**Focus:** Tracing instrumentation coverage, level appropriateness, and best practices

---

## Executive Summary

The library has solid tracing in the **discovery** layer (detection, fonts, OSC queries) but almost none in the **components** or **CLI** layers. The CLI has zero `tracing::*` calls — it relies entirely on a custom `--debug` flag with manual `eprintln!` for the `image` subcommand only. The two diagram renderers (`mermaid`, `graph_expression`) have `#[instrument]` spans and `info!` calls, but all other components (prose, table, compose, list, section, terminal_image, two_column, etc.) are completely uninstrumented.

### Trace count by area

| Area | Files with tracing | Files without | Total traces |
|------|--------------------|---------------|--------------|
| `discovery/` | 3 of 8 | 5 | ~61 |
| `components/` | 2 of 18 | 16 | 4 |
| `utils/` | 0 of 8 | 8 | 0 |
| `cli/src/` | 0 of 4 | 4 | 0 |
| `terminal.rs` | 0 | 1 | 0 |

---

## Findings

### 1. CLI has no tracing at all

**File:** `cli/src/main.rs`, `cli/src/commands.rs`, `cli/src/args.rs`, `cli/src/output.rs`

The CLI sets up `tracing_subscriber` correctly when `RUST_LOG` is set (line 91-95 in `main.rs`), but then never emits a single trace event. All diagnostic output is via a bespoke `--debug` flag using `eprintln!` in `render_image()`.

**Suggestions:**

- Add `debug!` at subcommand dispatch: log which command was selected and key arguments (width, layout flags, inverse mode). This helps diagnose "why does my output look wrong?" issues.
- Replace the `--debug` block in `render_image()` (commands.rs:166-229) with structured `debug!`/`trace!` calls that emit the same data. The `--debug` flag can remain as a user-facing shortcut that sets `RUST_LOG=biscuit_terminal_cli=debug`.
- Add `warn!` when terminal detection falls back (e.g., no image support, no cell size).

### 2. `terminal_image.rs` — the most complex component — has zero tracing

**File:** `lib/src/components/terminal_image.rs`

This module handles image loading, SVG rasterization, protocol selection (Kitty vs iTerm2), dimension resolution, cursor math, and scroll compensation. It is the single hardest thing to debug in the package, yet has no instrumentation.

**Suggestions:**

- `render_to_terminal()` (~line 709): Add `#[instrument(skip(self, term), fields(file = %self.relative, protocol))]` and record the chosen protocol, resolved dimensions, cursor_rows, and scroll compensation decision.
- `render_kitty_for_terminal()` / `render_iterm2_for_terminal()`: Add `trace!` for cell size source (detected vs fallback 8x16), raw_height, height_cells, x_offset.
- `load_image()` / `load_svg()`: Add `debug!` with file path, format, and image dimensions after load.
- Error paths: Already covered by `thiserror` display, but add `warn!` on fallback to empty string in `render()` (line 269) — silently swallowing errors makes debugging hard.

### 3. Level misuse in `detection.rs` — `info!` for routine detection

**File:** `lib/src/discovery/detection.rs:255-295`

`color_depth()` uses `tracing::info!` for every code path — including the happy path where `COLORTERM=truecolor`. Detection runs on every `Terminal::new()`, which is called frequently. These are routine, expected results.

**Suggestion:** Downgrade all detection results to `debug!`. Reserve `info!` for surprising or noteworthy conditions (e.g., terminfo query failure at line 290 could stay `info!` or become `warn!`).

### 4. `fonts.rs` — good coverage but all `debug!`, no `trace!` for noisy paths

**File:** `lib/src/discovery/fonts.rs`

This file has 34 trace points, all at `debug!` level. Some are fine at debug (e.g., "detected font name X"), but the repeated "skipping because not TTY" and config-file-scan loops produce a lot of noise at `debug!` level.

**Suggestions:**

- Downgrade early-exit guards ("not a TTY", "CI environment", "config file does not exist") to `trace!`.
- Keep actual detection results ("detected font X", "detected Nerd Font") at `debug!`.
- The fallback scan functions (`fallback_font_name_scan`, `fallback_font_size_scan`) iterate multiple config files — their per-file messages should be `trace!`, with the final result at `debug!`.

### 5. `osc_queries.rs` — good pattern, but missing error context

**File:** `lib/src/discovery/osc_queries.rs`

This module consistently uses `debug!` for skip reasons and parsed results. Good pattern. However, the timeout and parse-failure paths (lines ~294-338) log the raw response bytes but not the query type or what was expected.

**Suggestion:** Add the query context (e.g., "OSC11 background color query") to timeout/parse-failure messages so that a log line like `"query timed out"` is self-contained without surrounding context.

### 6. `cursor_position.rs` — uses string errors instead of tracing

**File:** `lib/src/discovery/cursor_position.rs`

`query_cursor_position()` returns `Result<_, String>` and uses string error messages. The error messages are descriptive but invisible unless the caller logs them. The callers (`terminal_image.rs`, `commands.rs`) discard the `Err` via `.ok()`.

**Suggestions:**

- Add `trace!` before returning each error — "DSR query failed: not a tty", "DSR query failed: CI environment", etc.
- In `terminal_image.rs` `render_to_terminal()` (~line 760), log at `trace!` when `cursor_position()` returns `None`, noting that scroll compensation is disabled as a consequence.

### 7. `clipboard.rs`, `mode_2027.rs`, `config_paths.rs`, `locale.rs` — no tracing

These discovery modules perform heuristic detection but have no instrumentation. They are called during `Terminal::new()`.

**Suggestion:** Add a single `debug!` per module for the final detection result:
- `clipboard.rs`: `debug!(supported = %result, "OSC52 clipboard support")`
- `mode_2027.rs`: `debug!(supported = %result, "Mode 2027 grapheme support")`
- `config_paths.rs`: `debug!(path = ?config_path, "Terminal config file path")`
- `locale.rs`: `debug!(raw = ?raw, tag = ?tag, "Detected locale")`

### 8. `Terminal::new()` — no aggregate trace

**File:** `lib/src/terminal.rs:34-67`

`new_terminal()` calls ~15 detection functions but produces no summary trace. After all detection completes, there is no way to see the aggregate result without enabling debug on every sub-module.

**Suggestion:** Add a single `info!` span or event after construction:
```rust
tracing::debug!(
    app = ?terminal.app,
    image_support = ?terminal.image_support,
    color_depth = ?terminal.color_depth,
    is_tty = terminal.is_tty,
    width = terminal.width(),
    "Terminal detected"
);
```

### 9. Components layer — entirely dark

**Files:** `prose.rs`, `table/`, `compose.rs`, `list.rs`, `section.rs`, `text_block.rs`, `two_column.rs`, `pad.rs`, `inline_content.rs`, `filesystem.rs`, `status.rs`, `block_quote.rs`, `todo.rs`

None of these have any tracing. Most are pure transform functions (text in, styled text out) where tracing would be noise. However:

**Suggestions (targeted, not blanket):**

- `compose.rs`: Add `trace!` with part count when rendering — helps debug "why is my output empty?" when a compose has zero parts.
- `filesystem.rs`: Add `debug!` on the root path and depth — this reads the filesystem and is a common source of "where is it looking?" questions.
- `two_column.rs`: Add `trace!` with resolved column widths — layout bugs are common with percentage-based widths.

### 10. `#[instrument]` usage is minimal

Only two functions in the entire package use `#[instrument]`:
- `mermaid.rs:301` — `render_to_cached_png()`
- `graph_expression.rs:222` — `render_to_cached_png()`

Both skip `self` appropriately.

**Suggestion:** Add `#[instrument]` to these high-value entry points:
- `Terminal::new()` — span for the entire detection phase
- `TerminalImage::render_to_terminal()` — span for image render pipeline
- `MermaidDiagram::try_render()` — span for the full mermaid→image→terminal pipeline
- CLI `render_image()`, `render_flowchart()`, etc. — span per subcommand execution

---

## Recommended Level Guide

For consistency across the package:

| Level | Use for |
|-------|---------|
| `error!` | Should not appear in library code (return `Result` instead). CLI may use for user-facing errors sent to stderr. |
| `warn!` | Unexpected conditions that are recovered from: fallback to default cell size, image support detected as None on a known-capable terminal, silent error swallowing. |
| `info!` | Aggregate detection summary (`Terminal::new()` result), cache hits/misses for diagram rendering. Rare — one or two per major operation. |
| `debug!` | Detection results (which terminal, color depth, font), resolved dimensions, protocol selection, CLI subcommand dispatch. |
| `trace!` | Guard/skip reasons ("not a TTY", "CI environment"), per-file config scans, raw escape sequences, cursor position queries, intermediate layout calculations. |

---

## Priority Actions

1. **Instrument `terminal_image.rs`** — highest debugging value per line of code added
2. **Add CLI subcommand dispatch tracing** — currently invisible which path was taken
3. **Downgrade `detection.rs` `info!` to `debug!`** — noisy at current level
4. **Add `Terminal::new()` summary debug event** — single line, huge diagnostic value
5. **Downgrade `fonts.rs` guard messages to `trace!`** — reduce noise at debug level
6. **Add result-level `debug!` to uninstrumented discovery modules** — clipboard, mode_2027, locale, config_paths
