---
name: biscuit-tui
description: Expert knowledge for the biscuit-tui package area in the rusty-biscuit monorepo. Provides reusable TUI input components (biscuit-tui library) and a CLI (question) for shell-scriptable prompts. Use when building or modifying ratatui-based input widgets, adding new components to the biscuit-tui library, working with the question CLI, or implementing standalone/embedded terminal prompts.
---

# biscuit-tui

Reusable TUI input components for Rust, built on ratatui. Provides both a library (`biscuit-tui`) and a CLI (`question`).

## Package Structure

| Crate | Path | Binary | Purpose |
|-------|------|--------|---------|
| `biscuit-tui` | `lib/` | — | Input widget library |
| `biscuit-tui-cli` | `cli/` | `question` | CLI front-end |

## Architecture

Every component follows the same pattern:

1. **Zero-sized widget** — implements `ratatui::widgets::StatefulWidget`
2. **External state** — caller-owned `*State` struct with builder API (`with_*`)
3. **Event handling** — returns `EventOutcome` (`Consumed` | `Ignored` | `Submitted` | `Cancelled`)
4. **Standalone runner** — `run_standalone(widget, state, height)` for fullscreen or inline prompts

## Components

### Atomic Components

#### TextInput
Single-line text input. Uses `tui_input` as private edit engine.

```rust
use biscuit_tui::prelude::*;
let state = TextInputState::new()
    .with_label(Label::new("Name", LabelPosition::Above))
    .with_max_length(50)
    .with_value("Alice");
```

#### TextAreaInput
Multi-line editor. Uses `ratatui_textarea` as private edit engine. Default submit is `Ctrl+S` (not Enter).

```rust
let state = TextAreaInputState::new(60, 10)
    .with_scrollbar(true)
    .with_value(&["line 1", "line 2"]);
```

#### BooleanSwitch
Toggle with customizable on/off labels. Always valid (no validation logic). Supports Space toggle, Left/Right force-set, vim h/l.

```rust
let state = BooleanSwitchState::new()
    .with_labels("YES", "NO")
    .with_value(true);
```

#### ChooseOne
Single-selection list with fuzzy filter, hotkeys, vim navigation, scrolling. Enter selects the active item and submits; Esc restores the initial selection and submits (exit 0). Ctrl/Alt hotkeys select and submit.

```rust
let input = ChoiceInput::new("colour", "Pick a colour")
    .with_options(vec![
        ChoiceOption::new("r", "Red", "red"),
        ChoiceOption::new("g", "Green", "green"),
    ])
    .required();
let state = ChooseOneState::new(input);
```

Active-item accessors read the **highlighted** row (distinct from the submitted `selected_value()`): `active_option()`, `active_value()`, and `active_description(&id_to_desc_map)` (sugar over `active_option().id → map`; `ChoiceOption` is unchanged). They return the option as-is — including a `disabled` one — and `None` when the list is empty. These are the entry point for a SplitPane master/detail pane that derives its content from the active highlight each frame.

#### ChooseMany
Multi-selection with `min_selections` / `max_selections`, `Ctrl+A` select all, `Ctrl+D` clear. Enter submits the current selection exactly as-is; Space toggles the active item.

```rust
let input = ChoiceInput::new("toppings", "Pick toppings")
    .with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms"),
    ])
    .with_max_selections(2);
let state = ChooseManyState::new(input);
```

### Container Components

#### FrameChrome
Wraps any `StatefulWidget` with optional border, title, and margin. Not an input itself — adds visual chrome. Used by the CLI via `--border` / `--border-label` / `--border-style` / `--margin` flags on choose commands.

```rust
use biscuit_tui::core::{BorderStyle, FrameChrome, FrameChromeConfig, Margin};

let config = FrameChromeConfig {
    border: BorderStyle::Rounded,
    border_label: Some("Settings".into()),
    margin: Margin::uniform(1),
    ..Default::default()
};
let frame = FrameChrome::from_config(BooleanSwitch::new(), &config);
```

See `docs/components/frame_chrome.md` for the full `BorderStyle` variant list (14 styles).

