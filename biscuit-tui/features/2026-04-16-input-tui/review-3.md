---
ready: false
---

# Review 3: TUI Inputs Implementation

This review evaluates the implementation of the `input-tui` feature, focusing on the `tui-chrome` library and the `question` CLI.

## 1. Functional Gaps & Design Deviations

### 1.1 InputTable Navigation Conflict (High Priority)
The current implementation of `InputTable::handle_event` unconditionally consumes `Up` and `Down` arrow keys for row navigation *before* delegating to the focused cell. This makes it impossible to navigate the internal lists of `ChooseOne` or `ChooseMany` components when they are embedded in a table.

- **Impact**: Users cannot use standard arrow keys to select options in a table. They must rely on `j/k` (if they are aware of vim-bindings) or hotkeys.
- **Suggested Fix**: `InputTable` should only consume `Up`/`Down` if the focused cell does not "want" them, or it should provide a "focus mode" (e.g., Enter to enter a cell, Esc to leave) to disambiguate navigation.

### 1.2 Validation Architecture (Medium Priority)
The `InputTable::submit` logic manually performs validation for `ChooseOne` and `ChooseMany` by checking their specific types and fields (`required`, `min_selections`).

- **Impact**: Brittle design. Adding a new component type with validation (e.g., a `required` flag for `TextInput`) requires updating `InputTable::submit` and `CellState` manually.
- **Suggested Fix**: Generalize validation by adding a `validate(&mut self) -> bool` method to the `CellState` enum (or a trait) that each component state implements. `InputTable::submit` should then simply iterate and call `cell.validate()`.

### 1.3 Keystroke-Time Rejection in ChooseMany
The spec requires "Keystroke-time rejection" for `max_selections` in `ChooseMany`. The implementation correctly checks `max_selections` during the toggle-on event and silently drops the keystroke if the limit is reached. This is well-implemented.

## 2. Technical Quality & Ergonomics

### 2.1 Component Composition
The separation of `State` and `Widget` follows Ratatui best practices. The use of private engines (`tui-input`, `tui-textarea`) is correctly abstracted, ensuring that internal dependencies do not leak into the public API.

### 2.2 CLI Output Contract
The CLI's output contract for `ChooseMany` and `InputTable` is correctly implemented:
- `ChooseMany` (Raw): Newline-separated list of values.
- `ChooseMany` (JSON): JSON array of values.
- `InputTable`: JSON array of row objects (keyed by column ID).

### 2.3 Standalone Runner
The `run_standalone` helper correctly implements both `Fullscreen` and `Inline` viewports. The handling of `Ctrl-C` at the runner layer provides a consistent "escape hatch" across all components.

## 3. Test Coverage

The test coverage is excellent:
- **Unit Tests**: Every component has a comprehensive `tests` module covering state transitions, validation, and key bindings.
- **Rendering Tests**: Components use `TestBackend` to verify correct rendering of labels, errors, and indicators.
- **Integration Tests**: `standalone.rs` includes tests for the event loop driver.

## 4. Performance

- **Redraw Optimization**: `drive_event_loop` correctly only redraws when an event is `Consumed` or a `Resize` occurs, avoiding unnecessary renders on `Ignored` events.
- **Memory**: The use of `Boxed` trait objects in `InputTable` is minimal and appropriate for a TUI environment.

## 5. Conclusion

The implementation is very high quality and technically sound. However, the **InputTable navigation conflict** is a significant UX hurdle that prevents the feature from being "production-ready" for complex table-based inputs.

**Status**: `ready: false` (Pending resolution of InputTable navigation conflict).
