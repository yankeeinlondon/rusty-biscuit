---
total_phases: 5
created: 2026-09-29
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - biscuit-tui/lib/Cargo.toml
    - biscuit-tui/lib/src/components/input_table/table/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - Cargo.lock
docs_updated_during_phase_1:
    - docs/dependencies.md
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - biscuit-tui
    - claudine-cli
    - claudine
source_files_during_phase_2:
    - biscuit-tui/lib/src/components/input_table/table.rs
    - biscuit-tui/lib/src/components/input_table/table/tests.rs
    - biscuit-tui/lib/src/components/input_table/column.rs
    - biscuit-tui/lib/src/components/input_table/cell.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/lib/src/composition/types.rs
docs_updated_during_phase_2:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - biscuit-tui/lib/src/components/input_table/table/tests.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Plan: sequence review screen

## Summary and definition of done

`claudine sequence` opens a review table (`InputTable` from `biscuit-tui`) with
one row per step. Three defects plus one row-mapping hazard are fixed in two
packages (`biscuit-tui` for layout and focus painting, `claudine-cli` for row
eligibility and mapping).

The work splits along a package boundary, so the two halves are independent
until the final docs and validation phase:

| Half | Package | Change |
| ---- | ------- | ------ |
| A | `biscuit-tui` | content-based static column widths, new allocation rules, `…` clipping that keeps graphemes intact, focus painting without blanket underline |
| B | `claudine-cli` | classify each step's outer executable, review only eligible drafts, merge submitted targets back at original indices, skip the table when nothing is eligible |

**Done means:** every acceptance criterion in the spec (1-7) is met and
evidenced; `just test` and `just lint` pass in `biscuit-tui` and `claudine`;
the `question` CLI unit tests still pass; docs describe current behavior
without naming this fix; and the stray-rule requirement is either closed with
buffer-style evidence or explicitly recorded as **unresolved**. The agent's
terminal state is "implementation complete, ready for review". It does not move
the spec to `_completed` or run `just complete`.

## Phase 1: Rulings, spike, and test scaffolding

Goal: settle the open design points, confirm the focus-rule cause once, and
lay down failing tests so later waves have a target.

### Necessary Rules

The spec leaves these unstated. Record each answer in the implementation log
before Wave 2 starts. Recommendations are given so the author can approve or
override quickly.

- [x] **R1: grapheme segmentation dependency.** The spec requires keeping
  graphemes intact, and `unicode-width` alone cannot segment. `unicode-segmentation`
  is already in `Cargo.lock` transitively but is not a direct dependency of
  `biscuit-tui`. *Recommendation:* add it as a direct dependency of
  `biscuit-tui/lib` and update `biscuit-tui/docs/dependencies.md` (and the
  root `docs/dependencies.md` if it lists per-crate crates).
- [x] **R2: what "display width" of a grapheme is.** *Recommendation:* the sum
  of `UnicodeWidthStr::width` per grapheme cluster (so a ZWJ emoji counts as
  the library reports for the whole cluster). A cluster wider than the
  remaining cells is dropped, never split, and the `…` takes its place.
- [x] **R3: width-2 character at a one-cell remainder.** A wide character that
  does not fit is omitted and the leftover cell is left blank rather than
  half-drawn. The `…` reserves one cell within the allocation, so at width 1
  the cell is `…` only.
- [x] **R4: where clipping lives.** *Recommendation:* clip when drawing
  (`draw_cell`), never mutate `CellState::StaticText` or returned `Row`
  values, so submission returns the full string.
- [x] **R5: eligibility keyed on `ExecutableField`.** Classify from
  `SequenceStep::executable.as_ref().map(|e| e.field)`: `None`, `Prompt`,
  `Task`, `Group` are eligible; `Shell` and `SideEffect` are not. This is a
  pure function of the normalized step; no composition or command execution.
- [x] **R6: what the row count check compares.** The submitted target count
  must equal the number of eligible drafts passed in. A mismatch is an error
  returned before any merge (not a panic, not a truncation). *Recommendation:*
  a dedicated `io::Error` kind or typed error distinct from the
  cancelled/aborted kinds so callers cannot mistake it for cancellation.
