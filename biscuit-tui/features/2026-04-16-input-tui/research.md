# TUI Inputs: Community Component Research

This document evaluates existing Ratatui community components against the [TUI Inputs Specification](./spec.md) to identify suitable starting points and implementation gaps.

## Summary of Findings

| Component | Recommended Community Base | Status |
| :--- | :--- | :--- |
| **TextInput** | `tui-input` | **Match.** Already specified. Headless logic is ideal for our `State` wrapping. |
| **TextAreaInput** | `tui-textarea` | **Match.** Already specified. Handles multi-line, scrolling, and cursor logic. |
| **BooleanSwitch** | `tui-checkbox` or `rat-widget::Checkbox` | **Close.** Need to wrap with "Switch" aesthetics (e.g., `[ON / OFF]`). |
| **ChooseOne** | `rat-widget::Choice` or `tui-widget-list` | **Good.** `Choice` handles single selection; `tui-widget-list` allows custom widget items. |
| **ChooseMany** | `tui-widget-list` + `tui-checkbox` | **Good.** Composable approach fits the spec's flexibility. |
| **InputTable** | `rat-widget::Table` or Custom | **Gap.** Spec requires heterogeneous cell widgets (Inputs, Selects), requiring a custom `StatefulWidget`. |

---

## Detailed Evaluation

### 1. TextInput
* **Component:** [tui-input](https://github.com/veeso/tui-input)
* **Features Addressed:**
    * Headless logic (handles insertions, deletions, cursor movement).
    * Lightweight `Input` state that fits perfectly into our `State` struct.
* **Missing Features:**
    * **Labels:** `tui-input` is headless; we must implement the `Widget` part to render labels (above, left, etc.) as per spec.
    * **Max Length:** Needs to be enforced in our `handle_event` wrapper (Hard Cap validation).
    * **Inline Errors:** Submit-time validation needs to be added in our wrapper.

### 2. TextAreaInput
* **Component:** [tui-textarea](https://github.com/rhysd/tui-textarea)
* **Features Addressed:**
    * Full multi-line editing support.
    * Configurable sizes and auto-scrollbar (built-in).
    * Vim-like bindings (optional, but good for "vim keys as backup").
* **Missing Features:**
    * **API Alignment:** Needs to be wrapped to match our `StatefulWidget` + `EventOutcome` pattern.
    * **Labels:** Similar to TextInput, needs a layout wrapper for the prompt/label.

### 3. BooleanSwitch
* **Component:** [tui-checkbox](https://github.com/veeso/tui-checkbox) / [rat-widget::Checkbox](https://github.com/rat-salsa/rat-widget)
* **Features Addressed:**
    * Basic boolean state toggle.
    * Keyboard interaction (Space/Enter).
* **Missing Features:**
    * **"Switch" UI:** Spec asks for a "toggle switch" look. We will likely need to customize the rendering to look more like a slider/switch (`[  ○]` vs `[●  ]`) rather than a simple checkbox.
    * **Custom Labels:** Support for "true/false" vs custom string pairs (e.g., "Enabled/Disabled").

### 4. ChooseOne & ChooseMany
* **Component:** [tui-widget-list](https://github.com/preiter93/tui-widget-list)
* **Features Addressed:**
    * Allows a list of *widgets* to be treated as items.
    * Handles scrolling and selection state management.
* **Missing Features:**
    * **Hotkeys:** `tui-widget-list` doesn't natively map specific keys (e.g., 'a', 'b', 'c') to items. We will need to implement the `ChoiceInput` logic for mapping `KeyEvent` to `Selection`.
    * **Selection Modes:** We need to implement the logic for `Single` vs `Multiple` on top of the list state.
    * **Generic Projection:** The mapping from `V` to UI labels is our responsibility.
    * **Data Sources:** CSV/Markdown/JSON5 loading needs to be implemented as helpers for `ChoiceInput`.

### 5. InputTable
* **Component:** [rat-widget::Table](https://github.com/rat-salsa/rat-widget)
* **Evaluation:**
    * Most Ratatui tables (including built-in) focus on rendering `Cell<'a>` which are static text/spans.
    * Our spec requires an **Editable Form Table** where each cell is an active input component.
* **Strategy:** 
    * Leveraging `ratatui::layout::Layout` to create a grid of widgets is more appropriate than using a standard `Table` widget.
    * `InputTableState` will need to manage a 2D focus coordinate `(row, col)` to route events to the correct sub-component's `handle_event`.

---

## Technical Recommendations

1. **State Wrapping:** Use `tui-input`'s `Input` and `tui-textarea`'s `TextArea` as *internal fields* of our `TextInputState` and `TextAreaInputState`. Do not expose the underlying crates directly to maintain API stability.
2. **Validation Mixin:** Create a shared `ValidationState` trait/struct that can be included in all `*State` types to handle the `validation_error()` and submit-time logic consistently.
3. **Standalone Runner:** The `run_standalone` helper should use `ratatui::Terminal` with `CrosstermBackend` and a simple loop that matches the `EventOutcome` logic.

## Key URLs for Implementation
* [tui-input Docs](https://docs.rs/tui-input)
* [tui-textarea Docs](https://docs.rs/tui-textarea)
* [rat-widget (Choice/Table/Checkbox)](https://github.com/rat-salsa/rat-widget)
* [tui-widget-list](https://github.com/preiter93/tui-widget-list)
