---
date: 2026-04-20
feature: biscuit-tui/features/2026-04-16-input-tui
ready: true
---

# Review 4: TUI Inputs Implementation

This fourth review evaluates the `tui-chrome` library and the `question`
CLI after the review-1 plan was fully implemented. The core surface
area is now very strong: typed `InputTable` rows (`Vec<Row>`),
configurable `KeyBindings` everywhere, a generic `ChoiceInput<V>` end
to end, overflow markers and focus-indicator glyphs on choice lists,
and a global `--height` on `question`. `cargo test` is green (217 lib
tests + 48 CLI tests) and `cargo clippy -- -D warnings` is clean on
both crates.

The remaining gaps fall into three buckets: **two production blockers
that have rolled forward from reviews 2 and 3 without being
resolved**, a handful of medium-severity spec/test-coverage gaps, and
several small documentation/ergonomics polish items.

## Verdict

`ready: false`. Two of the blockers — `InputTable` Up/Down navigation
and `InputTable` row scrolling — prevent the table container from
being usable for its intended embedding use cases. Either needs to be
fixed before the library can be recommended to consumers like
Claudine/darkmatter.

---

## 1. Production blockers (carried forward, not resolved)

### 1.1 `InputTable` Up/Down always navigates rows — breaks `ChooseOne` / `ChooseMany` cells

**Severity: High.** Review 3 (§1.1) called this out; it is still
present in `biscuit-tui/lib/src/components/input_table/table.rs:478-513`.

`try_navigate` unconditionally consumes `KeyCode::Up` / `KeyCode::Down`
and translates them into row focus movement *before* the focused cell
ever sees the event. The only special case is horizontal arrows being
allowed to pass into editable text cells (`inside_text_cell` branch at
line 485). There is no equivalent allow-list for `ChooseOne` or
`ChooseMany` cells, so once a user focuses a choice cell inside a
table, the arrow keys can no longer walk the option list — they jump
to the next row instead.

The practical consequence is that every `InputTable` containing a
`ChooseOne`/`ChooseMany` column forces the user into vim `j`/`k` (or
single-letter hotkeys) to pick options, which contradicts the spec's
"main navigation will be arrow keys (and vim keys for directional
navigation as backup)".

**Suggested fix.** Extend the `inside_text_cell` idea into an
`inside_choice_cell` branch: when the focused cell is
`CellState::ChooseOne(_)` or `CellState::ChooseMany(_)`, let Up/Down
fall through to `route_to_focus` instead of being consumed. Row
navigation inside choice cells can then use Tab/BackTab (already
supported) or an explicit chord (e.g. `Alt+Up`/`Alt+Down`) — add one
integration test per case.

### 1.2 `InputTable` has no row-level scrolling

**Severity: High.** Review 2 (§2) called this out; still unresolved.

`InputTable::render` calls `layout_rows` with a fixed
`area.height`-derived budget
(`biscuit-tui/lib/src/components/input_table/table.rs:311-347`). When
the sum of `row_heights` exceeds that budget, `ratatui::Layout` clamps
later rows to zero height and they silently disappear. There is no
`scroll_offset`, no viewport adjustment when focus moves off-screen,
and no overflow indicator at the grid edge.

For tables with more than a few rows — or with multi-row cells
(`TextAreaInput`, tall `ChooseMany`) — the trailing rows become
invisible **and unreachable**, since `move_focus` only wraps; it does
not scroll. Attempting Tab/Down past the last visible row still moves
focus, but the rendered frame never follows.

**Suggested fix.** Add a `row_scroll_offset: usize` to
`InputTableState`, adjust it in `move_focus`/`move_tab` to keep the
focused cell inside the visible window (mirror `adjust_scroll` in
`ChooseOneState`), and paint `▲` / `▼` overflow markers at the top /
bottom edge of the grid when rows are hidden. Cover both "focus
scrolls down past the viewport" and "focus returns up past the
viewport" with buffer-render tests.

---

## 2. Spec deviations (medium)

### 2.1 `ChoiceInput::shuffle_options` is declared but ignored

Review 2 (§1) flagged this; still unresolved. The field is defined in
`biscuit-tui/lib/src/components/choose.rs:124` and serialized in
`Default`, but neither `ChooseOneState::new`
(`biscuit-tui/lib/src/components/choose_one.rs:68-83`) nor
`ChooseManyState::new`
(`biscuit-tui/lib/src/components/choose_many.rs:73-89`) consults the
flag. A caller who sets `shuffle_options = true` silently gets
deterministic option order.

