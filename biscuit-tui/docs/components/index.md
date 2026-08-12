# TUI Chrome Components

The `biscuit-tui` library provides composable, Ratatui-based input widgets for terminal user interfaces. Every component follows the same pattern: a zero-sized `StatefulWidget` marker paired with a component-specific `*State` struct owned by the caller. All widgets implement `HandleEvent` so they can be driven by `run_standalone` or embedded in an application event loop.

## Getting Started

Add `biscuit-tui` to your project and import the prelude for common types:

```rust
use biscuit_tui::prelude::*;
```

The quickest way to see a component in action is via the `question` CLI (installed from the `biscuit-tui` crate):

```bash
question text-input --label "Your name"
```

For library usage, there are two integration modes:

1. **Standalone** — the component takes over the terminal for one prompt. Use [`run_standalone`](../theming.md#standalone-vs-embedded) for fullscreen or inline prompts.
2. **Embedded** — the component renders inside your own Ratatui application. You own the event loop and terminal lifecycle.

See the [CLI Reference](../cli-reference.md) for global flags and exit codes, and [Theming & Configuration](../theming.md) for shared visual and keyboard settings.

## Atomic Components

Single-purpose input widgets that capture one value type each.

| Component | CLI Command | Purpose |
| :--- | :--- | :--- |
| [Boolean Switch](boolean_switch.md) | `question boolean-switch` | Binary ON/OFF toggle with configurable labels |
| [Choose One](choose_one.md) | `question choose-one` | Single-selection list with radio-button indicators |
| [Choose Many](choose_many.md) | `question choose-many` | Multi-selection list with checkbox indicators |
| [Text Input](text_input.md) | `question text-input` | Single-line text entry with length capping |
| [Text Area Input](text_area_input.md) | `question text-area-input` | Multi-line scrollable text editor |

## Container Components

Widgets that compose or wrap other components to build more complex UIs.

| Component | CLI Command | Purpose |
| :--- | :--- | :--- |
| [FrameChrome](frame_chrome.md) | *(implicit via `--border` flags)* | Wraps any component with optional border, title, and margin |
| [SplitPane](split_pane.md) | *(library only — not a CLI command)* | Geometry-only two-pane layout primitive (`split(area) -> (Rect, Rect)`); container/layout like `FrameChrome`, not an input |
| [Input Table](input_table.md) | `question input-table` | Grid of heterogeneous editable cells (embeds atomic widgets as columns) |

## Shared Concepts

All components share these cross-cutting primitives from `biscuit_tui::core`:

- **`Label` / `LabelPosition`** — optional label rendered `Above`, `Below`, `Left`, or `Right` relative to the widget body
- **`ComponentTheme`** — centralised visual constants (colors, switch thumb character, help hint text)
- **`KeyBindings`** — configurable key mapping with vim-compatible defaults (`h/j/k/l`, `g/G`, `Enter`, `Esc`)
- **`EventOutcome`** — canonical enum returned from every component's `handle_event` method:
  - `Consumed` — the event was handled internally; the caller should redraw but take no further action
  - `Ignored` — the component did not handle the event; the caller may route it elsewhere (e.g. to a surrounding container)
  - `Submitted` — the user committed a value; the caller should read it from the state via `state.value()`
  - `Cancelled` — the user cancelled (typically `Esc` or `Ctrl-C`); the caller should tear down the component without reading a value

  Validation failures do not produce a separate variant. A component that receives a submit keystroke while its value is invalid returns `Consumed` and populates its internal validation error (retrievable via `state.validation_error()`).
- **`run_standalone`** / **`drive_event_loop`** — helpers for running a single component in a dedicated terminal session
- **`Padding`** — four-sided interior padding inside the border
- **`TerminalStyle`** / **`TerminalBackground`** / **`NerdFontStatus`** — conservative terminal capability detection

The choice components (`ChooseOne`, `ChooseMany`) also share:

- **`ChoiceOption<V>`** — individual option with `id`, `label`, `value`, `disabled` flag, and optional `hotkey`.
- **`ChoiceInput<V>`** — configuration struct for the option list, selection limits, filter settings, `orientation`, and `sort`. Defaults to `SelectionMode::Single`; `ChooseManyState::new` implicitly sets it to `Multiple`.
- **`SelectionMode`** — `Single` or `Multiple`.
- **`Orientation`** — `Vertical` (one item per row) or `Horizontal` (left-to-right, wrapping).
- **`HotkeySpec`** / **`HotkeyDisplayMode`** — explicit keyboard shortcuts (`Ctrl`/`Alt`) and when to render badges.
- **`ActiveChoiceColor`** — background colour for the actively hovered option.
- **`SortOrder`** / **`OptionSort`** — option ordering (`Natural`, `Inverse`/`Reverse`, `Asc`, `Desc`).

The `biscuit_tui::helpers` module provides builder functions for constructing choice inputs from CSV, Markdown lists, and dictionaries (`choose_one_from_csv`, `choose_many_from_markdown_list`, etc.).

## CLI Overview

The `question` CLI exposes each component as a subcommand. All subcommands share these global flags:

- **`--output <raw|json|null>`** — serialisation format for the submitted value (`raw` is the default).
- **`--height <CELLS_OR_PERCENT>`** — render inline at up to the given height (cells or percentage of terminal rows) instead of fullscreen. Treated as a maximum: clamps to the live terminal when smaller. Percentages are re-resolved on terminal resize so the inline viewport tracks the requested fraction mid-prompt.
- **`--show-input-on-exit`** — preserve the rendered prompt on exit instead of clearing it (default is fzf-style clear-on-exit). With this flag set, the cursor moves to the row just below the chrome so subsequent shell output follows the rendered border.

Exit codes: `0` on successful submission, `130` on Ctrl-C (SIGINT). For `ChooseOne`, `Esc` restores the initial selection and also exits `0`; for other components, `Esc` exits `1`.

See the [CLI Reference](../cli-reference.md) for full details on global flags, exit codes, and subcommand listings.
