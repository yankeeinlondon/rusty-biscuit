# Advanced Table Rendering Design

## Summary

`biscuit-terminal`'s current `Table` width strategy is too coarse for mixed content. It computes one "natural" width per column from the raw visible width of headers and cells, then proportionally shrinks columns that are considered reducible. That creates several problems:

- multi-line headers are measured incorrectly because `visible_width()` is applied to the full string instead of the widest explicit line
- `WordWrap::None` on a string column does not currently protect that column during width reduction
- width reduction is proportional rather than semantic, so a low-value numeric column can receive more space than its rows need while high-value prose columns wrap early
- the two rendering paths (`render_content()` and `render_with_cursor_positioning()`) duplicate width work instead of sharing one plan

This document proposes a new pre-render planning layer that turns width selection into an explicit, testable planning step. The core idea is:

1. Resolve the renderable width first.
2. Measure each visible column using header and cell content.
3. Partition columns into fixed, non-wrapping, and shrinkable groups.
4. Compute a natural break width for shrinkable columns.
5. Produce a `TableWidthPlan` before rendering any rows.
6. If the plan cannot fit, optionally drop marked columns before returning an error.

## Execution Plan

Implementation landed in these phases:

1. Add line-aware measurement helpers for headers and cell content.
2. Introduce shared planner types: `MeasuredColumn`, `TableWidthMeasurements`, `TableWidthPlan`, and `TableWidthError`.
3. Route both render paths through a single width plan.
4. Add `Table::available_render_width()`, `measure_widths()`, `plan_widths()`, and `would_wrap()`.
5. Add `TableColumn::drop_when_space_is_limited(...)` with right-to-left drop behavior and post-table notes.
6. Add planner-focused tests for multi-line headers, non-wrapping columns, and drop-note rendering.

## Current Implementation Findings

Relevant code today:

- `Table::render_content()` and `Table::render_with_cursor_positioning()` each call `with_visible_columns()` and `calculate_column_widths()`
- `Table::calculate_column_widths()` measures header width with `visible_width()` and data width with `visible_width(&cell.to_string())`
- `Table::constrain_widths_to_available()` proportionally shrinks columns based on `column_type.allows_word_wrap_override()`, not on the column's effective wrap behavior
- `wrap_cell_content()` only wraps after widths are already chosen
- `Layout::available_width()` already gives the correct renderable width budget once margins are resolved

The biggest bug behind the screenshot is measurement of headers that contain explicit line breaks. A header like `"Tool\nCalls"` only needs width `5`, but the current measurement path effectively counts the entire string and gives the column much more space than any rendered row requires.

## Design Goals

- Make width planning semantic and deterministic.
- Measure what will actually render, not the raw string as a single line.
- Respect `fixed_width`, `min_width`, `max_width`, and `effective_word_wrap()` together.
- Treat `WordWrap::None` as truly non-shrinkable unless the column is dropped.
- Expose measurements and plans through `Table` methods for tests and caller introspection.
- Use one width plan for both standard and cursor-positioned rendering.
- Preserve existing alignment, striping, vertical alignment, and ANSI-safe wrapping behavior.

## Non-Goals

- Changing `Layout` semantics outside of how `Table` consumes `available_width`
- Replacing the existing wrapping engine in `utils::block_constraint`
- Introducing automatic horizontal scrolling

## Proposed Architecture

Add a shared planning phase between width discovery and row rendering:

```text
Renderable::render/render_optimistic
  -> resolve terminal width
  -> resolve table renderable width via Layout
  -> Table::plan_render(...)
       -> apply conditional visibility
       -> measure visible columns
       -> resolve width plan
       -> optionally drop columns
  -> render table from plan
  -> optionally append dropped-column notes
  -> apply outer Layout or cursor positioning
```

### New Internal Types

These types should live with the table implementation, either in `table.rs` initially or in a new internal `planner.rs` module if the file starts getting too large.

