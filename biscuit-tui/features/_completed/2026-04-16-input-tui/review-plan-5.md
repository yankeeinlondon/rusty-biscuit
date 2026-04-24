# Review-5 Implementation Plan

Addresses all findings from [review-5](./review-5.md).

---

## Phase 1: Add `shuffle_options` to `ChoiceInput` (Finding 1.1)

### Changes

**`lib/src/components/choose.rs`**
- Add `shuffle_options: bool` field to `ChoiceInput<V>` (default `false`).
- Add `with_shuffle_options(mut self, shuffle: bool) -> Self` builder method.
- Update `ChoiceInput::new` to initialise `shuffle_options: false`.
- Update the `choice_input_new_defaults_to_single_mode` test to assert `shuffle_options` defaults to `false`.
- Update the `builder_methods_chain` test to cover `with_shuffle_options`.

**`lib/src/components/choose_one.rs`** — `ChooseOneState::new`
- After forcing `selection_mode = Single`, if `input.shuffle_options` is `true`, call `rand::rng()` + `rand::seq::SliceRandom::shuffle` on `input.options` **in place**.
- The shuffle happens before hotkeys are built and before `hover` is initialised, so the hotkey map and hover index naturally reflect the shuffled order.

**`lib/src/components/choose_many.rs`** — `ChooseManyState::new`
- Same shuffle logic as `ChooseOneState::new` — shuffle `input.options` in place when `shuffle_options` is `true`, before building `selected`, `hotkeys`, and `hover`.

**`lib/Cargo.toml`**
- Add `rand = "0.9"` to `[dependencies]`.

### Tests

| Test | Location | What it asserts |
|------|----------|-----------------|
| `shuffle_options_defaults_to_false` | `choose.rs` | New `ChoiceInput` has `shuffle_options == false`. |
| `with_shuffle_options_sets_flag` | `choose.rs` | Builder sets the flag. |
| `shuffle_randomises_order_choose_one` | `choose_one.rs` | Create a `ChooseOneState` with `shuffle_options(true)` and 20+ options; assert that the resulting option order differs from the original at least once (statistical — run a few rounds if needed, or use a seeded RNG for determinism). |
| `shuffle_randomises_order_choose_many` | `choose_many.rs` | Same as above for `ChooseManyState`. |
| `shuffle_false_preserves_order` | `choose_one.rs` | With `shuffle_options(false)` (default), option order is exactly as provided. |
| `shuffle_then_select_choose_one` | `choose_one.rs` | After shuffle, selecting by index and reading `.selected_value()` still returns the correct value object (values are not shuffled, only display order). |
| `shuffle_then_select_choose_many` | `choose_many.rs` | Same for multi-select. |
| `shuffle_preserves_hotkey_mapping` | `choose_one.rs` | After shuffle, hotkeys still map to the correct logical option by first-character match. |

### Lint checklist
- `cargo clippy --all-targets` on `tui-chrome` — ensure no unused import warnings from `rand`.
- `cargo test -p tui-chrome` — all existing + new tests pass.

---

## Phase 2: Add `choose_many_from_dictionary` helper (Finding 1.2)

### Changes

**`lib/src/helpers/choice_builders.rs`**
- Add public function:

  ```rust
  pub fn choose_many_from_dictionary(
      id: impl Into<String>,
      prompt: impl Into<String>,
      yaml_or_json: &str,
  ) -> Result<ChoiceInput<String>, ChoiceBuilderError> {
      Ok(ChoiceInput::new(id, prompt)
          .with_selection_mode(SelectionMode::Multiple)
          .with_options(options_from_dictionary(yaml_or_json)?))
  }
  ```

- This is structurally identical to the existing `choose_many_from_csv` / `choose_many_from_markdown_list` pattern — it wraps the shared `options_from_dictionary` with `SelectionMode::Multiple`.

### Tests

| Test | Location | What it asserts |
|------|----------|-----------------|
| `many_dictionary_parses_yaml_mapping` | `choice_builders.rs` | Returns `Ok`, `selection_mode == Multiple`, correct option count and values. |
| `many_dictionary_parses_json_object` | `choice_builders.rs` | Same for JSON input. |
| `many_dictionary_rejects_non_mapping` | `choice_builders.rs` | Returns `Err(ChoiceBuilderError::NotAMapping)`. |
| `many_dictionary_surfaces_parse_errors` | `choice_builders.rs` | Returns `Err(ChoiceBuilderError::Parse(_))`. |

### Lint checklist
- `cargo clippy --all-targets` on `tui-chrome`.
- `cargo test -p tui-chrome` — all existing + new tests pass.

---

## Phase 3: Fix `StaticText` sizing in `InputTable` (Finding 2.1)

### Changes

**`lib/src/components/input_table/table.rs`** — `compute_column_widths`

Current behaviour: leftover width is distributed equally to *all* columns (including `StaticText`).

New behaviour:
1. Compute `preferred` widths as before.
2. Identify `StaticText` columns — their preferred width is their **final** width.
3. Subtract the total `StaticText` preferred widths from `total_width` to get `available_for_focusable`.
4. Compute the sum of focusable-column preferred widths (`total_focusable_preferred`).
5. Distribute leftover width (if any) only among focusable columns, proportionally or equally.
6. If `total_preferred > total_width` (overflow case), fall back to the existing equal-split logic but still clamp `StaticText` columns to their preferred width (they should never shrink below natural width).

Pseudocode:

```rust
fn compute_column_widths(columns: &[InputTableColumn], total_width: u16) -> Vec<u16> {
    // ... compute preferred as now ...

    if columns.is_empty() { return Vec::new(); }

    let total_preferred: u32 = preferred.iter().map(|&w| w as u32).sum();

    if total_preferred <= total_width as u32 {
        // Sum of StaticText preferred widths.
        let static_total: u32 = columns.iter().zip(preferred.iter())
            .filter(|(col, _)| matches!(col, InputTableColumn::StaticText { .. }))
            .map(|(_, &w)| w as u32)
            .sum();

        let leftover = total_width as u32 - total_preferred;
        let focusable_count = columns.iter()
            .filter(|col| !matches!(col, InputTableColumn::StaticText { .. }))
            .count() as u32;

        if focusable_count == 0 {
            // All static — just use preferred.
            return preferred;
        }

        let per_focusable = leftover / focusable_count;
        let remainder = (leftover % focusable_count) as u16;
        let mut focusable_idx = 0u16;

        preferred.into_iter().enumerate().map(|(i, p)| {
            if matches!(columns[i], InputTableColumn::StaticText { .. }) {
                p  // StaticText stays at natural width
            } else {
                let extra = per_focusable as u16
                    + if focusable_idx < remainder { 1 } else { 0 };
                focusable_idx += 1;
                p + extra
            }
        }).collect()
    } else {
        // Overflow: use equal split, but clamp StaticText to preferred.
        let base = total_width / columns.len() as u16;
        let remainder = total_width % columns.len() as u16;
        (0..columns.len()).map(|i| {
            let base_w = base + if (i as u16) < remainder { 1 } else { 0 };
            if matches!(columns[i], InputTableColumn::StaticText { .. }) {
                base_w.min(preferred[i])
            } else {
                base_w
            }
        }).collect()
    }
}
```

### Tests

| Test | Location | What it asserts |
|------|----------|-----------------|
| `static_text_columns_stay_at_natural_width_with_leftover` | `table.rs` | 3 columns: StaticText("Hi"), TextInput, TextInput. Total width = 60. StaticText gets exactly 2 chars (natural width), not more. |
| `static_text_does_not_shrink_below_natural_in_overflow` | `table.rs` | 3 StaticText("LongLabel") + TextInput, total width = 10. StaticText gets its preferred width or the equal-split floor, whichever is smaller (should not go below natural). |
| `all_static_text_columns_use_preferred_widths` | `table.rs` | Only StaticText columns, total width > sum. No column exceeds natural width. |
| `render_static_text_stays_tight` | `table.rs` | Render a table with StaticText + TextInput columns at 40 cols wide. Read back the buffer and verify StaticText column cell width equals the text's unicode width (no trailing padding beyond what the layout allocates). |

### Lint checklist
- `cargo clippy --all-targets` on `tui-chrome`.
- `cargo test -p tui-chrome` — all existing + new tests pass.
- Specifically re-run `render_mixed_cell_table_paints_static_and_body` and any other render tests to verify no regressions.

---

## Phase 4 (Optional): Document `String`-only limitation for table choice columns (Finding 1.3)

### Decision

Making `InputTableColumn` generic over `V` would require `CellState`, `CellValue`, `Row`, `RowCell`, and `InputTableState` to all carry a `V` parameter, which cascades into every consumer and significantly complicates the API. The review marks this as **optional but recommended** — and the spec already states `ChoiceInput<String>` for `InputTableColumn` variants.

The recommended path is to **explicitly document the limitation** rather than refactor to generics.

### Changes

**`lib/src/components/input_table/column.rs`**
- Add a doc note to `InputTableColumn::ChooseOne` and `InputTableColumn::ChooseMany` variants:

  ```
  Table choice columns always operate on `ChoiceInput<String>`. Library
  consumers who need a typed value can use
  [`ChoiceOption::map_value`] on the options before passing them to the
  table, or project after extracting rows via
  [`InputTableState::rows_typed`].
  ```

**`spec.md`** (if the reviewer wants it updated)
- No spec change needed — the spec already specifies `ChoiceInput<String>` in the `InputTableColumn` enum. The finding is about the "same typed value flow" mandate; documenting the intentional limitation resolves the ambiguity.

### Tests

No new tests required — the existing `public_api_names.rs` integration test already confirms `InputTableColumn::ChooseOne(ChoiceInput<String>)` compiles. The documentation-only change needs no test coverage.

### Lint checklist
- `cargo doc -p tui-chrome --no-deps` — verify docs build without warnings.

---

## Phase 5: Final validation

Run the full validation suite:

```bash
cargo clippy --all-targets -p tui-chrome -- -D warnings
cargo clippy --all-targets -p tui-chrome-cli -- -D warnings
cargo test -p tui-chrome
cargo test -p tui-chrome-cli
cargo doc -p tui-chrome --no-deps
```

All tests must pass. All clippy lints must be clean. Docs must build without warnings.

---

## Summary of files touched

| File | Phase(s) | Nature of change |
|------|----------|-----------------|
| `lib/Cargo.toml` | 1 | Add `rand` dependency |
| `lib/src/components/choose.rs` | 1 | Add `shuffle_options` field + builder + tests |
| `lib/src/components/choose_one.rs` | 1 | Shuffle logic in `new()` + tests |
| `lib/src/components/choose_many.rs` | 1 | Shuffle logic in `new()` + tests |
| `lib/src/helpers/choice_builders.rs` | 2 | Add `choose_many_from_dictionary` + tests |
| `lib/src/components/input_table/table.rs` | 3 | Fix `compute_column_widths` + tests |
| `lib/src/components/input_table/column.rs` | 4 | Doc comment on `String` limitation |

Total: 7 files, ~15 new tests, 0 breaking API changes.
