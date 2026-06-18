---
phases: 7
start_phase: 7
source_files_during_phase_0:
  - Cargo.toml
  - CLAUDE.md
  - biscuit-tui/lib/Cargo.toml
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/helpers/mod.rs
  - biscuit-tui/cli/Cargo.toml
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/justfile
docs_updated_during_phase_0:
  - CLAUDE.md
docs_created_during_phase_0: []
skills_files_updated_during_phase0: []
source_files_during_phase_1:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/core/event.rs
  - biscuit-tui/lib/src/core/validation.rs
  - biscuit-tui/lib/src/core/label.rs
  - biscuit-tui/lib/src/core/theme.rs
  - biscuit-tui/lib/src/core/keybindings.rs
  - biscuit-tui/lib/src/core/standalone.rs
  - biscuit-tui/lib/src/prelude.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/components/text_input.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/output.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/text_input.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/components/boolean_switch.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/boolean_switch.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - biscuit-tui/lib/Cargo.toml
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/components/choose.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/helpers/mod.rs
  - biscuit-tui/lib/src/helpers/choice_builders.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/output.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/components/text_area_input.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/text_area_input.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
source_files_during_phase_6:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/components/input_table/mod.rs
  - biscuit-tui/lib/src/components/input_table/column.rs
  - biscuit-tui/lib/src/components/input_table/cell.rs
  - biscuit-tui/lib/src/components/input_table/table.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/input_table.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase6: []
source_files_during_phase_7:
  - biscuit-tui/lib/src/core/standalone.rs
  - biscuit-tui/lib/src/components/choose_many.rs
docs_updated_during_phase_7:
  - docs/dependencies.md
docs_created_during_phase_7: []
skills_files_updated_during_phase7: []
packages:
  - biscuit-tui
  - biscuit-tui-cli
---
# TUI Inputs — Implementation Plan

Source documents: [spec.md](./spec.md) | [tech-design.md](./tech-design.md) | [strategy.md](./strategy.md) | [research.md](./research.md)

---

## Current State

| Item | Status |
|------|--------|
| Workspace `Cargo.toml` | Correctly lists `"biscuit-tui/lib"` and `"biscuit-tui/cli"` as members. Both resolve. |
| `biscuit-tui/lib/Cargo.toml` | Exists, `name = "biscuit-tui"`, edition 2024. No dependencies, no source files. |
| `biscuit-tui/lib/src/` | Empty directory. |
| `biscuit-tui/cli/` | Has `Cargo.toml` (`name = "biscuit-tui-cli"`, binary `question`) and `src/main.rs`. Compiles but has no subcommands. |
| `biscuit-tui/justfile` | Copied from Claudine — references "Claudine" throughout. Needs rewrite for biscuit-tui. |
| Root `CLAUDE.md` | States `tui` is a "single crate" — contradicts lib/cli split design. Needs update. |
| Community crate strategy | Finalized in [strategy.md](./strategy.md). Adopt as private deps only. |

---

## Phase 0 — Workspace Scaffolding

**Goal:** Both crates compile (empty lib + minimal CLI `question --help`), workspace is consistent, justfile is usable.

### Tasks

1. **Update root `CLAUDE.md`** — change `tui` from "single crate" to `lib/cli` split, note binary name `question`, note the area is not covered by root justfile `areas` list.

2. **Fill `biscuit-tui/lib/Cargo.toml` dependencies:**
   ```toml
   [dependencies]
   ratatui = "0.29"
   crossterm = "0.28"
   tui-input = "0.11"
   tui-textarea = "0.7"
   thiserror = "2"
   unicode-width = "0.2"
   ```

   Note: `rat-widget` and `tui-widget-list` are deferred to the phases that need them (Phase 3, Phase 4) to keep initial dependency surface small.

3. **Create `biscuit-tui/lib/src/lib.rs`** — empty root with module declarations for `core`, `components`, `helpers`.

4. **CLI crate** — already scaffolded (`cli/Cargo.toml` + `cli/src/main.rs` with clap skeleton, binary `question`). No changes needed in Phase 0.

5. **Rewrite `biscuit-tui/justfile`** — adapt from `biscuit-hash` justfile pattern:
   - `PACKAGE := "biscuit-tui"`
   - `LIBRARY := "biscuit-tui"`, `CLI := "biscuit-tui-cli"`
   - `CLI_PATH := "./cli"`
   - Recipes: `build`, `install`, `test`, `lint`, `lint-fix`, `check`, `cli`
   - Keep imports of shared just files

