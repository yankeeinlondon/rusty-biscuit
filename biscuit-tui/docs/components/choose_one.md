# Choose One

The `choose_one` component is a TUI widget that provides a vertical single-selection list. It allows users to pick exactly one item from a set of options using a list-based interface with radio-button style indicators.

## Description

The `choose_one` component is designed for scenarios where a user must make a single choice from a predefined list. It renders each option with a selection indicator (`●` for selected, `○` for unselected) and a focus marker (`▶`) on the currently hovered row. It supports advanced features like fuzzy filtering (search-on-type), hotkey navigation, and automatic scrolling.

The component is split into two parts:
- **`ChooseOne`**: A zero-sized `StatefulWidget` responsible for rendering.
- **`ChooseOneState<V>`**: The mutable state holding the options, current selection, and transient UI state (hover, scroll, filter).

`ChooseOneState::new(input)` implicitly sets `SelectionMode::Single` on the underlying `ChoiceInput<V>`. This is the default for all single-selection use cases.

## Parameters

The component is primarily configured through a `ChoiceInput<V>` struct, which is then passed to `ChooseOneState::new()`. Additional UI-only settings can be applied directly to the state.

### ChoiceInput Configuration

| Parameter | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `id` | `String` | Stable identifier for the input. | (Required) |
| `prompt` | `String` | The question or prompt text shown to the user. | (Required) |
| `help_text` | `Option<String>` | Optional help description surfaced below the prompt. When set, the text appears in a dimmed style directly under the prompt row. | `None` |
| `options` | `Vec<ChoiceOption<V>>` | The list of selectable options. | `Vec::new()` |
| `required` | `bool` | If `true`, submitting with no selection fails validation. | `false` |
| `shuffle_options` | `bool` | If `true`, randomizes option order on initialization. | `false` |
| `filter_enabled` | `bool` | Enables inline fuzzy filtering on alphanumeric input. | `false` |

### ChooseOneState Extensions

| Method | Description |
| :--- | :--- |
| `with_label(Label)` | Attaches a label (prompt) rendered relative to the list. |
| `with_theme(ComponentTheme)` | Overrides the default visual styling. |
| `with_key_bindings(KeyBindings)` | Overrides the default key mapping. |
| `with_initial_selection(&str)` | Pre-selects an option by its stable `id`. |
| `with_initial_value(&str)` | Pre-selects an option by matching its `value` field using `PartialEq`. When no `--delimiter` is used, the `value` equals the `label`, so both methods behave similarly. |

### Key Bindings (Default)

- **`Space`**: Selects the currently hovered item.
- **`Enter`**: Submits the selection and exits.
- **`Esc`**: Cancels/Aborts the interaction. (If filtering, first `Esc` closes the filter).
- **`Up` / `k`**: Moves the hover cursor up.
- **`Down` / `j`**: Moves the hover cursor down.
- **`Home` / `g`**: Jumps to the first enabled option.
- **`End` / `G`**: Jumps to the last enabled option.
- **`Alphanumeric`**: If `filter_enabled` is true, starts a fuzzy search. Otherwise, jumps to and selects the option with the matching hotkey.

## Behavioral Notes

- **Auto-Selection on Submit**: If `Enter` is pressed when no item is selected, the component automatically selects the currently hovered item before submitting (provided it is enabled and visible).
- **Fuzzy Filtering**: When active, only options matching the pattern are displayed. The hover cursor is snapped to the first visible result, and matching characters are highlighted in the labels.
- **Hotkeys**: When filtering is inactive, pressing the first character of a label (case-insensitive) jumps focus to that option and selects it immediately.
- **Disabled Options**: Options can be marked as `disabled`. They are rendered dimmed, cannot be hovered or selected, and are skipped by navigation.

## Helper Functions

The `tui_chrome::helpers::choice_builders` module provides convenience functions for constructing `ChoiceInput<String>` from common sources:

- `choose_one_from_csv(id, prompt, csv)` — builds options from a comma-separated string.
- `choose_one_from_markdown_list(id, prompt, markdown)` — builds options from a Markdown bullet or numbered list.
- `choose_one_from_dictionary(id, prompt, yaml_or_json)` — builds options from a YAML/JSON mapping where keys become labels and values become option values.

See the [CLI Reference](../cli-reference.md) and [Theming & Configuration](../theming.md) docs for cross-cutting topics.

## Usage Examples

### 1. Basic Single-Select (Strings)
A simple list of options using the default `String` value type.

```rust
use tui_chrome::components::choose::{ChoiceInput, ChoiceOption};
use tui_chrome::components::choose_one::ChooseOneState;

let input = ChoiceInput::new("color", "Pick a color")
    .with_options(vec![
        ChoiceOption::new("r", "Red", "red"),
        ChoiceOption::new("g", "Green", "green"),
        ChoiceOption::new("b", "Blue", "blue"),
    ])
    .required();

let mut state = ChooseOneState::new(input);
```

### 2. Typed Values and Pre-selection
Using a custom enum for values and setting an initial choice.

```rust
#[derive(Clone, PartialEq)]
enum Mode { Fast, Safe, Deep }

let input = ChoiceInput::new("mode", "Select analysis mode")
    .with_options(vec![
        ChoiceOption::new("f", "Fast", Mode::Fast),
        ChoiceOption::new("s", "Safe", Mode::Safe),
        ChoiceOption::new("d", "Deep", Mode::Deep),
    ]);

let mut state = ChooseOneState::new(input)
    .with_initial_selection("s"); // Pre-selects "Safe"
```

### 3. Rendering in a Widget
Implementing the render logic within a Ratatui `render` loop.

```rust
fn render(area: Rect, buf: &mut Buffer, state: &mut MyState) {
    let widget = ChooseOne::new();
    // state.choose_one is a ChooseOneState<V>
    ratatui::widgets::StatefulWidget::render(widget, area, buf, &mut state.choose_one);
}
```

## CLI Usage

The `choose_one` component is exposed via the `question choose-one` command. It writes the selected value to STDOUT.

### Common Flags

- `--options <LIST>`: Comma-separated list of simple option strings (legacy; positional args are preferred).
- `--options-from-file <PATH>`: Load options from a markdown list file.
- `--options-from-dictionary <PATH>`: Load options from a YAML/JSON mapping file.
- `--selected <VALUE>`: Pre-select a specific value.
- `--required`: Fail if no item is selected.
- `--delimiter <CHAR>`: Split each option string into `label<CHAR>value`.
- `--no-filter`: Disable the fuzzy search prompt (uses hotkeys instead).

### Global Flags

- `--output <raw|json|null>`: Serialisation format for the submitted value (`raw` is the default). `null` emits the value followed by a NUL (`\0`) terminator instead of a newline.
- `--height <CELLS_OR_PERCENT>`: Render inline at an explicit height instead of fullscreen.

### Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | Value submitted successfully. |
| `130` | User pressed `Ctrl-C` (SIGINT). |
| `1` | User pressed `Esc` to abort. |

### Positional vs `--options`

Both syntaxes are valid. When no `--options*` flag is provided, trailing positional arguments become the option list. Positional args are the modern default; `--options` exists for backward compatibility.

```bash
# Positional args (preferred)
question choose-one Apple Banana Cherry

# Legacy comma-separated flag
question choose-one --options "Apple,Banana,Cherry"
```

### Example CLI Command

```bash
# Select a server from a list
question choose-one \
  --label "Target Server" \
  --delimiter ":" \
  "Production:prod-01" \
  "Staging:stg-01" \
  "Development:dev-01"
```

## Enhancement Suggestions

1. **Category Grouping**: Add support for section headers or categories within the list to organize large sets of options.
2. **Sub-labels/Descriptions**: Support secondary hint text for each option, displayed in a dimmed style below or beside the main label.
3. **Multi-column Layout**: Support rendering options in multiple columns for long lists with short labels to optimize terminal screen real estate.
4. **"None" Option**: Add a built-in "clear selection" or "(none)" option for non-required inputs to make unselecting easier.
