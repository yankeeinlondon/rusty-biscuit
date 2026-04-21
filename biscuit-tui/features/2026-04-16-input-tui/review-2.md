---
date: 2026-04-20
feature: biscuit-tui/features/2026-04-16-input-tui
ready: false
---

# Feature Review: TUI Inputs (input-tui)

## Summary

The implementation of the `input-tui` feature is highly professional, following the technical design closely and providing excellent test coverage for individual components. The library structure is clean, and the standalone runner provides a robust foundation for the CLI. However, there are a few significant gaps and architectural limitations, particularly in the `InputTable` component, that prevent it from being "production-ready" for complex use cases.

## Gaps in Functionality

### 1. Missing `shuffle_options` Implementation
The `ChoiceInput` struct defines a `shuffle_options` field as specified, but this is currently ignored by both `ChooseOneState` and `ChooseManyState`. Shuffling is useful for reducing selection bias in certain types of questionnaires.

### 2. Lack of `InputTable` Row Scrolling
The `InputTable` component does not support row-level scrolling. If a table has more rows than fit in the available terminal height (or if some rows are tall due to `TextAreaInput` or many-option choice components), the trailing rows are clipped and completely inaccessible to the user. This severely limits the table's utility for larger datasets.

## Bugs & Incomplete Implementation

### 1. JSON Output Type Mismatch for `ChooseMany`
The `InputTableState::values()` method flattens all cell values into `String`. For `ChooseMany`, this results in a comma-separated string (e.g., `"alpha,beta"`). When the CLI emits JSON, these are rendered as JSON strings instead of JSON arrays. This loses type information and introduces ambiguity if the individual values themselves contain commas. The table's value extraction should ideally return structured data (e.g., `serde_json::Value`).

### 2. Validation Error Label Overwrite
In components like `TextInput`, the validation error is rendered manually at `inner_area.y + 1`. If `LabelPosition::Below` is used and the total height is small (e.g., height 2), the error message will overwrite the label. The `render_with_label` helper or the component renderers should account for the combined vertical budget of body + label + error.

## Ergonomics & Performance Improvements

### 1. Aggressive `InputTable` Cell Heights
The `min_height` for choice components in a table returns the full count of options. While this avoids internal scrolling for the cell, it very quickly exhausts the vertical space of the entire table, making the lack of table-level scrolling even more noticeable. A "compact" mode or a reasonable cap on cell height would improve the layout.

### 2. Standardized Submit Key in CLI
The CLI subcommands for scalar inputs use `Enter` for submission, while `TextAreaInput` and `InputTable` use `Ctrl-S`. While this matches the design, providing a consistent hint (e.g., a small footer or help text) in the standalone runner would greatly improve the user experience for those unfamiliar with the specific component's bindings.

## Test Coverage Analysis

- **Component Units:** Excellent. Every component has a companion test module covering state transitions, validation, and key bindings.
- **Standalone Runner:** Strong integration tests using `drive_event_loop` with synthetic events.
- **CLI:** Basic argument parsing tests exist, but end-to-end integration tests (e.g., using `assert_cmd`) to verify the actual stdout contract for complex components like `InputTable` are missing in the code, although the infra is there.

## Conclusion

The core widgets are solid and ready for use in isolation. However, the `InputTable` requires a second pass to implement scrolling and structured data extraction before it can be used for anything beyond very small grids.

**Status:** `ready: false`
