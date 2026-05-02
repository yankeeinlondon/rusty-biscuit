# Code Review: Type Safety, DRY, and Test Coverage

**Date**: 2026-04-03
**Scope**: `biscuit-terminal` library + CLI
**Focus**: Type safety improvements, DRY violations, test coverage gaps

---

## Summary

The codebase is well-structured with clear module boundaries and good existing test coverage in key areas. The findings below are ordered by impact: type safety issues that prevent whole classes of bugs, DRY violations that increase maintenance burden, and test gaps that leave error paths unverified.

**Files reviewed**: 48 source files across `lib/src/`, `cli/src/`, `lib/tests/`, and `cli/tests/`.

---

## 1. Type Safety

### 1.1 `width: Option<String>` — Stringly-Typed Width Spec (HIGH)

**Files**: `cli/src/args.rs` (11 occurrences: lines 105, 173, 300, 424, 509, 590, 679, 757, 823, 905, 973)

Every diagram subcommand accepts `width: Option<String>`. The valid values are `"50%"`, `"80ch"`, `"80"`, or `"fill"`. Invalid strings like `"abc"` are accepted by clap and only fail deep inside `parse_width_spec` at render time.

**Recommendation**: Create a `WidthSpec` newtype implementing `FromStr`:

```rust
enum WidthSpec {
    Percent(u8),
    Chars(u32),
    Fill,
}

impl FromStr for WidthSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> { ... }
}
```

This rejects invalid values at argument parse time with a clear error message, and eliminates the 11 identical `/// Display width: ...` doc comments.

### 1.2 Hex Color Strings — No Parse-Time Validation (HIGH)

**Files**: `cli/src/args.rs` (lines 325-337: `q1_fill`, `q2_fill`, `q3_fill`, `q4_fill`), `cli/src/commands.rs` (`PieEntry.color: Option<String>`, `parse_hex_color`)

Color values are bare `String` until render time. Typos like `"#gggggg"` or `"red"` pass clap validation.

**Recommendation**: Create a `HexColor` newtype implementing `FromStr`:

```rust
struct HexColor(u8, u8, u8);

impl FromStr for HexColor {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // validate #rgb, #rrggbb, #rrggbbaa
    }
}
```

### 1.3 `ImageSupportResult.method` and `.reason` — Bare Strings (MEDIUM)

**File**: `lib/src/discovery/detection.rs` (lines 34-42)

```rust
pub struct ImageSupportResult {
    pub support: ImageSupport,
    pub reason: String,
    pub method: String,
}
```

`method` is always one of four known values: `"tty_check"`, `"viuer"`, `"env_heuristic"`, `"known_terminal"`. Tests already assert against these strings (line ~1534).

**Recommendation**:

```rust
enum DetectionMethod {
    TtyCheck,
    Viuer,
    EnvHeuristic,
    KnownTerminal,
}
```

This enables exhaustive pattern matching and prevents typos.

### 1.4 `completions: Option<String>` — Should Be `ValueEnum` (MEDIUM)

**Files**: `cli/src/args.rs` (line 76), `cli/src/main.rs` (line 500)

The `--completions` flag accepts a raw `String`, then `handle_completions` manually lowercases and matches against known shell names. Invalid shells hit `std::process::exit(1)`.

**Recommendation**: Use clap's `ValueEnum` derive on a `Shell` enum. This gives auto-generated help text, tab completion, and parse-time rejection.

### 1.5 Debug Formatting Used for JSON Serialization (MEDIUM)

**File**: `cli/src/output.rs` (lines 8, 53, 54, 68, 85, 96)

Six fields in `TerminalMetadata`, `ConnectionInfo`, `ColorInfo`, etc. use `format!("{:?}", some_enum)` to produce JSON string values. This means:
- Output depends on the `Debug` impl (not guaranteed stable)
- JSON consumers can't reliably match on these values

**Recommendation**: Derive `Serialize` on the source enums (`TerminalApp`, `ColorDepth`, `ImageSupport`, etc.) with `#[serde(rename_all = "kebab-case")]`, or implement `Display` for stable output.