6. **Verify:** `cargo check -p biscuit-tui -p biscuit-tui-cli` passes. `cargo run -p biscuit-tui-cli -- --help` prints usage.

### Verification

- [ ] `cargo check` succeeds for both crates
- [ ] `cargo run -p biscuit-tui-cli -- --help` exits 0
- [ ] `just check` in `biscuit-tui/` works

---

## Phase 1 — Core Types & Standalone Runner

**Goal:** Implement all shared types (`EventOutcome`, `ValidationState`, `Label`, `ComponentTheme`, `KeyBindings`) and the `run_standalone` helper. No visual components yet.

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/lib.rs` | Add `pub mod core;` |
| `lib/src/core/mod.rs` | Create, declare submodules |
| `lib/src/core/event.rs` | `EventOutcome` enum |
| `lib/src/core/validation.rs` | `ValidationState` trait |
| `lib/src/core/label.rs` | `LabelPosition`, `Label`, `render_with_label()` helper |
| `lib/src/core/theme.rs` | `ComponentTheme` struct + `Default` impl |
| `lib/src/core/standalone.rs` | `StandaloneState` trait + `run_standalone()` fn (fullscreen + inline modes) |
| `lib/src/prelude.rs` | Re-export core types for convenience |

### Key Design Decisions

- **`EventOutcome`**: 4 variants exactly as spec requires. No validation variant.
- **`ValidationState`**: Two methods — `validation_error()` and `clear_validation_error()`. Each component stores `Option<String>`.
- **`render_with_label`**: Takes an `Option<&Label>`, computes inner `Rect`, draws label text, calls closure for component body. Returns the inner `Rect`.
- **`run_standalone`**: Generic over `C: StatefulWidget`, `S: StandaloneState`. Handles `AlternateScreen` setup/teardown, raw mode, event loop. `height: Option<u16>` controls fullscreen vs inline.
- **`KeyBindings`**: Struct with `Vec<KeyEvent>` per action + `Default` impl providing vim-compatible defaults.

### Verification

- [ ] Unit tests for `EventOutcome` equality
- [ ] Unit tests for `Label` rendering with `TestBackend` (label above/below/left/right + inner content)
- [ ] Unit tests for `ComponentTheme::default()` (all fields populated)
- [ ] Integration-style test: `run_standalone` with a trivial widget (e.g., renders "hello", submits on Enter)
- [ ] `cargo check` and `cargo test -p biscuit-tui` pass

---

## Phase 2 — TextInput

**Goal:** First visual component. Wraps `tui-input` as private engine. End-to-end from library widget to CLI command.

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/components/mod.rs` | Create, declare `text_input` |
| `lib/src/components/text_input.rs` | `TextInput` widget + `TextInputState` |
| `lib/src/lib.rs` | Add `pub mod components;` |
| `lib/src/prelude.rs` | Add TextInput re-exports |
| `cli/src/main.rs` | Add `TextInput(TextInputArgs)` variant to `Commands` |
| `cli/src/commands/mod.rs` | Create, declare submodules |
| `cli/src/commands/text_input.rs` | Map CLI args → state → `run_standalone` → output |
| `cli/src/output.rs` | Create, `format_output()` for raw/json/null modes |

### State API

```rust
TextInputState::new()
    .with_label(Label { text: "...", position: LabelPosition::Above })
    .with_max_length(50)
    .with_value("initial")
```

### Event Handling

- Printable chars: check `max_length`, forward to `tui_input::Input::handle()`
- Backspace/Delete/Left/Right/Home/End: forward to tui-input
- Enter: `EventOutcome::Submitted`
- Esc: `EventOutcome::Cancelled`
- Other: `EventOutcome::Ignored`

### Rendering

- Use `render_with_label` for label placement
- Single-line `Line` with buffer content, cursor styled with `cursor_style`
- Validation error (if present) below input in `error_style`

### CLI Command

```
question text-input [--label <text>] [--label-position <above|below|left|right>] [--max-length <n>] [--initial <value>] [--height <n>] [--output <raw|json|null>]
```

### Verification

- [ ] Unit tests: state transitions (type chars, backspace, cursor movement)
- [ ] Unit test: `max_length` rejection (typing beyond limit silently drops)
- [ ] Unit test: Enter → Submitted, Esc → Cancelled
- [ ] Snapshot test: render with label above, label left, no label
- [ ] CLI integration: `echo "" | cargo run -p biscuit-tui-cli -- text-input --label Name` (manual)
- [ ] `cargo test -p biscuit-tui` passes
- [ ] `cargo test -p biscuit-tui-cli` passes

