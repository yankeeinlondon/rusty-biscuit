# biscuit-tui

Reusable terminal UI (TUI) input components for Rust, built on [ratatui](https://ratatui.rs). Provides both a library (`biscuit-tui`) for embedding interactive widgets in larger applications and a CLI (`question`) for shell-scriptable prompts.

## Overview

This package area contains:

| Crate | Binary | Purpose |
|-------|--------|---------|
| `biscuit-tui` (library) | — | Composable input widgets + event loop helpers |
| `biscuit-tui-cli` (CLI) | `question` | Shell-facing front-end to every library component |

## Components

The library provides six interactive input components, each following the same architectural pattern:

1. **Zero-sized widget marker** implementing `ratatui::widgets::StatefulWidget`
2. **External state struct** owned by the caller
3. **Uniform event handling** returning `EventOutcome` (`Consumed` | `Ignored` | `Submitted` | `Cancelled`)
4. **Standalone runner** via `run_standalone` for fullscreen or inline prompts

### Input Components

| Component | Description | Value Type |
|-----------|-------------|------------|
| `TextInput` | Single-line text input with optional `max_length` and label positioning | `String` |
| `TextAreaInput` | Multi-line text editor with scrollbar support and configurable dimensions | `String` |
| `BooleanSwitch` | Toggle switch with customizable on/off captions | `bool` |
| `ChooseOne` | Single-selection list with fuzzy filtering, hotkey shortcuts, and vim navigation | Generic `V` |
| `ChooseMany` | Multi-selection list with `min_selections` / `max_selections` constraints | `Vec<V>` |
| `InputTable` | 2D grid of heterogeneous editable cells (any component + static text) | `Vec<Row>` |

### Core Primitives

- **`EventOutcome`** — canonical result of handling any event
- **`StandaloneState`** / **`HandleEvent`** — traits for driving components to completion
- **`run_standalone`** / **`drive_event_loop`** — terminal event loops with `Ctrl-C` / `Esc` handling
- **`KeyBindings`** — fully configurable bindings (defaults include vim `h`/`j`/`k`/`l`)
- **`ComponentTheme`** — centralized visual constants (glyphs, styles, colors)
- **`FrameChrome`** / **`FrameChromeConfig`** — optional borders, margins, and height specs
- **`SplitPane`** / **`SplitDirection`** / **`SplitRatio`** — geometry-only two-pane layout primitive (`SplitPane::split(area) -> (Rect, Rect)`); a *container/layout* primitive like `FrameChrome`, not a 7th input component (captures no value, handles no input). `Auto` direction resolved from the area shape, 50/50 default; ratios clamp on construction
- **`FuzzyFilter`** — fast fuzzy search over option labels via `nucleo-matcher`
- **`ValidationState`** — uniform read access to submit-time validation errors

### Helper Utilities

- **`choice_builders`** — construct `ChoiceInput` from CSV strings, markdown lists, or YAML/JSON dictionaries

## CLI (`question`)

The `question` binary exposes each component as a subcommand:

```bash
# Single-line text input
question text-input --label "Enter your name" --max-length 50

# Multi-line editor (submits with Ctrl+S)
question text-area-input --width 80 --scrollbar

# Boolean toggle
question boolean-switch --labels "YES,NO" --initial true

# Single-select from positional arguments or stdin
question choose-one Red Green Blue
printf "%s\n" "Red" "Green" "Blue" | question choose-one

# Multi-select with constraints
question choose-many Red Green Blue --min-selections 1 --max-selections 2

# Editable grid (columns defined as JSON)
question input-table --columns '[{"type":"text","id":"name"},{"type":"boolean","id":"active"}]'
```

### Global Flags

- `--output {raw|json|null}` — serialization format (default: `raw`)
- `--height <N>` or `--height <P%>` — inline mode instead of fullscreen alternate screen

### Exit Codes

- `0` — submitted successfully
- `1` — user pressed `Esc` (cancelled)
- `130` — user pressed `Ctrl-C` (interrupted)

## Usage

### Embedded in a TUI Application

```rust
use biscuit_tui::{TextInput, TextInputState, EventOutcome, HandleEvent};
use crossterm::event::{Event, KeyCode};

let mut state = TextInputState::new()
    .with_label(Label::new("Name", LabelPosition::Above))
    .with_max_length(50);

// In your event loop:
if let Event::Key(key_event) = crossterm::event::read()? {
    match TextInput.handle_event(&mut state, key_event) {
        EventOutcome::Submitted => { /* read state.value() */ }
        EventOutcome::Cancelled => { /* handle cancel */ }
        _ => {}
    }
}

// In your draw callback:
terminal.draw(|frame| {
    frame.render_stateful_widget(TextInput, frame.area(), &mut state);
})?;
```

### Standalone Prompt

```rust
use biscuit_tui::{run_standalone, TextInput, TextInputState};

let state = TextInputState::new().with_max_length(100);
match run_standalone(TextInput, state, None) {
    Ok(value) => println!("Submitted: {value}"),
    Err(_) => eprintln!("Cancelled"),
}
```

## Architecture

```
biscuit-tui/
├── core/           # Cross-cutting primitives (events, themes, keys, frames, fuzzy, validation)
├── components/     # Per-component widgets + state structs
│   ├── text_input
│   ├── text_area_input
│   ├── boolean_switch
│   ├── choose_one
│   ├── choose_many
│   └── input_table/   # Grid with cell, column, and table submodules
└── helpers/        # Choice builders (CSV, markdown, dictionary)

question (CLI)
├── commands/       # One module per subcommand
└── output.rs       # Raw / JSON / NUL serialization
```

## Design Principles

- **Consistent API** — every component shares the same widget + state + event pattern
- **Two-tier validation** — hard limits reject at keystroke time; `required` / `min` checks at submit time
- **Typed values** — choice components are generic over `V`; `InputTable` preserves booleans and arrays
- **Testable** — `drive_event_loop` accepts any event source, enabling synthetic event injection
- **Embeddable or standalone** — components work inside larger TUIs or run as fullscreen/inline prompts

## Development

This package area uses `just` for task running. Available recipes:

```bash
just build      # build library and CLI
just test       # run tests for both crates
just lint       # clippy both crates
just install    # install the `question` binary
just cli <args> # run CLI in development mode
just docs       # build and open library docs
```

### Testing Strategy

Verification spans three rigor levels — see
[`cli/README.md` "Test Rigor"](cli/README.md#test-rigor--level-1--level-2--level-3)
for full details:

- **Level 1** (always-on) — unit tests + PTY tests with manufactured input bytes.
  Runs in default `cargo test`.
- **Level 2** (host-gated) — spawn the binary inside a real terminal
  (`wezterm cli`, `kitty @`, or `tmux`) and capture rendered pane text. Skips
  cleanly when the host lacks the required tooling.
- **Level 3** (`RUN_LEVEL3=1`) — OS-level keyboard injection via `cliclick`
  (macOS) or `xdotool` (Linux). Gated because focus stability is platform-
  specific and would otherwise produce flaky failures during a normal
  developer workflow.

The Level 2/3 harness implementations live in the shared
[`biscuit-test-harness`](../biscuit-test-harness/README.md) crate — see its
README for the harness variants, when to use each, and the environment each
requires. A modifier-press requirement covered only by Level-1 tests is
**not** "production ready" — Level 2 with kitty bytes through
`wezterm cli send-text` is the minimum for end-to-end terminal rendering
verification.

## License

AGPL-3.0-only
