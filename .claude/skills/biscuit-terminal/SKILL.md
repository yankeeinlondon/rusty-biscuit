---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library - the authority for terminal capability detection (13+ emulators) and rich terminal rendering. Provides inline image rendering (Kitty/iTerm2 protocols), terminal-facing Mermaid and graph adapters backed by biscuit-visualized, OS/font detection, escape code analysis, color system (BasicColor, WebColor, Tailwind), and composable rendering components. Use when building CLI apps with terminal-aware features, rendering images or diagrams inline, detecting color/underline/italics/dim support, or querying terminal environment. Darkmatter depends on this for terminal Mermaid rendering.
---

# biscuit-terminal

Terminal detection and rich rendering library for Rust. The authority for terminal-aware features in the dockhand monorepo.

## Core Principles

1. **Detection before rendering**: Always check terminal capabilities first
2. **Graceful fallback**: Use `fallback_render()` and explicit app-level text fallback where needed
3. **Static vs dynamic**: `Terminal` struct fields for static properties, methods for dynamic
4. **Input validation + policy**: `TerminalImage::new()` validates local paths; apply app-level policies with `TerminalImageOptions`

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::ImageSupport;

let term = Terminal::new();

// Static properties (detected once)
println!("Terminal: {:?}, Image: {:?}", term.app, term.image_support);

// Dynamic queries (width/height are instance methods, color_mode is static)
let (width, mode) = (term.width(), Terminal::color_mode());

// Conditional rendering
if term.supports_italic { println!("\x1b[3mItalic\x1b[0m"); }
```

## Topics

| Topic | Description |
|-------|-------------|
| [Terminal Struct](./terminal-struct.md) | Main struct, static vs dynamic properties, enums |
| [Components](./components.md) | All renderable components: BlockQuote, Compose, FileSystem, GraphExpression, HorizontalRule, InlineContent, MermaidDiagram, OrderedList, UnorderedList, PadLeft, PadRight, Progress, Prose, Section, Status, StatusBlock, Table, TerminalImage, TextBlock, Todo, TwoColumn |
| [Image Rendering](./image-rendering.md) | Kitty/iTerm2 protocols, width parsing, cursor behavior, policy controls |
| [Mermaid Diagrams](./mermaid-diagrams.md) | Terminal-facing `MermaidDiagram` adapter backed by biscuit-visualized |
| [Color System](./color-system.md) | BasicColor, RgbColor, WebColor, Tailwind, HdrColor with TermColor trait |
| [Detection Functions](./discovery.md) | App, color, underline, italics, dim, multiplex detection |
| [OS & Environment](./os-environment.md) | OS, distro, CI, fonts, locale |
| [Escape Codes](./escape-codes.md) | Strip, analyze, visual width calculation |
| [Styling](./styling.md) | Terminal-aware styling, Prose component, TextBlock |
| [bt Command](./cli.md) | CLI tool: 17 commands for inspection, diagrams, text, and filesystem |

## Common Patterns

### Conditional Image Rendering

```rust
match term.image_support {
    ImageSupport::Kitty | ImageSupport::ITerm => {
        TerminalImage::new(path)?.render_to_terminal(&term)?;
    }
    ImageSupport::None => println!("[Image: {}]", path.display()),
}
```

### Mermaid Diagram

```rust
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::terminal::Terminal;

let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
let term = Terminal::new();

if let Err(err) = diagram.try_render(&term) {
    eprintln!("Diagram render failed: {err}");
    // Optional app-level fallback if you want textual output:
    println!("{}", diagram.fallback_code_block());
}
```

### Graph Diagram

```rust
use biscuit_terminal::components::graph_expression::{
    GraphExpression, GraphInputSyntax, GraphOrientation,
};

let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph");
```

### Light/Dark Adaptation

```rust
let fg = match Terminal::color_mode() {
    ColorMode::Light => "black",
    ColorMode::Dark | ColorMode::Unknown => "white",
};
```

### Status Blocks

```rust
use biscuit_terminal::prelude::{Prose, StatusBlock, StatusState};

let block = StatusBlock::new(StatusState::Error)
    .header("<b>Shell Expansion Failed</b>")
    .body(Prose::new("Missing closing brace in `${...}` directive."))
    .hint("Check the template syntax and retry.");
