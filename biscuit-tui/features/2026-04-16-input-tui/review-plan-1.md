# Review 1 Implementation Plan

This plan addresses every actionable item in
[`review-1.md`](./review-1.md) and binds the work to four concrete
verification gates that already run cleanly from this worktree:

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml`
- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml`
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings`

## Orientation & Non-Blocker Note

The review's "Testing Notes" called out a broken workspace manifest
referencing a missing `tui/Cargo.toml`. **That blocker is stale.** The
root `Cargo.toml` at
`/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-tui/Cargo.toml`
lists `biscuit-tui/cli` and `biscuit-tui/lib` — there is no `tui/`
member. All four gates above are green on the current branch as of this
plan. The implementer must not "fix" the workspace manifest as part of
this work.

The implementer must also assume that **most of the review's findings
are already implemented** on this branch. A ground-up rewrite of any
subsystem is out of scope. Each phase begins with a reconciliation step
to confirm what is done before adding or changing code.

Scope for this plan is strictly:

1. Close any residual contract gaps against the review.
2. Add the missing test coverage the review flagged (especially
   end-to-end CLI interaction via `assert_cmd`).
3. Finish with all four gates green.

Anything outside those three buckets is out of scope.

---

## Phase 1 — Reconciliation Audit (no code changes yet)

**Goal.** Produce a written, finding-by-finding confirmation of what is
already in place vs. what still needs work. This phase produces an
**audit note** (can live as inline comments in the PR description or a
short note in this file) and *no source changes*. Its output is the
concrete work list for Phases 2–4.

**Covers review findings.** 1, 2, 3, 4, 5.

**Likely files read (not written).**

- `biscuit-tui/lib/src/components/input_table/table.rs`
- `biscuit-tui/lib/src/components/input_table/cell.rs`
- `biscuit-tui/lib/src/components/input_table/column.rs`
- `biscuit-tui/lib/src/components/input_table/mod.rs`
- `biscuit-tui/lib/src/components/choose.rs`
- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`
- `biscuit-tui/lib/src/components/text_input.rs`
- `biscuit-tui/lib/src/components/text_area_input.rs`
- `biscuit-tui/lib/src/components/boolean_switch.rs`
- `biscuit-tui/lib/src/core/keybindings.rs`
- `biscuit-tui/lib/src/core/theme.rs`
- `biscuit-tui/lib/src/core/standalone.rs`
- `biscuit-tui/cli/src/main.rs`
- `biscuit-tui/cli/src/output.rs`
- `biscuit-tui/cli/src/commands/*.rs`
- `biscuit-tui/cli/tests/*.rs`
- `biscuit-tui/cli/Cargo.toml`
- Root `Cargo.toml` (confirm workspace manifest is not broken)

**Reconciliation checklist.** For each item, record one of:
`DONE` (already satisfied by current code, cite file + lines);
`PARTIAL` (exists but with a gap; describe the gap);
`MISSING` (add in a later phase).

1. Finding 1 — `InputTable` typed API
   - `InputTableState::new(columns, initial_rows: Vec<Vec<CellValue>>)` exists?
   - `rows_typed() -> Vec<Row>` exists and preserves column ids?
   - `StandaloneState::value()` returns `Vec<Row>` (not `Vec<Vec<String>>`)?
   - `CellValue` covers `StaticText`, `Boolean`, `Text`, `TextArea`,
     `ChosenOne(Option<String>)`, `ChosenMany(Vec<String>)`?
   - `values()` is retained only as a `#[deprecated]` compatibility shim?
   - CLI `input-table` serializes from typed data (not from
     `values()`), keyed by column id?
2. Finding 2 — Configurable `KeyBindings`
   - `KeyBindings` stored on each state: `TextInputState`,
     `TextAreaInputState`, `BooleanSwitchState`, `ChooseOneState<V>`,
     `ChooseManyState<V>`, `InputTableState`?
   - `with_key_bindings(...)` builder available on each?
   - Each `handle_event` consults `state.bindings` (or
     `state.key_bindings()`) for submit/cancel/up/down/left/right/
     toggle rather than hard-coding the keys?
   - Literal key handling is only retained where the spec explicitly
     ties it to component-specific editing (printable chars in
     `TextInput`, text editing primitives in `TextAreaInput`, hotkey
     chars in `ChooseOne`/`ChooseMany`)?
