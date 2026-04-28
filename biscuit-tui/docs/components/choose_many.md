# Choose Many

The `choose_many` component is a TUI widget that provides a vertical multi-selection list. It allows users to pick zero or more items from a set of options using a list-based interface with checkbox indicators.

## Description

The `choose_many` component is designed for scenarios where a user needs to select multiple items from a predefined list, such as selecting toppings for a pizza, picking tags for a blog post, or choosing which features to install. It renders each option with a checkbox glyph (`☑` for selected, `☐` for unselected) and supports advanced features like fuzzy filtering, hotkeys, and selection limits.

The component is split into two parts:
- **`ChooseMany`**: A zero-sized `StatefulWidget` responsible for rendering.
- **`ChooseManyState<V>`**: The mutable state holding the options, current selection, and transient UI state (hover, scroll, filter).

`ChooseManyState::new(input)` implicitly sets `SelectionMode::Multiple` on the underlying `ChoiceInput<V>`, enabling multi-select behaviour by default.

## Parameters

The component is primarily configured through a `ChoiceInput<V>` struct, which is then passed to `ChooseManyState::new()`. Additional UI-only settings can be applied directly to the state.

### ChoiceInput Configuration

| Parameter | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `id` | `String` | Stable identifier for the input. | (Required) |
| `prompt` | `String` | The question or prompt text shown to the user. | (Required) |
| `help_text` | `Option<String>` | Optional help description surfaced below the prompt. | `None` |
| `options` | `Vec<ChoiceOption<V>>` | The list of selectable options. | `Vec::new()` |
| `required` | `bool` | If `true`, submitting with no selection fails validation. | `false` |
| `min_selections` | `Option<usize>` | Minimum number of items that must be selected. | `None` |
| `max_selections` | `Option<usize>` | Maximum number of items allowed to be selected. | `None` |
| `shuffle_options` | `bool` | If `true`, randomizes option order on initialization. | `false` |
| `filter_enabled` | `bool` | Enables inline fuzzy filtering on alphanumeric input. | `false` |

### ChooseManyState Extensions

| Method | Description |
| :--- | :--- |
| `with_label(Label)` | Attaches a label (prompt) rendered relative to the list. |
| `with_theme(ComponentTheme)` | Overrides the default visual styling. |
| `with_key_bindings(KeyBindings)` | Overrides the default key mapping. |
| `with_initial_selection(&[&str])` | Pre-selects options by their stable `id`. |
| `with_initial_values(&[&str])` | Pre-selects options by matching their `value` field. |

### Key Bindings (Default)

- **`Space`**: Toggles the selection of the currently hovered item.
- **`Enter`**: Submits the selection and exits.
- **`Esc`**: Cancels/Aborts the interaction. (If filtering, first `Esc` closes the filter).
- **`Up` / `k`**: Moves the hover cursor up.
- **`Down` / `j`**: Moves the hover cursor down.
- **`Home` / `g`**: Jumps to the first enabled option.
- **`End` / `G`**: Jumps to the last enabled option.
- **`Ctrl+A`**: Selects all enabled options.
- **`Ctrl+D`**: Clears all selections.
- **`Alphanumeric`**: If `filter_enabled` is true, starts a fuzzy search. Otherwise, jumps to and toggles the option with the matching hotkey.

## Behavioral Notes

- **Auto-Selection on Submit**: If `Enter` is pressed when zero items are selected, the component automatically selects the currently hovered item before submitting (provided it is enabled and visible).
- **Selection Enforcement**: `max_selections` is enforced at the moment of toggling (further selections are silently blocked). `min_selections` and `required` are validated at submission time, displaying an error message if unsatisfied.
- **Fuzzy Filtering**: When active, only options matching the pattern are displayed. The hover cursor is snapped to the first visible result.
- **Disabled Options**: Options can be marked as `disabled`. They are rendered dimmed, cannot be hovered or toggled, and are skipped by `Ctrl+A`.

## Helper Functions

The `tui_chrome::helpers::choice_builders` module provides convenience functions for constructing `ChoiceInput<String>` from common sources:

- `choose_many_from_csv(id, prompt, csv)` — builds options from a comma-separated string.
- `choose_many_from_markdown_list(id, prompt, markdown)` — builds options from a Markdown bullet or numbered list.
- `choose_many_from_dictionary(id, prompt, yaml_or_json)` — builds options from a YAML/JSON mapping where keys become labels and values become option values.

See the [CLI Reference](../cli-reference.md) and [Theming & Configuration](../theming.md) docs for cross-cutting topics.

## Usage Examples

### 1. Basic Multi-Select (Strings)
A simple list of options using the default `String` value type.

```rust
use tui_chrome::components::choose::{ChoiceInput, ChoiceOption};
use tui_chrome::components::choose_many::ChooseManyState;

let input = ChoiceInput::new("toppings", "Pick your toppings")
    .with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms"),
        ChoiceOption::new("o", "Olives", "olives"),
    ])
    .required();

let mut state = ChooseManyState::new(input);
```

### 2. Typed Values and Limits
Using a custom enum for values and enforcing a selection limit.

```rust
#[derive(Clone, PartialEq)]
enum Tag { News, Tech, Life }

let input = ChoiceInput::new("tags", "Select tags (max 2)")
    .with_options(vec![
        ChoiceOption::new("1", "News", Tag::News),
        ChoiceOption::new("2", "Technology", Tag::Tech),
        ChoiceOption::new("3", "Lifestyle", Tag::Life),
    ])
    .with_max_selections(2);

let mut state = ChooseManyState::new(input);
```

### 3. Rendering in a Widget
Implementing the render logic within a Ratatui `render` loop.

```rust
fn render(area: Rect, buf: &mut Buffer, state: &mut MyState) {
    let widget = ChooseMany::new();
    // state.choose_many is a ChooseManyState<V>
    ratatui::widgets::StatefulWidget::render(widget, area, buf, &mut state.choose_many);
}
```

## CLI Usage

The `choose_many` component is exposed via the `question choose-many` command. By default, it writes the selected values (one per line) to STDOUT.

### Common Flags

- `--options <LIST>`: Comma-separated list of simple option strings (legacy; positional args are preferred).
- `--options-from-file <PATH>`: Load options from a markdown list file.
- `--options-from-dictionary <PATH>`: Load options from a YAML/JSON mapping file.
- `--selected <VALUE>`: Pre-select a value (repeatable).
- `--required`: Fail if no items are selected.
- `--min-selections <N>`: Require at least N items.
- `--max-selections <N>`: Limit to at most N items.
- `--no-filter`: Disable the fuzzy search prompt (uses hotkeys instead).

### Global Flags

- `--output <raw|json|null>`: Serialisation format for the submitted values (`raw` is the default). `null` emits each value followed by a NUL (`\0`) terminator instead of a newline.
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
question choose-many Apple Banana Cherry Date

# Legacy comma-separated flag
question choose-many --options "Apple,Banana,Cherry,Date"
```

### Example CLI Command

```bash
# Select multiple fruits with a limit of 2
question choose-many \
  --label "Pick your favorite fruits" \
  --max-selections 2 \
  Apple Banana Cherry Date
```

## Enhancement Suggestions

1. **Grouped Options**: Add support for section headers or categories within the list to organize large sets of options.
2. **Invert Selection**: Implement a `Ctrl+I` shortcut to flip the selection state of all enabled options.
3. **Selection Counter**: Display a real-time counter (e.g., `(2/5 selected)`) in the prompt or search row to help users track their progress against limits.
4. **Multi-column Layout**: Support rendering options in multiple columns for long lists with short labels to optimize terminal screen real estate.