#### `MeasuredColumn`

Represents all width-related facts for a single visible column.

Suggested fields:

- `original_index: usize`
- `header_width: usize`
- `header_line_width: usize`
- `cell_max_width: usize`
- `cell_line_max_width: usize`
- `columnar_width_requirement: usize`
- `fixed_width: Option<usize>`
- `min_width: Option<usize>`
- `max_width: Option<usize>`
- `effective_word_wrap: WordWrap`
- `natural_break_width: usize`
- `resolved_width: usize`
- `is_non_wrapping: bool`
- `is_shrinkable: bool`
- `drop_note: Option<String>`
- `header_lines: Vec<String>` or a lightweight cached form if useful

Important distinction:

- `header_width` / `cell_max_width` can capture raw full-string visible width if useful for diagnostics
- `header_line_width` / `cell_line_max_width` must be the max width of explicit lines
- `columnar_width_requirement` must use line-aware widths, not raw string width

#### `TableWidthMeasurements`

Captures table-level width facts before final fitting.

Suggested fields:

- `available_render_width: usize`
- `border_overhead: usize`
- `content_budget: usize`
- `fixed_width_consumption: usize`
- `non_wrapping_consumption: usize`
- `working_width: usize`
- `word_wrap_needed: bool`
- `columns: Vec<MeasuredColumn>`

#### `TableWidthPlan`

The resolved render plan consumed by both renderers.

Suggested fields:

- `available_render_width: usize`
- `visible_column_indices: Vec<usize>`
- `dropped_column_indices: Vec<usize>`
- `columns: Vec<MeasuredColumn>` with `resolved_width` finalized
- `table_width: usize`
- `dropped_notes: Vec<String>`

#### `TableWidthError`

A structured error that can still be rendered to the current string-based error message.

Suggested variants:

- `NoVisibleColumns`
- `InsufficientWidthForFixedColumns`
- `InsufficientWidthForNonWrappingColumns`
- `InsufficientWidthForWrappingColumns`
- `InsufficientWidthAfterDropping`

## Width Semantics

### 1. Available Render Width

Use the same source of truth in both render paths:

- standard path: `self.layout.available_width(term_width)`
- cursor path: resolve left and right margins directly, then compute the same renderable width

This width is the full table budget, including borders and cell padding.

### 2. Border Overhead

Keep the current table border formula:

- start border and left padding: `2`
- end padding and right border: `2`
- each interior separator: `3`

For `n` columns:

```text
border_overhead = 4 + 3 * (n - 1)
content_budget = available_render_width - border_overhead
```

All column widths in this design refer to content width only, not the surrounding spaces and borders.

### 3. Columnar Width Requirement

For each visible column, compute the width required to render without wrapping by using the widest explicit line across:

- header text or header prose content
- every formatted cell in the column

This must be line-aware:

```text
columnar_width_requirement =
  clamp(
    max(max_header_line_width, max_cell_line_width),
    min_width?,
    max_width?
  )
```

Constraint rules:

- `fixed_width` wins immediately and becomes both the requirement and resolved width
- `max_width` caps the requirement
- `min_width` floors the requirement
- if both `min_width` and `max_width` exist and `min_width > max_width`, normalize to `max(min_width, max_width)` in a documented way rather than leaving ambiguous behavior

Recommendation: keep builder methods permissive, but normalize during planning and report normalized values in diagnostics.

### 4. Non-Wrapping Columns

A column is non-wrapping when its effective behavior cannot reduce width by line-breaking:

- `WordWrap::None`
- numeric columns, because `effective_word_wrap()` already forces `WordWrap::None`

These columns consume their full `columnar_width_requirement` in the non-wrapping bucket.

`WordWrap::Truncate` should not be classified as non-wrapping for planning purposes. It is shrinkable, because it can fit within narrower widths while preserving a deterministic rendering strategy.

### 5. Shrinkable Columns

