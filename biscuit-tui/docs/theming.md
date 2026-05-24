# Theming & Configuration

This page documents the shared configuration types used by every `tui-chrome` component: [`Label`](#label), [`LabelPosition`](#labelposition), [`ComponentTheme`](#componenttheme), and [`KeyBindings`](#keybindings).

## `Label`

A [`Label`](https://docs.rs/tui-chrome/latest/tui_chrome/core/struct.Label.html) pairs display text with a [`LabelPosition`](#labelposition) describing where it renders relative to the component body.

```rust
use tui_chrome::{Label, LabelPosition};

let label = Label::new("Username", LabelPosition::Left);
```

### Behaviour

- **Vertical labels** (`Above`, `Below`) occupy one full row. If the available area is too small (height `< 2`), the label is silently dropped.
- **Horizontal labels** (`Left`, `Right`) reserve width equal to the label's display width plus a single spacer column. If the area is too narrow, the label is silently dropped.
- Labels are styled with `ComponentTheme::label_style`.

## `LabelPosition`

The four supported label positions:

| Variant | Description |
| :--- | :--- |
| `Above` | Renders on the row above the component. |
| `Below` | Renders on the row below the component. |
| `Left` | Renders to the left of the component on the same row, separated by one space. |
| `Right` | Renders to the right of the component on the same row, separated by one space. |

## `ComponentTheme`

[`ComponentTheme`](https://docs.rs/tui-chrome/latest/tui_chrome/core/struct.ComponentTheme.html) centralises the glyphs and styles used across every component. Each state struct owns its own theme instance, which can be mutated at runtime or replaced wholesale via `.with_theme(...)`.

### Fields

| Field | Default | Description |
| :--- | :--- | :--- |
| `focus_indicator` | `"▶"` | Prefix glyph for the currently focused/hovered row. |
| `selected_indicator` | `"●"` | Glyph for a selected option (filled radio / checkbox). |
| `unselected_indicator` | `"○"` | Glyph for an unselected option (empty radio / checkbox). |
| `switch_on` | `" ON "` | Text on the "on" side of a `BooleanSwitch` track. |
| `switch_off` | `" OFF"` | Text on the "off" side of a `BooleanSwitch` track. |
| `switch_thumb` | `'●'` | Character painted inside the active side of a switch. |
| `cursor_style` | `REVERSED` | Style applied to the character beneath the cursor in `TextInput`. |
| `selected_style` | `Black on Cyan, BOLD` | Style for the selected option row in choice lists. |
| `error_style` | `Red, BOLD` | Style for inline validation error messages. |
| `label_style` | `BOLD` | Style for rendered label text. |
| `disabled_style` | `DarkGray, DIM` | Style for disabled options. |
| `selected_label_style` | `Cyan, UNDERLINED` | Style for the label of a selected but non-hovered option. |
| `overflow_up_indicator` | `"▲"` | Glyph at the top of a list when content is scrolled above the viewport. |
| `overflow_down_indicator` | `"▼"` | Glyph at the bottom of a list when content is scrolled below the viewport. |
| `help_hint` | `"Enter=Submit  Esc=Cancel"` | One-line footer rendered at the bottom of standalone components. Set to empty to suppress. |
| `search_indicator` | `"/ "` | Prefix glyph for the inline fuzzy search prompt row. |
| `search_style` | `default` | Style for the search prompt row as a whole. |
| `search_match_style` | `Cyan, BOLD` | Style for per-character matches within option labels when filtering. |
| `no_matches_text` | `"(no matches)"` | Text shown when the fuzzy filter matches no options. |
| `no_matches_style` | `DIM` | Style for the "no matches" row. |

### Usage

```rust
use tui_chrome::ComponentTheme;

let mut theme = ComponentTheme::default();
theme.help_hint = "Space=Toggle  Enter=Submit  Esc=Cancel".to_string();

let state = TextInputState::new().with_theme(theme);
```

## `KeyBindings`

[`KeyBindings`](https://docs.rs/tui-chrome/latest/tui_chrome/core/struct.KeyBindings.html) is a plain struct of `Vec<KeyEvent>` per logical action. Components match incoming `KeyEvent`s against the binding lists and fall through to `EventOutcome::Ignored` when no binding matches.

### Default Bindings

| Action | Default Keys |
| :--- | :--- |
| `up` | `↑`, `k` |
| `down` | `↓`, `j` |
| `left` | `←`, `h` |
| `right` | `→`, `l` |
| `toggle` | `Space` |
| `submit` | `Enter` |
| `cancel` | `Esc` |
| `select_all` | `Ctrl+A` |
| `deselect_all` | `Ctrl+D` |

### Customising Bindings

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_chrome::KeyBindings;

let bindings = KeyBindings {
    submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
    ..KeyBindings::default()
};

let state = TextInputState::new().with_key_bindings(bindings);
```

## Standalone vs Embedded

`tui-chrome` components support two integration patterns:

### Standalone

The component owns the terminal for the duration of a single prompt. Use [`run_standalone`](https://docs.rs/tui-chrome/latest/tui_chrome/fn.run_standalone.html) or [`run_standalone_with_chrome`](https://docs.rs/tui-chrome/latest/tui_chrome/fn.run_standalone_with_chrome.html):

```rust
use tui_chrome::{run_standalone, TextInput, TextInputState};

let state = TextInputState::new();
let result = run_standalone(TextInput::new(), state, None);
```

- `None` as the height runs fullscreen (alternate screen).
- `Some(HeightSpec::Cells(10))` runs inline for up to 10 rows; ratatui's `autoresize` clamps the viewport to the live terminal height when smaller.
- `Some(HeightSpec::Percent(50))` runs inline for 50% of the terminal height (floor of 3 rows). The percentage is re-resolved on every terminal resize, so the inline viewport tracks the requested fraction as the terminal grows or shrinks mid-prompt.
- The runner handles raw mode, terminal restoration, and exit-code mapping.

### Embedded

The component renders inside a larger Ratatui application that you control:

```rust
use ratatui::widgets::StatefulWidget;

fn render(area: Rect, buf: &mut Buffer, state: &mut MyState) {
    let widget = TextInput::new();
    StatefulWidget::render(widget, area, buf, &mut state.text_input);
}
```

In embedded mode you are responsible for:
- Driving the event loop (or delegating events to `component.handle_event(state, key)`).
- Decoding `EventOutcome` variants (`Consumed`, `Ignored`, `Submitted`, `Cancelled`).
- Redrawing the terminal when events are consumed or on resize.

See the [CLI Reference](cli-reference.md) for exit-code conventions and global flag behaviour.