---

## Phase 3 — BooleanSwitch

**Goal:** Toggle switch component. No validation needed (boolean is always valid).

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/components/boolean_switch.rs` | `BooleanSwitch` widget + `BooleanSwitchState` |
| `lib/src/components/mod.rs` | Add `boolean_switch` |
| `lib/src/prelude.rs` | Add BooleanSwitch re-exports |
| `cli/src/commands/boolean_switch.rs` | Map CLI args → state → `run_standalone` → output |
| `cli/src/main.rs` | Add `BooleanSwitch(BooleanSwitchArgs)` variant |

### Rendering

Visual toggle switch:
```
  [● ON |   OFF ]     ← checked
  [  ON | ● OFF ]     ← unchecked
```
Focused state indicated by colored border or `focus_indicator`.

### Event Handling

- Space/Enter: toggle `checked`, return `Consumed`
- Left: set to off, Right: set to on
- Tab/Enter (configurable submit key): `Submitted`
- Esc: `Cancelled`

### CLI Command

```
question boolean-switch [--label <text>] [--labels <on,off>] [--initial <true|false>] [--height <n>] [--output <raw|json|null>]
```

### Dependency Note

Build bespoke rendering for BooleanSwitch (no `rat-widget` yet). If `rat-widget::Checkbox` is adopted later, it remains private.

### Verification

- [ ] Unit tests: toggle on/off, initial state, custom labels
- [ ] Unit test: Space toggles, Left sets off, Right sets on
- [ ] Snapshot test: checked state, unchecked state, focused state
- [ ] `value()` returns `bool`
- [ ] CLI: output is `"true"` or `"false"` for raw mode

---

## Phase 4 — ChooseOne + ChooseMany + Choice Helpers

**Goal:** Selection components with hotkeys, validation, and choice builder helpers.

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/components/choose.rs` | Shared types: `SelectionMode`, `ChoiceOption<V>`, `ChoiceInput<V>`, `ChoiceOption::map_value()` |
| `lib/src/components/choose_one.rs` | `ChooseOne` widget + `ChooseOneState<V>` |
| `lib/src/components/choose_many.rs` | `ChooseMany` widget + `ChooseManyState<V>` |
| `lib/src/components/mod.rs` | Add `choose`, `choose_one`, `choose_many` |
| `lib/src/helpers/mod.rs` | Create |
| `lib/src/helpers/choice_builders.rs` | `choose_one_from_csv`, `choose_many_from_csv`, `choose_one_from_markdown_list`, `choose_many_from_markdown_list`, `choose_one_from_dictionary` |
| `lib/src/lib.rs` | Add `pub mod helpers;` |
| `cli/src/commands/choose_one.rs` | CLI for choose-one |
| `cli/src/commands/choose_many.rs` | CLI for choose-many |
| `cli/src/main.rs` | Add `ChooseOne` and `ChooseMany` variants |

### Key Types (from spec)

```rust
pub enum SelectionMode { Single, Multiple }

pub struct ChoiceOption<V = String> {
    pub id: String,
    pub label: String,
    pub value: V,
    pub disabled: bool,
}

pub struct ChoiceInput<V = String> {
    pub id: String,
    pub prompt: String,
    pub help_text: Option<String>,
    pub selection_mode: SelectionMode,
    pub options: Vec<ChoiceOption<V>>,
    pub required: bool,
    pub min_selections: Option<usize>,
    pub max_selections: Option<usize>,
    pub shuffle_options: bool,
}
```

### ChooseOne Rendering

```
  ● Option A       ← selected + hovered
  ○ Option B       ← hovered only
  ○ Option C       ← neither
```

Viewport scrolling with `▲`/`▼` indicators when list exceeds visible area.

### ChooseMany Rendering

```
  ☑ Option A       ← selected
  ☐ Option B       ← hovered (highlight bg)
  ☐ Option C
```

### Validation

- **ChooseOne submit**: if `required` and nothing selected → `Consumed` + validation error
- **ChooseMany submit**: check `required` and `min_selections` → `Consumed` + validation error
- **ChooseMany toggle-on**: if `max_selections` reached → silently drop keystroke (`Consumed`)

### Hotkey Mapping

Built during `new()` from first unique char of each option label. Stored as `HashMap<KeyCode, usize>`.

### CLI Commands

```
question choose-one --options "Red,Green,Blue" [--required] [--label <text>]
question choose-many --options "Red,Green,Blue" [--required] [--min-selections <n>] [--max-selections <n>]
```

