# Component Documentation Review

## Component Coverage

All source-level components now have documentation, including `FrameChrome` which was initially missed and has since been added.

| Source File | Doc File | Status |
| :--- | :--- | :--- |
| `lib/src/components/boolean_switch.rs` | `docs/components/boolean_switch.md` | Present |
| `lib/src/components/choose_one.rs` | `docs/components/choose_one.md` | Present |
| `lib/src/components/choose_many.rs` | `docs/components/choose_many.md` | Present |
| `lib/src/components/text_input.rs` | `docs/components/text_input.md` | Present |
| `lib/src/components/text_area_input.rs` | `docs/components/text_area_input.md` | Present |
| `lib/src/components/input_table/` | `docs/components/input_table.md` | Present |
| `lib/src/core/frame.rs` (`FrameChrome`) | `docs/components/frame_chrome.md` | Present |

## Per-Document Review

### boolean_switch.md

- **Missing: `--output` flag documentation.** The source (`cli/src/commands/boolean_switch.rs`) accepts the global `--output` flag but the CLI section of the doc does not mention it. The other component docs have the same gap, though `text_input.md` is the only one that mentions `--output json`.
- **Missing: exit code behavior.** Exit code `130` (Ctrl-C) and `1` (Esc/abort) are not documented.
- **Minor: `--labels` parsing note.** The CLI uses `splitn(2, ',')` and trims whitespace, which means `--labels "YES,NO"` works but so does `--labels " enabled , disabled "`. The doc doesn't mention the trimming behavior or that missing the comma-delimited off-label is a hard error.

### choose_one.md

- **Missing: `SelectionMode` is never explicitly mentioned.** The source uses `ChoiceInput<V>` which has a `selection_mode` field defaulting to `Single`. The doc doesn't explain that `ChooseOneState::new(input)` implicitly sets `SelectionMode::Single`.
- **Missing: `with_initial_value` vs `with_initial_selection` distinction could be clearer.** Both are listed in the table but there's no explanation of the difference (one matches by `id`, the other by the `value` field's string representation via `PartialEq`).
- **Missing: `help_text` usage.** `ChoiceInput` has a `help_text` field but the doc doesn't show how it's rendered or where it appears.

### choose_many.md

- **Same `SelectionMode` gap as choose_one.** Should note it implicitly sets `SelectionMode::Multiple`.
- **Missing: `Ctrl+A` / `Ctrl+D` are in the default key bindings for `KeyBindings` (see `keybindings.rs:74-75`) and ARE documented here, which is good.
- **Minor inconsistency: `--options` vs positional args.** The CLI example uses positional args (`Apple Banana Cherry Date`) but the flags table lists `--options <LIST>` as comma-separated. It's unclear whether both syntaxes are valid.

### text_input.md

- **Good: the only component doc that mentions `--output` formats.** However it only shows `--output json` and doesn't document the `null` mode (NUL-terminated output).

### text_area_input.md

- **Missing: `--output` flag entirely.** The CLI section lists `--width`, `--scrollbar`, etc., but doesn't mention that `--output` controls the format. Since text area can produce multi-line content, the interaction with `--output null` (NUL-terminated) is particularly relevant.
- **Missing: `height` parameter is not a `TextAreaInputState` configuration.** The doc doesn't mention how the editor height is determined. The source (`cli/src/commands/text_area_input.rs:110-116`) shows it defaults to 10 rows and uses `--height` from the CLI global flag when specified as cells.

### input_table.md

- **Well-structured.** Covers column types, navigation, and validation aggregation thoroughly.
- **Missing: `StandaloneState` trait relationship.** `InputTableState` implements `StandaloneState` with `Value = Vec<Row>`, but this isn't mentioned.
- **Missing: `RowCell` construction details.** The example shows `RowCell::new("name", CellValue::Text(...))` but doesn't enumerate all `CellValue` variants or explain the `id` matching.

## Cross-Cutting Documentation Gaps

### 1. Global CLI flags not consistently documented

All component docs should mention:
- `--output <raw|json|null>` — the serialisation format
- `--height <CELLS_OR_PERCENT>` — inline height control
- Exit codes: `0` (submitted), `130` (Ctrl-C), `1` (Esc/abort)

Currently only `text_input.md` mentions `--output` and none document exit codes.

### 2. `helpers` module undocumented at component level

The `tui_chrome::helpers::choice_builders` module provides `choose_one_from_csv`, `choose_one_from_markdown_list`, `choose_one_from_dictionary` (and `choose_many_*` equivalents). These are valuable library integration helpers but none of the choice component docs mention them. At minimum, `choose_one.md` and `choose_many.md` should link to these builders.

### 3. Shared `choose.rs` types undocumented

The `choose.rs` module defines `ChoiceOption<V>` and `ChoiceInput<V>` which are the backbone of both choice components. Both component docs describe these parameters in their own sections, but there's no standalone documentation of the shared type system. Consider a brief section in `index.md` or a dedicated `choose_types.md`.

### 4. `Label` and `LabelPosition` details assumed

Every component supports `.with_label(Label::new(..., LabelPosition::*))` but none of the docs explain the `Label` API in detail. The available positions (`Above`, `Below`, `Left`, `Right`) are shown in examples but never enumerated in a shared reference.

### 5. `ComponentTheme` and `KeyBindings` customization is superficially documented

Each component doc lists `with_theme(ComponentTheme)` and `with_key_bindings(KeyBindings)` as builder methods, but the actual fields of these structs are never documented. A user would need to read the source to know what `ComponentTheme` exposes (e.g., `label_style`, `selected_style`, `switch_thumb`, `help_hint`). Consider a shared "Theming & Key Bindings" reference doc.

### 6. `FrameChrome` was initially missed

`FrameChrome` lives in `lib/src/core/frame.rs` rather than `lib/src/components/`, which made it easy to overlook during the initial documentation sweep. It has since been documented. Future reviews should check both `components/` and `core/` for user-facing widgets.

### 7. `run_standalone` vs embedded usage pattern

The docs show `run_standalone` usage in some examples and raw `StatefulWidget::render` in others, but never clearly explain the two integration modes. A brief section in `index.md` covering "Standalone vs Embedded" usage would help new users.

## Structural Suggestions

1. **Add a "Getting Started" section to `index.md`** that walks through the basic library import pattern (`use tui_chrome::prelude::*`) and the two integration modes.
2. **Create a shared "CLI Reference" doc** (`docs/cli-reference.md`) that documents the global `--output`, `--height` flags and exit codes once, so individual component docs don't need to repeat them.
3. **Create a "Theming & Configuration" doc** (`docs/theming.md`) that documents `ComponentTheme`, `KeyBindings`, `Label`, and `LabelPosition` in one place, linked from each component doc.
