# TextAreaInput

The `TextAreaInput` is a multi-line text editing component for terminal user interfaces. It provides a robust, scrollable editor with support for labels, validation errors, and customizable key bindings.

## Description

`TextAreaInput` is designed for capturing longer blocks of text (e.g., notes, descriptions, or configuration snippets). It separates the rendering logic (`TextAreaInput` widget) from the mutable edit buffer and configuration (`TextAreaInputState`). 

Unlike single-line inputs, `TextAreaInput` treats the `Enter` key as a "newline" command rather than a submission trigger. By default, it uses `Ctrl-S` for submission and `Esc` for cancellation.

## Parameters & Defaults

When using the library, you configure the component via `TextAreaInputState`.

| Parameter | Method | Default | Description |
|-----------|--------|---------|-------------|
| **Width** | `new(width, ...)` | `60` | The preferred width of the editor in terminal cells. |
| **Height** | `new(..., height)` | `10` | The preferred height of the editor in terminal cells. |
| **Label** | `with_label(Label)` | `None` | An optional `Label` rendered relative to the editor body. |
| **Scrollbar** | `with_scrollbar(bool)` | `false` | If `true`, a vertical scrollbar overlay appears when content exceeds the viewport height. |
| **Initial Value** | `with_value(&[String])` | `[""]` | A slice of strings to initialize the editor buffer. |
| **Theme** | `with_theme(Theme)` | `Default` | Customizes colors, styles, and the help hint text. |
| **Submit Key** | `with_submit_key(...)` | `Ctrl-S` | The keystroke used to complete editing and return the value. |
| **Key Bindings** | `with_key_bindings(...)` | `Default` | Full control over submit and cancel key combinations. |

## Usage Examples

### Basic Multi-line Input

```rust
use tui_chrome::prelude::*;

let state = TextAreaInputState::new(60, 5)
    .with_value(&["Initial line"]);

// Value retrieval
let content: String = state.value(); 
```

### Advanced Configuration with Label and Scrollbar

```rust
use tui_chrome::prelude::*;

let state = TextAreaInputState::new(80, 15)
    .with_label(Label::new("Comments", LabelPosition::Above))
    .with_scrollbar(true)
    .with_submit_key(KeyCode::F(2), KeyModifiers::NONE);
```

### Rendering as a Widget

```rust
use ratatui::widgets::StatefulWidget;

// Inside your render loop:
frame.render_stateful_widget(TextAreaInput::new(), area, &mut state);
```

## Behavioral Notes

- **Submission vs. Newline:** Pressing `Enter` always inserts a newline. Users must use the submit key (default `Ctrl-S`) to finish editing.
- **Cursor Management:** When initialized with `with_value`, the cursor is automatically moved to the end of the content.
- **Scrollbar Logic:** The scrollbar only renders if `with_scrollbar(true)` is set **and** the number of lines in the buffer exceeds the height of the rendering area.
- **Validation:** You can set an active validation error using `state.set_validation_error("message")`. The error will be rendered below the editor body, and any subsequent typing will automatically clear the error.

## CLI Usage

The `text_area_input` component is available as a subcommand in the `question` CLI (part of `biscuit-tui`).

```bash
# Basic usage
question text-area-input --label "Description"

# Pre-filled with content and scrollbar enabled
question text-area-input --label "Notes" --initial "Line 1\nLine 2" --scrollbar --width 40
```

### CLI Arguments
- `--label <TEXT>`: Text to display as a label.
- `--label-position <above|below|left|right>`: Where to put the label (default: `above`).
- `--width <CELLS>`: Preferred width of the editor (default: `60`).
- `--scrollbar`: Enable the vertical scrollbar.
- `--initial <TEXT>`: Initial text to load into the editor. Use `\n` for line breaks.

## Functional Enhancement Suggestions

1. **Syntax Highlighting:** Integrate with a library like `syntect` to provide real-time syntax highlighting for common formats like Markdown, YAML, or Rust.
2. **Line Numbers:** Add an option to render line numbers in a gutter on the left side of the editor.
3. **Search and Replace:** Implement a modal or hotkey-driven interface for finding and replacing text within the buffer.
4. **Auto-indentation:** Add basic logic to preserve the indentation level of the previous line when the user presses `Enter`.