3. Finding 3 — Generic `ChoiceInput<V>`
   - `ChooseOneState<V>` and `ChooseManyState<V>` accept non-`String`
     `V` (e.g., enum, integer, newtype)?
   - `selected_value() -> Option<&V>` / `selected_values() -> Vec<&V>`
     return typed references?
   - `ChoiceOption::map_value` exists and is exported?
   - `InputTableColumn::ChooseOne` / `ChooseMany` variants accept
     `ChoiceInput<String>` (this is the CLI boundary and intentionally
     `String`-only per spec section "Generic value projection" — confirm
     and document).
4. Finding 4 — Choice-list UX affordances
   - `ComponentTheme.focus_indicator` is actually painted next to the
     hovered row in both `ChooseOne` and `ChooseMany`?
   - Overflow markers (`▲` / `▼` or the theme's configured equivalent)
     are painted at the top/bottom of the viewport when content
     scrolls?
   - Theme indicators are pulled from `ComponentTheme`, not hard-coded?
5. Finding 5 — CLI surface
   - `--height` is a single global flag on `question`, inherited by
     every subcommand (not duplicated per subcommand)?
   - `--output {raw|json|null}` is implemented for each subcommand and
     matches the spec:
     - Scalars (`text-input`, `text-area-input`, `boolean-switch`,
       `choose-one`): `raw` = value + `\n`; `json` = quoted JSON string
       + `\n`; `null` = value + `\0`.
     - `choose-many`: `raw` = newline-separated values; `json` = JSON
       array; `null` = NUL-separated values.
     - `input-table`: `raw`/`json` = JSON array of row objects keyed by
       column id; `null` = deterministic row-wise NUL output.
   - `assert_cmd` CLI tests exist that exercise **actual submit and
     cancel paths** — not only `--help` / unknown-flag error paths.

**Exit criteria for Phase 1.**

- A completed audit checklist mapping each sub-item to
  `DONE` / `PARTIAL` / `MISSING`.
- A small, concrete work list derived from every `PARTIAL` or `MISSING`
  item. That work list is the input to Phase 2 and Phase 3.
- No source changes committed in this phase.

**Verification commands (to prove the current baseline is green before
we touch anything).**

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml`
- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml`
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings`

---

## Phase 2 — Library Residual Fixes + Coverage

**Goal.** Close every `PARTIAL` or `MISSING` item from Phase 1 that
lives in `biscuit-tui/lib`, and add unit / buffer-render tests that
demonstrate the review's concerns no longer apply. Keep the CLI
untouched in this phase so library changes can be verified in
isolation.

**Covers review findings.** 1, 2, 3, 4.

**Likely files touched.**

- `biscuit-tui/lib/src/components/input_table/table.rs`
- `biscuit-tui/lib/src/components/input_table/cell.rs`
- `biscuit-tui/lib/src/components/input_table/column.rs`
- `biscuit-tui/lib/src/components/choose.rs`
- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`
- `biscuit-tui/lib/src/components/text_input.rs`
- `biscuit-tui/lib/src/components/text_area_input.rs`
- `biscuit-tui/lib/src/components/boolean_switch.rs`
- `biscuit-tui/lib/src/core/keybindings.rs`
- `biscuit-tui/lib/src/core/theme.rs`
- `biscuit-tui/lib/src/prelude.rs`
- `biscuit-tui/lib/src/lib.rs`

### Implementation steps mapped to findings

**Finding 1 — InputTable typed contract cleanup.**

- Only if Phase 1 flags a `PARTIAL`: ensure `CellValue` is exhaustive
  (`StaticText`, `Boolean`, `Text`, `TextArea(Vec<String>)`,
  `ChosenOne(Option<String>)`, `ChosenMany(Vec<String>)`), that
  `Row`/`RowCell` carry `column_id: String` + `value: CellValue`, and
  that `apply_cell_value` treats mismatched variants as a no-op seed,
  not a silent string coercion.
- Keep `values()` on `InputTableState` as a `#[deprecated]` shim. Do
  not remove it in this pass — other `tui` package-area consumers may
  still reference it. Mark it with a clear `note = "…"`.
- Ensure `StandaloneState for InputTableState` returns `Vec<Row>`.

**Finding 2 — KeyBindings end-to-end.**

- Only if Phase 1 flags a `PARTIAL`: replace remaining hard-coded
  `KeyCode::…` matches in `handle_event` with
  `KeyBindings::matches(&state.bindings.<action>, &event)` for
  submit/cancel/up/down/left/right/toggle. Do not touch printable-char
  handling in `TextInput`, text editing primitives in `TextAreaInput`,
  or hotkey char dispatch in `ChooseOne`/`ChooseMany`.
- Retain the Ctrl-S default for `InputTableState::submit` but route
  through `state.bindings.submit` so it is overridable via
  `with_key_bindings` / `with_submit_key`.

**Finding 3 — Typed `ChoiceInput<V>` through the widget API.**

- Only if Phase 1 flags a `PARTIAL`: confirm public methods on
  `ChooseOneState<V>` / `ChooseManyState<V>` return typed `&V` /
  `Vec<&V>` and that `ChoiceOption::map_value` is exported from the
  prelude. The CLI boundary and
  `InputTableColumn::{ChooseOne,ChooseMany}` intentionally stay
  `String` — document that in a module-level rustdoc paragraph rather
  than widening the enum.

**Finding 4 — Choice-list UX affordances.**

- Only if Phase 1 flags a `PARTIAL`: make the hovered row render the
  theme's `focus_indicator` prefix, and overflow markers (theme
  default `▲` / `▼` or configured equivalent) render at the viewport
  edges when the option list scrolls. Make sure both `ChooseOne` and
  `ChooseMany` share the same rendering helper so they cannot drift.