### 1.6 Mixed Integer Types for Widths/Dimensions (MEDIUM)

**Files**: Across library and CLI

| Location | Type | Used For |
|----------|------|----------|
| `detection::terminal_width/height()` | `u32` | Terminal dimensions |
| `eval::line_widths()` | `Vec<u16>` | Line widths |
| `text::content_length()` | `Vec<u32>` | Line byte counts |
| `block_constraint::visible_width()` | `u32` | Visible width |
| `layout::Margin::Chars()` | `u32` | Margin in chars |

**Recommendation**: Standardize on `u16` for visible-width calculations (matches `terminal_size` crate). Use a type alias `type Col = u16` and `type Row = u16` to make intent clear.

### 1.7 `MultiplexSupport` Carries Always-True Booleans (LOW)

**File**: `lib/src/discovery/detection.rs` (lines 900-973)

Every `MultiplexSupport` variant (`Tmux`, `Zellij`, `Native`) returns all-`true` for every capability field. The fields exist in the type system but are never actually detected.

**Recommendation**: Replace with unit variants (`MultiplexSupport::Tmux`) or implement actual version-based detection if capabilities vary.

### 1.8 `aspect_ratio: Option<f32>` — No Positivity Validation (LOW)

**Files**: `cli/src/args.rs` (lines 605, 694)

Zero, negative, NaN, or infinity values are accepted by clap.

**Recommendation**: Create a `PositiveF32` newtype with `FromStr` that validates `> 0.0 && value.is_finite()`.

### 1.9 `extract_color` Returns Unnamed Tuple (LOW)

**File**: `cli/src/commands.rs` (line 702)

```rust
fn extract_color(s: &str) -> (&str, Option<String>)
```

**Recommendation**: Return a named struct or at minimum a type alias.

---

## 2. DRY Violations

### 2.1 Escape Code Stripping — Four Implementations (CRITICAL)

Four locations implement escape code stripping with inconsistent regex patterns:

| Location | Regex Compilation | Patterns Covered | ST Terminator |
|----------|-------------------|-----------------|---------------|
| `discovery/eval.rs` | `LazyLock<Regex>` (once) | CSI + OSC + Fe | Both BEL + ST |
| `utils/escape_codes.rs` | Per-call `Regex::new()` | CSI + OSC + other | BEL only |
| `utils/text.rs` | Per-call `Regex::new()` | CSI + OSC | BEL only |
| `utils/block_constraint.rs` | Per-call `Regex::new()` | CSI + OSC | BEL only |

`eval.rs` is the best implementation: compiled once via `LazyLock`, handles both BEL (`\x07`) and ST (`\x1b\\`) terminators, and uses `UnicodeWidthStr::width()` for proper width calculation.

**Recommendation**:
1. Extract the canonical `ANSI_ESCAPE_RE` from `eval.rs` into `utils/escape_codes.rs`
2. Have `text::content_length()` and `block_constraint::visible_width()` call the shared function
3. Use `LazyLock` everywhere — per-call regex compilation is wasteful

### 2.2 Raw Mode Terminal I/O — Three Near-Identical Implementations (CRITICAL)

| Location | RAII Guard | Mutex | Error Type |
|----------|-----------|-------|------------|
| `discovery/osc_queries.rs` | `RawModeGuard` struct | `TERMINAL_QUERY_MUTEX` | `OscQueryError` enum |
| `discovery/cursor_position.rs` | `RawModeGuard` struct | None | `Result<_, String>` |
| `discovery/fonts.rs` (window_size_pixels) | Inline (no guard) | None | Implicit |

**Recommendation**:
1. Extract a shared `RawModeGuard` into a common module (e.g., `discovery/raw_mode.rs`)
2. Have `cursor_position.rs` and `fonts.rs` use the shared guard and mutex
3. Convert `cursor_position.rs` error type from `String` to `OscQueryError` (or a shared error enum)

### 2.3 NO_COLOR Check + Escape Code Setup — Five Locations (HIGH)

The pattern of checking `NO_COLOR` and declaring `bold`, `dim`, `reset`, `green` strings appears in:

- `cli/src/main.rs` — `print_example_command()` (lines 65-75)
- `cli/src/commands.rs` — `handle_mermaid_error()` (lines 1642-1646)
- `cli/src/commands.rs` — `handle_graph_error()` (lines 1683-1687)
- `cli/src/output.rs` — `print_content_analysis()` (lines 301-305)
- `cli/src/output.rs` — `print_pretty()` (lines 358-366)

**Recommendation**: Extract into a shared struct:

```rust
struct CliStyles {
    bold: &'static str,
    dim: &'static str,
    reset: &'static str,
    green: &'static str,
    // ...
}

impl CliStyles {
    fn detect() -> Self {
        if env::var("NO_COLOR").is_ok() {
            Self::plain()
        } else {
            Self::ansi()
        }
    }
}
```

### 2.4 Vertical Margin Emission — Helper Exists But Unused (HIGH)

**File**: `cli/src/commands.rs`

`emit_vertical_margins()` (line 75) exists but is NOT used by `render_image` (lines 136-142), `display_mermaid` (lines 323-348), or `display_graph` (lines 373-410). These three functions manually emit top/bottom margins inline:

```rust
for _ in 0..layout.margin_top.unwrap_or(0) { println!(); }
```

**Recommendation**: Use the existing helper in all render functions.

### 2.5 RenderMeta Construction — Copy-Pasted Three Times (MEDIUM)

The pattern of measuring elapsed time, getting file metadata, and constructing `RenderMeta` is identical in `render_image`, `display_mermaid`, and `display_graph`.

**Recommendation**: Extract:

```rust
fn build_render_meta(start: Instant, output_path: &Path, format: &str) -> RenderMeta
```

### 2.6 JSON Output Pattern — Nine Render Functions (MEDIUM)

Every render function contains:

```rust
if json {
    let output = serde_json::json!({ ... });
    println!("{}", serde_json::to_string_pretty(&output)?);
    return Ok(());
}
```

**Recommendation**: Extract:

```rust
fn output_json(value: &serde_json::Value) -> color_eyre::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
```

### 2.7 Escape Sequence Unescaping — Four Functions (MEDIUM)

**File**: `cli/src/commands.rs`

`render_prose`, `render_quote`, `render_list`, and `render_columns` all do:

```rust
.replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r")
```

**Recommendation**: Extract `fn unescape_shell_escapes(s: &str) -> String`.

### 2.8 ColorInfo Construction — Three Copies (LOW)

**File**: `cli/src/output.rs` (lines 198-217)

Identical construction logic for `bg_color`, `text_color`, `cursor_color`:

```rust
let bg_color = osc_queries::bg_color().map(|c| ColorInfo {
    r: c.r, g: c.g, b: c.b,
    hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
});
```

**Recommendation**: Add `impl From<RgbColor> for ColorInfo`.

### 2.9 BarChart and LineChart — Nearly Identical Structs (LOW)

**File**: `cli/src/args.rs`

BarChart and LineChart share 11 identical fields. The only difference is BarChart has `line: bool` and LineChart has `bar: bool`.

**Recommendation**: Share a common `XyChartArgs` struct with a discriminant.

### 2.10 Empty File: `utils/truncate.rs` (LOW)

File exists with 0 lines, not declared in `mod.rs`.

**Recommendation**: Delete it, or populate it if truncation utilities are planned.

### 2.11 `word_wrap()` Discards Its Result (LOW)

**File**: `lib/src/utils/word_wrap.rs` (lines 18-22)

```rust
pub fn word_wrap<T: Into<String>>(content: T, strategy: WordWrap, width: u32) {
    let lines = split_lines(content);
    let _ = wrap_lines(lines, &strategy, width);
}
```

**Recommendation**: Return `Vec<String>` or deprecate in favor of `block_constraint::wrap_lines()`.

---

## 3. Error Handling

### 3.1 `std::process::exit(1)` Bypasses Error Reporting (HIGH)

Three locations call `std::process::exit(1)` directly, bypassing `color_eyre` error reporting:

- `cli/src/main.rs:519` — invalid shell in `handle_completions`
- `cli/src/commands.rs:1672` — `handle_mermaid_error`
- `cli/src/commands.rs:1711` — `handle_graph_error`