Also support `--options-from-file <path>` for markdown list / dictionary input.

### Verification

- [ ] Unit tests: single selection, hover navigation, hotkey direct select
- [ ] Unit test: multi-select toggle, `max_selections` cap, `min_selections` validation
- [ ] Unit test: required validation (submit with no selection)
- [ ] Unit test: disabled options render with disabled style, not selectable
- [ ] Unit test: `map_value()` projects `ChoiceOption<String>` → `ChoiceOption<u32>`
- [ ] Unit tests for choice builders: CSV parsing, markdown list parsing, dictionary parsing
- [ ] Snapshot tests: selected/hovered/disabled states for both components
- [ ] CLI: choose-one outputs single string, choose-many outputs newline-separated values

---

## Phase 5 — TextAreaInput

**Goal:** Multi-line text editor wrapping `tui-textarea`.

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/components/text_area_input.rs` | `TextAreaInput` widget + `TextAreaInputState` |
| `lib/src/components/mod.rs` | Add `text_area_input` |
| `lib/src/prelude.rs` | Add TextAreaInput re-exports |
| `cli/src/commands/text_area_input.rs` | CLI for text-area-input |
| `cli/src/main.rs` | Add `TextAreaInput(TextAreaInputArgs)` variant |

### State API

```rust
TextAreaInputState::new(60, 10)        // width x height
    .with_label(Label { ... })
    .with_scrollbar(true)
    .with_value(&["line 1", "line 2"])
```

### Rendering

- `render_with_label` for label
- Delegate body rendering to `tui_textarea::TextArea`
- Optional scrollbar overlay on rightmost column when content exceeds visible height
- Thumb position proportional to scroll offset

### Event Handling

- Forward all key events to `TextArea::input()`
- Submit key (configurable, default `Ctrl-S`): `Submitted`
- Esc: `Cancelled`
- Other: `Consumed`

### CLI Command

```
question text-area-input [--label <text>] [--width <n>] [--height <n>] [--scrollbar] [--output <raw|json|null>]
```

### Verification

- [ ] Unit tests: multi-line input, line breaks, cursor movement
- [ ] Unit test: Ctrl-S submits, Esc cancels
- [ ] Snapshot test: scrollbar visibility with overflow, without overflow
- [ ] `value()` returns joined lines with `\n`
- [ ] CLI: raw output is multi-line text

---

## Phase 6 — InputTable

**Goal:** Grid container with heterogeneous editable cells. The most complex component.

### Files to Create/Modify

| File | Action |
|------|--------|
| `lib/src/components/input_table/mod.rs` | Module root |
| `lib/src/components/input_table/table.rs` | `InputTable` widget + `InputTableState` |
| `lib/src/components/input_table/column.rs` | `InputTableColumn` enum |
| `lib/src/components/input_table/cell.rs` | `CellState` enum |
| `lib/src/components/mod.rs` | Add `input_table` |
| `cli/src/commands/input_table.rs` | CLI for input-table |
| `cli/src/main.rs` | Add `InputTable(InputTableArgs)` variant |

### Key Types

```rust
pub enum InputTableColumn {
    StaticText(String),
    BooleanSwitch(BooleanSwitchConfig),
    TextInput(TextInputConfig),
    TextAreaInput(TextAreaInputConfig),
    ChooseOne(ChoiceInput<String>),
    ChooseMany(ChoiceInput<String>),
}