Shrinkable columns are those using:

- `WordWrap::WrapProse`
- `WordWrap::BespokeProse`
- `WordWrap::Truncate`

These columns share the `working_width` budget after fixed and non-wrapping consumption are reserved.

## Natural Break Width

Natural break width is the smallest width that still looks structurally correct for the column before we start forcing obviously bad wrapping.

### Rule A: Small columns

If a shrinkable column's `columnar_width_requirement` is `<= 5`, set:

```text
natural_break_width = columnar_width_requirement
```

These columns are not worth squeezing further during the first-fit phase.

### Rule B: `WrapProse` / `BespokeProse`

For remaining shrinkable columns:

1. Split header and cell content into explicit lines.
2. For each explicit line, split by the column's effective break tokens.
3. Measure the visible width of each segment.
4. Take that explicit line's widest segment.
5. Take the maximum of those values across the column.

That result is the column's `natural_break_width`.

Break token rules:

- `WrapProse` uses the same break characters the wrap engine already prefers
- `BespokeProse` uses its configured `Vec<char>`
- explicit newlines create separate measurement units before token splitting

Important: this measurement should be built on the same tokenization assumptions as `wrap_lines()` so the planner and renderer agree.

### Rule C: `Truncate`

For `WordWrap::Truncate`, natural break width should be the smallest practical width, not the unwrapped content width.

Recommendation:

```text
natural_break_width = max(min_width.unwrap_or(1), 1)
```

Rationale:

- truncation remains semantically valid at very small widths
- the indicator is already handled by the truncation routine and can itself be truncated

### Rule D: Natural Break Floor

After calculation:

```text
natural_break_width = max(natural_break_width, min_width.unwrap_or(1))
```

If `max_width` exists, also cap the value there.

## Planning Algorithm

### Phase 0: Preprocess and Visibility

1. Resolve `available_render_width`.
2. Filter columns through the existing `Conditional` logic.
3. Carry original indices forward so row data, alignment metadata, and dropped-note reporting stay stable.

### Phase 1: Measure Columns

For each visible column:

1. Format cells to strings exactly as current rendering does.
2. Split header and cell strings with `split_lines()`.
3. Measure each explicit line with `visible_width()`.
4. Compute `columnar_width_requirement`.
5. Determine `effective_word_wrap()`.
6. Classify the column as fixed, non-wrapping, or shrinkable.
7. Compute `natural_break_width`.

### Phase 2: Aggregate Consumptions

Compute:

```text
fixed_width_consumption = sum(fixed columns.resolved_width)
non_wrapping_consumption = sum(non_wrapping columns.columnar_width_requirement)
working_width = content_budget - fixed_width_consumption - non_wrapping_consumption
```

If `content_budget` is already negative or `working_width` is negative, move to the drop-column phase before failing.

### Phase 3: Decide Whether Wrapping Is Needed

Compute:

```text
full_unwrapped_content_width = sum(all columns.columnar_width_requirement)
word_wrap_needed = full_unwrapped_content_width > content_budget
```

If `word_wrap_needed == false`:

- assign every non-fixed column its `columnar_width_requirement`
- return the plan immediately

### Phase 4: Fit Shrinkable Columns Into Working Width

For shrinkable columns:

1. Pre-assign columns with `columnar_width_requirement <= 5` to that width.
2. Compute `remaining_working_width`.
3. Sum `natural_break_width` across all other shrinkable columns.

Cases:

- if `sum(natural_break_widths) <= working_width`, set:
  - `resolved_width = natural_break_width` for low-value small columns
  - `resolved_width = natural_break_width` for the rest as the baseline fit
- if `sum(natural_break_widths) > working_width`, the table cannot fit as-is and must attempt column dropping before erroring

### Phase 5: Distribute Surplus Width

When shrinkable columns fit within `working_width`, there may still be spare width:

```text
surplus = working_width - sum(resolved_width for shrinkable columns)
```

