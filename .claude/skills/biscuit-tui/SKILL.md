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

### TextInput
Single-line text input. Uses `tui_input` as private edit engine.

```rust
use tui_chrome::prelude::*;
let state = TextInputState::new()
    .with_label(Label::new("Name", LabelPosition::Above))
    .with_max_length(50)
    .with_value("Alice");
```

### TextAreaInput
Multi-line editor. Uses `tui_textarea` as private edit engine. Default submit is `Ctrl+S` (not Enter).

```rust
let state = TextAreaInputState::new(60, 10)
    .with_scrollbar(true)
    .with_value(&["line 1", "line 2"]);
```

### BooleanSwitch
Toggle with customizable on/off labels.

```rust
let state = BooleanSwitchState::new()
    .with_labels("YES", "NO")
    .with_value(true);
```

### ChooseOne
Single-selection list with fuzzy filter, hotkeys, vim navigation, scrolling.

```rust
let input = ChoiceInput::new("colour", "Pick a colour")
    .with_options(vec![
        ChoiceOption::new("r", "Red", "red"),
        ChoiceOption::new("g", "Green", "green"),
    ])
    .required();
let state = ChooseOneState::new(input);
```

### ChooseMany
Multi-selection with `min_selections` / `max_selections`, `Ctrl+A` select all, `Ctrl+D` clear.

```rust
let input = ChoiceInput::new("toppings", "Pick toppings")
    .with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms"),
    ])
    .with_max_selections(2);
let state = ChooseManyState::new(input);
```

### InputTable
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
| `run_standalone` / `drive_event_loop` | Terminal event loops with Ctrl-C/Esc handling |
| `KeyBindings` | Configurable bindings (vim `h`/`j`/`k`/`l` by default) |
| `ComponentTheme` | Visual constants (glyphs, colors, styles) |
| `FrameChrome` / `FrameChromeConfig` | Borders, margins, height specs |
| `FuzzyFilter` | Fuzzy search via `nucleo-matcher` |
| `ValidationState` | Uniform validation error read access |
| `Label` / `LabelPosition` | Shared label placement |

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
```

**Global flags:** `--output {raw|json|null}`, `--height <N|P%>`

**Exit codes:** `0` = submitted, `1` = Esc cancelled, `130` = Ctrl-C interrupted

## Key Design Principles

1. **Consistent API** — every component is `StatefulWidget` + `*State` + `HandleEvent`
2. **Two-tier validation** — keystroke-time rejection (silent) + submit-time validation (error message)
3. **Typed values** — choice components generic over `V`; InputTable preserves booleans and arrays
4. **Testable** — `drive_event_loop` accepts any event source for synthetic injection
5. **Embeddable or standalone** — works in larger TUIs or as fullscreen/inline prompts

## Module Map

```
lib/src/
├── lib.rs          # Public exports
├── prelude.rs      # Convenience re-exports
├── core/
│   ├── event.rs        # EventOutcome
│   ├── standalone.rs   # run_standalone, drive_event_loop, StandaloneState, HandleEvent
│   ├── keybindings.rs  # KeyBindings
│   ├── theme.rs        # ComponentTheme
│   ├── frame.rs        # FrameChrome, BorderStyle, Margin, HeightSpec
│   ├── fuzzy.rs        # FuzzyFilter
│   ├── validation.rs   # ValidationState
│   ├── label.rs        # Label, LabelPosition, render_with_label
│   └── sort.rs         # SortOrder
├── components/
│   ├── text_input.rs
│   ├── text_area_input.rs
│   ├── boolean_switch.rs
│   ├── choose.rs           # ChoiceInput, ChoiceOption, SelectionMode
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
├── main.rs         # Clap CLI, dispatch
├── output.rs       # OutputMode (raw/json/null)
└── commands/
    ├── mod.rs
    ├── text_input.rs
    ├── text_area_input.rs
    ├── boolean_switch.rs
    ├── choose_one.rs
    ├── choose_many.rs
    ├── common_choose.rs
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