pub enum CellState {
    StaticText,
    BooleanSwitch(BooleanSwitchState),
    TextInput(TextInputState),
    TextAreaInput(TextAreaInputState),
    ChooseOne(ChooseOneState),
    ChooseMany(ChooseManyState),
}
```

### Focus Model

- State tracks `(focus_row: usize, focus_col: usize)`
- Arrow keys navigate between cells (Left/Right within row, Up/Down between rows)
- Tab wraps to next cell (right then next row)
- Ctrl-S: validate all cells, submit if clean
- Esc: Cancel

### Validation on Submit

Iterate all cells row-major. If any cell has active validation error:
1. Move focus to first offending cell
2. Return `Consumed`
3. Cell renders its own inline error

### Rendering

- `Layout` to create grid based on column widths
- Each cell renders via its own widget
- Focused cell gets highlighted border

### CLI Command

```
question input-table --columns <json> --rows <json> [--height <n>] [--output <raw|json|null>]
```

Where `--columns` is JSON array of column definitions and `--rows` is JSON array of row value arrays.

### Verification

- [ ] Unit tests: focus navigation (all 4 directions + Tab wrap)
- [ ] Unit test: submit validation aggregates errors from child cells
- [ ] Unit test: focus moves to first offending cell on failed submit
- [ ] Unit test: each cell type handles its own events correctly within the table
- [ ] Snapshot test: 3x3 table with mixed cell types
- [ ] CLI: JSON output is array of row objects with column keys

---

## Phase 7 — Polish & Hardening

**Goal:** Cross-cutting quality pass. Not a separate implementation phase — these tasks run after Phase 6.

### Tasks

1. **Key binding audit** — verify consistent defaults across all components. Ensure vim keys (h/j/k/l) work everywhere directional input is expected.

2. **Terminal emulator testing** — verify rendering on at least:
   - macOS Terminal.app
   - iTerm2
   - Kitty
   - WezTerm
   - tmux (inside any of the above)

3. **Theme finalization** — audit default `ComponentTheme` values for readability on both light and dark backgrounds.

4. **CLI output modes** — complete `--output raw|json|null` for all commands:
   - `raw`: scalar → string + newline, ChooseMany → newline-separated, InputTable → not applicable (error?)
   - `json`: force JSON for every component
   - `null`: NUL-byte separated for multi-value outputs

5. **Exit code verification** — all commands return 0 on submit, 130 on cancel. No stdout on cancel.

6. **Documentation** — rustdoc on all public types. Examples on `TextInputState`, `ChooseOneState`, etc.

7. **Drift maintenance** — update:
   - `CLAUDE.md` workspace section
   - `docs/dependencies.md` if one exists for biscuit-tui
   - Root justfile `areas` list if biscuit-tui should be covered

8. **Performance** — verify no unnecessary re-renders in event loop. `run_standalone` should only redraw on `Consumed`/`Submitted`/`Cancelled`, not on `Ignored`.

---

## Dependency Adoption Timeline

| Phase | New Dependency | Purpose |
|-------|---------------|---------|
| Phase 2 | `tui-input` | TextInput edit buffer (private) |
| Phase 5 | `tui-textarea` | TextAreaInput engine (private) |
| Phase 3 | (none — bespoke) | BooleanSwitch rendering |
| Phase 4 | (none — bespoke) | ChooseOne/ChooseMany rendering |
| Phase 6 | (none — bespoke) | InputTable |

`rat-widget` and `tui-widget-list` are **not** adopted in v1. The strategy document's recommendation to prefer bespoke rendering for BooleanSwitch, ChooseOne, ChooseMany, and InputTable avoids the heavier integration cost and keeps the dependency tree minimal. These can be reconsidered in v2 if their widgets provide significant rendering improvements.

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| `tui-input` / `tui-textarea` API incompatibility with our event model | Low | Both are headless/state engines. Our wrapper translates `KeyEvent` → their input model. If translation breaks, we can fork or reimplement the small subset we use. |
| `run_standalone` inline mode (non-fullscreen) terminal state corruption | Medium | Inline mode is the trickiest part — must save/restore cursor position, avoid alternate screen. Test early in Phase 1 with a trivial widget. Fall back to fullscreen-only for v1 if inline proves fragile. |
| InputTable cell delegation complexity | Medium | Largest surface area. Mitigate by building it last (Phase 6) when all cell types are proven. The `CellState` enum delegates to existing `handle_event` impls. |
| Cross-terminal rendering inconsistencies | Medium | Use only standard ANSI escape codes in v1 (no Sixel/Kitty protocols). Rely on ratatui's abstracted rendering. Defer terminal-specific optimizations to polish phase. |
| `KeyBindings` customization complexity | Low | Start with `Default` impl only. Custom bindings are a config struct — callers can mutate fields. No runtime key parsing needed in v1. |

---

## File Count Estimate

| Category | Files | Approximate |
|----------|-------|-------------|
| Library source (`lib/src/`) | 16 | core (6) + components (8) + helpers (2) |
| CLI source (`cli/src/`) | 9 | main + commands (7) + output |
| Cargo.toml | 2 | lib + cli |
| Justfile | 1 | area justfile |
| Tests | ~8 | One per component + core + helpers |
| **Total new files** | | **~36** |

---

## Success Criteria

1. All 6 components render correctly and handle keyboard input per spec.
2. Every component works both embedded (StatefulWidget) and standalone (run_standalone).
3. `question` CLI exposes all components with correct output contract (exit codes, format modes).
4. All tests pass: `cargo test -p biscuit-tui -p biscuit-tui-cli`.
5. No community crate types leak into the public API.
6. Validation model (keystroke rejection + submit-time inline errors) works uniformly.
