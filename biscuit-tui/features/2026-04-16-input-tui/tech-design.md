# TUI Inputs — Technical Design

This document is the engineering companion to the [spec](./spec.md). Where the spec defines *what* the library provides, this document defines *how* it is built: module layout, type relationships, event flow, rendering architecture, and implementation phasing.

---

## 1. Crate Structure

```
tui-chrome/                          # library crate (tui-chrome)
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── prelude.rs
    ├── core/
    │   ├── mod.rs
    │   ├── event.rs                 # EventOutcome, KeyEvent bridging
    │   ├── validation.rs            # ValidationState shared trait
    │   ├── label.rs                 # LabelPosition, Label rendering
    │   ├── theme.rs                 # ComponentTheme (colors, indicators)
    │   └── standalone.rs            # run_standalone()
    ├── components/
    │   ├── mod.rs
    │   ├── text_input.rs            # TextInput widget + TextInputState
    │   ├── text_area_input.rs       # TextAreaInput widget + TextAreaInputState
    │   ├── boolean_switch.rs        # BooleanSwitch widget + BooleanSwitchState
    │   ├── choose.rs                # ChooseOne / ChooseMany shared types
    │   ├── choose_one.rs            # ChooseOne widget + ChooseOneState
    │   ├── choose_many.rs           # ChooseMany widget + ChooseManyState
    │   └── input_table/
    │       ├── mod.rs
    │       ├── table.rs             # InputTable widget + InputTableState
    │       ├── column.rs            # InputTableColumn enum
    │       └── cell.rs              # CellState enum (per-cell mutable state)
    └── helpers/
        ├── mod.rs
        └── choice_builders.rs       # from_csv, from_markdown, from_dictionary

tui-chrome-cli/                      # binary crate (question)
├── Cargo.toml
└── src/
    ├── main.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── text_input.rs
    │   ├── text_area_input.rs
    │   ├── boolean_switch.rs
    │   ├── choose_one.rs
    │   ├── choose_many.rs
    │   └── input_table.rs
    └── output.rs                    # OutputMode, format_value()
```

### Dependencies

```toml
# tui-chrome/Cargo.toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tui-input = "0.11"          # private — TextInput edit buffer
tui-textarea = "0.7"        # private — TextAreaInput engine
rat-widget = "0.4"          # private — Checkbox for BooleanSwitch, Choice for ChooseOne
tui-widget-list = "0.13"    # private, optional — rendering helper for ChooseMany
thiserror = "2"
unicode-width = "0.2"       # label/column width measurement

# tui-chrome-cli/Cargo.toml
[dependencies]
tui-chrome = { path = "../lib" }
clap = { version = "4", features = ["derive"] }
serde_json = "1"
```

All community crates are **private** dependencies. They never appear in the public API.

---

## 2. Core Types

### 2.1 EventOutcome

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventOutcome {
    Consumed,
    Ignored,
    Submitted,
    Cancelled,
}
```

No additional variants for validation failures. A failed validation returns `Consumed` and populates the state's validation error.

### 2.3 ValidationState Trait

Every component state struct implements this trait so that validation access is uniform across components and so that `InputTable` can delegate validation queries to child cells.

```rust
pub trait ValidationState {
    fn validation_error(&self) -> Option<&str>;
    fn clear_validation_error(&mut self);
}
```

Components store the error as `Option<String>` internally. The trait supplies the read accessor; each component's `handle_event` sets/clears it on submit attempts.

### 2.4 Label System

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelPosition {
    Above,
    Below,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub text: String,
    pub position: LabelPosition,
}
```

Rendering is handled by a shared helper `render_with_label(area: Rect, buf: &mut Buffer, label: Option<&Label>, inner: impl FnOnce(Rect, &mut Buffer))`. This function computes the inner area after reserving space for the label, draws the label text, and delegates the component body to the closure.

### 2.5 Component Theme

A single `ComponentTheme` struct centralizes visual constants so that all components render consistently. A default is provided; callers can override per-component or globally.