**Recommendation**: Return `Err(color_eyre::eyre::eyre!(...))` instead. This allows `color_eyre` to display the error with context and backtrace.

### 3.2 Error Chain Lost via String Formatting (MEDIUM)

Throughout `commands.rs`, library errors are converted to strings:

```rust
.map_err(|e| color_eyre::eyre::eyre!("{}", e))
```

This loses the original error chain.

**Recommendation**: Use `.wrap_err("context")` or `eyre!(e)` to preserve the chain.

---

## 4. Test Coverage Gaps

### 4.1 Library: Files With Zero Tests

| File | Lines | Public API Surface |
|------|-------|--------------------|
| `utils/escape_codes.rs` | 76 | 6 public functions |
| `utils/styling.rs` | 345 | `Style`, `FontWeight`, `Stylist` trait |
| `utils/text.rs` | 21 | `content_length()` |
| `utils/word_wrap.rs` | 47 | `word_wrap()`, `truncate()` |
| `discovery/locale.rs` | 243 | Locale detection |
| `discovery/mode_2027.rs` | 154 | Terminal mode detection |

### 4.2 CLI: Subcommands With Zero Integration Tests

| Subcommand | Test Count |
|------------|-----------|
| `image` | 0 |
| `quadrant` | 0 |
| `git-graph` | 0 |
| `pad-left` | 0 |
| `pad-right` | 0 |
| `quote` | 0 |
| `list` | 0 |
| `dir` | 0 |

### 4.3 CLI: Untested Error Paths

- Invalid width values (`"abc"`, `"200%"`, `"-1"`)
- Invalid hex colors
- Invalid XY data (NaN, infinity)
- Invalid timeline event format (missing colon)
- Invalid state diagram transitions
- Missing required files for `image` subcommand
- `elvish` and `powershell` completions (only bash, zsh, fish tested)

### 4.4 CLI: Untested Helper Functions

| Function | File |
|----------|------|
| `format_axis_label` | `commands.rs` |
| `is_dark_mode` | `commands.rs` |
| `emit_vertical_margins` | `commands.rs` |
| `build_mermaid_diagram` | `commands.rs` |
| `print_content_analysis` | `output.rs` |
| `print_pretty` | `output.rs` |
| `font_completer` | `main.rs` |
| `image_completer` | `main.rs` |

---

## 5. Priority-Ordered Recommendations

### Immediate (prevents bugs)

1. **Create `WidthSpec` newtype** (1.1) — Rejects invalid width strings at parse time, removes 11 duplicate doc comments
2. **Consolidate escape code stripping** (2.1) — Single correct implementation, fix OSC ST-terminator bug in 3 locations, eliminate per-call regex compilation
3. **Replace `std::process::exit(1)`** (3.1) — Return proper errors through `color_eyre`

### High Impact (reduces maintenance burden)

4. **Extract shared `RawModeGuard`** (2.2) — Eliminates ~200 lines of duplicated raw mode setup
5. **Extract `CliStyles` helper** (2.3) — Eliminates 5 NO_COLOR code blocks
6. **Use `emit_vertical_margins` consistently** (2.4) — Already exists, just needs to be called
7. **Create `HexColor` newtype** (1.2) — Rejects invalid colors at parse time
8. **Create `DetectionMethod` enum** (1.3) — Replaces bare strings with exhaustive matching
9. **Make `completions` a `ValueEnum`** (1.4) — Proper clap integration

### Medium Impact (improves code quality)

10. **Extract `build_render_meta` helper** (2.5)
11. **Extract `output_json` helper** (2.6)
12. **Extract `unescape_shell_escapes` helper** (2.7)
13. **Preserve error chains** (3.2) — Use `.wrap_err()` instead of string formatting
14. **Derive `Serialize` on source enums** (1.5) — Replace `Debug`-formatting in JSON output
15. **Add tests for `escape_codes.rs` and `styling.rs`** (4.1)
16. **Add integration tests for untested subcommands** (4.2) — Priority: `image`, `quadrant`, `quote`, `list`