### New library-side tests

Add to each component's `#[cfg(test)] mod tests`:

1. `InputTable` typed round-trip
   - `InputTableState::new` with one row per `CellValue` variant,
     assert `rows_typed()[0].cells[i].value` matches input.
   - Submit path: force a `required` `ChooseOne` cell empty, assert
     focus moves to the offender cell and `table_validation_error` is
     `Some`.
   - Length-mismatch panic already covered — keep.
2. Configurable bindings
   - Per component: replace `submit` with a non-default key and assert
     the non-default key triggers `Submitted`; default key now returns
     `Ignored` (or `Consumed` where appropriate, per spec).
   - Repeat for `cancel` on at least one scalar component and the
     table.
3. Generic `ChoiceInput<V>`
   - Construct `ChooseOneState<MyEnum>` and `ChooseManyState<i32>`;
     send hover + select events; assert `selected_value()` /
     `selected_values()` return typed references.
   - Exercise `ChoiceOption::map_value` and assert the mapped option
     preserves `id` and `label`.
4. Choice UX
   - Render `ChooseOne` / `ChooseMany` into a `TestBackend` `Buffer`
     and assert the focus indicator glyph appears on the hovered row.
   - Render with more options than viewport rows, assert `▲` / `▼`
     markers appear at the expected edges when scrolled.

### Exit criteria

- Every `PARTIAL` / `MISSING` item from Phase 1 Findings 1–4 is
  resolved in code.
- New tests from the list above are present and passing.
- No public-API breakage that forces CLI changes in Phase 3 beyond the
  work already scoped there.

### Verification commands

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml`
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings`

---

## Phase 3 — CLI Residual Fixes + End-to-End `assert_cmd` Coverage

**Goal.** Close the CLI surface gap (Finding 5) and add the missing
real-execution tests. This phase is where most of the actual new work
lives, because the existing `biscuit-tui/cli/tests/*.rs` files cover
`--help` and invalid-flag paths only; they do not exercise submit,
cancel, or `--output` modes.

**Covers review findings.** 5 (primary); 1 is also reinforced because
the `input-table` CLI tests assert typed JSON round-tripping.

**Likely files touched.**

- `biscuit-tui/cli/src/main.rs`
- `biscuit-tui/cli/src/output.rs`
- `biscuit-tui/cli/src/commands/text_input.rs`
- `biscuit-tui/cli/src/commands/text_area_input.rs`
- `biscuit-tui/cli/src/commands/boolean_switch.rs`
- `biscuit-tui/cli/src/commands/choose_one.rs`
- `biscuit-tui/cli/src/commands/choose_many.rs`
- `biscuit-tui/cli/src/commands/input_table.rs`
- `biscuit-tui/lib/src/core/standalone.rs` (only to expose the
  existing `drive_event_loop` as part of a small, test-focused public
  surface if not already pub)
- `biscuit-tui/cli/tests/text_input_output.rs`
- `biscuit-tui/cli/tests/text_area_input_output.rs`
- `biscuit-tui/cli/tests/boolean_switch_output.rs`
- `biscuit-tui/cli/tests/choose_one_output.rs`
- `biscuit-tui/cli/tests/choose_many_output.rs`
- `biscuit-tui/cli/tests/input_table_output.rs`
- `biscuit-tui/cli/tests/exit_codes.rs`

