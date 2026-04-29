---
ready: false
---

# Review 5: TUI Inputs Implementation

This review evaluates the final implementation of the "input-tui" feature against the specification and technical design. While the majority of the functionality is robust, well-tested, and idiomatically implemented, there are a few remaining gaps and deviations from the spec that should be addressed before production release.

## 1. Gaps in Functionality

### 1.1 Missing `ChoiceInput::shuffle_options`
The `ChoiceInput` struct (in `choose.rs`) is missing the `shuffle_options: bool` field required by the specification (§4.3.2). This feature was designed to reduce selection bias in surveys/questionnaires. 
- **Status:** Not implemented.
- **Impact:** Gaps in spec compliance.

### 1.2 Missing `choose_many_from_dictionary` Helper
While `choose_one_from_dictionary` exists in `choice_builders.rs`, the corresponding multi-select version `choose_many_from_dictionary` is missing. The specification implies that all builders should support both selection modes for consistency.
- **Status:** Missing from `choice_builders.rs`.
- **Impact:** Minor ergonomic inconsistency.

### 1.3 `InputTable` Choice Type Limitation
The specification (§4.6) states that "Table cells MUST support the same typed value flow as standalone components." However, `InputTableColumn` and `CellValue` currently only support `ChoiceInput<String>`. 
- **Status:** Standing violation of the "same typed value flow" mandate.
- **Impact:** Library consumers cannot use strong-typed enums or integers as values for choice-based cells in an `InputTable`, though they can for standalone `ChooseOne<V>`.

## 2. Implementation Deviations

### 2.1 `StaticText` Sizing in `InputTable`
The technical design (§4.6) and prior review suggestions called for `StaticText` columns to be clamped to their natural width. The current `compute_column_widths` implementation (in `table.rs`) adds leftover width (`per_col`) to every column, including `StaticText` columns.
- **Status:** Deviation from design.
- **Impact:** Visual inconsistency; static labels may take up more horizontal space than intended, pushing editable fields off-screen on narrow terminals.

## 3. Strengths and Successes

- **Strong Test Coverage:** Every module (lib and cli) includes comprehensive unit and integration tests. The PTY-based testing for CLI commands is particularly thorough.
- **Excellent Ergonomics:** The `ChoiceInput<V>` generic and `map_value` utility provide a very clean API for library consumers to work with domain-specific types.
- **Robust CLI:** The CLI implementation perfectly handles output serialization (`raw`, `json`, `null`) and the `height` option for inline rendering.
- **Validation Flow:** The submit-time validation in `InputTable` correctly aggregates errors and automatically routes focus to the first offending cell.

## 4. Recommendation

**Ready for Production:** No

The core functionality is high quality, but the missing `shuffle_options` and the limitation on typed choices in tables are significant deviations from the agreed-upon specification. 

### Suggested Action Items:
1. Add `shuffle_options: bool` to `ChoiceInput` and implement shuffling logic in `ChooseOneState::new` and `ChooseManyState::new`.
2. Add `choose_many_from_dictionary` to `choice_builders.rs`.
3. Update `compute_column_widths` to only distribute leftover width to focusable columns, keeping `StaticText` at its natural width.
4. (Optional but Recommended) Evaluate if `InputTableColumn` can be made generic to support typed choices, or explicitly update the spec to reflect the `String` limitation in tables.