**Fix.** In each `new`, when `input.shuffle_options` is `true`, shuffle
`input.options` (and remap the `hover`/`hotkeys` indices afterwards)
using a stable seeded source. `rand` is not in scope for the crate;
the simplest dependency-light approach is to require the caller to
pre-shuffle — in which case drop the field from `ChoiceInput` and
update the spec. Implementing it or removing it is fine; leaving a
silently-ignored public field is not.

### 2.2 JSON5 dictionary input is claimed in spec but not implemented

Spec section "Helpers" requires `choose_one_from_dictionary()` to
accept "a JSON5 or YAML structure". The implementation in
`biscuit-tui/lib/src/helpers/choice_builders.rs:164-180` uses
`serde_yaml_ng` only. Because YAML is a superset of standard JSON,
strict JSON literals happen to round-trip, but JSON5 features
(trailing commas, unquoted keys, single-quoted strings, comments) are
rejected. Either add a real JSON5 parse attempt (e.g. try the `json5`
crate first, fall back to YAML), or narrow the spec text to "JSON or
YAML" and add a unit test that proves JSON5-specific features are
rejected with a helpful error.

### 2.3 Scalar `boolean-switch --output json` emits a JSON string, not a JSON boolean

`cli/src/commands/boolean_switch.rs:81-83` stringifies the boolean via
`bool_to_str`, then hands the `"true"` / `"false"` string to
`write_scalar`, which JSON-encodes it as `"\"true\""`. Compare to
`cli/src/commands/input_table.rs:462-471` where the same boolean
value inside an `InputTable` cell is emitted as a JSON boolean
(`json!(b)`).

The spec's "single JSON string for scalars" phrasing is admittedly
ambiguous, but the inconsistency is user-hostile: the same
`true`/`false` value round-trips differently depending on whether it
came from a scalar or a table. Pick one shape and align both paths —
either force scalars to emit `true`/`false` without quotes, or force
table booleans to be JSON strings. Lock the choice in a CLI
integration test.

### 2.4 No end-to-end `assert_cmd` coverage for submit / cancel paths

The spec explicitly calls for "strong unit and integration testing"
and the tech design's §9 lists CLI tests that "[verify] stdout
content matches the output contract per component" and "exit code is
`0` on submit, `130` on cancel". The current integration tests in
`biscuit-tui/cli/tests/*.rs` are limited to `--help` parsing, unknown
flags, and static argument errors. All submit/cancel coverage lives
inside `#[cfg(test)] mod tests { … }` blocks that call the private
`run_with_writer` seam directly — they bypass `clap` parsing, the real
`main` dispatch, and the process exit code path.

The review-1 plan (Phase 3.4) listed exactly these tests as mandatory;
the audit in Phase 1 recorded the CLI suite as `PARTIAL` and the
follow-through never landed. Add `assert_cmd` tests that shell out to
the real `question` binary with `--initial`/`--rows`/`--columns` flags
that make the submit path deterministic, and assert:

- exit code `0` with the expected `--output {raw|json|null}` bytes on
  stdout for at least one happy path per subcommand,
- exit code `130` with empty stdout on cancel (drive cancellation via
  an input pipe that closes — or via the already-wired Ctrl-C handler
  in `drive_event_loop`).

### 2.5 `InputTable` choice-cell heights are still aggressive

Review 2 (§3) flagged this; unchanged. `CellState::min_height` at
`biscuit-tui/lib/src/components/input_table/cell.rs:178-185` returns
`state.options().len()` for `ChooseOne`/`ChooseMany` cells. A cell
with 10 options demands 10 rows. Combined with blocker 1.2, a single
`ChooseMany` column with a realistic option list pushes rows off the
screen immediately.

**Fix.** Cap the min-height (e.g. at 3–5 rows) and let the individual
choice widget's own viewport scrolling take over — that code already
paints `▲`/`▼` overflow markers via the theme. Add a regression test
that confirms a 10-option cell renders in a 5-row allocation.

---

## 3. Polish / small ergonomics gaps

### 3.1 `ChooseOne`: selected-but-not-hovered option has no visual distinction

`biscuit-tui/lib/src/components/choose_one.rs:444-450` computes
`label_style` as `disabled_style` if disabled, else
`selected_style` when `idx == state.hover`, else `Style::default()`.
The *selected* state is only conveyed via the `●` glyph. Once the
user moves the hover off the selected row, the label reverts to plain
text and only the tiny indicator glyph tells them what they picked.
For a 20-option menu that's easy to lose.