- [x] **R7: empty-eligible path and the provider gate.** When no eligible drafts
  remain, return baseline targets without calling the review function. The
  existing gate (`needs_review`, the no-TTY abort) is evaluated first and is
  unchanged, so a shell-only sequence without a resolvable provider still
  fails headless exactly as today. Confirm this ordering when implementing.
- [x] **R8: `Ctrl+S` on a table with hidden steps.** Hidden entries keep the
  baseline target and its reasons, including provider/model reason, produced by
  the same draft-to-target conversion as the review-bypassed branch. That
  conversion is currently an inline closure; extract it once so both branches
  share it (see Task 2.B1).
- [x] **R9: `SequenceStepDraft` location and label.** Row labels use the
  original one-based position (`step_index + 1`) and name. Confirm the draft
  keeps `step_index`, and that the label is built from it, never from the
  filtered row number.
- [x] **R10 (author decision, wider than the spec): cross-OS measurement.** No
  spike is scheduled beyond WezTerm/macOS. If the author wants the focus
  rendering checked on a Windows terminal, say so; this plan does not add it.

### Spike

- [x] **Focus-style spike (runs once, one host, headless).** In a scratch test in
  `biscuit-tui`, render an `InputTable` with a focused `ChooseOne` cell in a
  `ratatui` `Buffer` using the default `ComponentTheme`, and print each cell's
  `modifier` for the focused rectangle. Answers: does `UNDERLINED` land on blank
  cells (spec's stated candidate)? Does the first row differ from others? Does
  the previous focus's style clear after focus moves? Write the finding into the
  implementation log. If underline on blanks is *not* observed, mark
  Requirement "Focus does not draw rules" **unresolved** and do not proceed to
  Task 2.A3's removal claim; instead record the evidence (spec allows a
  real-terminal check only if headless cannot answer, and it must use the
  repository harness without window focus).
    - Do not repeat this in later phases; Task 2.A3 consumes its result.

### Tasks

- [x] **Load skills.** Load `biscuit-tui`, `rust-testing`, `ratatui`, and
  `claudine` before touching code. Prerequisite for every later task.
- [x] **Failing tests: layout (A).** Add tests in
  `biscuit-tui/lib/src/components/input_table/table/tests.rs` for spec criteria 1
  and 2 (headless render at 80 columns with schema `Step` and row `12 review-5`,
  off-screen longer row, resize down and up, narrow/emergency/zero/one widths,
  wide char, combining sequence, multiple static columns, all-static table,
  full text returned on submit). They fail until Wave 2.
- [x] **Failing tests: mapping (B).** Add tests in `claudine-cli` for criteria 3
  and 4 against the new pure functions' intended signatures (see Wave 2,
  Task 2.B1). Keep them compile-clean by adding the function stubs first.

- **Validation checkpoint 1:** R1-R9 recorded with answers; spike finding
  written; new tests compile and fail for the expected reason only.

## Phase 2: Implementation

Two independent waves. They touch different packages and can run in parallel.

### Wave 1: biscuit-tui layout and focus (Half A)

- [x] **Task 2.A1: Content widths.** In `compute_column_widths`
  (`biscuit-tui/lib/src/components/input_table/table.rs`), take the current
  `InputTableState` rows into account. For each `StaticText` column the
  preferred width is `max(width(schema text), width(value) for every row
  including off-screen rows, 3)`, computed with saturating `u32`/`usize`
  arithmetic and converted to `u16` with saturation. Recompute each render, so
  resize and caller row updates take effect. An empty table uses schema text
  alone. Change the signature to receive the rows (or a precomputed per-column
  max) rather than only `columns`. Check the call sites
  (`grep compute_column_widths`).