```rust
#[derive(Debug, Clone)]
pub struct ComponentTheme {
    pub focus_indicator: String,        // e.g. "▶" or ">"
    pub selected_indicator: String,     // e.g. "●" or "[x]"
    pub unselected_indicator: String,   // e.g. "○" or "[ ]"
    pub switch_on: String,              // e.g. " ON "
    pub switch_off: String,             // e.g. " OFF"
    pub switch_thumb: char,             // e.g. '●' or '◉'
    pub cursor_style: Style,
    pub selected_style: Style,
    pub error_style: Style,
    pub label_style: Style,
    pub disabled_style: Style,
}
```

The theme is stored inside each component's `State` (not the widget) so it can be mutated at runtime if needed. A `Default` impl provides sensible values.

---

## 3. Event Handling Flow

### 3.1 Single-Component Flow (embedded)

```
caller receives crossterm::event::Event
    │
    ├─ convert to KeyEvent (if key event)
    │
    └─ component.handle_event(&mut state, key_event)
         │
         ├─ return EventOutcome::Consumed    (handled internally)
         ├─ return EventOutcome::Ignored     (not ours, caller decides)
         ├─ return EventOutcome::Submitted   (caller reads state.value())
         └─ return EventOutcome::Cancelled   (caller tears down)
```

### 3.2 Standalone Flow (run_standalone)

```
run_standalone(component, state)
    │
    ├─ init crossterm terminal (AlternateScreen + RawMode)
    │
    └─ loop {
         ├─ terminal.draw(|f| f.render_stateful_widget(&component, area, &mut state))
         ├─ read crossterm::event::read()
         ├─ if Event::Key(key) → component.handle_event(&mut state, key)
         ├─ match outcome:
         │    Submitted → restore terminal → return Ok(state.value())
         │    Cancelled → restore terminal → return Err(Cancelled)
         │    Consumed  → continue
         │    Ignored   → continue (Ctrl-C handling at this layer)
         └─ }
```

### 3.3 InputTable Event Delegation

`InputTableState` maintains a focus coordinate `(row: usize, col: usize)`. On each key event:

1. Translate arrow keys into focus navigation. Left/Right move between columns within a row; Up/Down move between rows. Wrap behavior is configurable but defaults to clamped (no wrap).
2. If the event is the global Submit key (default `Ctrl-S`), validate all cells; if any fail, set focus to the first offending cell and return `Consumed`.
3. Otherwise, delegate the event to the focused cell's `handle_event`. The cell returns its own `EventOutcome`; the table maps `Ignored` to its own navigation layer (e.g. Tab to next cell).

---

## 4. Component Technical Design

### 4.1 TextInput

**Internal engine:** `tui_input::Input` — stores the edit buffer and cursor position.

**State:**

```rust
pub struct TextInputState {
    inner: tui_input::Input,
    label: Option<Label>,
    max_length: Option<usize>,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl TextInputState {
    pub fn new() -> Self;
    pub fn with_label(mut self, label: Label) -> Self;
    pub fn with_max_length(mut self, max: usize) -> Self;
    pub fn with_value(mut self, value: &str) -> Self;
    pub fn value(&self) -> &str;
    pub fn validation_error(&self) -> Option<&str>;
}
```

**Widget:** `TextInput` (zero-sized marker struct, implements `StatefulWidget`).

**Rendering:**

1. Call `render_with_label` to carve out the label area.
2. Draw the input area: a single-line `Line` with the buffer content. The character at the cursor position gets `cursor_style`. If the component has focus, draw a block cursor at the visible position.
3. If `validation_error.is_some()`, render the error text below the input in `error_style`.

**Event handling:**

| Key | Behavior |
|-----|----------|
| Printable char | If `max_length` allows, forward to `tui_input::Input::handle()` |
| Backspace / Delete / Left / Right / Home / End | Forward to `tui_input::Input::handle()` |
| Enter | Return `Submitted` |
| Esc | Return `Cancelled` |
| Any other | Return `Ignored` |

