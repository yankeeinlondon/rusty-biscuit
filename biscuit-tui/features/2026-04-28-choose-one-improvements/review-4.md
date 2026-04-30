---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 4)

## Summary

The implementation covers most of the specified behavior and the focused package test suite passes. I found several remaining production blockers around visual capability detection and CLI normalization/source contracts, plus a horizontal-layout edge case that is not currently covered by tests.

## Findings

### 1. Terminal capability detection is implemented but never used by `ChooseOne` / `ChooseMany`

The spec requires Nerd Font indicators when available and active-row foreground/background choices based on detected terminal background. `TerminalStyle::from_env()` exists, but both components render with `TerminalStyle::default()` every time:

- `biscuit-tui/lib/src/components/choose_one.rs:416` and `biscuit-tui/lib/src/components/choose_one.rs:430`
- `biscuit-tui/lib/src/components/choose_many.rs:428` and `biscuit-tui/lib/src/components/choose_many.rs:442`

That means real `ChooseOne` / `ChooseMany` renders never use the Nerd Font glyphs even when `NERD_FONT` or a known `TERM_PROGRAM` is set, and light terminal backgrounds never get the light-background contrast palette. Existing tests cover `ChoiceRenderContext` directly with injected `TerminalStyle`, but not the actual component render path.

Recommendation: add terminal-style state or detection plumbing to the component render path, then add component-level render tests that set `NERD_FONT` / `COLORFGBG` and verify the emitted glyph/style.

### 2. `--file *.toml` is documented as supported but normal TOML option arrays are rejected

The spec and design require `--file` to support TOML arrays. The implementation dispatches `.toml` files to `parse_toml()` at `biscuit-tui/cli/src/option_sources.rs:198`, but `parse_toml()` immediately calls `extract_toml_string_array(&value)` at `biscuit-tui/cli/src/option_sources.rs:249`. A parsed TOML document is normally a table, so a practical file such as:

```toml
options = ["Red", "Green"]
```

is rejected as `NotAnArray`; the current test at `biscuit-tui/cli/src/option_sources.rs:573` pins that rejection. As a result, TOML is listed in docs/help as a supported source format but does not work for a usable TOML file shape.

Recommendation: define and implement a concrete TOML shape, likely a top-level `options` array of strings or inline tables, and add CLI integration coverage for `--file options.toml`.

### 3. Explicit `::` values are still transformed by `--value-convention`

The technical design says delimited values take precedence over convention-generated values: with `--value snake-case`, `"Red Delicious::Apple"` should return `Apple`, not `apple`. The current normalization first splits `::` at `biscuit-tui/cli/src/choice_normalize.rs:334`, but then unconditionally applies conventions to both label and value at `biscuit-tui/cli/src/choice_normalize.rs:289`.

This changes explicitly supplied values and violates the designed escape hatch for cases where naming conventions are not enough.

Recommendation: track whether label/value came from an explicit object field or `::` split and skip convention transforms for explicitly supplied sides. Add tests for the exact `Red Delicious::Apple` + `--value snake-case` example from the design.

### 4. Horizontal hotkey-badge rendering can overrun short viewports

The prior badge-collision issue is mostly addressed by using `row_height = 2` when badges are visible. However, `ChooseOne` and `ChooseMany` still pass raw terminal row count as `visible` (`choose_one.rs:406-411`, `choose_many.rs:418-423`), and `render_horizontal()` treats that as a count of logical option rows (`choice_render.rs:490-492`). When badges are visible, each logical row consumes two terminal rows, so a 3-row area can attempt to draw logical rows at screen y values `0`, `2`, and `4`.

Recommendation: in horizontal mode, compute visible logical rows as `body_rows / row_height` (clamped to at least one where appropriate), use that same value for scroll adjustment and rendering, and add a render-buffer test with badges visible in a short viewport.

## Test Coverage Notes

Ran:

```bash
cargo test -p tui-chrome horizontal_multi_row_badges_do_not_overwrite_next_row_options -- --nocapture
cargo test -p tui-chrome -p tui-chrome-cli
```

Both passed. The remaining issues are contract/coverage gaps: existing tests primarily exercise lower-level helpers or intentionally pin behavior that conflicts with the spec.

## Production Readiness

Not ready for production. The core interaction behavior is in good shape, but the implementation still misses documented visual behavior and has CLI contract mismatches for TOML sources and explicit delimited values.