Distribute surplus back toward `columnar_width_requirement` in a stable order.

Recommendation:

- use left-to-right distribution for predictability
- stop each column at its `columnar_width_requirement`
- this preserves the "minimal natural fit first, then expand" principle

This produces tables that fit while still using the full available width naturally.

## Drop-When-Space-Is-Limited

### Public API

Add a builder to `TableColumn`:

```rust
pub fn drop_when_space_is_limited<T: Into<String>>(mut self, msg: Option<T>) -> Self
```

Add a new field:

```rust
pub drop_when_space_is_limited: Option<String>
```

Semantics:

- `None` means never auto-drop
- `Some(String)` means drop and later emit a note
- builder called with `None` should mark the column as droppable without a note

Because `Option<T>` cannot distinguish "builder not called" from "called with no note" if stored directly as `Option<String>`, the implementation should use an internal enum instead:

```rust
enum DropBehavior {
    Keep,
    DropSilently,
    DropWithMessage(String),
}
```

### Drop Order

Recommendation: drop eligible columns from right to left.

Why:

- trailing columns are usually lower priority detail columns
- this preserves left-hand scanability
- it is deterministic and easy to test

### Drop Loop

When the measured plan does not fit:

1. Find droppable columns among the visible set.
2. Remove the rightmost droppable column.
3. Re-run the planning step from measurement aggregation onward.
4. Repeat until the plan fits or no droppable columns remain.

Use full replanning after each drop rather than incremental subtraction. This is simpler and safer because:

- border overhead changes when column count changes
- `working_width` changes
- `uniform_alignment` maxima change
- the final visible set must stay identical across both renderers

### Dropped Column Notes

If a dropped column has a message, append it after the table as unordered list output.

Recommendation:

- gather all messages in drop order
- append them after the bottom border
- render them via the existing `UnorderedList` component if practical

If using `UnorderedList` is awkward inside the current render flow, the first implementation can emit the same bullet format directly, but the design target should still be to reuse the list component.

## Public Introspection API

The measurements should be accessible from `Table`, as requested.

Recommended methods:

```rust
impl Table {
    pub fn available_render_width(&self, terminal_width: u32) -> u32;
    pub fn measure_widths(&self, terminal_width: u32) -> Result<TableWidthMeasurements, TableWidthError>;
    pub fn plan_widths(&self, terminal_width: u32) -> Result<TableWidthPlan, TableWidthError>;
    pub fn would_wrap(&self, terminal_width: u32) -> Result<bool, TableWidthError>;
}
```

Notes:

- these methods should resolve layout margins exactly the same way rendering does
- `plan_widths()` should include dropped-column results
- tests can inspect `natural_break_width`, `working_width`, `word_wrap_needed`, and final `resolved_width`

If exposing the full internal structs publicly feels too heavy, expose read-only public view types and keep the fully detailed planner structs crate-private.

## Renderer Integration

Both renderers should consume the same `TableWidthPlan`.

### Standard Renderer

`render_content()` should:

1. build the width plan
2. render headers and rows using `plan.columns[*].resolved_width`
3. append any dropped notes

### Cursor Renderer

`render_with_cursor_positioning()` should:

1. build the same width plan
2. compute `table_width`, `table_start`, and `fill_end_col` from the planned widths
3. recompute `max_content_widths` only for the final visible columns

Do not drop columns after cursor positions are computed.

## Measurement Helpers

Add small helpers to keep the planner readable and testable:

- `measure_explicit_line_widths(content: &str) -> Vec<usize>`
- `measure_max_explicit_line_width(content: &str) -> usize`
- `measure_break_segments(content: &str, wrap: &WordWrap) -> Vec<usize>`
- `column_break_tokens(wrap: &WordWrap) -> BreakTokenPolicy`
- `formatted_column_cells(&self, column_index: usize) -> impl Iterator<Item = String>`