`max_length` enforcement: before forwarding a printable char, check `inner.value().len() < max_length`. If the limit would be exceeded, return `Consumed` silently.

### 4.2 TextAreaInput

**Internal engine:** `tui_textarea::TextArea` — handles multi-line editing, line wrapping, scrolling.

**State:**

```rust
pub struct TextAreaInputState {
    inner: tui_textarea::TextArea<'static>,
    width: u16,
    height: u16,
    label: Option<Label>,
    show_scrollbar: bool,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl TextAreaInputState {
    pub fn new(width: u16, height: u16) -> Self;
    pub fn with_label(mut self, label: Label) -> Self;
    pub fn with_scrollbar(mut self, show: bool) -> Self;
    pub fn with_value(mut self, lines: &[&str]) -> Self;
    pub fn value(&self) -> String;       // joins lines with \n
    pub fn lines(&self) -> &[String];    // direct access to line buffer
    pub fn validation_error(&self) -> Option<&str>;
}
```

**Rendering:**

1. `render_with_label` for label area.
2. Render `TextArea` into the inner area (the TextArea widget handles its own line rendering and cursor).
3. If `show_scrollbar` is true and content lines exceed the visible height, draw a vertical scrollbar track on the rightmost column of the inner area. The thumb position is proportional to the scroll offset.

**Event handling:** Forward all key events to `TextArea::input()`. The TextArea crate returns its own outcome type; we translate:

- If the event was the submit key (configurable, default `Ctrl-S`): return `Submitted`.
- If `Esc`: return `Cancelled`.
- Otherwise: return `Consumed`.

### 4.3 BooleanSwitch

**Internal engine:** `rat_widget::checkbox::Checkbox` for state management (checked/unchecked toggle).

**State:**

```rust
pub struct BooleanSwitchState {
    checked: bool,
    label_on: String,       // default: "true"
    label_off: String,      // default: "false"
    label: Option<Label>,
    theme: ComponentTheme,
}

impl BooleanSwitchState {
    pub fn new() -> Self;
    pub fn with_labels(mut self, on: impl Into<String>, off: impl Into<String>) -> Self;
    pub fn with_label(mut self, label: Label) -> Self;
    pub fn with_initial(mut self, checked: bool) -> Self;
    pub fn value(&self) -> bool;
}
```

No validation error — a boolean is always valid.

**Rendering:** The switch renders as a horizontal track:

```
  [● ON |   OFF ]     ← checked
  [  ON | ● OFF ]     ← unchecked
```

The track is a fixed-width cell pair. The thumb character (`●`) moves to the active side. Focused state is indicated by a colored border or the `focus_indicator` from the theme.

**Event handling:**

| Key | Behavior |
|-----|----------|
| Space / Enter | Toggle `checked`, return `Consumed` |
| Left / Right | Set to off/on respectively, return `Consumed` |
| Tab / Enter (when configured) | Return `Submitted` |
| Esc | Return `Cancelled` |

### 4.4 ChooseOne

**Internal engine:** `rat_widget::choice::Choice` for single-selection state tracking.

**State:**

```rust
pub struct ChooseOneState<V = String> {
    config: ChoiceInput<V>,
    selected_index: Option<usize>,
    hovered_index: usize,
    hotkeys: HashMap<KeyCode, usize>,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl<V: Clone + PartialEq> ChooseOneState<V> {
    pub fn new(config: ChoiceInput<V>) -> Self;
    pub fn value(&self) -> Option<&V>;
    pub fn validation_error(&self) -> Option<&str>;
}
```

**Hotkey mapping:** Built during `new()` from the first unique character of each option label (or explicit hotkey configuration). Stored as `HashMap<KeyCode, usize>` mapping to the option index.

**Rendering:**

```
  ● Option A       ← selected + hovered (selected_indicator + highlight)
  ○ Option B       ← hovered only (unselected_indicator + highlight)
  ○ Option C       ← neither
```