- [x] **Task 2.A2: Allocation rules.** Rewrite the else-branch so it follows the
  four spec tiers in order, each covered by the Phase 1 tests:
    1. Everything fits: static columns get preferred widths; focusable columns
       share leftover left-to-right remainder as today; an all-static table may
       leave unused space.
    2. Focusable extras removed first (they stay at their protected budgets:
       8 switch, 20 text/choice, `preferred_width` text area), then static
       columns shrink toward 3, reductions shared evenly, remainder left to
       right.
    3. If 3 per static plus focusable budgets does not fit: divide the width
       evenly across all columns, remainder left to right, static capped at
       preferred.
    4. The sum never exceeds `total_width`; zero-width columns allowed.
  Include a property-style test that sums never exceed the available width for
  a range of widths and column mixes.
  *Depends on:* 2.A1.
- [x] **Task 2.A3: Ellipsis clipping.** In `draw_cell` for `StaticText`, clip to
  the allocated width without mutating state: if the text fits, draw it;
  otherwise draw as many whole grapheme clusters as fit in `width - 1` cells
  followed by `…`. Width 1 draws only `…`; width 0 draws nothing. A cluster or
  wide char that would straddle the boundary is omitted. Rendering must never
  write outside `area`. (R1-R4.)
  *Depends on:* 2.A1 (can share the width helper); may proceed alongside 2.A2.
- [x] **Task 2.A4: Focus painting.** Using the spike result: in
  `paint_focus_background`, remove the blanket `Modifier::UNDERLINED` while
  keeping a visible focus cue (e.g. the theme's `label_style` patch only,
  or a non-underline modifier), preserve the `ChooseOne` widget's active-option
  styling, and do not strip underlining from caller-supplied themes or links.
  Add buffer-style tests for criterion 5: default focused choice cell has no
  blanket underline on blank cells; moving focus and rendering again into the
  *same* buffer clears the old styling; focus remains visible; and one other
  editable cell type (e.g. `TextInput`) is covered. If the spike did not
  support the hypothesis, skip removal and record it as unresolved.
  *Depends on:* Phase 1 spike.
- [x] **Task 2.A5: Consumer expectations.** Run `just test` in `biscuit-tui` and
  update only snapshots/expectations explained by content widths, narrow
  layouts, or the focus correction. Check the `question` CLI unit tests and
  `biscuit-tui/lib/tests/public_api_names.rs` (no new public width config
  should appear).
  *Depends on:* 2.A1-2.A4.

### Wave 1b: claudine-cli row mapping (Half B), parallel with Wave 1

- [x] **Task 2.B1: Baseline targets.** In
  `claudine/cli/src/commands/wrap/sequence/mod.rs`, extract the inline
  draft-to-target closure in the non-review branch into a named function that
  builds a `ResolvedExecutionTarget` from a draft plus
  `explicit_provider`/`list_one` (same provider and model reasons as today).
  Use it for both the bypass branch and the baseline vector. Keep one entry per
  original step in original order.
- [x] **Task 2.B2: Eligibility.** Add a small pure function classifying a
  `SequenceStep` (or its `ExecutableField`) as review-eligible per the spec
  table; `None`, `prompt`, `task`, `group` are eligible; `shell`,
  `side_effect` are not. Unit-test all six kinds. It reads only the normalized
  step, never composes files or runs commands.
- [x] **Task 2.B3: Filter and merge.** Add pure functions: (a) select eligible
  drafts preserving `step_index`; (b) merge submitted targets into the baseline
  vector by `step_index`, returning an error when
  `submitted.len() != eligible.len()` before touching the baseline. Never map
  by name or row number. Tests cover: interleaved prompt/shell/side-effect/task/
  group/body steps; repeated names; hidden first step; hidden last step;
  all hidden; distinct submitted provider/model choices; hidden targets and
  reasons unchanged; full vector length.
  *Depends on:* 2.B1, 2.B2.
- [x] **Task 2.B4: Wire into review.** In the `needs_review` branch, filter
  drafts, and if none remain return the baseline without calling the review
  function; otherwise call it with the eligible drafts and merge. Inject the
  review function (a closure or a small trait-free parameter) so unit tests can
  exercise the path without the standalone event loop. Preserve: the existing
  agent-message print, the `130` return on a cancelled error, and the
  no-TTY/explicit-provider/dry-run behavior unchanged.
  *Depends on:* 2.B3.