#### SplitPane
Geometry-only two-pane layout primitive — a container/layout primitive like `FrameChrome`, **not** an input (it captures no value and handles no input). Divides a `Rect` into two child `Rect`s along one axis via `SplitPane::split(area) -> (Rect, Rect)`; the caller renders each child with its own `render_stateful_widget`. Defaults to 50/50 with `Auto` direction (resolved from the area's shape each split). Nest `split()` calls for N-way layouts.

```rust
use biscuit_tui::core::{SplitDirection, SplitPane, SplitRatio};

// 30% sidebar | 70% main, side by side.
let (sidebar, main) = SplitPane::new()
    .with_direction(SplitDirection::Horizontal)
    .with_ratio(SplitRatio::Percent(30))
    .split(frame.area());
```

`ResolvedAxis` (the concrete axis after `Auto` resolution) is crate-private. v1 ships geometry only — no render wrapper, no `question` CLI command. Ratios clamp on construction (`Percent` to `1..=99`, `*Fixed` to `>= 1`) so no pane is voluntarily starved.

##### One border around both panes (compose with `FrameChrome`)

`SplitPane` is geometry-only, so you **cannot** pass it to `FrameChrome` directly (`FrameChrome` requires its inner to be a `StatefulWidget`; `SplitPane` is not a widget). To draw a single border around both panes, wrap the two panes in a small zero-sized composite `StatefulWidget` and hand *that* to `FrameChrome` — the split runs inside `FrameChrome`'s already-bordered/padded inner rect, so the full `FrameChromeConfig` path (`border_label`, style, padding) keeps working unchanged:

```rust
use biscuit_tui::core::{FrameChrome, FrameChromeConfig, SplitDirection, SplitPane, SplitRatio};
use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};

// Zero-sized composite, like every other widget here. Its `render` receives
// FrameChrome's inner rect (after margin + border + padding) and splits it.
struct TwoPane;

impl StatefulWidget for TwoPane {
    type State = (LeftState, RightState);
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (left, right) = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::Percent(30))
            .split(area);
        LeftWidget.render(left, buf, &mut state.0);
        RightWidget.render(right, buf, &mut state.1);
    }
}

let config = FrameChromeConfig {
    border_label: Some("Panes".into()),
    ..Default::default()
};
let frame = FrameChrome::from_config(TwoPane, &config); // one border around both
```

This wraps the *whole* layout in one border. A border around **each** pane is a separate, deferred feature — see `features/_unscheduled/per-pane-borders.md`.

#### InputTable
2D grid of heterogeneous cells. Supports: `StaticText`, `BooleanSwitch`, `TextInput`, `TextAreaInput`, `ChooseOne`, `ChooseMany`.

- Focus: arrows navigate; Tab/Shift+Tab wrap; `Ctrl+S` validates and submits
- Value: `Vec<Row>` where each `Row` has `Vec<RowCell>` with typed `CellValue`
- Construction: `new` panics on invalid input (row-shape/column-id/cell-type mismatch); use the fallible `try_new` for caller-provided data, which returns a typed `InputTableError` (re-exported from the crate root and prelude). `with_blank_rows` is infallible (seeds from column schema).

```rust
let columns = vec![
    InputTableColumn::StaticText { id: "name".into(), text: "Alice".into() },
    InputTableColumn::BooleanSwitch { id: "active".into(), config: Default::default() },
];
// static/known-good data: panics on misuse
let state = InputTableState::new(columns, vec![]);
// untrusted data: use try_new -> Result<_, InputTableError>
// let state = InputTableState::try_new(columns, rows)?;
```

## Core Primitives

| Type | Purpose |
|------|---------|
| `EventOutcome` | Canonical event result (Consumed, Ignored, Submitted, Cancelled) |
| `StandaloneState` / `HandleEvent` | Traits for driving components |
| `run_standalone` / `run_standalone_with_chrome` | Terminal event loops with Ctrl-C/Esc handling; `*_with_chrome` variant wraps with `FrameChrome` |
| `drive_event_loop` / `drive_event_loop_with_chrome` | Lower-level loop drivers accepting any event source |
| `KeyBindings` | Configurable bindings (vim `h`/`j`/`k`/`l` by default) |
| `ComponentTheme` | Visual constants (glyphs, colors, styles) |
| `FrameChrome` / `FrameChromeConfig` | Container widget for borders, margins, and titles |
| `SplitPane` / `SplitDirection` / `SplitRatio` | Geometry-only two-pane layout primitive (`split(area) -> (Rect, Rect)`); container/layout like `FrameChrome`, not an input. `Auto` direction resolved from area shape, 50/50 default; ratios clamp on construction |
| `BorderStyle` | 14 border glyph styles (None, Rounded, Sharp, Bold, Double, Block, ThinBlock, Horizontal, Vertical, Line, Top, Bottom, Left, Right) |
| `Margin` | Four-sided margin outside the border |
| `HeightSpec` | Parsed `--height` flag (cells or percent). Treated as a maximum (clamps to live terminal); `Percent` re-resolves on resize so the inline viewport tracks the requested fraction mid-prompt |
| `SortOrder` | Ordering for choice options (Natural, Reverse, Asc, Desc) |
| `OptionSort` | Preferred ordering vocabulary (Natural, Inverse, Asc, Desc) with `From` conversions to `SortOrder` |
| `Orientation` | Choice list layout direction (`Vertical`, `Horizontal`) |
| `HotkeySpec` / `HotkeyDisplayMode` | Keyboard shortcuts for choice options and when to render badges |
| `ActiveChoiceColor` | Background colour for the actively hovered option |
| `Padding` | Four-sided interior padding inside the border |
| `TerminalStyle` / `TerminalBackground` / `NerdFontStatus` | Conservative terminal capability detection |
| `FuzzyFilter` | Fuzzy search via `nucleo-matcher` |
| `ValidationState` | Uniform validation error read access |
| `Label` / `LabelPosition` | Shared label placement (Above, Below, Left, Right) |

## Helpers

`choice_builders` — construct `ChoiceInput<String>` from:

- `choose_one_from_csv` / `choose_many_from_csv`
- `choose_one_from_markdown_list` / `choose_many_from_markdown_list`
- `choose_one_from_dictionary` / `choose_many_from_dictionary` (YAML/JSON)

The underlying canonical parsers are also public:

- `choice_options_from_csv`
- `choice_options_from_markdown_list`
- `choice_options_from_dictionary`

The `question` CLI source resolver reuses these helpers for CSV,
markdown-list, and dictionary compatibility sources.

## Rendering Bridge (`TuiRenderable`)

Behind the off-by-default **`renderables`** feature, `biscuit-tui` renders any
biscuit-terminal / darkmatter `TerminalRenderable` component (`CodeBlock`,
`Table`, `Prose`, lists, ...) as native ratatui `Text` via one method:

```rust
fn to_tui_text(&self, width: u16) -> ratatui::text::Text<'static>;
```

Lives in `lib/src/renderable.rs`; the trait is here (not in `renderable`)
because `renderable` is intentionally ratatui-free. **Tier 0** (shipped) is one
blanket `impl<T: TerminalRenderable> TuiRenderable`: render the component to
ANSI via `render_optimistic(width)`, parse with `ansi-to-tui`, reuse
biscuit-terminal's layout + syntect highlighting. Static/read-only; covers the
whole catalog for free. **Tier 1** (native `RenderNode` → `Text` fold) and
**Tier 2** (interactive `StatefulWidget`s) are future work behind the unchanged
`to_tui_text` seam — see `docs/tui-renderable.md` for the design and the
when-to-graduate decision heuristic.

### Implementing Tier 0 in a caller

**1. Enable the feature, and depend on the component's home crate.** The
`renderables` feature gives you `to_tui_text`, but the *component types* come
from their own crates, which the caller must add directly: `darkmatter` for
`CodeBlock`, `biscuit-terminal` for `Table` / `Prose` / lists.

```toml
[dependencies]
biscuit-tui = { path = "...", features = ["renderables"] }
darkmatter  = { path = "..." }   # for CodeBlock
# biscuit-terminal = { path = "..." }  # for Table / Prose / lists
```

**2. Build the component once (cheap to keep in app state), render per frame.**
Pass the *target area's width* each draw so layout tracks resizes:

```rust
use biscuit_tui::TuiRenderable;                 // brings `to_tui_text` into scope
use darkmatter::markdown::code_block::CodeBlock;
use ratatui::widgets::{Block, Borders, Paragraph};

let code = CodeBlock::rust(source);             // ::rust / ::yaml / ::new(...).with_fence_language(...)

terminal.draw(|frame| {
    let area = frame.area();
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);              // lay out to the *content* width, not the bordered outer width
    let text = code.to_tui_text(inner.width);   // owned, fully-styled Text<'static>
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .scroll((scroll_y, 0)),             // vertical scroll for free
        area,
    );
})?;
```

`Table` / `Prose` work identically — `use biscuit_terminal::prelude::{Table, Prose};`,
construct, then `.to_tui_text(width)`.

**3. Wrap in `Paragraph` for scroll — but never `.wrap()` a `Table`.** The
component already laid itself out at `width` (column planner, word wrap), so
ratatui re-wrapping corrupts the columns. `Paragraph::scroll` is fine for any
component; `Paragraph::wrap` is only safe for content the component did *not*
pre-wrap.

**4. Cache in a hot redraw loop.** `to_tui_text` re-renders (and, for code,
re-runs syntect) every call. If you only redraw on events it is fine; in a
high-FPS loop, memoize by `(content_hash, width)` and re-render only when either
changes.

**Width gotcha:** if the component renders inside a bordered/padded `Block`,
pass the *inner* width, not the outer `area.width`, or the content overflows the
border. Compute the inner rect first (`block.inner(area)`), then
`to_tui_text(inner.width)`.

Known Tier-0 limitations (ragged code-block background, no row-selection handle,
links/images dropped) and their Tier-1 fixes are catalogued in
`docs/tui-renderable.md`.

The feature is off by default because biscuit-terminal carries heavy transitive
weight (syntect, resvg via biscuit-visualized) that pure input-widget consumers
should not pay for.

## CLI (`question`)

```bash
question text-input --label "Name" --max-length 50
question text-area-input --width 80 --scrollbar
question boolean-switch --labels "YES,NO" --initial true
question choose-one Red Green Blue
question choose-many --options "Red,Green,Blue" --min-selections 1
question input-table --columns '[{"type":"text","id":"name"}]'
question completions zsh
```

**Global flags:** `--output {raw|json|null}`, `--height <N|P%>`, `--show-input-on-exit`

The inline viewport clears on exit by default (fzf-style); pass `--show-input-on-exit` to leave the final frame on screen with the cursor placed just below the chrome.

**Exit codes:** `0` = submitted (ChooseOne Esc also exits `0` by restoring the initial selection), `1` = Esc cancelled (non-ChooseOne components), `130` = Ctrl-C interrupted

### Choose-specific Flags (`choose-one`, `choose-many`)

**Option sources (mutually exclusive):**
- Positional args: `question choose-one Alpha Beta Gamma`
- `--csv "Red,Green,Blue"` — comma-separated list
- `--list "a\nb\nc"` — newline-separated list
- `--rows "a::1\nb::2"` — newline-separated `label::value` pairs
- `--file <PATH>` — JSON/YAML/TOML/CSV array, or JSONL/NDJSON lines
- `--md <PATH> <PROP>` — YAML frontmatter array property from Markdown file
- `--options <TEXT>` — hidden alias for `--csv` (backward compatibility)
- Piped stdin (automatic when stdin is not a TTY)

**Selection & filtering:**
- `--selected <VALUE>` — pre-select by value (repeatable for `choose-many`)
- `--required` — fail if nothing selected
- `--min-selections <N>` / `--max-selections <N>` — choose-many limits
- `--delimiter <CHAR>` — split each option into `label<CHAR>value` (legacy)
- `--no-filter` — disable fuzzy search (use hotkey shortcuts instead)
- `--sort <natural|reverse|asc|desc>` — reorder options before display

**Hotkeys & normalization:**
- `[CTRL+X]` / `[ALT+X]` / `[OPT+X]` prefix in option text — explicit hotkey
- `--numeric-hot-keys` — auto-assign Ctrl+1..0, then Alt+1..0
- `--label-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`
- `--value-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`
- `::` delimiter — split `label::value` (takes precedence over conventions)

**Chrome (FrameChrome wrapping):**
- `--border` — draw a border (defaults to rounded)
- `--border-label <TEXT>` — title in the border (implies `--border`)
- `--border-style <STYLE>` — glyph style (implies `--border` unless `none`)
- `--margin <CELLS>` — uniform margin outside border
- `--mt` / `--mb` / `--ml` / `--mr` — per-side margin overrides
- `--padding <CELLS>` / `-p <CELLS>` — uniform padding inside the border
- `--pt` / `--pb` / `--pl` / `--pr` — per-side padding overrides

### input-table JSON Boundary

`--columns` and `--rows` JSON is strictly validated. Numeric column-config
fields (`max_length`, `preferred_width`, `preferred_height`,
`min_selections`, `max_selections`) reject overflow instead of truncating;
present-but-wrong-type optional fields error rather than defaulting. Row
values that mismatch the column's cell type produce an `InvalidInput`
error with row/column context. A small set of **documented permissive
coercions** remain as intentional compatibility (see
`docs/components/input_table.md`): boolean rows accept bool/number/the
`true|on|yes|1|false|off|no|0` strings; text-area rows accept an array or
a newline-split string; choose-many rows accept an array or a
comma-split string. Library and CLI share one validation code path via
`InputTableState::try_new` / `InputTableError`.

## Key Design Principles

1. **Consistent API** — every component is `StatefulWidget` + `*State` + `HandleEvent`
2. **Two-tier validation** — keystroke-time rejection (silent) + submit-time validation (error message)
3. **Typed values** — choice components generic over `V`; InputTable preserves booleans and arrays
4. **Testable** — `drive_event_loop` accepts any event source for synthetic injection
5. **Embeddable or standalone** — works in larger TUIs or as fullscreen/inline prompts
6. **Container composition** — `FrameChrome` wraps any widget; `InputTable` embeds atomic widgets as cells
7. **Component docs** — detailed per-component docs live in `biscuit-tui/docs/components/`

## Module Map

```
lib/src/
├── lib.rs          # Public exports
├── prelude.rs      # Convenience re-exports
├── core/
│   ├── event.rs        # EventOutcome
│   ├── standalone/     # standalone runner traits, event loops, terminal lifecycle, inline viewport handling
│   ├── keybindings.rs  # KeyBindings
│   ├── theme.rs        # ComponentTheme
│   ├── frame.rs        # FrameChrome, FrameChromeConfig, BorderStyle, Margin, Padding, HeightSpec
│   ├── fuzzy.rs        # FuzzyFilter
│   ├── validation.rs   # ValidationState
│   ├── label.rs        # Label, LabelPosition, render_with_label
│   ├── sort.rs         # SortOrder, OptionSort
│   ├── split_pane.rs   # SplitPane, SplitDirection, SplitRatio (geometry-only 2-pane layout; ResolvedAxis crate-private)
│   └── terminal_style.rs # TerminalStyle, TerminalBackground, NerdFontStatus
├── components/
│   ├── choice_state.rs     # Shared ChooseOne/ChooseMany runtime state and hotkey helpers
│   ├── text_input.rs
│   ├── text_area_input.rs
│   ├── boolean_switch.rs
│   ├── choose.rs           # ChoiceInput, ChoiceOption, SelectionMode, Orientation, HotkeySpec, HotkeyDisplayMode, ActiveChoiceColor
│   ├── choice_render/      # Shared choice renderer split by badge/highlight/vertical/horizontal rendering
│   ├── choose_one.rs
│   ├── choose_one/         # ChooseOne tests
│   ├── choose_many.rs
│   ├── choose_many/        # ChooseMany tests
│   └── input_table/
│       ├── mod.rs
│       ├── cell.rs     # CellState, CellValue, Row, RowCell
│       ├── column.rs   # InputTableColumn, configs
│       ├── table.rs    # InputTable, InputTableState
│       └── table/      # InputTable tests
├── helpers/
│   └── choice_builders.rs
└── renderable.rs      # TuiRenderable (feature = "renderables"): TerminalRenderable -> ratatui Text

cli/src/
├── main.rs              # Clap CLI, dispatch
├── output.rs            # OutputMode (raw/json/null)
├── option_sources.rs    # Source resolution: --csv, --list, --rows, --file, --md, stdin
├── choice_normalize.rs  # Hotkey parsing, naming conventions, delimiter splitting
├── completions.rs       # Shell completion generation with hotkey-prefix support (bash/zsh)
└── commands/
    ├── mod.rs
    ├── text_input.rs
    ├── text_area_input.rs
    ├── boolean_switch.rs
    ├── choose_one.rs
    ├── choose_many.rs
    ├── common_choose.rs   # Shared choose args: ChooseSourceArgs, ChooseChromeArgs, source/input resolution, run plumbing, FrameChrome/build helpers
    └── input_table/       # input-table command, column parsing, tests
```

## Testing Conventions

- Unit tests live in sibling `tests.rs` modules for decomposed components and in inline `#[cfg(test)]` modules for smaller files
- Use `TestBackend` for rendering tests
- Synthetic events via `drive_event_loop` with `Vec<Event>` iterator
- Prefer `assert_eq!` on `EventOutcome` variants

### Headless guard

`run_standalone_with_chrome` returns an immediate `io::Error` when both
stdout and stderr are piped (`!is_terminal()`). This is **load-bearing**
for the test suite: without it, a subprocess that spawns the binary
from an interactive `just test` shell would call `enable_raw_mode()`
on `/dev/tty`, mutating the **shared** controlling terminal's termios
(OPOST off → `\n` loses CR translation) and corrupting any redrawing
UI in the parent (nextest progress bar, indicatif, etc.). The stderr
check preserves the shell command-substitution case
(`FOO=$(question ...)` — stderr stays attached to the terminal).

Do not write integration tests that spawn `question` just to assert
"reached the event loop and bailed" — that pattern relies on
`enable_raw_mode()` failing with ENXIO when no controlling tty is
present, which is environment-dependent. Use the Writer-seam pattern
below instead.

### Writer-seam unit testing

Each `run` function in `cli/src/commands/*.rs` is factored into a
`run_with_writer(args, output, height, writer, run_prompt)` shape
where `run_prompt: FnOnce(State, Option<HeightSpec>) -> io::Result<V>`
is injectable. Tests substitute a closure that returns a synthetic
value or `io::Error::new(CANCELLED_KIND|ABORTED_KIND, ...)` without
ever calling `run_standalone`. This is the canonical place to assert
flag plumbing, height propagation, hotkey normalization, and exit-code
mapping. See `run_propagates_*_height_to_prompt` and `run_writes_*` in
`choose_one.rs` and `choose_many.rs` for working examples.

## DevOps

```bash
just build      # build library + CLI
just test       # test both crates
just lint       # clippy both
just install    # install `question` binary
just cli <args>  # run in dev mode
```

The CLI's internal `terminal-tests` feature exposes its real-terminal, L3, and
Windows-console integration targets. Ordinary local L1 omits the terminal
harness and those standalone targets; `just test-l2`, `just test-l3`, the
Windows console recipe, and CI enable them explicitly.

## Dependencies

- `ratatui` 0.30 — core TUI framework
- `crossterm` 0.29 — terminal events
- `tui-input` 0.15 — single-line edit engine
- `ratatui-textarea` 0.9 — multi-line edit engine (Ratatui-org fork of `tui-textarea`, which caps at ratatui 0.29)
- `nucleo-matcher` 0.3 — fuzzy matching
- `serde_yaml_ng` 0.10 — dictionary parsing
- `thiserror` 2 — error types
- `unicode-width` 0.2 — width calculations
- `rand` 0.9 — option shuffling

Optional (`renderables` feature only):

- `biscuit-terminal` (path) — source of `TerminalRenderable` components
- `ansi-to-tui` 8 — parses rendered ANSI into ratatui `Text` (tracks ratatui 0.30)