Focused item gets the `focus_indicator` prefix. The hovered item has a distinct background. The selected item shows the `selected_indicator`. If an option is `disabled`, render with `disabled_style`.

If the list is taller than the available area, render a viewport window centered on the hovered item, with scroll indicators (`▲` / `▼`) at the top/bottom edges when there is hidden content.

**Event handling:**

| Key | Behavior |
|-----|----------|
| Up / k | Move hover up, return `Consumed` |
| Down / j | Move hover down, return `Consumed` |
| Space / Enter | Select hovered item, return `Consumed` |
| Hotkey char | Select the corresponding item directly, return `Consumed` |
| Tab / Ctrl-S | Submit: if `required` and nothing selected, set validation error, return `Consumed`; else return `Submitted` |
| Esc | Return `Cancelled` |

### 4.5 ChooseMany

**Internal engine:** bespoke, with optional `tui_widget_list` for rendering the scrollable list viewport.

**State:**

```rust
pub struct ChooseManyState<V = String> {
    config: ChoiceInput<V>,
    selected: HashSet<usize>,
    hovered_index: usize,
    hotkeys: HashMap<KeyCode, usize>,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl<V: Clone + PartialEq> ChooseManyState<V> {
    pub fn new(config: ChoiceInput<V>) -> Self;
    pub fn values(&self) -> Vec<&V>;
    pub fn validation_error(&self) -> Option<&str>;
}
```

**Rendering:** Similar to ChooseOne but with toggle indicators:

```
  ☑ Option A       ← selected
  ☐ Option B       ← hovered (highlight bg)
  ☐ Option C
```

**Event handling:** Identical to ChooseOne except:

- Space toggles the hovered item rather than replacing the selection.
- Before toggling on, check `max_selections`. If the cap is reached, silently drop the keystroke (`Consumed`).
- On submit, check `required` and `min_selections`. If violated, set validation error, return `Consumed`.

### 4.6 InputTable

**Internal engine:** bespoke. Each cell holds a boxed trait object that can handle events and render itself.

**Column definition:**

```rust
pub enum InputTableColumn {
    StaticText(String),
    BooleanSwitch(BooleanSwitchConfig),
    TextInput(TextInputConfig),
    TextAreaInput(TextAreaInputConfig),
    ChooseOne(ChoiceInput<String>),
    ChooseMany(ChoiceInput<String>),
}
```

**Cell state:** Each cell variant holds the corresponding component state. Navigation and focus are managed at the table level.

```rust
pub enum CellState {
    StaticText,
    BooleanSwitch(BooleanSwitchState),
    TextInput(TextInputState),
    TextAreaInput(TextAreaInputState),
    ChooseOne(ChooseOneState),
    ChooseMany(ChooseManyState),
}
```

**Table state:**

```rust
pub struct InputTableState {
    columns: Vec<InputTableColumn>,
    rows: Vec<Vec<CellState>>,
    focus_row: usize,
    focus_col: usize,
    theme: ComponentTheme,
    validation_error: Option<String>,
    submit_key: KeyEvent,
}
```

**Row data:** The caller provides `initial_rows` as `Vec<Vec<CellState>>`. The number of cells per row must match `columns.len()`.

**Rendering:** Uses `ratatui::layout::Layout` to create a grid. Column widths are determined by the column type:

- `StaticText`: natural text width (clamped to a configured max).
- `BooleanSwitch`: fixed width (track width + labels).
- `TextInput`: configurable, default 20 chars.
- `TextAreaInput`: uses its configured width/height.
- `ChooseOne` / `ChooseMany`: configured width, height based on option count.

Each cell renders using its own widget. The focused cell gets a highlighted border.

**Focus navigation:**

| Key | Behavior |
|-----|----------|
| Left | Move focus to previous column in same row |
| Right | Move focus to next column in same row |
| Up | Move focus to same column in previous row |
| Down | Move focus to same column in next row |
| Tab | Move to next cell (right, then wrap to next row) |
| Ctrl-S | Validate all cells, submit if clean |
| Esc | Cancel |

