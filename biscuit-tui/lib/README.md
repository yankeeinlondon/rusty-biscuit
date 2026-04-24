# tui-chrome

A library of reusable TUI input components built on [ratatui](https://ratatui.rs).

## Components

The library provides the following interactive input components:

- **TextInput** — single-line text input with configurable max length and label positioning
- **TextAreaInput** — multi-line text editor with scrollbar and configurable dimensions
- **BooleanSwitch** — toggle switch with customizable on/off labels and visual track
- **ChooseOne** — single-selection list with hotkey support and vim navigation
- **ChooseMany** — multi-selection list with validation constraints (min/max selections)
- **InputTable** — grid of mixed input cells (any combination of the above components)

All components follow a consistent API:

1. Each component is a `StatefulWidget` with an external `State` struct owned by the caller.
2. Event handling returns `EventOutcome` (`Consumed` | `Ignored` | `Submitted` | `Cancelled`).
3. Components can be embedded in a larger TUI or run as a standalone fullscreen/inline window via `run_standalone`.

## Usage

### Embedded in a TUI application

```rust
use tui_chrome::{TextInput, TextInputState, EventOutcome, HandleEvent};
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
use tui_chrome::{run_standalone, TextInput, TextInputState};

let state = TextInputState::new().with_max_length(100);
match run_standalone(TextInput, state, None) {
    Ok(value) => println!("Submitted: {value}"),
    Err(_) => eprintln!("Cancelled"),
}
```

The third parameter to `run_standalone` is `height: Option<HeightSpec>`:

- `None` → fullscreen mode using `AlternateScreen`
- `Some(HeightSpec::Cells(n))` → inline mode rendering `n` rows below
  the current cursor
- `Some(HeightSpec::Percent(p))` → inline mode sized at `p` percent of
  the live terminal height (clamped to a floor of 3 rows)

## Key Bindings

All components support configurable key bindings. The defaults are:

- **Submit**: `Enter` (single components), `Ctrl-S` (InputTable)
- **Cancel**: `Esc`
- **Navigation**: arrow keys + vim keys (`h`/`j`/`k`/`l`)
- **Toggle**: `Space` (BooleanSwitch, ChooseOne, ChooseMany)

Customize via `state.with_key_bindings(KeyBindings::default())` (introduced in Phase 2).

## Typed Values

Choice components (`ChooseOne`, `ChooseMany`) are generic over the value type `V`:

```rust
use tui_chrome::{ChooseOneState, ChoiceInput, ChoiceOption};

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
use tui_chrome::{InputTableState, InputTableColumn, CellValue, Row};

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

## Validation

Two-tier validation model:

1. **Keystroke-time rejection** — hard limits (`max_length`, `max_selections`) silently block input that would exceed the cap.
2. **Submit-time validation** — `required` and `min_selections` are checked on submit. If violated, `handle_event` returns `Consumed` and the component renders an inline error message. The error text is accessible via `state.validation_error()`.

## Documentation

For complete design rationale and implementation details, see:

- [spec.md](../features/2026-04-16-input-tui/spec.md)
- [tech-design.md](../features/2026-04-16-input-tui/tech-design.md)
