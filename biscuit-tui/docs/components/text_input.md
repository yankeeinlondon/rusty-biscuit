# TextInput

The `TextInput` is a single-line text input component for terminal user interfaces. It provides a lightweight way for users to type and edit short strings, such as names, identifiers, or simple commands.

## Description

`TextInput` is designed for capturing a single line of text. It wraps `tui_input` to provide a robust editing engine while exposing a clean API that integrates with the `tui-chrome` theme and label systems. It supports character length capping, validation error display, and customizable key bindings.

The component follows the standard `ratatui` stateful widget pattern, separating the rendering logic (`TextInput` widget) from the mutable buffer and configuration (`TextInputState`).

## Parameters & Defaults

When using the library, you configure the component via `TextInputState`.

| Parameter | Method | Default | Description |
|-----------|--------|---------|-------------|
| **Label** | `with_label(Label)` | `None` | An optional `Label` rendered relative to the input body. |
| **Max Length** | `with_max_length(usize)` | `None` | Caps the number of characters the user can type. |
| **Initial Value** | `with_value(&str)` | `""` | Initial text to populate the input buffer. |
| **Theme** | `with_theme(ComponentTheme)` | `Default` | Customizes colors (label, cursor, error) and the help hint. |
| **Key Bindings** | `with_key_bindings(KeyBindings)` | `Default` | Configures keys for submission (`Enter`) and cancellation (`Esc`). |

## Usage Examples

### Basic Text Input
```rust
use tui_chrome::prelude::*;

let state = TextInputState::new()
    .with_value("Initial value");

// Retrieve the value after interaction
let value: String = state.value(); 
```

### Input with Label and Length Limit
```rust
use tui_chrome::prelude::*;

let state = TextInputState::new()
    .with_label(Label::new("Username", LabelPosition::Left))
    .with_max_length(20);
```

### Standalone Runner
```rust
use tui_chrome::{run_standalone, TextInput, TextInputState};

let state = TextInputState::new().with_max_length(100);
// Runs the prompt in the terminal (fullscreen or inline)
let result = run_standalone(TextInput, state, None);
```

## Behavioral Notes

- **Cursor Placement:** When initialized with `with_value`, the cursor is automatically placed at the end of the provided text.
- **Length Enforcement:** If `max_length` is set, the component performs keystroke-time rejection. Any typed characters that would exceed the limit are silently dropped.
- **Validation:** You can set an active validation error using `state.set_validation_error("message")`. The error message renders on the row immediately below the input body. Any subsequent typing by the user automatically clears this error state.
- **Event Handling:** The component supports standard text navigation and editing keys:
    - `Left` / `Right`: Move cursor by character.
    - `Home` / `End`: Jump to start or end of the buffer.
    - `Backspace` / `Delete`: Remove characters.
    - `Enter`: Submit the value (returns `EventOutcome::Submitted`).
    - `Esc`: Cancel the interaction (returns `EventOutcome::Cancelled`).

## CLI Usage

The `text_input` component is available as a subcommand in the `question` CLI tool. It writes the submitted value to `stdout` and uses exit codes to indicate success or cancellation.

```bash
# Basic usage
question text-input --label "Enter your username"

# Pre-filled with a character limit and specific output format
question text-input --label "Code" --initial "ABC" --max-length 3 --output json
```

### CLI Arguments
- `--label <TEXT>`: Text to display as a label.
- `--label-position <above|below|left|right>`: Position of the label relative to the input (default: `above`).
- `--max-length <COUNT>`: Maximum number of characters the user is allowed to type.
- `--initial <TEXT>`: Initial value to load into the input buffer.
- `--output <raw|json|null>`: How to format the result on `stdout`.

## Functional Enhancement Suggestions

1.  **Masked Input (Password Mode):** Implement a "sensitive" mode where typed characters are rendered as placeholders (e.g., `*` or `•`). This would allow the component to be used for passwords or API tokens without exposing them on the screen.
2.  **Placeholder Text:** Support rendering dimmed "hint" text (e.g., "Enter name...") when the input buffer is empty. This provides better UX than a completely blank line.
3.  **Input Masking & Formatting:** Add support for predefined masks (e.g., `(###) ###-####` for phone numbers) that automatically insert formatting characters and restrict input to specific types (e.g., numeric only).
4.  **Autocomplete / History:** Integrate a history buffer or a static suggestion list that users can cycle through using `Up` / `Down` keys, similar to a shell prompt.
