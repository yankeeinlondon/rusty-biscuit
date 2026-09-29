---
spec: /Volumes/coding/wt/rusty-biscuit/fix-review-screen/claudine/fixes/2026-09-27-sequence-review-screen/spec.md
plan: claudine/fixes/2026-09-27-sequence-review-screen/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
    - biscuit-tui
    - claudine-cli
    - claudine
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

## Phase 2

Half A (`biscuit-tui`, tasks 2.A1–2.A5) and Half B (`claudine-cli`, tasks
2.B1–2.B6) touch different packages and ran in parallel.

### Half B: claudine-cli row mapping

- **2.B1–2.B3** (`sequence/review.rs`): the stubs are implemented.
  `baseline_targets` is the former inline bypass closure (Claude fallback
  when a draft has no provider, shared provider reason, draft model and model
  reason). `merge_reviewed_targets` checks the count before it writes.
  `review_targets` filters by `is_review_eligible(&steps[draft.step_index])`,
  returns the baseline without calling `review` when nothing is eligible, and
  wraps a `ReviewRowCountMismatch` (now a `thiserror` type, message
  `sequence review returned {found} targets for {expected} rows`) as the
  source of an `InvalidData` `io::Error`, which keeps the typed cause the
  `error_guards` transport scan requires. The
  module-level `allow(dead_code)` is gone.
- **2.B4: wiring.** Two further pure functions in `review.rs` carry the whole
  Phase 1b decision so it can be tested with an injected callback:
    - `needs_review(is_tty, shared_state)`: the unchanged gate (a prompting
      state on a terminal; `None` means an explicit provider flag).
    - `live_targets(needs_review, steps, drafts, provider_reason, review)`
      returns `Ok(None)` when the review ends with `CANCELLED_KIND`
      (`Ctrl+C`) or `ABORTED_KIND` (`Esc`), and `execute_sequence` turns that
      into `SEQUENCE_INTERRUPT_EXIT_CODE` (`130`) before any step starts.
  `execute_sequence` computes the provider reason once (explicit flag, then
  `ListOneInstalled`, else `FrontmatterSingle`, as the bypass did) and calls
  `live_targets` with a closure that prints the state's pre-prompt message and
  opens `review_sequence` on the eligible drafts only.
- **Ruling on cancellation (Phase 1 finding).** Adopted the recommendation:
  `Esc` and `Ctrl+C` on the review screen now exit `130` with no step started.
  Before, the `ErrorKind::Other` + `"cancelled"` match never fired, so both
  keys surfaced as a non-zero error exit (also with no step started). This
  matches the evident intent of the old branch and `steer/interact.rs`.
- **Departure (small): the pre-prompt message.** The `Invalid Agent:` /
  zero-installed-list message is now printed inside the review callback, so
  it appears only when the table actually opens. A sequence with only shell
  and side-effect steps opens no table and prints no prompt message.
- **2.B5.** `build_initial_rows` already built labels from `step_index + 1`
  (R9). The `review_sequence` doc now says each supplied draft is a row, that
  the caller passes only reviewable steps, that targets come back one per
  draft in draft order, and that empty drafts return an empty vector without
  opening the table (the doc and the early return now agree).
  `SequenceStepDraft` (in the `claudine` lib) had drifted docs: it now says
  every step has a draft but only reviewable steps are shown, that
  `step_index` is the whole-sequence index used for mapping, and
  `resolved_provider` is the provisional provider (the old doc claimed it was
  `None` unless locked, which the code never did).
- **Gate order observed.** Shell pre-flight approval runs before the agent
  gate, so a headless shell-only sequence with an unapproved command fails at
  approval before it reaches the agent gate. The new CLI test uses `--yolo`
  to get past approval and proves the agent gate still aborts.
- Dispatch inventory regenerated
  (`CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`).

#### Tests (Half B)