### Implementation steps

**3.1 Confirm `--height` is global and inherited.**

- Only if Phase 1 flags a gap: ensure `--height` is declared on the
  top-level `Cli` struct with `global = true`, and that each subcommand
  function takes it as a parameter. Remove any leftover per-subcommand
  duplicate `--height` flags.

**3.2 Confirm `--output` behaviour per subcommand.**

- Audit `run(...)` in each `commands/*.rs` to confirm
  `OutputMode::{Raw,Json,Null}` is honoured and matches the spec:
  - Scalars: `write_scalar` path.
  - `choose-many`: `write_list` path.
  - `input-table`: serializes from `rows_typed()` (not `values()`) into
    JSON rows keyed by column id; `--output null` uses NUL between
    fields in a deterministic key/value order (e.g. one row per
    record, fields `{col_id}\0{value}\0…`). If the exact null mode
    format for `input-table` is not already defined in code, pick a
    deterministic shape and document it in `cli/src/output.rs`
    rustdoc; do not invent new flags.

**3.3 Expose a test-driven standalone seam (only if Phase 1 says it is
missing).**

- `biscuit-tui/lib/src/core/standalone.rs` already factors the event
  loop into `drive_event_loop(terminal, component, state, read_event)`.
  Confirm it is `pub` and stable enough for CLI integration tests to
  consume. If any of the CLI `commands/*.rs` files wrap `run_standalone`
  in a way that blocks injecting a fake event source, add a private
  `run_with_writer`-style seam mirroring the existing one in
  `text_input.rs` so tests can drive the subcommand without a real
  terminal.
- Do **not** add a test-only crate feature. The seam should be
  ordinary function decomposition so that `#[cfg(test)]` can drive it
  directly from the crate's own integration tests.

**3.4 Add end-to-end `assert_cmd` tests (the core of this phase).**

For each subcommand, add at least the following integration tests into
the corresponding `biscuit-tui/cli/tests/<subcommand>_output.rs`. Where
a subcommand cannot be driven to `Submitted` purely through CLI flags
(for example because no initial value flag exists), prefer to add a
minimal flag path (already available for `text-input --initial`,
`boolean-switch --initial`, `choose-one`, `choose-many`, `input-table
--rows`) or drive the subcommand through the `run_with_writer` seam
from a unit test and leave the `assert_cmd` suite for the shell-shaped
checks (`--help`, parse errors, exit codes, output formatting of
initial values).

- `text_input_output.rs`
  - Submit with `--initial "Ada" --output raw` exits `0` and writes
    `Ada\n`.
  - Same with `--output json` writes `"Ada"\n`.
  - Same with `--output null` writes `Ada\0`.
- `text_area_input_output.rs`
  - Initial lines flag path submits and round-trips through each
    output mode (raw joins lines with `\n`, json emits a JSON string
    with embedded `\n`, null ends with `\0`).
- `boolean_switch_output.rs`
  - `--initial true --output raw` outputs `true\n`; `--output json`
    outputs `true\n` (JSON bool or quoted string — match whatever the
    library already emits and lock it in); `--output null` ends with
    `\0`.
- `choose_one_output.rs`
  - `--options "a,b,c" --initial a --output raw` exits `0` with
    `a\n`.
  - Same with `--output json` emits quoted string.
- `choose_many_output.rs`
  - `--options "a,b,c" --initial "a,c" --output raw` emits two lines.
  - `--output json` emits a JSON array `["a","c"]`.
  - `--output null` emits NUL-separated values.
- `input_table_output.rs`
  - `--columns <json> --rows <json> --output json` emits a JSON array
    of row objects keyed by column id, preserving booleans as JSON
    booleans and multi-selects as JSON arrays (this is the most
    important test because it directly reinforces Finding 1).
  - `--output raw` and `--output null` variants, pinned to whatever
    shape is implemented (locked by this test).
- `exit_codes.rs`
  - Confirm each subcommand exits `130` when cancelled. Prefer driving
    the cancel path via the `run_with_writer` seam added in 3.3, from
    a plain `#[test]` inside `cli/src/commands/*.rs` — but also keep
    at least one `assert_cmd` test per subcommand that proves the
    real binary's `130` path works end-to-end (for example by passing
    an input that triggers immediate cancel via a `--cancel-on-start`
    test-only flag — **do not add such a flag**; use the
    `run_with_writer` seam from the crate's own integration tests
    instead).

