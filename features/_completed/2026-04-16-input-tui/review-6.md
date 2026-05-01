---
ready: true
---

# Review 6: TUI Inputs Implementation

## Overview

The implementation of the `tui-chrome` library and the `question` CLI successfully delivers all functional requirements defined in the specification and technical design. The code is well-structured, follows established patterns, and includes a comprehensive test suite.

## Functionality Gaps & Improvements

### 1. TextInput Horizontal Scrolling
- **Observation:** `TextInput` currently lacks horizontal scrolling. When the input text exceeds the available `area.width`, the content is truncated, and the cursor becomes invisible once it moves past the right edge.
- **Recommendation:** While not explicitly required by the v1 spec, adding a `view_offset` to `TextInputState` would significantly improve the UX for longer inputs.

### 2. ChooseMany Enforcement Feedback
- **Observation:** `ChooseMany` silently drops "toggle-on" events when `max_selections` is reached.
- **Recommendation:** Providing visual feedback (e.g., a temporary validation message or a specific "at-cap" style) would clarify to the user why their input is being ignored.

### 3. InputTable Width Calculation
- **Observation:** In `compute_column_widths`, when the table must shrink to fit the terminal, the calculation for `StaticText` columns uses `base_w.min(preferred[i])`. This can sometimes leave a small gap on the right if the sum of widths is slightly less than `total_width`.
- **Recommendation:** Redistribute any remaining pixels to the last focusable column to ensure the table always fills the available width.

## Test Coverage

- **Library:** Every component has extensive unit tests covering navigation, state transitions, validation, and rendering.
- **CLI:** Subcommands are verified through `run_with_writer` tests that simulate terminal interaction and validate output formatting (Raw, JSON, Null).
- **Core:** Layout and event loop logic are well-covered with synthetic event injection.

## Ergonomics & Performance

- The use of zero-sized marker widgets (`TextInput`, `BooleanSwitch`, etc.) paired with stateful structs is idiomatic for Ratatui and keeps the API clean.
- The `helpers::choice_builders` provide excellent developer ergonomics for common use cases like CSV or Markdown-based choice lists.
- `InputTableState` correctly manages the synchronization between transient component state and typed `CellValue` rows, making it easy for consumers to extract data.

## Final Assessment

The feature is **ready for production**. The implementation is professional, stable, and highly consistent across all components.