| Requirement | Test |
| ----------- | ---- |
| Eligibility for all six executable kinds (crit. 3) | `review::tests::eligibility_follows_the_outer_executable` |
| Shipped `prompts/review-loop.md`: 22 steps, 17 rows, 22 targets | `review::tests::shipped_review_loop_hides_exactly_its_stage_steps` |
| Baseline matches the bypass conversion, reasons included | `review::tests::baseline_targets_match_the_review_bypassed_conversion` |
| Interleaved kinds, distinct choices, hidden targets unchanged, full length | `review::tests::interleaved_steps_review_only_eligible_rows_and_merge_by_step_index` |
| Repeated names | `review::tests::repeated_names_map_by_position_not_name` |
| Hidden first and last | `review::tests::hidden_first_and_last_steps_keep_their_baseline` |
| All hidden: table never opens (crit. 4) | `review::tests::all_hidden_steps_never_open_the_review`, `a_shell_only_sequence_on_a_terminal_never_opens_the_review` |
| Unexpected row count rejected before merging | `unexpected_row_count_is_rejected_before_merging`, `merge_rejects_a_count_mismatch_without_touching_the_baseline`, `a_failed_review_is_an_error_not_a_cancellation` |
| Gate unchanged: explicit provider, no TTY, auto-selected states never open the review | `review_opens_only_for_a_prompting_state_on_a_terminal`, `a_bypassed_review_never_calls_the_callback_and_keeps_the_baseline` |
| Cancellation (`Esc`, `Ctrl+C`) yields no targets, so no step starts | `leaving_the_review_with_esc_or_ctrl_c_yields_no_targets`, `review_cancellation_propagates_its_kind` |
| Submitted review returns one target per step | `a_submitted_review_returns_one_target_per_step` |
| Headless shell-only sequence without an agent still aborts; its shell never runs | `l1 wrap_sequence_composition::sequence_live_shell_only_no_agent_still_aborts_headless` (new, `cfg(unix)` like its neighbors) |
| Dry-run never prompts | existing `wrap_sequence_composition::sequence_dry_run_*` tests (the dry-run arm returns before any draft exists; unchanged) |

All unit tests are L1 in the `claudine-cli` bin's test module; the CLI test
is in the declared `tests/l1/` binary. None use a tier marker. No test opens
`run_standalone`, launches a provider, or plays audio.

### Half A: biscuit-tui layout and focus

Implemented by a sub-agent in parallel. Changes are in
`biscuit-tui/lib/src/components/input_table/{table.rs, table/tests.rs,
column.rs, cell.rs}`.

- **2.A1: content widths.** `compute_column_widths(columns, rows,
  total_width)` now receives the rows and runs every render. A new
  `preferred_column_widths` gives each static column the widest of its schema
  text and every row's value, off-screen rows included, with a floor of 3,
  converted with `u16::try_from(..).unwrap_or(u16::MAX)`. Focusable budgets
  are unchanged (switch 8, text and choice 20, text area its configured width).
- **2.A2: allocation tiers.** Arithmetic is `u64`. (1) Everything fits: static
  columns get their preferred width; focusable columns share the leftover,
  remainder left to right. (2) Budgets plus 3 per static fit: focusable
  columns get their budgets and static columns give up equal shares, the
  rightmost giving up the extra cells (R11); a column at the floor stops and
  the others absorb the rest in later passes. (3) Otherwise: even split,
  remainder left to right, static capped at preferred.
- **Deviation noted (tier 3):** cells that a capped static column cannot use
  are left unused, not redistributed. This follows the spec's wording and
  keeps the old `width 5 in 10 cells` expectation.
- **2.A3: clipping.** `clip_with_ellipsis` in `draw_cell` works on grapheme
  clusters (`unicode-segmentation`), drops a straddling cluster whole, puts
  `…` after the last drawn cluster, shows only `…` at width 1 and nothing at
  width 0, and writes with `set_stringn` bounded by the cell width. It never
  changes `CellState` or the returned rows.