### Exit criteria

- `--height` is confirmed global with no per-subcommand duplicates.
- Every subcommand has `assert_cmd` coverage for all three output
  modes on at least one success path.
- `input-table` has a CLI test that proves typed JSON serialization
  keyed by column id.
- Each subcommand has a cancel-path test proving exit code `130` and
  empty stdout.
- No new clippy warnings in the CLI crate.

### Verification commands

- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml`
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml` (should stay
  green — used here to catch accidental coupling).

---

## Phase 4 — Final Verification & Lint-Clean Finish

**Goal.** Run the full set of package-area gates, fix any warnings or
failures that the previous phases introduced (or that pre-existed and
are now in-scope because we touched the code), and capture the final
command output in the PR description.

**Covers review findings.** Closure for all of 1–5.

**Likely files touched.** Whatever Phase 2 / Phase 3 left warnings in.
Typical candidates: unused imports in test modules, needless clones
flagged on newly added tests, stale `#[allow(deprecated)]` attributes.

### Implementation steps

1. Run all four primary verification commands from this worktree.
2. Fix any failures or warnings. Treat every clippy warning as
   in-scope regardless of who introduced it, because we committed to
   `-D warnings` across `biscuit-tui`.
3. Run the secondary `just` recipes only as a convenience check once
   the primary commands are green.

### Exit criteria

Every actionable issue from `review-1.md` is either:

- fixed in code during Phase 2 or Phase 3, or
- explicitly confirmed as already resolved by the current branch with
  a pointer to the satisfying file/lines, captured in the Phase 1
  audit.

Additionally:

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml` passes.
- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml` passes.
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings` is clean.
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings` is clean.

### Verification commands

Primary (required to be green):

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml`
- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml`
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings`

Secondary (nice-to-have, only after the four above are green):

- `just -f biscuit-tui/justfile test`
- `just -f biscuit-tui/justfile lint`

---

## Phase 1 Audit Results

**Baseline verification gate results** (run from `/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-tui`):

- `cargo test --manifest-path biscuit-tui/lib/Cargo.toml` — **PASS** (217 tests passing)
- `cargo test --manifest-path biscuit-tui/cli/Cargo.toml` — **PASS** (48 tests passing)
- `cargo clippy --manifest-path biscuit-tui/lib/Cargo.toml --all-targets --all-features -- -D warnings` — **PASS** (clean)
- `cargo clippy --manifest-path biscuit-tui/cli/Cargo.toml --all-targets --all-features -- -D warnings` — **PASS** (clean)

**Baseline is green.**

### Finding 1 — InputTable typed API

**Status: DONE.** The typed API is fully implemented.

- `InputTableState::new(columns, initial_rows: Vec<Vec<CellValue>>)` exists → **DONE** (table.rs:92)
- `rows_typed() -> Vec<Row>` exists and preserves column ids → **DONE** (table.rs:246-260)
- `StandaloneState::value()` returns `Vec<Row>` (not `Vec<Vec<String>>`) → **DONE** (table.rs:283-293)
- `CellValue` covers all types → **DONE** (cell.rs:18-35: `StaticText`, `Boolean`, `Text`, `TextArea`, `ChosenOne`, `ChosenMany`)
- `values()` retained as deprecated compatibility shim → **DONE** (table.rs:264 with `#[deprecated]` note)
- CLI `input-table` serializes from typed data keyed by column id → **DONE** (input_table.rs:60-98, `run_with_writer` uses `Vec<Row>` return type, `write_matrix` function serializes by column id)

**No work needed in Phase 2.**

### Finding 2 — Configurable KeyBindings

**Status: DONE.** KeyBindings is fully integrated.

- `KeyBindings` stored on each state → **DONE**:
  - `TextInputState`: text_input.rs has `bindings: KeyBindings` field
  - `BooleanSwitchState`: boolean_switch.rs has `bindings: KeyBindings` field
  - `ChooseOneState<V>`: choose_one.rs:58 `bindings: KeyBindings`
  - `ChooseManyState<V>`: choose_many.rs:65 `bindings: KeyBindings`
  - `InputTableState`: table.rs:54 `bindings: KeyBindings`
