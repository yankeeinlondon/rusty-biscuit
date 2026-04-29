# Choose One

The `choose_one` component is a TUI widget that provides a single-selection list. It allows users to pick exactly one item from a set of options using a list-based interface with radio-button style indicators. Options can be arranged vertically (one per row) or horizontally (left-to-right, wrapping).

## Description

The `choose_one` component is designed for scenarios where a user must make a single choice from a predefined list. It renders each option with a selection indicator (Nerd Font `󰐱`/`󰄱` when detected, otherwise `●`/`○`) and a focus marker (`▶`) on the currently hovered row in vertical mode. In horizontal mode the active option is highlighted with a background colour instead. It supports advanced features like fuzzy filtering (search-on-type), explicit Ctrl/Alt hotkeys, first-letter shortcuts, and automatic scrolling.

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
| `orientation` | `Orientation` | Layout direction (`Vertical` or `Horizontal`). | `Vertical` |
| `sort` | `Option<SortOrder>` | Optional ordering applied before state construction. | `None` |

### ChooseOneState Extensions

| Method | Description |
| :--- | :--- |
| `with_label(Label)` | Attaches a label (prompt) rendered relative to the list. |
| `with_theme(ComponentTheme)` | Overrides the default visual styling. |
| `with_key_bindings(KeyBindings)` | Overrides the default key mapping. |
| `with_initial_selection(&str)` | Pre-selects an option by its stable `id`. |
| `with_initial_value(&str)` | Pre-selects an option by matching its `value` field using `PartialEq`. When no `--delimiter` is used, the `value` equals the `label`, so both methods behave similarly. |

### Key Bindings (Default)

- **`Space`**: Selects the currently hovered item (without submitting).
- **`Enter`**: Selects the currently hovered enabled item and submits.
- **`Esc`**: Restores the initial selection and submits. (If filtering, first `Esc` closes the filter; second `Esc` restores and submits).
- **`Up` / `k`**: Moves the hover cursor up (vertical) or to the closest column in the row above (horizontal).
- **`Down` / `j`**: Moves the hover cursor down (vertical) or to the closest column in the row below (horizontal).
- **`Left` / `h`**: Moves to the previous option.
- **`Right` / `l`**: Moves to the next option.
- **`Home` / `g`**: Jumps to the first enabled option.
- **`End` / `G`**: Jumps to the last enabled option.
- **`Ctrl+<char>`**: Selects the option with the matching explicit `Ctrl` hotkey and submits.
- **`Alt+<char>`**: Selects the option with the matching explicit `Alt` hotkey and submits.
- **`Alphanumeric`**: If `filter_enabled` is true, starts a fuzzy search. Otherwise, jumps to and selects the option with the matching first-letter hotkey.

## Behavioral Notes

- **Enter Behavior**: `Enter` always selects the currently hovered enabled item and submits. There is no automatic selection of the hovered item on submit; the user must explicitly select with `Space` or `Enter`.
- **Esc Behavior**: `Esc` restores the selection to whatever it was when the component started (the `initial_selected` value) and then submits with exit code `0`. If the user navigated or changed the selection with `Space`, those changes are discarded. This makes `Esc` a "reset and submit" action, not a cancel.
- **Fuzzy Filtering**: When active, only options matching the pattern are displayed. The hover cursor is snapped to the first visible result, and matching characters are highlighted in the labels.
- **First-Letter Hotkeys**: When filtering is inactive, pressing the first character of a label (case-insensitive) jumps focus to that option and selects it immediately.
- **Explicit Hotkeys**: Options can carry explicit `Ctrl` or `Alt` hotkeys (e.g., `[CTRL+R]`). These are parsed from option text and select + submit when pressed.
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

**Option sources (mutually exclusive):**
- Positional arguments — `question choose-one Apple Banana Cherry`
- `--csv <TEXT>` — comma-separated list
- `--list <TEXT>` — newline-separated list
- `--rows <TEXT>` — newline-separated `label::value` pairs
- `--file <PATH>` — JSON, JSONL, NDJSON, YAML, TOML, or CSV file containing an array
- `--md <PATH> <PROP>` — YAML frontmatter array property from a Markdown file
- `--options <TEXT>` — hidden alias for `--csv` (backward compatibility)
- Piped stdin (automatic when stdin is not a TTY)

**Selection & filtering:**
- `--selected <VALUE>`: Pre-select a specific value.
- `--required`: Fail if no item is selected.
- `--delimiter <CHAR>`: Split each option string into `label<CHAR>value`.
- `--no-filter`: Disable fuzzy search (use hotkey shortcuts instead).
- `--sort <natural|inverse|asc|desc>`: Reorder options before display. `reverse` is a hidden alias for `inverse`.

**Hotkeys & normalization:**
- `--numeric-hot-keys`: Auto-assign Ctrl+1..9,0 then Alt+1..9,0 to the first 20 options.
- `--label-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`: Transform option labels.
- `--value-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`: Transform option values.
- `::` delimiter in option text splits `label::value` (takes precedence over conventions).
- `[CTRL+X]`, `[ALT+X]`, `[OPT+X]` prefixes in option text assign explicit hotkeys.

**Chrome:**
- `--border`, `--border-label <TEXT>`, `--border-style <STYLE>`: Border chrome.
- `--margin <N>`, `--mt <N>`, `--mb <N>`, `--ml <N>`, `--mr <N>`: Outer margin.
- `--padding <N>` / `-p <N>`, `--pt <N>`, `--pb <N>`, `--pl <N>`, `--pr <N>`: Inner padding.

### Global Flags

- `--output <raw|json|null>`: Serialisation format for the submitted value (`raw` is the default). `null` emits the value followed by a NUL (`\0`) terminator instead of a newline.
- `--height <CELLS_OR_PERCENT>`: Render inline at an explicit height instead of fullscreen.

### Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | Value submitted successfully (including `Esc`, which restores the initial selection). |
| `130` | User pressed `Ctrl-C` (SIGINT). |

### Positional vs `--csv`

Positional arguments are the modern default. `--options` is a hidden backward-compatibility alias for `--csv`.

```bash
# Positional args (preferred)
question choose-one Apple Banana Cherry

# Comma-separated flag
question choose-one --csv "Apple,Banana,Cherry"
```

### Example CLI Commands

```bash
# Select a server from a list
question choose-one \
  --label "Target Server" \
  --delimiter ":" \
  "Production:prod-01" \
  "Staging:stg-01" \
  "Development:dev-01"

# Fuzzy search with inverse sort and numeric hotkeys
question choose-one --csv "Apple,Banana,Cherry" --sort inverse --numeric-hot-keys

# Load options from a file with padding
question choose-one --file options.json --padding 2 --border

# Label::value pairs with convention transforms
question choose-one --rows $'Red::apple\nGreen::pear' --label-convention title-case
```

## Enhancement Suggestions

1. **Category Grouping**: Add support for section headers or categories within the list to organize large sets of options.
2. **Sub-labels/Descriptions**: Support secondary hint text for each option, displayed in a dimmed style below or beside the main label.
3. **Multi-column Layout**: Support rendering options in multiple columns for long lists with short labels to optimize terminal screen real estate.
4. **"None" Option**: Add a built-in "clear selection" or "(none)" option for non-required inputs to make unselecting easier.