- **2.A4: focus painting.** The spike's hypothesis held, so the fix was
  applied. `paint_focus_background` is gone. Each cell's rectangle is cleared
  (`ratatui::widgets::Clear`) before it is drawn, which removes styling left in
  the same buffer by an earlier render. The focused cell gets the theme's
  `label_style` (bold by default) with no added underline. The choice
  widget's active-option styling still draws on top, and underlines coming
  from a caller's theme are not removed. Side effect: anything painted under
  the table inside a cell rectangle is overwritten.
- **Evidence the focus tests are load-bearing:** with the underline and the
  missing reset temporarily restored, the three underline and cleanup tests
  failed; the "still distinguishable" test passes either way, as intended.
- **Stray-rule requirement: resolved** with buffer evidence (criterion 5).
- **2.A5.** All nine `phase-2 scaffold` ignores are removed and pass without
  changing their expectations. The only edits to existing tests are the
  signature change in the four tests that call `compute_column_widths`
  directly (three pass an empty row slice, `render_static_text_stays_tight`
  passes `state.rows()`); no assertion values changed. No `question` CLI
  expectation needed updating; `public_api_names` passes; no public width
  option was added. `InputTableColumn::StaticText` and `CellState::StaticText`
  docs were corrected (they claimed identical text in every row).

#### Tests added (Half A, L1, `table/tests.rs`)

| Test | Covers |
| ---- | ------ |
| `column_widths_never_exceed_the_available_width_and_follow_the_tiers` | Property test: widths 0–200 across five column mixes (incl. all-static and a wide-character label); sum ≤ width, static ≤ preferred, per-tier rules |
| `a_static_column_at_the_floor_passes_its_share_to_the_others` | R11 floor case: preferred 4, 20, 20 in 34 cells → 3, 11, 20 |
| `very_long_static_values_saturate_instead_of_wrapping` | Saturating width arithmetic |
| `focused_choice_cell_draws_no_underline_on_blank_cells` | Criterion 5 |
| `focused_choice_cell_stays_distinguishable_and_keeps_the_active_option_style` | Criterion 5: focus visible, active option still bold |
| `moving_focus_clears_the_old_focus_styling_in_the_same_buffer` | Criterion 5: after `Alt+Down`, the reused buffer equals a fresh render |
| `focused_text_input_cell_has_no_underline_and_clears_when_focus_moves` | Criterion 5 for a second editable cell type |

Plus the nine Phase 1 scaffolds for criteria 1 and 2, now active.

### Gates (Phase 2)

- `biscuit-tui`: `just test` gives 1010 passed, 7 skipped (recipe filters,
  no `#[ignore]` left). `just lint` exits 0.
- `claudine`: `just lint` exits 0. The first run failed
  `error_guards::production_sources_pass_every_scan_backed_guard` because
  the row-count error was flattened with `format!`; fixed by making
  `ReviewRowCountMismatch` the `io::Error`'s typed source.
- `claudine`: `just test` (with `NEXTEST_TEST_THREADS=6`) gives 8048 passed,
  9 skipped, 2 timed out. The two timeouts were untouched `claudine-gen`
  tests (`generate_ux::roster_alias_rename_drifts_the_darkmatter_name_table_until_regenerated`,
  `generate_ux::steering_activation_policy_gates_check_and_generate`) at a
  host load average of 109; both pass alone in about 5 s
  (`just test-gen generate_ux::`, 12 passed). Load contention, not this change.
- `rg 'phase-2 scaffold' biscuit-tui claudine` finds nothing outside the
  Markdown log and plan.
- No `cargo fmt` was run.
- Portability: all new code is pure (no `cfg`, paths, or processes) except
  the new CLI test, which is `#[cfg(unix)]` like every neighboring
  `sequence_live_*` test because its provider stub is a shell script. No
  cross-host run was needed.
- Skills: nothing in `.claude/skills/claudine/` or `.claude/skills/biscuit-tui/`
  describes the review screen, column sizing, or focus underline, so no skill
  edits were needed this phase (Doc 4.4 revisits this).