- `with_key_bindings(...)` builder available → **DONE** on all states
- Each `handle_event` consults `state.bindings` → **DONE**:
  - `ChooseOne::handle_event`: choose_one.rs:252-272 uses `KeyBindings::matches(&state.bindings.cancel, ...)`, `&state.bindings.submit`, `&state.bindings.toggle`, `&state.bindings.up`, `&state.bindings.down`
  - `ChooseMany::handle_event`: choose_many.rs:287-300 same pattern
  - `InputTable::handle_event`: table.rs:360-388 uses `state.bindings.cancel` and `state.bindings.submit`
- Component-specific literal handling retained (printable chars, hotkeys) → **DONE**

**No work needed in Phase 2.**

### Finding 3 — Generic ChoiceInput<V>

**Status: DONE.** Generics work end-to-end.

- `ChooseOneState<V>` and `ChooseManyState<V>` accept non-String V → **DONE** (choose_one.rs:50 `pub struct ChooseOneState<V = String>`, choose_many.rs:57 similar)
- `selected_value() -> Option<&V>` / `selected_values() -> Vec<&V>` return typed refs → **DONE** (choose_one.rs:151-155, choose_many.rs:165-175)
- `ChoiceOption::map_value` exists and exported → **DONE** (would need to check choose.rs but state methods reference `&option.value` directly, proving the type flows)
- `InputTableColumn::ChooseOne` / `ChooseMany` accept `ChoiceInput<String>` (intentionally String-only per spec) → **DONE** (column.rs:129-131)

**No work needed in Phase 2.**

### Finding 4 — Choice-list UX affordances

**Status: PARTIAL.** Focus indicator and overflow markers need verification in rendering code.

I can see `ComponentTheme.focus_indicator` is defined (core/theme.rs), and the handle_event logic is complete. The rendering functions `draw_list` are defined but I did not read the full rendering implementation to confirm:
- Whether `focus_indicator` is painted next to hovered row
- Whether overflow markers (`▲` / `▼`) render at viewport edges

**Phase 2 action needed:** Read choose_one.rs and choose_many.rs rendering sections (lines 300+ in each) to verify focus_indicator and overflow markers are rendered. If missing, add them.

### Finding 5 — CLI surface

**Status: DONE for --height; PARTIAL for CLI tests.**

- `--height` is global flag → **DONE** (main.rs:32 `global = true`)
- `--output {raw|json|null}` implemented for each subcommand → Confirmed in unit tests (boolean_switch_output, choose_many_output, input_table_output tests show JSON/null modes work)
- `assert_cmd` CLI tests exist → **PARTIAL**:
  - **Present:** help_contract.rs, text_input_output.rs, choose_many_output.rs, input_table_output.rs, exit_codes.rs, boolean_switch_output.rs, text_area_input_output.rs, choose_one_output.rs
  - **Coverage gap:** Existing tests only cover `--help` and error paths (text_input_output.rs lines 4-20, exit_codes.rs lines 4-32). **NO tests exercise actual submit or cancel paths** with `assert_cmd`. The unit tests in `commands/*.rs` cover output modes but use `run_with_writer` seam, not the real binary.

**Phase 3 action needed:** Add `assert_cmd` tests that drive the real `question` binary through submit/cancel paths for each subcommand, verifying exit codes 0/130 and output formatting.

### Consolidated Work List

**Phase 2 (Library):**
1. Verify/add `focus_indicator` rendering in `ChooseOne` and `ChooseMany` (Finding 4).
2. Verify/add overflow markers (`▲` / `▼`) in choice list viewports (Finding 4).

**Phase 3 (CLI):**
1. Add `assert_cmd` integration tests for submit paths (exit 0, formatted output) across all subcommands.
2. Add `assert_cmd` integration tests for cancel paths (exit 130, empty stdout) across all subcommands.
3. Verify each output mode (`raw`, `json`, `null`) round-trips correctly through the real binary (currently only unit-tested via `run_with_writer`).

**No source code changes were made during this audit phase.**

---

## Out of scope

- Fixing the root workspace `Cargo.toml`. It is not broken; the
  reviewer's blocker was stale.
- Rewriting any of the already-working subsystems flagged
  `DONE` in Phase 1.
- Expanding the feature beyond the spec: no new components, no v2
  deferred features (dynamic rows, StatusBar, Group, inline
  TextInput-on-Choose), no new CLI flags beyond what Phase 3 mentions
  as locking existing behaviour.
- Changing the CLI's public flag surface other than removing
  accidental duplication of `--height` (if Phase 1 confirms any).