**Validation on submit:** Iterate all cells in row-major order. If any cell's `validation_error()` is `Some`, move focus to the first such cell and return `Consumed`. The cell renders its own inline error. The table does not introduce additional validation.

---

## 5. Standalone Runner

```rust
pub fn run_standalone<C, S, V>(
    component: C,
    state: S,
    height: Option<u16>,
) -> std::io::Result<V>
where
    C: StatefulWidget<State = S>,
    S: StandaloneState<Value = V>,
{
    // ...
}
```

**`StandaloneState` trait:**

```rust
pub trait StandaloneState {
    type Value;
    fn value(&self) -> Self::Value;
    fn validation_error(&self) -> Option<&str>;
}
```

**Terminal setup:**

- If `height` is `None`: enter `AlternateScreen` fullscreen mode.
- If `height` is `Some(h)`: use inline mode. Position the widget at the current cursor row, spanning `h` lines. This avoids fullscreen takeover for quick prompts.

**Event loop:**

```
loop {
    terminal.draw(|f| {
        let area = f.area();
        f.render_stateful_widget(&component, area, &mut state);
    })?;

    match crossterm::event::read()? {
        Event::Key(key) => {
            let outcome = component.handle_event(&mut state, key);
            match outcome {
                EventOutcome::Submitted => break Ok(state.value()),
                EventOutcome::Cancelled => break Err(io::Error::new(
                    ErrorKind::Interrupted, "cancelled")),
                _ => {}
            }
        }
        Event::Resize(..) => continue,  // terminal.draw handles it
        _ => {}
    }
}
```

**Restore:** On any exit path (submit, cancel, error), restore the terminal to its prior state (disable raw mode, leave alternate screen, show cursor).

---

## 6. CLI Architecture

The `question` binary uses `clap` with derive macros.

```rust
#[derive(Parser)]
#[command(name = "question")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    output: Option<OutputMode>,

    #[arg(long, global = true)]
    height: Option<u16>,
}

#[derive(Clone, ValueEnum)]
enum OutputMode {
    Raw,
    Json,
    Null,
}

#[derive(Subcommand)]
enum Commands {
    TextInput(TextInputArgs),
    TextAreaInput(TextAreaInputArgs),
    BooleanSwitch(BooleanSwitchArgs),
    ChooseOne(ChooseOneArgs),
    ChooseMany(ChooseManyArgs),
    InputTable(InputTableArgs),
}
```

**Per-command args** include component-specific flags (e.g. `--label`, `--max-length`, `--options` as CSV).

**Output formatting** (in `output.rs`):

```rust
pub fn format_output<V: Display>(value: &V, mode: OutputMode) -> String { ... }
pub fn format_multi_output<V: Display>(values: &[V], mode: OutputMode) -> String { ... }
pub fn format_table_output(rows: &[serde_json::Value], mode: OutputMode) -> String { ... }
```

**Exit codes:** `0` on submit, `130` on cancel. No stdout on cancel.

---

## 7. Choice Builders (Helpers)

These are free functions that construct `ChoiceInput<String>` from various text formats. They live in `helpers::choice_builders`.

```rust
pub fn choose_one_from_csv(csv: &str) -> ChoiceInput<String>;
pub fn choose_many_from_csv(csv: &str) -> ChoiceInput<String>;
pub fn choose_one_from_markdown_list(md: &str) -> ChoiceInput<String>;
pub fn choose_many_from_markdown_list(md: &str) -> ChoiceInput<String>;
pub fn choose_one_from_dictionary(input: &str) -> ChoiceInput<String>;
```

**Parsing rules:**

- **CSV:** split on commas, trim whitespace, each token becomes a `ChoiceOption { id: label.clone(), label, value: label.clone(), disabled: false }`.
- **Markdown list:** parse lines matching `[-*] \s*(.+)` or `\d+\. \s*(.+)`, extract capture group as label.
- **Dictionary:** parse as JSON5 or YAML (auto-detect by trying JSON5 first). Keys become labels, values become the `value` field on `ChoiceOption`.