## Phase 3

Input robustness. The plan rules the Input Robustness Matrix not applicable
(no parser, manifest, or config reader is added), so this phase audits the
two input surfaces the fix does add: width arithmetic on arbitrarily long
static labels, and the submitted-row count.

### Audit of existing coverage

- **Width.** `very_long_static_values_saturate_instead_of_wrapping` (Phase 2)
  asserts the private helpers only (`preferred_column_widths`,
  `compute_column_widths`) at one width (80) with one saturated column. It
  does not render, does not probe the `u16::MAX` boundary itself, never puts
  two saturated static columns side by side (their preferred sum exceeds
  `u16`, which is what the `u64` sums in `compute_column_widths` guard), and
  never allocates across a `u16::MAX`-wide area.
- **Count mismatch.** `unexpected_row_count_is_rejected_before_merging`
  asserts through `review_targets`, but production (`sequence/mod.rs`) calls
  `live_targets`, which maps the cancel/abort kinds to `Ok(None)` (exit 130).
  `a_failed_review_is_an_error_not_a_cancellation` reaches `live_targets` but
  covers only a short answer, no hidden steps, and checks only the kind.

### Tests added

Test files only; no production code changed in this phase.

| Test | Asserts |
| ---- | ------- |
| `a_label_at_the_u16_boundary_saturates_its_preferred_width` (biscuit-tui `input_table/table/tests.rs`) | label widths 65 534, 65 535, 65 536 give preferred widths 65 534, `u16::MAX`, `u16::MAX` |
| `saturated_static_columns_render_clipped_at_every_width_without_overflow` (same file) | two static columns both saturated at `u16::MAX` (ASCII 65 534–70 000 and 32 768 two-cell `界` clusters) plus a text column: at widths 0, 1, 2, 3, 26, 80, 200, `u16::MAX - 1`, `u16::MAX` the widths never sum past the area and rendering does not panic; at 80 the layout is `[30, 30, 20]` with `…` clipping and no split wide cluster; at `u16::MAX` the sum is exact and the text column keeps 20; at 1 the row is `…`; `Ctrl+S` returns every full label |
| `a_wrong_row_count_with_hidden_steps_fails_the_run_instead_of_cancelling_or_truncating` (claudine-cli `sequence/review/tests.rs`) | through `live_targets`, the seam `sequence/mod.rs` calls: with shell and side-effect steps hidden (two rows shown), answers of 0, 1, 3, 4 (the full step count) and 5 rows are each an `Err` of kind `InvalidData`, never `Ok(None)` (exit 130) and never a merged vector, and the error's typed source is `ReviewRowCountMismatch { expected: 2, found }` |

### Mutation checks

Each new test was checked against a deliberate regression, then the source was
restored (`git diff` on the production file empty afterward):

- `preferred_column_widths` using `widest as u16` instead of saturating: both
  biscuit-tui tests fail.
- `merge_reviewed_targets` using `<` instead of `!=`: the claudine-cli test
  fails with the truncated four-entry merge as its `expect_err` payload.

### Input Robustness Matrix

Not applicable, as the plan ruled: the fix reads no file format or
configuration. The two inputs it does take, label text and the submitted row
count, have no absent/null/wrong-type shapes. Their boundary and mismatch
cases are the tests above.

### Gates (Phase 3)

- `biscuit-tui`: `just test` gives 1012 passed, 7 skipped (includes the
  `question` CLI unit tests). `just lint` exits 0.
- `claudine`: `just test` gives 8051 passed, 9 skipped, no failures or
  timeouts. `just lint` exits 0.
- No `cargo fmt` was run. No pre-existing failures were seen.
- Portability: both new tests are pure, with no `cfg`, paths, processes, or
  terminals, so no cross-host run was needed.
- Skills: nothing changed that `.claude/skills/claudine/` or
  `.claude/skills/biscuit-tui/` describes.