- [x] **Task 2.B5: Row labels.** In `selection_ui.rs::build_initial_rows`,
  keep labels from the original one-based `step_index + 1` and name. Confirm
  the `Step` schema text no longer constrains labels (now handled by Wave 1).
  Update the docs on `review_sequence` and the draft type so they say only
  eligible drafts are displayed and that returned targets are in row order of
  the drafts supplied (not necessarily one per step).
  Also fix the `if drafts.is_empty()` doc/behavior consistency.
  *Depends on:* 2.B4.
- [x] **Task 2.B6: Behavior tests.** Criterion 4 tests: empty table never opens
  (fake review callback asserts not called); explicit-provider and no-TTY
  gates unchanged; dry-run never reaches the callback; cancellation returns
  `130` and no step starts. Do not invoke the real `run_standalone`, real
  providers, shell actions, or lifecycle audio (use the fixtures from the
  `claudine` skill's L1 spawn contract: `CliProcessFixture`, private
  `PLAYA_SPOOL_DIR`, `PLAYA_DRY_RUN=1`).

- **Validation checkpoint 2:** `just test` and `just lint` pass in
  `biscuit-tui` and `claudine`; Phase 1 tests now pass; no `cargo fmt` was run.

## Phase 3: Input robustness

The matrix in this plan's template applies to code that *reads a file format
or configuration*. This fix adds no parser, manifest, or config reader, so the
matrix is **not applicable**. The one new input surface is the submitted-row
count and the width arithmetic, both covered by R6 and Task 2.A1/2.A2 tests
(overflow saturation and count mismatch). Nothing to schedule beyond
verifying those tests exist.

- [x] Confirm a test with an extremely long label (near `u16::MAX` width) does
  not overflow or panic in width calculation.
- [x] Confirm the count-mismatch test asserts on the public result (an error),
  not on an internal helper's return shape.

## Phase 4: Documentation and comments

All independent; run as one parallel wave.

- [ ] **Doc 4.1:** `biscuit-tui/docs/components/input_table.md`: sizing rules
  with a compact example per rule, display-only clipping, and a Mermaid diagram
  of the allocation tiers. Audience: a developer with no repo experience.
  Update the biscuit-tui README and `docs/dependencies.md` (R1).
- [ ] **Doc 4.2:** `claudine/docs/cli/sequence.md` and
  `claudine/docs/topics/execution-flow.md`: row eligibility table, original
  numbering, empty-table exception, unchanged gates. Describe behavior
  directly; do not name the fix or the dated spec directory.
- [ ] **Doc 4.3:** Correct touched symbol docs that imply every draft is
  displayed or that static text cannot vary by row (`InputTableColumn::StaticText`,
  `compute_column_widths`, `review_sequence`, draft type). Apply the Code
  Comment Quality rules: fix drift, delete tautologies.
- [ ] **Doc 4.4:** `.claude/skills/claudine/` and `.claude/skills/biscuit-tui/`:
  update guidance only where it describes the changed workflow.
- [ ] **Doc 4.5:** Write the implementation log (in the spec directory):
  rulings taken, spike evidence, any departure from the spec, and the
  focus-rule outcome (fixed with evidence, or **unresolved**).

## Phase 5: Final validation

- [ ] `just test` and `just lint` in `biscuit-tui`.
- [ ] `just test` and `just lint` in `claudine` (includes `question` unit tests).
- [ ] `just test-l2` only if real-terminal coverage was added or changed; keep
  windows out of focus.
- [ ] Portability sweep: no `#[cfg(unix)]`-only assumptions in new code;
  load the `os` skill before claiming any OS untestable. Macos runs locally;
  Linux and Windows are covered by CI (`ci:all-os` label if the author wants
  Windows before merge).
- [ ] Grep for stale statements: `rg "four cells|StaticText.*schema" biscuit-tui claudine/docs`.
- [ ] Manual read-through against spec criteria 1-7; tick each in the log.
- [ ] Stop at "implementation complete, ready for review". Do not commit unless
  told to, do not move the spec, do not run `just complete`.
