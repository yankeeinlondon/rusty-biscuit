---
spec: /Volumes/coding/wt/rusty-biscuit/fix-review-screen/claudine/fixes/2026-09-27-sequence-review-screen/spec.md
plan: claudine/fixes/2026-09-27-sequence-review-screen/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
    - biscuit-tui
    - claudine-cli
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
---

# Implementation Log for 2026-09-27-sequence-review-screen (5 phases)

## Phase 1

Skills loaded: `claudine`, `biscuit-tui`, `rust-testing`, `ratatui`.

### Rulings (R1–R11)

- **R1: grapheme segmentation.** Adopted. `unicode-segmentation = "1"` is
  now a direct dependency of `biscuit-tui` (`lib/Cargo.toml`, with a comment
  saying why). It resolves to the 1.13.2 already in `Cargo.lock`, so no new
  crate enters the build. The root `docs/dependencies.md` gained an entry.
  `biscuit-tui` has no per-area `docs/dependencies.md`. Its README and skill
  dependency lists are Phase 4 work (Doc 4.1 and 4.4).
- **R2: display width of a grapheme.** Adopted. A cluster's width is
  `UnicodeWidthStr::width(cluster)`, so a ZWJ emoji counts as whatever the
  library reports for the whole cluster. A cluster wider than the remaining
  cells is dropped whole and `…` takes its place.
- **R3: wide char at a one-cell remainder.** Adopted. The cluster is omitted
  and `…` follows the last drawn cluster immediately. Any cell left after the
  `…` is blank. Example: `12 日本語テキスト` in 7 cells becomes `12 日… `.
  At width 1 the cell shows only `…`, and at width 0 it shows nothing.
- **R4: where clipping lives.** Adopted. Clipping happens in `draw_cell`
  only. `CellState::StaticText` and the returned `Row` values are never
  changed. The test
  `clipping_is_display_only_and_submission_returns_full_text` already passes
  and guards this.
- **R5: eligibility.** Adopted. Eligibility is a function of
  `SequenceStep::executable.as_ref().map(|e| e.field)`. `None`, `Prompt`,
  `Task`, and `Group` are eligible; `Shell` and `SideEffect` are not. The
  signature is `is_review_eligible(&SequenceStep) -> bool`.
- **R6: row-count check.** Adopted. `merge_reviewed_targets` returns a
  typed `ReviewRowCountMismatch { expected, found }` before it touches the
  baseline. `review_targets` turns that into an `io::Error` of kind
  `InvalidData`, a kind that matches neither `CANCELLED_KIND`
  (`Interrupted`) nor `ABORTED_KIND` (`ConnectionAborted`). The message must
  name both counts.
- **R7: empty-eligible path and the gate.** Confirmed from the code. The
  explicit-provider shortcut, the no-TTY `AgentResolutionFailed` abort, and
  `needs_review` all run before any draft exists
  (`sequence/mod.rs`, around the "No-TTY prompting-state gate"). Filtering
  therefore belongs inside the `needs_review` branch only. A shell-only
  sequence without a resolvable provider still aborts headless. `--dry-run`
  returns earlier still, before any draft is built.
- **R8: `Ctrl+S` with hidden steps.** Adopted. Hidden entries keep their
  baseline target. The inline bypass closure becomes
  `baseline_targets(&[SequenceStepDraft], ProviderResolutionReason)`, with
  the reason computed once from `explicit_provider` / `list_one`. Both
  branches use it.
- **R9: draft location and label.** Confirmed. `SequenceStepDraft`
  (`claudine/lib/src/composition/types.rs`) keeps `step_index`, and
  `build_initial_rows` builds the label from `step_index + 1` and
  `step_name`. The new test `filtered_drafts_keep_original_step_positions`
  (passing) guards this for a filtered subset.
- **R10: cross-OS focus check.** Not requested by the author. No Windows
  terminal spike was added. The headless buffer evidence below is
  OS-independent.
- **R11 (added; the spec is ambiguous).** "Sharing reductions evenly and
  assigning remainders left to right" in the static-shrink tier means each
  static column is reduced by an equal share. When the reduction does not
  divide evenly, the **leftmost** columns keep the spare cells (they are
  reduced one less). This matches how leftover cells go to the leftmost
  focusable columns. A column that reaches the three-cell floor stops
  shrinking, and its unused share of the reduction is split among the
  others. Examples, both asserted by tests: static preferred widths 10 and 10
  in 13 cells become 7 and 6; 10 and 11 in 12 cells become 6 and 6.