**Fix.** Blend the two axes: e.g. hovered → reversed/bold; selected →
foreground colour or underline; hovered+selected → both. The theme
already exposes the styles; the change is inside `draw_list`.

### 3.2 `input_table/mod.rs` module docs are stale

The top-of-module rustdoc at
`biscuit-tui/lib/src/components/input_table/mod.rs:28-38` still
describes the old `state.value() → Vec<Vec<String>>` contract,
including a bullet list of string encodings per variant. The real
contract is `Vec<Row>` with typed `CellValue`s (see `table.rs:283-293`).
Library consumers reading that doc will assume the wrong return type.

**Fix.** Replace the "Value Shape" paragraph with a pointer to
`InputTableState::rows_typed` and show the typed round-trip shape.

### 3.3 Prelude omits `Row`, `RowCell`, `CellValue`

These are re-exported from the crate root (`lib.rs:23-27`) but not
from `prelude.rs`. A consumer who `use tui_chrome::prelude::*;` to get
`InputTable` still has to import `Row`/`CellValue` from
`tui_chrome::{...}` to read the typed rows.

**Fix.** Add them to the prelude — they are the value types a caller
needs alongside the widget.

### 3.4 No help hint in `run_standalone` about which key submits

Users running `question text-input` don't see any UI chrome telling
them Enter submits, while `question text-area-input` (where Enter
inserts a newline) silently requires Ctrl-S. Similarly the CLI help
for `question input-table` doesn't mention the Ctrl-S convention for
submit. The spec notes the non-negotiable defaults in §8 "Key Binding
Configuration" but does not require on-screen hints; still, a
one-line footer in `run_standalone` showing the active
submit/cancel binding would sharply reduce confusion.

### 3.5 `compute_column_widths` divides width evenly across all columns

`biscuit-tui/lib/src/components/input_table/table.rs:659-668`
allocates equal shares to every column regardless of type. The tech
design §4.6 calls for type-aware sizing — `StaticText` clamped to its
natural width, `BooleanSwitch` a small fixed size,
`TextInput`/`TextAreaInput` using their configured widths. With the
current implementation, a 1-character static label gets the same
horizontal share as a 40-column text area.

**Fix.** Collect a per-column preferred width (pulled from each
variant's config / natural width) and pass them as
`Constraint::Length` or `Constraint::Min` / `Constraint::Max` to
`Layout`. Falls back to equal split only when preferences sum under
the available width.

---

## 4. Strengths worth preserving

- Configurable `KeyBindings` is cleanly implemented: every component
  stores `bindings: KeyBindings`, exposes `with_key_bindings`, and
  routes events through `KeyBindings::matches`. The per-component
  tests cover both custom-binding and default-binding paths.
- The typed `Row`/`RowCell`/`CellValue` API is a big improvement; the
  CLI's `write_matrix` writes JSON booleans and JSON arrays correctly.
- `drive_event_loop` skips redraws on `Ignored` / key-release events
  and has good test coverage (`render_count` assertions).
- Overflow markers (`▲` / `▼`) and focus indicators are now rendered
  for `ChooseOne` / `ChooseMany`, with buffer-render tests locking
  them in.
- Validation lifecycle is correct: cell-level errors clear as soon as
  the offending constraint is resolved (e.g. selecting a required
  `ChooseOne` option clears the error before the next submit attempt).

---

## 5. Testing summary

| Target | Count | Notes |
|---|---|---|
| lib unit + render tests | 217 | Excellent; every component has state / render / key-binding coverage. |
| lib doctests | 10 | Cover the prelude example per component. |
| cli unit tests | 59 | Drive submit & cancel via `run_with_writer`; good output-mode coverage. |
| cli integration (`assert_cmd`) | 19 | Help and error-path only; no real-binary submit/cancel paths. |

The gap is `assert_cmd` submit/cancel coverage — the work item from
§2.4 above.

---

## 6. Suggested next-iteration priorities

1. Fix `InputTable` Up/Down delegation to choice cells (§1.1).
2. Add `InputTable` row scrolling with overflow markers (§1.2).
3. Either implement `shuffle_options` or remove it from the public
   `ChoiceInput` surface and update the spec (§2.1).
4. Land the `assert_cmd` submit/cancel integration tests (§2.4).
5. Align scalar vs table boolean JSON shape and pin it with a test
   (§2.3).

Once (1) and (2) are in, this feature can move to `ready: true` even
if the polish items from §3 are deferred.