```

Use `StatusBlock` when you need the common Claudine-style `Status` header plus a colored
`BlockQuote` body and optional hint as one renderable. It defaults to a `┃ ` border,
`left_margin = 0`, `right_margin = 5`, and `WordWrap::WrapProse(Some(8), None)` so the
body border lines up with the preceding `Status` icon/header line.

### Horizontal Rules

```rust
use biscuit_terminal::prelude::*;  // HorizontalRule, RuleStyle, RuleAlignment, RuleWeight, BrowserRenderable

let rule = HorizontalRule::new()
    .style(RuleStyle::Waves)
    .alignment(RuleAlignment::Centered)
    .weight(RuleWeight::Medium)
    .width("75%");

// Terminal rendering — honors color_depth and width from the passed `Terminal`
let output = rule.render(&terminal);

// Browser rendering — default SVG declares --hr-weight / --hr-color / --hr-width
let svg = rule.render_to_browser();

// Browser rendering with per-instance CSS variable overrides
use std::collections::HashMap;
let mut overrides = HashMap::new();
overrides.insert("hr-weight".to_string(), "12".to_string());
let svg_override = rule.render_to_browser_with_inline_variables(&overrides);
```

The `HorizontalRule` component implements both `Renderable` (terminal output) and `BrowserRenderable` (HTML/SVG output).

**Supported attributes:**

- `style`: `dashes` (default), `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`
- `alignment`: `full` (default), `centered`, `left`, `right`
- `weight`: `thin`, `medium` (default), `thick` — heavy Unicode glyphs in Tier 2, 2/4/8px stroke in browser
- `width`: CSS-like string (e.g. `"75%"`, `"200px"`)
- `color`: CSS color name or `#rrggbb` — emits ANSI escapes in terminal when `color_depth` supports it

**Terminal rendering tiers:**

1. **Tier 1 (SVG → PNG via `resvg` + `TerminalImage`):** primary path when the terminal advertises Kitty-compatible image support.
2. **Tier 2 (Unicode):** fallback; gated on `locale::env_says_utf8()`.
3. **Tier 3 (ASCII):** fallback when the locale does not signal UTF-8.

All four main types (`HorizontalRule`, `RuleStyle`, `RuleAlignment`, `RuleWeight`) plus the `BrowserRenderable` trait are re-exported through `biscuit_terminal::prelude`.

`StatusState::Error` is now the canonical error severity. `StatusState::Failure` remains as a
deprecated compatibility variant, and persisted JSON `"Failure"` still deserializes as
`StatusState::Error`. Prefer `StatusState::default_color()` when you want the canonical border
or accent color for a severity instead of re-encoding the Tailwind mapping yourself. See
[`biscuit-terminal/README.md`](../../../biscuit-terminal/README.md) for the full severity table
and override knobs.

## Terminal Support Matrix

| Terminal | Image | OSC8 | Italics |
|----------|-------|------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | ITerm | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| Warp | Kitty | Yes | Yes |
| Wast | Kitty | No | No |
| Alacritty | - | Yes | Yes |
| Apple Terminal | - | No | Yes |
| GNOME Terminal | - | Yes | Yes |
| Foot | - | Yes | Yes |
| Contour | - | Yes | Yes |
| VS Code | Kitty* | Yes | Yes |

\* Requires `terminal.integrated.enableImages` and GPU acceleration enabled in VS Code settings.

## bt CLI Commands (17 commands)

```bash
# Terminal inspection
bt                              # Pretty-printed capabilities
bt --json                       # JSON output for scripting

# Styled text and layout
bt prose "Hello {{bold}}world{{reset}}!"
bt prose "<red>Error</red>: message"
bt quote --attribution "Shakespeare" "To be or not to be"
bt list "First item" "Second item" "Third item"
bt columns --gap 6 --left 40% "Title" "Description"

# Filesystem
bt dir src --depth 2 --filter ".rs"
bt dir --size --tokens --modified

# Diagrams (10 Mermaid types + 1 graph type)
bt flowchart "A --> B --> C"
bt quadrant "Task: [0.5, 0.5]"
bt pie-chart "Dogs: 50" "Cats: 30"
bt git-graph "commit" "branch feature" "merge feature"
bt bar-chart --horizontal --show-data-label --aspect-ratio 2.0 --inverse 10 20 15 25
bt line-chart --width 60% --horizontal --show-data-label 1 8 7 5
bt timeline "2020: Started" "2022: Launch"
bt state-diagram "[*] --> Idle" "Idle --> Running"
bt erd "Customer ||--o{ Order : places"
bt graph-expression "a -> b -> c"          # Arrow syntax (directed)
bt graph-expression "a -- b -- c"          # Dash syntax (undirected)
bt graph-expression --syntax dot "digraph { A -> B; }"  # DOT syntax
```