### Spike: focus styling (headless, macOS, one run)

A scratch test (removed afterwards) rendered the review-screen shape:
`Step` static text, a `ChooseOne` provider column (3 options), and a
`ChooseOne` model column (2 options), with 3 rows. It rendered into an
80×16 `Buffer` with the default `ComponentTheme`, then printed each cell's
modifiers. `U` = UNDERLINED, `B` = BOLD.

```text
 0 |Step▶  Claude                            ▶  (default)                         |
   |....UUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUBBBBBBBBBBBBBB........................|
 1 |       Codex                                opus                              |
   |....UUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUU......................................|
 2 |       Gemini                                                                  |
   |....UUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUU......................................|
 3 |Step▶  Claude                            ▶  (default)                         |
   |....BBBBBBBBBBB...........................BBBBBBBBBBBBBB........................|
```

Findings:

1. **UNDERLINED lands on blank cells.** Every cell in the focused provider
   rectangle is underlined, including the padding after `Claude`, `Codex`,
   and `Gemini`. On a terminal that draws each row's underline across the
   full column width, this looks like horizontal rules. The hypothesis is
   **confirmed**: `paint_focus_background` is the cause.
2. **Only the first row differs** because initial focus is `(0, 1)`: the
   first row's provider cell. Other rows show only the widget's own BOLD
   active option. This explains why the report saw the rules on the first
   row only.
3. **Old styling persists when rendering into the same buffer.** After
   `Alt+Down` moved focus to `(1, 1)`, the same buffer showed the underline
   on both row 0 and row 1. Nothing resets the style of cells a widget does
   not write to. A live `ratatui::Terminal` hides this because it gives each
   frame a fresh buffer, but criterion 5 requires cleanup within one buffer.
   Task 2.A4 therefore needs two changes: remove the blanket underline, and
   reset each cell rectangle's style before drawing (for example
   `buf.set_style(area, Style::reset())` or a blank fill), whether or not the
   cell has focus.

No real-terminal check is needed; the headless evidence answers the
question.

### Other findings for later phases

- **The cancellation-to-`130` branch in `execute_sequence` is dead today.**
  It matches `ErrorKind::Other` whose message contains `"cancelled"`, but
  `run_standalone` returns `ABORTED_KIND` (`ConnectionAborted`, message
  `"cancelled"`) on `Esc` and `CANCELLED_KIND` (`Interrupted`, message
  `"interrupted"`) on `Ctrl+C`. Both currently propagate as errors: no step
  starts and the exit is non-zero, but the code is not `130`. The spec asks
  to "retain existing cancellation behavior", while the plan (Task 2.B4) says
  to "preserve the `130` return". Phase 2 must pick one reading and record
  it. The recommendation is to match on `CANCELLED_KIND` and `ABORTED_KIND`
  (as `steer/interact.rs` does), so the evident intent works. That is a
  small behavior change, and it should get a test through the injected
  review callback.
- **The shipped `prompts/review-loop.md` has 22 steps, not 21.** The
  report counted an earlier revision. Five `stage-N` shell steps sit at
  indices 3, 7, 11, 15, and 19. The shipped-artifact test asserts 22 steps,
  17 rows, and 22 targets.
- Existing `biscuit-tui` tests that call `compute_column_widths(&columns, w)`
  directly (`static_text_columns_stay_at_natural_width_with_leftover`,
  `static_text_does_not_shrink_below_natural_in_overflow`,
  `all_static_text_columns_use_preferred_widths`,
  `render_static_text_stays_tight`) must follow the new signature in Task
  2.A1. `static_text_does_not_shrink_below_natural_in_overflow` asserts the
  old tier-free emergency result (width 5 at 10 columns). Under the new
  rules that width still falls in the emergency tier (3 + 20 > 10), so the
  expectation stays 5.

### Scaffolded tests

Tests that fail until Phase 2 carry `#[ignore = "phase-2 scaffold: …"]`, so
`just test` stays green. **Phase 2 must remove every such `#[ignore]`.** Find
them with `rg 'phase-2 scaffold' biscuit-tui claudine`. Each one was run with
`--run-ignored only` and confirmed to fail for the expected reason only.

`biscuit-tui/lib/src/components/input_table/table/tests.rs` (criteria 1–2,
all rendered through the public `InputTable` widget, so they survive the
width-helper signature change):

