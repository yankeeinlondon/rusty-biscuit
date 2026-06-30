# biscuit-tui

A library of reusable TUI input components built on [ratatui](https://ratatui.rs).

## Components

The library provides the following interactive input components:

- **TextInput** — single-line text input with configurable max length and label positioning
- **TextAreaInput** — multi-line text editor with scrollbar and configurable dimensions
- **BooleanSwitch** — toggle switch with customizable on/off labels and visual track
- **ChooseOne** — single-selection list with fuzzy filtering, hotkey support, and vim navigation
- **ChooseMany** — multi-selection list with validation constraints (min/max selections)
- **InputTable** — grid of mixed input cells (any combination of the above components plus static text)

All components follow a consistent API:

1. Each component is a `StatefulWidget` with an external `State` struct owned by the caller.
2. Event handling returns `EventOutcome` (`Consumed` | `Ignored` | `Submitted` | `Cancelled`).
3. Components can be embedded in a larger TUI or run as a standalone fullscreen/inline window via `run_standalone`.

## Core Primitives

The [`core`](src/core) module provides cross-cutting primitives shared by every component:

- **`EventOutcome`** — canonical event result enum (`Consumed`, `Ignored`, `Submitted`, `Cancelled`)
- **`StandaloneState`** / **`HandleEvent`** — traits that enable `run_standalone` and `drive_event_loop`
- **`run_standalone`** — drives a single component in a dedicated terminal (fullscreen or inline)
- **`KeyBindings`** — fully configurable key bindings with vim-compatible defaults (`h`/`j`/`k`/`l`)
- **`ComponentTheme`** — centralized visual constants (glyphs, colors, styles)
- **`FrameChrome`** / **`FrameChromeConfig`** — optional borders, margins, padding, and height specs
- **`SplitPane`** / **`SplitDirection`** / **`SplitRatio`** — geometry-only two-pane layout primitive (`SplitPane::split(area) -> (Rect, Rect)`); a *container/layout* primitive like `FrameChrome`, not an input (captures no value, handles no input). `Auto` direction is resolved from the area's shape each split, 50/50 default; ratios clamp on construction
- **`FuzzyFilter`** — fast fuzzy search over option labels via `nucleo-matcher`
- **`ValidationState`** — uniform read access to submit-time validation errors
- **`Label`** / **`LabelPosition`** / **`render_with_label`** — shared label placement
- **`Padding`** — four-sided interior padding inside the border
- **`TerminalStyle`** / **`TerminalBackground`** / **`NerdFontStatus`** — conservative terminal capability detection
- **`ActiveChoiceColor`** / **`resolve_active_style`** — choice-list active-row styling driven by spec-aligned palettes (Grey/Green/Yellow/Red) tuned per-background, returned as a `ratatui::style::Style` with bold + contrast-correct foreground

The [`prelude`](src/prelude.rs) module re-exports the most commonly used types for convenient glob imports.

## Usage

### Embedded in a TUI application

```rust
use biscuit_tui::{TextInput, TextInputState, EventOutcome, HandleEvent};
use ratatui::prelude::*;
use crossterm::event::{Event, KeyCode, KeyModifiers};

let mut state = TextInputState::new()
    .with_label(Label { text: "Name".into(), position: LabelPosition::Above })
    .with_max_length(50);

// In your event loop:
if let Event::Key(key_event) = crossterm::event::read()? {
    let outcome = TextInput.handle_event(&mut state, key_event);
    match outcome {
        EventOutcome::Submitted => {
            let value = state.value();
            // use the value
        }
        EventOutcome::Cancelled => { /* handle cancel */ }
        _ => {}
    }
}

// In your draw callback:
terminal.draw(|frame| {
    let area = frame.area();
    frame.render_stateful_widget(TextInput, area, &mut state);
})?;
```

### Standalone window

```rust
use biscuit_tui::{run_standalone, TextInput, TextInputState};

let state = TextInputState::new().with_max_length(100);
match run_standalone(TextInput, state, None) {
    Ok(value) => println!("Submitted: {value}"),
    Err(_) => eprintln!("Cancelled"),
}
```

The third parameter to `run_standalone` is `height: Option<HeightSpec>`.
Both inline variants are treated as a **maximum** — the inline viewport
is clamped to whatever rows the live terminal actually has, so the
prompt never overflows the screen.

- `None` → fullscreen mode using `AlternateScreen`
- `Some(HeightSpec::Cells(n))` → inline mode rendering up to `n` rows
  below the current cursor (ratatui's autoresize clamps when the
  terminal is smaller than `n`)
- `Some(HeightSpec::Percent(p))` → inline mode sized at `p` percent of
  the live terminal height (clamped to a floor of 3 rows). The
  percentage is **re-resolved on every terminal resize**, so the inline
  viewport tracks the requested fraction as the terminal grows or
  shrinks mid-prompt

## Key Bindings

All components support configurable key bindings. The defaults are:

- **Submit**: `Enter` (single components), `Ctrl-S` (InputTable)
- **Cancel / Reset**: `Esc` — behaviour varies by component:
  - `ChooseOne`: restores the initial selection and submits (exit `0`)
  - `ChooseMany`, `TextInput`, etc.: cancels the interaction (exit `1`)
- **Ctrl-C**: always cancels with exit `130`
- **Navigation**: arrow keys + vim keys (`h`/`j`/`k`/`l`)
- **Toggle / Select**: `Space` (BooleanSwitch, ChooseOne, ChooseMany)
- **Select all / Clear**: `Ctrl+A` / `Ctrl+D` (ChooseMany)

Customize via `state.with_key_bindings(KeyBindings::default())`.

## Typed Values

Choice components (`ChooseOne`, `ChooseMany`) are generic over the value type `V`:

```rust
use biscuit_tui::{ChooseOneState, ChoiceInput, ChoiceOption};

// Library consumers can project string options into typed values
let options = vec![
    ChoiceOption { id: "one".into(), label: "First".into(), value: "1".into(), disabled: false },
    ChoiceOption { id: "two".into(), label: "Second".into(), value: "2".into(), disabled: false },
]
.into_iter()
.map(|opt| opt.map_value(|s| s.parse::<u32>().unwrap()))
.collect();

let input = ChoiceInput { id: "num".into(), prompt: "Pick".into(), options, ..Default::default() };
let state = ChooseOneState::new(input);

// After submission:
let value: Option<u32> = state.value();
```

The CLI always uses `V = String`.

## InputTable Typed Rows

`InputTable` returns typed rows (introduced in Phase 5):

```rust
use biscuit_tui::{InputTableState, InputTableColumn, CellValue, Row};

let columns = vec![
    InputTableColumn::StaticText { id: "name".into(), text: "Name".into() },
    InputTableColumn::BooleanSwitch { id: "active".into(), config: Default::default() },
];

let initial_rows = vec![
    vec![CellValue::StaticText("Alice".into()), CellValue::Boolean(false)],
    vec![CellValue::StaticText("Bob".into()), CellValue::Boolean(true)],
];

let state = InputTableState::new(columns, initial_rows);

// After submission:
let rows: Vec<Row> = state.value();
for row in rows {
    if let Some(CellValue::Boolean(active)) = row.get("active") {
        println!("active: {active}");
    }
}
```

`InputTableState::new` panics on invalid input (row-shape mismatch,
duplicate/unknown/missing column ids, or a typed cell mismatch). For rows
sourced from user or config data, use the fallible
`InputTableState::try_new`, which returns a typed `InputTableError`
instead of panicking. `InputTableError` is re-exported from the crate
root and prelude alongside the other public table types.

## Validation

Two-tier validation model:

1. **Keystroke-time rejection** — hard limits (`max_length`, `max_selections`) silently block input that would exceed the cap.
2. **Submit-time validation** — `required` and `min_selections` are checked on submit. If violated, `handle_event` returns `Consumed` and the component renders an inline error message. The error text is accessible via `state.validation_error()`.

## Testing

Library unit tests cover state transitions, layout math, fuzzy filter
behaviour, and rendering of choice badges / hotkey display modes. Run
them with `cargo test -p biscuit-tui --lib`.

For end-to-end and real-terminal verification (Level 1 / Level 2 /
Level 3 testing rigor — including the `wezterm cli` / `kitty @` / `tmux`
harnesses and `cliclick` keyboard injection on macOS), see the
[`biscuit-tui-cli` README's "Test Rigor" section](../cli/README.md#test-rigor--level-1--level-2--level-3)
and the shared harness crate's
[`biscuit-test-harness/README.md`](../../biscuit-test-harness/README.md),
which documents the harness variants and the environment each requires.

The lib's render correctness for hotkey badges is verified at Level 2 by
piping kitty keyboard-protocol bytes into a real WezTerm pane via
`wezterm cli send-text` and capturing the rendered output — see
`cli/tests/real_terminal_render.rs::level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`.

## Documentation

For complete design rationale and implementation details, see:

- [spec.md](../features/2026-04-16-input-tui/spec.md)
- [tech-design.md](../features/2026-04-16-input-tui/tech-design.md)
