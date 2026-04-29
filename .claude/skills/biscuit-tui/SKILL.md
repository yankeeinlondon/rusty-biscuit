---
name: biscuit-tui
description: Expert knowledge for the biscuit-tui package area in the rusty-biscuit monorepo. Provides reusable TUI input components (tui-chrome library) and a CLI (question) for shell-scriptable prompts. Use when building or modifying ratatui-based input widgets, adding new components to the tui-chrome library, working with the question CLI, or implementing standalone/embedded terminal prompts.
---

# biscuit-tui

Reusable TUI input components for Rust, built on ratatui. Provides both a library (`tui-chrome`) and a CLI (`question`).

## Package Structure

| Crate | Path | Binary | Purpose |
|-------|------|--------|---------|
| `tui-chrome` | `lib/` | — | Input widget library |
| `tui-chrome-cli` | `cli/` | `question` | CLI front-end |

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
use tui_chrome::prelude::*;
let state = TextInputState::new()
    .with_label(Label::new("Name", LabelPosition::Above))
    .with_max_length(50)
    .with_value("Alice");
```

#### TextAreaInput
Multi-line editor. Uses `tui_textarea` as private edit engine. Default submit is `Ctrl+S` (not Enter).

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
use tui_chrome::core::{BorderStyle, FrameChrome, FrameChromeConfig, Margin};

let config = FrameChromeConfig {
    border: BorderStyle::Rounded,
    border_label: Some("Settings".into()),
    margin: Margin::uniform(1),
    ..Default::default()
};
let frame = FrameChrome::from_config(BooleanSwitch::new(), &config);
```

See `docs/components/frame_chrome.md` for the full `BorderStyle` variant list (14 styles).

#### InputTable
2D grid of heterogeneous cells. Supports: `StaticText`, `BooleanSwitch`, `TextInput`, `TextAreaInput`, `ChooseOne`, `ChooseMany`.

- Focus: arrows navigate; Tab/Shift+Tab wrap; `Ctrl+S` validates and submits
- Value: `Vec<Row>` where each `Row` has `Vec<RowCell>` with typed `CellValue`

```rust
let columns = vec![
    InputTableColumn::StaticText { id: "name".into(), text: "Alice".into() },
    InputTableColumn::BooleanSwitch { id: "active".into(), config: Default::default() },
];
let state = InputTableState::new(columns, vec![]);
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
| `BorderStyle` | 14 border glyph styles (None, Rounded, Sharp, Bold, Double, Block, ThinBlock, Horizontal, Vertical, Line, Top, Bottom, Left, Right) |
| `Margin` | Four-sided margin outside the border |
| `HeightSpec` | Parsed `--height` flag (absolute cells or percentage of terminal) |
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

**Global flags:** `--output {raw|json|null}`, `--height <N|P%>`

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
│   ├── standalone.rs   # run_standalone, run_standalone_with_chrome, drive_event_loop, StandaloneState, HandleEvent
│   ├── keybindings.rs  # KeyBindings
│   ├── theme.rs        # ComponentTheme
│   ├── frame.rs        # FrameChrome, FrameChromeConfig, BorderStyle, Margin, Padding, HeightSpec
│   ├── fuzzy.rs        # FuzzyFilter
│   ├── validation.rs   # ValidationState
│   ├── label.rs        # Label, LabelPosition, render_with_label
│   ├── sort.rs         # SortOrder, OptionSort
│   └── terminal_style.rs # TerminalStyle, TerminalBackground, NerdFontStatus
├── components/
│   ├── text_input.rs
│   ├── text_area_input.rs
│   ├── boolean_switch.rs
│   ├── choose.rs           # ChoiceInput, ChoiceOption, SelectionMode, Orientation, HotkeySpec, HotkeyDisplayMode, ActiveChoiceColor
│   ├── choose_one.rs
│   ├── choose_many.rs
│   └── input_table/
│       ├── mod.rs
│       ├── cell.rs     # CellState, CellValue, Row, RowCell
│       ├── column.rs   # InputTableColumn, configs
│       └── table.rs    # InputTable, InputTableState
└── helpers/
    └── choice_builders.rs

cli/src/
├── main.rs              # Clap CLI, dispatch
├── output.rs            # OutputMode (raw/json/null)
├── option_sources.rs    # Source resolution: --csv, --list, --rows, --file, --md, stdin
├── choice_normalize.rs  # Hotkey parsing, naming conventions, delimiter splitting
└── commands/
    ├── mod.rs
    ├── text_input.rs
    ├── text_area_input.rs
    ├── boolean_switch.rs
    ├── choose_one.rs
    ├── choose_many.rs
    ├── common_choose.rs   # Shared args: ChooseChromeArgs, source resolution, FrameChrome/build helpers
    └── input_table.rs
```

## Testing Conventions

- Unit tests in `#[cfg(test)] mod tests` within source files
- Use `TestBackend` for rendering tests
- Synthetic events via `drive_event_loop` with `Vec<Event>` iterator
- Prefer `assert_eq!` on `EventOutcome` variants

## DevOps

```bash
just build      # build library + CLI
just test       # test both crates
just lint       # clippy both
just install    # install `question` binary
just cli <args>  # run in dev mode
```

## Dependencies

- `ratatui` 0.29 — core TUI framework
- `crossterm` 0.28 — terminal events
- `tui-input` 0.11 — single-line edit engine
- `tui-textarea` 0.7 — multi-line edit engine
- `nucleo-matcher` 0.3 — fuzzy matching
- `serde_yaml_ng` 0.10 — dictionary parsing
- `thiserror` 2 — error types
- `unicode-width` 0.2 — width calculations
- `rand` 0.9 — option shuffling