| Test | Criterion | Fails today because |
| ---- | --------- | ------------------- |
| `static_column_shows_full_label_at_80_columns` | 1 | `"1 imX…"`: the original report reproduced exactly |
| `static_column_width_includes_off_screen_rows_and_survives_scrolling` | 1 | text column at 4, not 19 |
| `resizing_down_clips_with_ellipsis_and_resizing_up_restores` | 1 | label clipped at 80 columns |
| `multiple_static_columns_share_the_reduction_left_to_right` | 2 | `"abcklmX"` |
| `emergency_allocation_divides_width_evenly_and_caps_static_columns` | 2 | `"12 rX"` |
| `one_cell_static_column_shows_only_the_ellipsis` | 2 | `"1"`, not `…` |
| `wide_characters_are_omitted_whole_and_never_cross_the_column` | 2 | label clipped at 80 columns |
| `grapheme_clusters_are_never_split_by_clipping` (combining `é` and a ZWJ family emoji, widths 23–28) | 2 | static width fixed at 4 |
| `all_static_table_sizes_to_content_and_shrinks_left_to_right` | 2 | `"alpbet"` |
| `zero_cell_static_column_draws_nothing` (not ignored, passes) | 2 | — |
| `clipping_is_display_only_and_submission_returns_full_text` (not ignored, passes) | 2 | — |

`claudine/cli/src/commands/wrap/sequence/review/tests.rs` (criterion 3 plus
the empty-table and cancellation parts of criterion 4). All of them panic
with `not implemented: phase 2: …` in the stubs in
`sequence/review.rs`:

- `eligibility_follows_the_outer_executable`: all six executable kinds
- `shipped_review_loop_hides_exactly_its_stage_steps`: the real
  `prompts/review-loop.md` via `include_str!`. It already normalizes through
  `resolve_sequence_plan` and fails only at the stub.
- `baseline_targets_match_the_review_bypassed_conversion`: provider and
  model reasons, including the `Claude` fallback
- `interleaved_steps_review_only_eligible_rows_and_merge_by_step_index`:
  prompt, shell, side_effect, task, group, and body steps, with distinct
  submitted choices and hidden targets unchanged
- `repeated_names_map_by_position_not_name`
- `hidden_first_and_last_steps_keep_their_baseline`
- `all_hidden_steps_never_open_the_review`: the callback is never called
- `unexpected_row_count_is_rejected_before_merging`: short and long returns
  give `InvalidData`, not a cancel kind
- `merge_rejects_a_count_mismatch_without_touching_the_baseline`
- `review_cancellation_propagates_its_kind`

Plus `selection_ui::tests::filtered_drafts_keep_original_step_positions`
(passes; guards R9).

`sequence/review.rs` holds the stubs and a module-level
`cfg_attr(not(test), allow(dead_code, reason = …))`. **Phase 2 removes
that attribute** once `execute_sequence` calls the functions.

Not scaffolded here, as planned for Phase 2: the criterion-5 focus tests
(Task 2.A4), the width-sum property test (Task 2.A2), and the gate tests
(explicit provider, no TTY, dry run) that need the `execute_sequence` wiring
(Task 2.B6).

### Gates

- `biscuit-tui`: `just test` gives 994 passed, 16 skipped (the 9 scaffolds
  plus pre-existing ignores). `just lint` exits 0.
- `claudine`: `just lint` exits 0 after two clippy fixes in the new test file
  (`is_multiple_of`, and a `ShownRows` type alias). `just test` gives 8033
  passed, 19 skipped.
    - The first full run failed `dispatch_inventory_matches_committed_file`,
      the site guard over `cli/src`, which counts `Provider::` references
      in test modules too. The new test sites drifted it (1703 → 1710 sites).
      Regenerated with
      `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`. The
      diff holds only the new test-file sites and a one-line shift from
      `mod review;`. **Phase 2 must regenerate again**, because the inventory
      records line numbers.
    - Two later full runs each failed one untouched shell-spawning test in
      the `claudine` lib: `an_early_wait_error_still_reaps_the_whole_tree`
      (LEAK) and `direct_composition_runs_shell_in_configured_working_directory`
      (TIMEOUT, 30 s). The host load average was 48 on 16 cores from other
      sessions. Both pass alone in under 0.2 s, and the full suite passed
      with `NEXTEST_TEST_THREADS=6`. These are pre-existing contention
      sensitivities, not caused by this phase.
- Portability: the new code is pure (no `cfg`, paths, or processes). The
  `include_str!` path to `prompts/review-loop.md` is resolved at compile
  time relative to the source file, which is portable and visible to CI's
  test-input index. No cross-host run was needed for this phase.