This also fixes the current header bug cleanly by centralizing line-aware measurement.

## Error Handling

Preserve the current user-facing behavior of returning a string error when the table cannot be rendered, but drive it from a structured error.

Suggested error content:

- available render width
- border overhead
- fixed width consumption
- non-wrapping consumption
- remaining working width
- which columns blocked rendering
- whether any droppable columns were available

Example shape:

```text
Table could not be rendered in 42 columns.
Required content width after fixed and non-wrapping columns: 51.
Unable to fit wrapping columns even after dropping eligible columns.
```

## Interaction With Existing Types

### `Layout`

No semantic changes required. `Layout` remains the source of renderable width. The important change is that `Table` should stop doing ad hoc width logic independently in both render paths.

### `WordWrap`

No enum changes are required for the first version. The planner only needs to interpret existing variants more carefully:

- `None`: preserve full width
- `WrapProse`: natural-break measurement by prose tokens
- `BespokeProse`: natural-break measurement by configured tokens
- `Truncate`: shrinkable with minimal natural width

### `TableColumn`

Add:

- `drop_when_space_is_limited(...)`
- internal `DropBehavior`

Keep existing width and wrap builders unchanged.

## Testing Strategy

Add focused unit tests around planning first, then rendering.

### Measurement Tests

- multi-line header measures by widest explicit line, not total string width
- numeric columns are classified as non-wrapping
- string column with `with_word_wrap(WordWrap::None)` is classified as non-wrapping
- `max_width` caps `columnar_width_requirement`
- `min_width` floors `columnar_width_requirement`
- `fixed_width` bypasses measurement-based sizing

### Natural Break Tests

- `WrapProse` natural break width uses longest token, not longest line
- `BespokeProse` honors custom break chars
- `Truncate` produces a tiny shrinkable natural break width
- columns with requirement `<= 5` keep that exact natural break width

### Planning Tests

- no-wrap-needed case returns full `columnar_width_requirement`
- wrapping-needed case fits shrinkable columns into `working_width`
- non-wrapping consumption prevents protected columns from shrinking
- surplus width redistribution expands toward unwrapped width without exceeding it

### Drop Tests

- rightmost droppable column drops first
- multiple droppable columns can be removed until fit succeeds
- silent drop emits no note
- message drop appends note after render
- no droppable columns returns structured width error

### Rendering Regression Tests

- standard and cursor renderers use identical planned widths
- `uniform_alignment` is recomputed after drops
- striped rows remain visually intact after planning changes
- existing explicit-newline and ANSI-color cell behavior still works

## Implementation Sequence

1. Add line-aware measurement helpers.
2. Introduce `MeasuredColumn`, `TableWidthMeasurements`, `TableWidthPlan`, and `TableWidthError`.
3. Replace `calculate_column_widths()` and `constrain_widths_to_available()` with the new planner.
4. Update both render paths to consume the shared plan.
5. Add `DropBehavior` and `drop_when_space_is_limited(...)`.
6. Append dropped-column notes after rendering.
7. Add planner-focused tests before touching cursor-path rendering details.
8. Remove or reduce old width-calculation helpers once parity is proven.

## Recommended Initial Refactor Boundary

The cleanest first refactor is:

- keep `wrap_cell_content()`, `calculate_row_heights()`, `apply_vertical_padding()`, `pad_cell()`, and `render_row_with_cursor_positioning()` largely intact
- replace only the width-discovery phase

That gives the new behavior without forcing a rewrite of the stable rendering mechanics.

## Final Recommendation

Implement the redesign as a shared table width planner, not as incremental patches to `calculate_column_widths()`. The current bugs are coming from missing concepts:

- explicit line-aware measurement
- true non-wrapping protection
- natural-break sizing
- structured failure and column dropping

Once those concepts exist in a `TableWidthPlan`, the rest of the renderer becomes simpler, more testable, and much less likely to repeat the current width mistakes.
