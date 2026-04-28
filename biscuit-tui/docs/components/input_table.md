# InputTable

The `InputTable` is a grid-based form component for terminal user interfaces. It allows you to arrange multiple rows and columns of heterogeneous editable cells, making it ideal for complex data entry tasks where a single flat list of prompts is insufficient.

## Description

`InputTable` manages a 2D matrix of input widgets. Each column in the table is configured with a specific type, and every row contains a corresponding cell. The component supports a variety of cell types, including single-line text, multi-line text areas, boolean switches (checkboxes), and single or multi-choice selections.

It follows the standard `ratatui` stateful widget pattern, separating the rendering logic (`InputTable` widget) from the schema and data management (`InputTableState`).

`InputTableState` implements the [`StandaloneState`](../../theming.md#standalonestate) trait with `Value = Vec<Row>`, so it can be driven by [`run_standalone`](../../theming.md#run_standalone) or embedded in a larger application event loop.

## Parameters & Defaults

Configuration is handled via `InputTableState`.

| Parameter | Method | Default | Description |
|-----------|--------|---------|-------------|
| **Columns** | `new(columns, ...)` | Required | A vector of `InputTableColumn` definitions defining the table schema. |
| **Initial Rows**| `new(..., rows)` | Required | A vector of `Row` objects seeding the table with initial values. |
| **Blank Rows** | `with_blank_rows(cols, n)` | N/A | Convenience constructor for `n` empty rows based on column defaults. |
| **Theme** | `with_theme(ComponentTheme)`| `Default` | Customizes colors and the help hint (default: "Ctrl+S=Submit Esc=Cancel"). |
| **Key Bindings**| `with_key_bindings(KeyBindings)`| `Default` | Configures keys for submission (`Ctrl+S`) and cancellation (`Esc`). |
| **Submit Key** | `with_submit_key(Code, Mod)`| `Ctrl+S` | Convenience method to override only the submission key. |

### Column Types
- **StaticText**: Non-editable display text.
- **TextInput**: Single-line text entry.
- **BooleanSwitch**: Toggleable ON/OFF switch.
- **TextAreaInput**: Multi-line text entry.
- **ChooseOne**: Single-selection from a list of options.
- **ChooseMany**: Multi-selection from a list of options.

## Usage Examples

### Basic Table with Blank Rows
```rust
use tui_chrome::prelude::*;
use tui_chrome::components::input_table::{
    BooleanSwitchConfig, InputTable, InputTableColumn, InputTableState,
    TextInputConfig,
};

let columns = vec![
    InputTableColumn::StaticText { id: "idx".into(), text: "1".into() },
    InputTableColumn::TextInput { id: "name".into(), config: TextInputConfig::default() },
    InputTableColumn::BooleanSwitch { id: "enabled".into(), config: BooleanSwitchConfig::default() },
];

let mut state = InputTableState::with_blank_rows(columns, 5);
```

### Initializing with Data
```rust
use tui_chrome::components::input_table::{CellValue, InputTableState, Row, RowCell};

let columns = vec![/* ... column definitions ... */];
let initial_rows = vec![
    Row::new(vec![
        RowCell::new("name", CellValue::Text("Alice".into())),
        RowCell::new("enabled", CellValue::Boolean(true)),
    ]),
];
let state = InputTableState::new(columns, initial_rows);

// Access values after interaction
let values: &[Row] = state.value();
```

### `RowCell` and `CellValue`

Each cell in a row is a `RowCell { column_id, value }`. The `column_id` must match the `id` field of the corresponding `InputTableColumn`. `CellValue` preserves the semantic type of each cell:

| Variant | Source Column | Description |
| :--- | :--- | :--- |
| `CellValue::StaticText(String)` | `StaticText` | Display-only text. |
| `CellValue::Boolean(bool)` | `BooleanSwitch` | Toggle state. |
| `CellValue::Text(String)` | `TextInput` | Single-line text. |
| `CellValue::TextArea(Vec<String>)` | `TextAreaInput` | Multi-line text (one string per line). |
| `CellValue::ChosenOne(Option<String>)` | `ChooseOne` | Selected option value, or `None`. |
| `CellValue::ChosenMany(Vec<String>)` | `ChooseMany` | Selected option values in option order. |

### Standalone Runner
```rust
use tui_chrome::{run_standalone, InputTable, InputTableState};

let columns = vec![/* ... */];
let state = InputTableState::with_blank_rows(columns, 3);
let result = run_standalone(InputTable::new(), state, None);
```

## Behavioral Notes

- **Navigation:**
    - `Up` / `Down`: Move focus between rows (except inside choice cells; use `Alt+Up/Down` or `Tab`).
    - `Left` / `Right`: Move focus between columns (except inside text cells where they control the cursor).
    - `Tab` / `Shift+Tab`: Cycle through all focusable cells in a wrapping row-major order.
    - `Alt+Up` / `Alt+Down`: Force row navigation regardless of cell type.
- **Validation Aggregation:** When submission is attempted (`Ctrl+S`), the table validates every cell. If any cell has an error (e.g., a required choice is unset), the focus is automatically moved to the first offending cell, and a global error message is displayed.
- **Scrolling:** The table automatically handles vertical scrolling and renders overflow indicators (▲/▼) when the number of rows exceeds the available height.
- **Type Safety:** The `value()` method returns typed `CellValue` variants (e.g., `Boolean`, `Text`, `ChosenMany`), preserving semantic data types rather than flattening everything to strings.

## CLI Usage

The `input-table` component is available as a subcommand in the `question` CLI tool. It accepts JSON-encoded schema and row definitions.

```bash
# Complex form with mixed inputs
question input-table \
  --columns '[
    {"type": "static-text", "text": "ID"},
    {"type": "text-input", "id": "name", "initial": "New User"},
    {"type": "choose-one", "id": "role", "options": ["Admin", "User", "Guest"]}
  ]' \
  --rows '[["1", "Alice", "Admin"], ["2", "Bob", "User"]]'
```

### CLI Arguments
- `--columns <JSON>`: A JSON array of column objects. Each object requires a `type` and can optionally include an `id`, `initial` value, or type-specific configuration (like `options` for choice cells).
- `--rows <JSON>`: (Optional) A JSON array of arrays, where each inner array provides initial values for a row, matching the column order.

The CLI outputs a JSON array of row objects upon successful submission.

### Global Flags
- `--output <raw|json|null>`: Serialisation format for the submitted values (`json` is the default for `input-table`).
- `--height <CELLS_OR_PERCENT>`: Render inline at an explicit height instead of fullscreen.

### Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | Value submitted successfully. |
| `130` | User pressed `Ctrl-C` (SIGINT). |
| `1` | User pressed `Esc` to abort. |

## Functional Enhancement Suggestions

1.  **Dynamic Row Management:** Add support for interactive row insertion and deletion (e.g., `Ctrl+N` for new row, `Ctrl+D` to delete current row) to allow users to build lists of arbitrary length.
2.  **Column Sorting & Filtering:** Implement the ability to sort the table by a specific column or filter rows based on a search string, improving usability for large datasets.
3.  **Cell-Level Tooltips:** Add support for "hint" or "description" text that appears in the help bar or a popup when a specific cell is focused, guiding the user on valid input for that field.
4.  **Conditional Styling:** Allow for styling rules that change the appearance of a cell or row based on its value (e.g., highlighting a row in red if a boolean "Urgent" field is checked).