---

## 8. Key Binding Configuration

All components support a configurable key binding map for non-negotiable keys (navigation, submit, cancel). Default bindings:

| Action | Default |
|--------|---------|
| Up | `KeyCode::Up` / `KeyCode::Char('k')` |
| Down | `KeyCode::Down` / `KeyCode::Char('j')` |
| Left | `KeyCode::Left` / `KeyCode::Char('h')` |
| Right | `KeyCode::Right` / `KeyCode::Char('l')` |
| Toggle / Select | `KeyCode::Char(' ')` |
| Submit | `KeyCode::Enter` (single components) / `KeyCode::Char('S') + Ctrl` (table) |
| Cancel | `KeyCode::Esc` |

Configuration is passed through each component's config struct:

```rust
#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub up: Vec<KeyEvent>,
    pub down: Vec<KeyEvent>,
    pub left: Vec<KeyEvent>,
    pub right: Vec<KeyEvent>,
    pub toggle: Vec<KeyEvent>,
    pub submit: Vec<KeyEvent>,
    pub cancel: Vec<KeyEvent>,
}

impl Default for KeyBindings { ... }
```

Components match incoming `KeyEvent` against the binding lists in priority order. Unmatched events fall through to `Ignored`.

---

## 9. Testing Strategy

### Unit tests (per component)

Each component gets a companion `tests` module exercising:

1. **State transitions:** construct state, feed `KeyEvent` sequence, assert `value()` and `EventOutcome` at each step.
2. **Validation:** trigger submit with invalid state, assert `validation_error()` is `Some`, fix the state, assert error clears.
3. **Keystroke rejection:** send a char that violates `max_length` or `max_selections`, assert the value did not change.
4. **Key bindings:** verify that custom bindings work and defaults are sane.

### Snapshot/rendering tests

Use `ratatui::backend::TestBackend` to render each component into a `Buffer`, then assert cell contents. This catches regressions in layout and label placement without needing a real terminal.

### Integration tests (standalone runner)

Spawn `run_standalone` with a `TestBackend`-based terminal. Feed synthetic events, verify return values and exit conditions.

### CLI tests

Invoke the `question` binary as a subprocess (or use `clap`'s `try_get_matches_from`). Verify:

- Stdout content matches the output contract per component.
- Exit code is `0` on submit, `130` on cancel.
- `--output json` produces valid JSON.
- `--output null` uses NUL separators.

---

## 10. Implementation Phases

### Phase 1: Foundation

Set up crate scaffolding, implement core types (`EventOutcome`, `ValidationState`, `Label`, `ComponentTheme`, `KeyBindings`), and the `run_standalone` helper with both fullscreen and inline modes.

### Phase 2: TextInput

Wrap `tui-input`, implement `TextInputState` + `TextInput` widget, label rendering, `max_length` enforcement, and unit tests. Wire into CLI `text-input` command.

### Phase 3: BooleanSwitch

Implement bespoke rendering with `rat-widget::Checkbox` as internal state. Wire into CLI.

### Phase 4: ChooseOne + ChooseMany

Implement `ChoiceInput<V>`, option list rendering with hotkeys, validation (`required`, `min_selections`, `max_selections`). Wire both into CLI. Implement choice builders.

### Phase 5: TextAreaInput

Wrap `tui-textarea`, implement `TextAreaInputState`, scrollbar overlay, label integration. Wire into CLI.

### Phase 6: InputTable

Implement `InputTableColumn`, `CellState`, grid layout, focus navigation, cell delegation, table-level validation aggregation. Wire into CLI.

### Phase 7: Polish

- Audit keyboard UX across all components for consistency.
- Test on multiple terminal emulators (via `biscuit-terminal` capability detection).
- Finalize theme defaults.
- Complete CLI output modes (`--output raw|json|null`).
- Documentation and examples.