Diagram options: `--example`, `--width`, `--inverse`, `--title`, `--json`, `--meta`
Bar/line chart extras: `--horizontal`, `--show-data-label`, `--aspect-ratio`
Graph extras: `--syntax` (auto, expression, dot), `--orientation` (left-to-right, top-to-bottom)
Graph note: mixed `->` and `--` expression syntax is rejected; use separate graphs instead.

## Module Structure

```
biscuit_terminal/
├── terminal.rs           # Terminal struct + TerminalBuilder
├── prelude.rs            # Public re-exports
├── discovery/            # Detection
│   ├── detection.rs      # App, color, image, multiplex
│   ├── os_detection.rs   # OS, distro, CI
│   ├── fonts.rs          # Font name, size, cell_size
│   ├── osc_queries.rs    # bg/fg/cursor color queries
│   ├── clipboard.rs      # OSC52 clipboard
│   ├── locale.rs         # Locale, character encoding
│   ├── mode_2027.rs      # Grapheme cluster support
│   ├── cursor_position.rs # Cursor position queries
│   └── eval.rs           # Escape analysis
├── components/           # Rendering
│   ├── renderable.rs     # Renderable trait + RenderableContent
│   ├── compose.rs        # Compose (combine multiple renderables)
│   ├── section.rs        # Section with heading levels (h1-h6)
│   ├── block_quote.rs    # BlockQuote with attribution
│   ├── prose.rs          # Styled text with tokens
│   ├── status_block.rs   # Status + BlockQuote + hint composite
│   ├── text_block.rs     # Uniform block styling
│   ├── inline_content.rs # Inline concatenation without newlines
│   ├── list.rs           # OrderedList, UnorderedList
│   ├── table/            # Table with box-drawing borders
│   ├── two_column.rs     # TwoColumn side-by-side layout
│   ├── todo.rs           # Todo with states (Open, InProgress, etc.)
│   ├── progress.rs       # Progress indicator rendering
│   ├── filesystem.rs     # File/directory tree rendering
│   ├── terminal_image.rs # Image (Kitty/iTerm2 protocols)
│   ├── image_options.rs  # Policy options/helpers (app-enforced)
│   ├── mermaid.rs        # Terminal-facing Mermaid adapter
│   └── graph_expression.rs # Terminal-facing graph adapter
└── utils/
    ├── layout.rs         # Layout, Margin, WordWrap, Alignment
    ├── color.rs          # Color, BasicColor, RgbColor, WebColor, Tailwind, HdrColor
    ├── styling.rs        # Stylist trait, FontWeight, Style
    ├── escape_codes.rs   # ANSI escape code generation
    ├── block_constraint.rs # Visual width, line splitting
    ├── word_wrap.rs      # Word wrapping strategies
    ├── text.rs           # Content length calculation (escape-aware)
    ├── truncate.rs       # Text truncation
    └── multiplex.rs      # Multiplexing detection
```

## NO_COLOR Support

The `bt` CLI respects the `NO_COLOR` environment variable. When set, the
following commands strip SGR (color/style) sequences from their output
while preserving structural sequences such as OSC8 hyperlinks:

- `bt prose` — strips `\x1b[…m` sequences from rendered prose
- `bt quote`, `bt list`, `bt columns`, `bt padleft`, `bt padright`

The default `bt` terminal-inspection output also respects `NO_COLOR`.

## Testing

biscuit-terminal follows the Level 1 / 2 / 3 testing vocabulary from the
`cli` skill (see `cli` skill → "Test Rigor: Level 1 / Level 2 / Level 3"):

- **Level 1** — PTY-based tests in `lib/tests/` using `expectrl` and a
  thin `discovery_probe` example binary. These exercise library code
  through a pseudo-terminal without requiring a real terminal emulator.
- **Level 2** — Real-terminal tests in `cli/tests/level2_*.rs` using the
  shared `biscuit-test-harness` crate (WezTerm, Kitty, tmux). These
  validate escape-sequence output, glyph widths, scroll behaviour, and
  image protocol bytes against the actual terminal's display path.
- **Level 3** — Not applicable (biscuit-terminal has no interactive input).

### biscuit-test-harness (shared crate)

The `biscuit-test-harness` workspace member provides:

- `TerminalHarness` trait — `spawn`, `send_text`, `capture`, `settle`.
- `WezTermHarness`, `KittyHarness`, `TmuxHarness` implementations.
- `CapturedFrame { raw, plain }` plus a robust ECMA-48 `strip_ansi` helper.
- `available()` probes that check the binary on `$PATH` plus required env
  (`WEZTERM_UNIX_SOCKET`, `KITTY_LISTEN_ON`, `TMUX`).
- `skip_with_reason()` for clean test skips.

Level-2 tests skip cleanly when the required terminal emulator is
unavailable; no `#[ignore]` markers are used.

### Adding new Level-1 tests

1. Create `lib/tests/level1_<topic>.rs`.
2. Spawn the `discovery_probe` example binary via `expectrl` inside a PTY.
3. Manufacture terminal replies (e.g., OSC 11 for bg color) and assert on
   parsed output.
4. Use the helpers in `lib/tests/common/pty.rs` for standardized env setup
   (`CI=1`, `NO_COLOR=1`).

### Adding new Level-2 tests

1. Create `cli/tests/level2_<topic>.rs` with the skip-clean contract note
   at the top of the file.
2. Import `biscuit_test_harness::{skip_with_reason, TerminalHarness}` and
   `common::send_bt_command`.
3. Check `Harness::available()` and early-return with `skip_with_reason`
   if the terminal is absent.
4. Spawn a fresh shell per test: `harness.spawn_shell()`.
5. Send `bt` commands via `send_bt_command(&mut harness, "...")`.
6. Capture and assert on `frame.raw` (with ANSI) or `frame.plain` (stripped).

### Level-2 assertion strategies

| Terminal | Image protocol in capture? | Recommended assertion |
|----------|---------------------------|----------------------|
| WezTerm | No — `get-text --escapes` strips OSC/APC image sequences | Use `bt image --debug` and assert on debug output (e.g., `app: Wezterm`, `--- image debug ---`). For diagrams, use `--meta` and assert on JSON metadata. |
| Kitty | Yes — `kitty @ get-text --ansi` preserves `�_G` | Assert `frame.raw.contains("\x1b_G")` for Kitty graphics protocol bytes. |
| tmux | N/A — no image protocols | Assert fenced code block fallback: `frame.plain.contains("```mermaid")`. |

### Practical patterns for Level-2 tests

**Join wrapped JSON before searching:**
Terminal wrapping splits long JSON lines. Join lines before substring assertions:
```rust
let joined: String = frame.plain.lines().collect();
assert!(joined.contains("\"filename\"") && joined.contains("\"render_time_ms\""));
```

**Position the cursor predictably:**
Use `tput cup` to avoid racing shell initialization and unpredictable prompt heights:
```rust
harness.send_text(b"clear\n").expect("send_text failed");
harness.settle();
harness.send_text(b"tput cup 5 0\n").expect("send_text failed");
harness.settle();
send_bt_command(&mut harness, "image --debug fixtures/tiny.png");
```

**Add shell-ready delay for custom prompts:**
Custom prompts (e.g., starship, powerlevel10k) take time to initialize. Add ~1.5 s after `spawn_shell` before sending commands:
```rust
harness.spawn_shell().expect("spawn_shell failed");
std::thread::sleep(Duration::from_millis(1500));
```

**Use `--debug` for images and `--meta` for diagrams in WezTerm:**
Since WezTerm strips image protocol bytes from `get-text`, rely on stderr output:
- `bt image --debug fixtures/tiny.png` — prints cursor math, scroll predictions, and app detection.
- `bt flowchart --meta "A --> B"` — prints JSON with `filename`, `cache_hit`, `render_time_ms`.

**Test scroll compensation at bottom margin:**
Position cursor near bottom with `tput cup 22 0`, then run `bt image --debug`. Assert on `SCROLL needed` in the debug output.

**Test Warp floor rounding:**
Spoof Warp detection with `export TERM_PROGRAM=WarpTerminal`, then run `bt image --debug`. Assert on `app: Warp` and `floor=` in debug output.

**Balanced save/restore sequences:**
When testing components that use cursor save/restore (images, horizontal
rules, two-column layouts), assert that `\x1b[s` and `\x1b[u` counts match
in `frame.raw` to catch orphan sequences.

## Key Dependencies

- `sniff` - Git/repo/monorepo detection used by `Terminal::new()`
- `biscuit-visualized` - Owns Mermaid and graph artifact generation, theming, rasterization, and caching

## Resources

- [biscuit-terminal/lib](../../../biscuit-terminal/lib/) - Library source
- [biscuit-terminal/cli](../../../biscuit-terminal/cli/) - CLI source
