---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-27-union-partial-file-completion/spec.md
plan: claudine/fixes/2026-09-27-union-partial-file-completion/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/schema_interactive/tests.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/tests/common/pty.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/level1_provider_picker_pty.rs
    - biscuit-terminal/lib/src/errors/source_context.rs
    - biscuit-terminal/lib/src/errors/mod.rs
    - darkmatter/lib/src/markdown/dsl/mod.rs
    - darkmatter/lib/src/markdown/code_block.rs
    - darkmatter/lib/src/markdown/output/code_block.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
docs_updated_during_phase_2:
    - biscuit-terminal/README.md
    - darkmatter/docs/topics/code-blocks.md
    - darkmatter/docs/topics/yamlblock-migration.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/rendering.md
source_files_during_phase_3:
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/cli/src/completion/autocomplete_ui.rs
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/composition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/frontmatter_excerpt/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - biscuit-terminal/lib/src/errors/source_context.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/composition.md
    - biscuit-terminal/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/timeline.md
source_code:
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/frontmatter_excerpt/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/schema_interactive/tests.rs
    - claudine/cli/src/commands/wrap/selection_ui.rs
    - claudine/cli/src/completion/autocomplete_ui.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/common/pty.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
    - claudine/cli/tests/l1/level1_provider_picker_pty.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - biscuit-terminal/lib/src/errors/source_context.rs
    - biscuit-terminal/lib/src/errors/mod.rs
    - darkmatter/lib/src/markdown/dsl/mod.rs
    - darkmatter/lib/src/markdown/code_block.rs
    - darkmatter/lib/src/markdown/output/code_block.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
documentation:
    - claudine/docs/topics/composition.md
    - biscuit-terminal/README.md
    - darkmatter/docs/topics/code-blocks.md
    - darkmatter/docs/topics/yamlblock-migration.md
    - .claude/skills/darkmatter/rendering.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
completed_phase: 5
implemented: true
packages:
    - claudine-cli
    - claudine
    - biscuit-terminal
    - darkmatter
---

# Implementation Log for 2026-09-27-union-partial-file-completion (5 phases)

## Phase 1

Phase 1 covered the rulings, the R7.1 reproduction, and spikes S1–S4. The
spikes ran as four parallel read-only agents. Their findings are in the plan
under "Spike findings", and each claim the rulings depend on was checked
against the code.

### Reproduction (R7.1)

- Added `union_partial_with_templated_file_sibling_reaches_chooser` to
  `claudine/cli/tests/l1/level1_provided_partial_file_pty.rs`, plus two local
  helpers: `plan_with_union_templated_sibling` and
  `stage_recording_goose_stub`. The file is already declared in the `l1`
  binary. The test name carries no tier marker, so it runs in L1.
- It is **red on purpose**, as the plan directs. It fails with no chooser, the
  `no existing file matched reference` error, and the whole 11-line
  frontmatter block with no highlight. That shows C1 and C5 together. The full
  transcript is in the plan's "Reproduction record".
- The test uses `--goose` (the file's shared `compose_command`), not
  `--claude`. The stub records argv, and stdin when stdin is not a TTY. A
  manual run with the debug binary confirmed that the resolved spec path
  reaches the stub's argv (`run -t "<prompt>"`), so the final assertion checks
  something real.
- The body uses `{{spec}}`. `{{doc}}` in a body renders Darkmatter's reserved
  `doc` namespace (the whole document), not the frontmatter property called
  `doc`. That behavior is documented in Darkmatter (`docs/inline/interpolation.md`).

### Rulings (N1–N9)

All nine are confirmed under `yolo: true`. Amendments are recorded inline in
the plan:

- **N3:** the sequence-item indentation trap. `  - spec:` is at indent 2, but
  its sibling `doc:` is at indent 4.
- **N7:** confirmed by S2. The caller records are already reachable at
  `translate_schema_failure`.
- **N8:** amended. No sizing helper or cap exists today; every call site
  hard-codes its rows. `HeightSpec::Cells(n)` reserves `n` rows and is
  clamped only to the terminal height. I first wrote that it shrinks to fit
  its content, checked `HeightSpec::resolve`, and corrected the amendment. The
  new helper sizes by option count plus chrome, capped at the existing 8.
- **N9:** confirmed by S1.

### Key spike conclusions

- **S1:** every non-success outcome of the supplied-file pass already errors
  before provider selection. The C2 gap is caused only by C1: an empty
  pending list for a union with no selected arm.
- **Open item (S1):** `Ctrl-C` on the single-candidate confirm dialog is
  probably ignored (`read_confirm_key`). Phase 3 should check it.
- **S2:** the late error is built in `translate_schema_failure`
  (`schema/translate.rs:75`). `classify_unresolved_file_reference` is not on
  the late path.
- **S3:** the inline `run_standalone` never enters the alternate screen. The
  provider picker has **no title text**, so R7.3 must assert on provider
  option labels or on the raw-mode entry.
- **S4:** there are no insta snapshots. About 30 tests assert on excerpt
  content, and about 7 of them will change. The two Level 2 inline-mismatch
  tests use YAML anchors, which N3 treats as unsafe, so Phase 4 must decide
  how they behave.

### Gates

- `just lint` (`claudine/`): green.
- `just test --no-fail-fast` (`claudine/`): 7370 tests run, 7369 passed, 1
  failed (the intentional R7.1 red), 9 skipped.
- Plain `just test` stops at the first failure: 4866 of 7370 tests run, with
  the same single failure.
- No cross-OS runs. The only change is a `#![cfg(unix)]` PTY test in an
  existing file.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| R7.1 (reproduce C1) | `union_partial_with_templated_file_sibling_reaches_chooser`. Red now; it asserts the fixed behavior: the chooser, the stub launch, the path in the prompt, and no NoMatch error. |

## Phase 2

Phase 2 built the four foundation tracks. R1 was done in the main session;
the biscuit-terminal locator, the Darkmatter `start_line`, and the inline
picker ran as three parallel subagents on disjoint file sets. Each of their
diffs was reviewed afterward.

### Tolerant arm selection (R1, N5)

- Extracted `is_composition_independent(problem, instance)` in
  `claudine/lib/src/composition/schema/mod.rs`. The pre-validator now filters
  through it, with its `caller_resolved_eager_files` exclusion layered on top
  for non-`Missing` problems. Behavior is unchanged: all 92 existing
  `composition::schema::` tests pass.
- `supplied_file_shape` (`schema/supplied.rs`) now accepts an arm when no
  problem survives the predicate against the `candidate` instance, instead of
  requiring `validate(..).valid`. The eager relaxation is unchanged. Its doc
  now names composition tolerance as the second relaxation.
- The existing `supplied.rs` tests are untouched: the diff removes no test
  lines.
- New tests, in `supplied.rs`, all L1:
  - `templated_file_sibling_does_not_rule_out_the_applicable_union_arm`: the
    exact R7.1 fixture shape, with `doc` as `"{{spec || design}}"`,
    `'{{ spec }}'`, and `"$(echo spec.md)"`. Each gives one pending
    `UnresolvedFileReference` with `property`, `provided`, `patterns`
    (`**/*spec*.md`), `is_array`, and the slot checked.
  - `literal_invalid_sibling_still_rules_out_the_union_arm`: `count: number`
    with `count: 3` (control, selected) against `count: many` (ruled out).
  - `templated_sibling_does_not_select_conflicting_or_string_only_arms`.
  - `supplied_files_select_shipped_clarify_spec_arm`: the real shipped
    `prompts/clarify.md`, via `include_str!` so CI sees the input, with
    `spec=fix`.
- **Regression proof:** with the predicate swapped back to `.valid`, the
  templated, literal-invalid (control half), and shipped-clarify tests fail,
  and the other six pass. The file was restored afterward.
- **Gotcha found:** a plain `file` property with no `eager` accepts both
  missing paths and the number `42`. The first two drafts of the
  literal-invalid fixture therefore passed on both old and new code, and were
  replaced by the typed `count: number` sibling.
- The R7.1 PTY test `union_partial_with_templated_file_sibling_reaches_chooser`
  is now **green**. The other four tests in the file still pass.

### Focused-region locator (biscuit-terminal, N3)

- New public API in `biscuit_terminal::errors`:
  - `FocusedRegion { start_line, end_line, highlighted }`, with 1-based,
    absolute, inclusive line numbers.
  - `SourceContext::focused_yaml_regions(&[YamlKeyPath], context) ->
    Option<Vec<FocusedRegion>>`.
  - `YamlKeyPath::in_every_arm(parent, key)`.
- Regions are sorted and never touch. Ancestors always show, and a sequence
  item's `- …` line counts as an ancestor. It returns `None` for no match or
  for unsafe YAML (`&`, `*`, `<<`). The `- key:` trap is handled: the key sits
  at the marker column plus 2, and each item is its own scope.
- `focused_yaml_excerpt` is reimplemented on top of it. The old fallback
  stays, and the existing output is byte-identical.
- 10 new unit tests and one doctest. `biscuit-terminal/README.md` was
  updated. biscuit-terminal `just test`: 3325 passed, 55 skipped. `just lint`
  is green.

### CodeBlock start line (Darkmatter, N1)

- Added `CodeBlockMeta::start_line: Option<usize>`,
  `CodeBlockMeta::first_line()`, and `CodeBlock::with_start_line(n)`. Both the
  terminal and HTML renderers number from it and match highlights in absolute
  numbering. The terminal gutter width covers `first_line + len - 1`. The
  default output is byte-identical.
- There is no fence key (the planned default). Because `CodeBlock` renders
  through the render tree, which re-parses the info string, `start_line`
  travels as a darkmatter-private extension hint (`darkmatter.code` /
  `start_line`) on the node. It is looked up only when the node carries extra
  data, which keeps the `structural_gate` zero-lookup test green. A typed
  field on `renderable`'s `CodeRenderHints` would be the cleaner shape if a
  second consumer ever appears. The namespaced extension bag is the existing
  sanctioned mechanism, so it was kept.
- **Caveat for Phase 4:** under `ColorDepth::None` the terminal code hook
  falls back to a plain block with no gutter. This predates the change, and
  it means Claudine's appendix has no line numbers under `NO_COLOR` today.
- 9 new tests and one doctest. Docs updated: `code-blocks.md`,
  `yamlblock-migration.md`, and the darkmatter skill's `rendering.md`.
  Darkmatter `just test` was re-run after the biscuit-terminal change landed:
  8507 passed, 12 skipped. `just lint` is clean.

### Inline provider picker (R5, N8)

- `pub(crate) fn chooser_height(option_count)` is in
  `schema_interactive/mod.rs`. It returns `min(options + CHOOSER_CHROME_ROWS,
  CHOOSER_MAX_ROWS)`, with `CHOOSER_CHROME_ROWS = 2` and
  `CHOOSER_MAX_ROWS = 8`.
  - The value 2 was derived from `biscuit-tui`'s `ChooseOne`/`ChooseMany`:
    one label row plus the help-hint row that `run_standalone` overlays on
    the last row.
  - The filter line and validation-error line cannot appear for these
    callers.
  - The `ChooseOne` enum chooser, `ChooseMany`, and
    `prompt_one_shot_provider` all use it.
- `prompt_one_shot_provider` doc: it now says the picker is inline and
  bounded. Doc drift was corrected: `Esc` restores the initial selection and
  submits; only `Ctrl-C` errors (`CANCELLED_KIND`).
- `claudine/cli/tests/common/pty.rs`: the `ALT_SCREEN_ENTER` doc now lists
  the one-shot picker among the inline prompts. The fullscreen example there
  (the review picker) is still accurate.
- New tests:
  - `level1_provider_picker_pty::level1_pty_provider_picker_is_inline_and_launches_selection`
    is `#[cfg(unix)]` and declared in `tests/l1/main.rs`. It uses
    `command_std()` and `expect_level!(Level::L1, pty_available(), …)`. It
    stages `claude` and `goose` stubs, passes no provider flag, and presses
    `j` then Enter. It asserts that the capture has no `ALT_SCREEN_ENTER`,
    that goose launched, and that claude did not. The subagent confirmed it
    fails with the old `None` height.
  - `commands::schema_interactive::tests::chooser_height_fits_short_lists_and_caps_long_ones`.
- **Changed expectations:** none. No existing PTY test depended on the fixed
  8 rows. The visible change is that enum choosers with fewer than 6 options
  shrink to fit (for example, 3 options take 5 rows).

### Gates

- `claudine/`: `just test` ran 7376 tests: 7376 passed, 9 skipped. `just
  lint` exits 0. Its only warning is the pre-existing macOS linker `__eh_frame
  section too large` message, which is unrelated.
- `biscuit-terminal/`: `just test` (3325 passed) and `just lint` are green.
- `darkmatter/`: `just test` (8507 passed, 12 skipped) and `just lint` are
  green.
- No cross-OS runs. The changes are platform-neutral Rust. The only new
  `cfg(unix)` is on the PTY test, which follows the existing convention. CI's
  Linux and macOS legs cover the PR.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| R1 / R7.5 (tolerant selection, templated `{{…}}` and `$(…)` siblings) | `templated_file_sibling_does_not_rule_out_the_applicable_union_arm` |
| R7.5 (a literal invalid sibling still rules the arm out) | `literal_invalid_sibling_still_rules_out_the_union_arm` |
| R7.5 (conflicting discriminants and string-only alternatives stay `None`) | `templated_sibling_does_not_select_conflicting_or_string_only_arms`, plus the unchanged `supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched` |
| R1 end-to-end on the shipped artifact | `supplied_files_select_shipped_clarify_spec_arm`; the R7.1 PTY test `union_partial_with_templated_file_sibling_reaches_chooser` (now green) |
| N5 (no behavior change in the pre-validator) | the existing `composition::schema::tests::pre_validate_*` suite (92 tests green) |
| R5 / R7.7 (inline picker, and selection launches the stub) | `level1_pty_provider_picker_is_inline_and_launches_selection` |
| N8 (sizing) | `chooser_height_fits_short_lists_and_caps_long_ones` |
| N3 (locator) | the 10 `focused_regions_*` tests in `biscuit-terminal/lib/src/errors/source_context.rs` |
| N1 (`start_line`) | 6 renderer tests in `output/code_block.rs` and 3 render-tree tests in `code_block.rs` |

There are no skipped or pre-existing failures in the three gates. The 9, 55,
and 12 skips are the usual gated tests (tier or feature).

## Phase 3

Phase 3 added the D1 union fallback, fixed the caller base directory on the
late verdict (R4), made `Ctrl-C` cancel the single-file dialog, and added the
R7.2, R7.3, R7.4, and R7.6 coverage. It ran in the main session with no
subagents.

### D1 union fallback (R2, N6)

- `lib/.../schema/supplied.rs`: `supplied_file_shape` became
  `supplied_file_arms`, which returns `SuppliedArms::Unique(shape)` or
  `SuppliedArms::Undecided(arms)`. The arms are the applicable ones, or every
  arm when none applies. Arm judging (R1 tolerance, eager relaxation) is
  unchanged.
- `SuppliedArms::eager_file_target(name)` returns a `FileTarget { is_array,
  patterns }`.
  - For an undecided union, every contending arm must declare `name` as an
    eager `file(match)` with the same `is_array`.
  - Patterns merge in arm order, then pattern order, de-duplicated.
  - An arm that does not declare the property blocks the fallback. N6 said
    "every arm that declares it". I tightened that because an arm with no
    declaration makes the verdict depend on which arm wins, which is exactly
    what R2 rules out.
- The pending entry is built by a new shared `unresolved_caller_file(..)`.
  That keeps the reason text in one place (`caller_no_match_reason`).
- **Changed expectation:** the existing test
  `supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched` had a
  case `[{spec: eager file(**/*.md)}, {spec: eager file(**/*spec*.md)}]`
  expecting *no* pending entry. Ruling D1 now requires completion for exactly
  that shape, so the case moved into
  `undecided_union_deduplicates_patterns_in_arm_order`, where it expects the
  merged `["**/*.md", "**/*spec*.md"]`. The other three cases in the test
  (string alternative, conflicting discriminants, an arm without `spec`) are
  unchanged and still expect nothing. The plan's claim that the existing tests
  would pass unchanged did not hold for this one case. It is recorded in the
  plan's changed-expectations table.
- The inline tests grew past the 300-line placement budget
  (`test_placement::repository_test_placement`), so they moved verbatim to
  `schema/supplied/tests.rs`. The move only dedented them and deepened the
  `include_str!` paths by one level.

### Caller-origin late verdict (R4, N7)

- **Departure from the plan:** the plan said to emit a typed
  `UnresolvedFileReference` on the late path. A first version did that, and
  two problems showed up:
  - The error rendered as "composition failed", because that variant is an
    internal signal that the CLI always downgrades before display. The late
    path never reopens the chooser (S2).
  - It needed a completion glob. A root union whose arms disagree on array
    shape, or a bare eager `file` with no `match`, still named `…/prompts`,
    and R4 says *every* surface.
- The shipped fix is `rebase_caller_file_problems(&mut problems, records)` in
  `supplied.rs`. For each problem that carries Darkmatter's `NoMatch` text on
  a property that has a caller record, it re-resolves the raw value that the
  message names against `record.origin()`. If the value is still unresolved,
  it rewrites the message to the early pass's text, which names the caller's
  base directory. It needs no schema metadata, and multi-problem messages keep
  every problem.
- It is called at the top of `translate_schema_failure`, and also in
  `handle_retry_error`, which now takes the caller records. That was the one
  cheap secondary site. `post_shell_validate` is still not covered; see
  below.
- `classify_unresolved_file_reference` (the pre-validator, used only by
  documents without `initialize`) now handles root unions through
  `supplied::file_reference_target`. It uses the arm that declares the
  property as `file(match)`, or the D1 merge of several such arms. Its doc was
  updated, and it shares the `NO_MATCH` constant.
- The CLI's `downgrade_to_schema_validation` was briefly moved into the lib and
  then moved back unchanged once the late path stopped needing it. There is no
  net diff in `schema_interactive/`.

### Early-failure ordering and `Ctrl-C` (R2, R3, N9)

- No `prep.rs` change was needed. S1 had shown that every non-success outcome
  of the supplied-file pass already returns before target selection; the gap
  was only the empty pending list for unions. The comment at `prep.rs:176`
  still states the contract accurately. The chooser already consumes the
  `patterns` it is given, so the D1 merged globs needed no CLI change.
- S1's open item was confirmed and fixed. `read_confirm_key` ignored `Ctrl-C`,
  which arrives as a key in raw mode, so the `Use this file?` dialog kept
  waiting. It now returns an error of `CANCELLED_KIND`, the same as the
  `run_standalone` choosers, and the supplied-file pass reports it as
  unresolved before the picker. The doc on `confirm_one_file` was updated.
  The other caller, autocomplete (`operation_file.rs`), already mapped any
  dialog error to `AutocompleteNotInteractive`, as it does for the chooser's
  own `Ctrl-C`, so that is unchanged.

### Tests added (all L1)

| Requirement | Test | Regression proof |
|---|---|---|
| D1 merged patterns (N6) | `schema::supplied::tests::undecided_union_merges_every_arms_eager_file_patterns` | fails with the fallback disabled |
| D1 de-duplication and arm order | `…::undecided_union_deduplicates_patterns_in_arm_order` | fails with the fallback disabled |
| D1 negative cells: string, lazy, bare `file`, `file[]` arm, undeclared arm | `…::undecided_union_without_a_shared_eager_file_shape_has_no_fallback` | — (negative; the control is in the next row) |
| D1: a resolvable literal gives no pending entry (with control) | `…::undecided_union_leaves_a_resolvable_literal_alone` | its control fails with the fallback disabled |
| Pre-validator union classification | `schema::tests::provided_file_match_partial_in_a_union_reports_unresolved_file_reference` (single `file(match)` arm, D1 merge, array-shape conflict gives the generic error) | fails with the old `Union => None` |
| R4 lib: the late verdict names the caller's base | `prepare::service::tests::late_caller_file_verdict_names_the_callers_base_directory` (array-disagreeing union, bare eager union, single schema; each with a resolvable-literal control) | fails with the rebase call removed |
| R4 / R7.6 CLI, launch root, document in `prompts/`, after `initialize` | `compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory` (asserts `events.log` proves the late verdict) | covered by the lib proof (same call site) |
| R2 non-interactive, before `initialize` (templated sibling and D1) | `compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` (each with a control that runs `initialize`) | the D1 case fails with the fallback disabled (`initialize` runs) |
| R7.2 two matches: chooser, pick, the pick reaches the provider | `level1_provided_partial_file_pty::union_partial_with_two_matches_opens_the_chooser_and_launches_the_pick` | — |
| R7.3 zero candidates: error, picker never opens, no `initialize` | `…::union_partial_with_zero_matches_fails_before_the_provider_picker` | — |
| R7.3 dialog before picker, then the picked provider gets the path | `…::union_partial_file_dialog_renders_before_the_provider_picker` (byte offset of `Use this file` < `KBD_ENHANCEMENT_PUSH`) | — |
| R2 declined and `Ctrl-C` | `…::union_partial_declined_or_cancelled_fails_before_the_provider_picker` | the `Ctrl-C` case fails (hangs) without the `read_confirm_key` fix |
| R7.4 D1 end to end, with and without `initialize` | `…::d1_union_chooser_lists_both_trees_and_the_fixes_pick_composes` | the `initialize` variant fails with the fallback disabled; the other variant is carried by the pre-validator's union classification, so both surfaces are pinned |

PTY tests assert on "picker never rendered" through the absence of
`KBD_ENHANCEMENT_PUSH` and of the `Goose` option label. The picker has no
title (S3). The chooser tests initialize a git repository in the fixture, so
that labels are repository-relative and readable at the PTY's default width.

### Manual check (Checkpoint 3)

The debug build ran against the real `prompts/clarify.md` from the repo root,
with a stub `claude`, a throwaway `HOME`, and silenced audio, driven by a
Python `pty` harness:

- `spec=union-partial`: the dialog appeared first, nothing launched before
  the answer, and after `y` the stub's argv carried the chosen spec path.
- `spec=fix`: the chooser opened at 0.3 s. Enter launched the stub with the
  selected `fixes/…/spec.md`.
- Non-TTY: relative and absolute spec paths launch, and `spec=fix` fails
  early, naming the repo root.
- Harness gotcha: an inline `run_standalone` prompt fails at once if the
  driver does not answer its DSR (`ESC[6n`) and OSC 10/11 color queries. The
  L1 harness does this in `common/pty.rs`. An ad-hoc driver must do it too, or
  the chooser "fails", which the supplied pass reports as unresolved.

### Found, not fixed (pre-existing; outside the spec's C1–C5)

- **Darkmatter root-union caller projection judges relative caller paths from
  the document's directory.**
  - Reproduction: in a non-git directory, put the D1 two-arm document at
    `prompts/p.md` and run `claudine compose --goose prompts/p.md
    spec=fixes/x/spec.md` from the root, with that file existing. It fails
    with `no existing file matched reference … while resolving from
    …/prompts`.
  - These work: an absolute path, the document at the launch root, and a
    single (non-union) schema.
  - Inside a git repo, the document without `initialize` fails even for an
    absolute path (the value arrives relative). With `initialize` it
    succeeds.
  - Why the rebase does not apply: the value *does* resolve from the caller's
    origin, so the verdict is not a caller `NoMatch`. It is
    `collect_applicable_root_schema_fragments` /
    `project_root_arm_caller_values`
    (`darkmatter/lib/src/markdown/compose/schema_validation.rs:779`, `:932`)
    selecting no arm and falling back to document-context resolution.
  - Impact: the interactive chooser returns absolute paths, and the real
    `clarify.md` (git repo, has `initialize`) works. A D1-shaped document in
    a subdirectory, run without `initialize` inside a git repo, can still fail
    after a successful pick.
  - The R2 non-interactive L1 test uses an absolute path for its control
    because of this.
- **A Darkmatter union where a caller file's mode differs by arm** (for
  example eager `file` in one arm, `string` or lazy `file` in the other, with
  a templated discriminant) is refused with `caller file parameter … changed
  file mode during composition`, even for valid paths. That is Darkmatter's
  deliberate guard, noted here because it shaped the R4 fixtures.
- `post_shell_validate` (`schema/mod.rs:221`) still reports Darkmatter's base
  for caller values. It has no caller records, and compose fails before it for
  these inputs (S2), so it was left alone.

### Gates

- `claudine/`: `just test` ran 7552 tests: 7552 passed, 9 skipped. The first
  run failed only on the placement budget, which the test move fixed. `just
  lint` exits 0; its only warning is the known macOS `__eh_frame` linker
  message. `just check-tier-coverage claudine` reports nothing stranded.
- Windows (`just cross-check claudine --os windows composition::schema
  late_caller_file_verdict`): 96 of 97 passed, including every new lib test.
  The one failure,
  `composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`,
  is a `/` versus `\` spelling in the shipped implement prompt's `git add`
  command. That code is untouched by this phase, and the composition itself
  succeeds, so it is unrelated. It is left for fix-forward per the Windows CI
  policy.
- The new CLI tests are `#[cfg(unix)]` (the PTY convention, and
  `compose_schema_cli.rs`'s shell-stub convention). No new `cfg` appears in
  production code.

## Phase 4

Phase 4 replaced the whole-block frontmatter excerpt with focused regions (R6,
N2, N4). Everything here is L1 (unit tests plus the existing CLI suites); the
L2 capture tests that S4 flagged were re-run against the new output.

### Design, and where it departs from the plan

- **Two locators, split by job.** The plan (N3) and R6 ("reuse, don't
  re-implement") have `biscuit-terminal` supply the line selection. Claudine's
  diagnostics, though, name *semantic* paths that `YamlKeyPath` does not
  model: sequence indexes (`initialize.stack[0].action[1].set`,
  `…metadata.files[0]`) and re-rooted sequence-task paths
  (`tasks[0].setup[0]…` for authored `sequence[0].setup[0]…`, matched by
  unique suffix). The split is now:
  - Claudine's `locate_property_line` (kept, and fixed; see below) resolves a
    semantic path to a **line**.
  - A new `biscuit-terminal` method, `SourceContext::focused_line_regions(lines,
    context)`, does everything else: the ±context window, enclosing header
    lines found by indentation, merging, clamping, and the unsafe-YAML refusal.
    It shares one private `select_regions` with `focused_yaml_regions`, so the
    two cannot drift.
  - `focused_yaml_regions` (Phase 2) stays for Darkmatter's
    `focused_yaml_excerpt`. It is not used by Claudine.
- **Locator bug fixed.** `locate_property_line` pushed a `- key:` line as one
  parent at the marker's indent, so the item's later keys became children of
  its first key (`$schema[0].spec.doc`). This is the same trap the N3 note
  recorded for `biscuit-terminal`. It now pushes the item (`a[0]`) at the
  marker column and the key (`a[0].key`) at the key column, and records the
  item path too, so `failure.stack[0]` resolves. Pinned by
  `item_keys_after_the_first_are_siblings_not_children`.
- **The anchor exception (S4's open question).** The fully-covered exception
  now means "the selection holds every line *between* the delimiters". A
  delimiter hides nothing, and the inline-compose mismatch fixture (anchored
  `sequence: &seq … alias: *seq`) has its `prompt`/`sequence` windows reaching
  every line except the closing `---`. Without this, that L1 PTY test and both
  L2 mismatch captures would lose their excerpt. A partial slice of an
  anchored block is still refused.
- **Highlight variants.** `BlockOnly` is gone. Besides `Property`, `Line`,
  and `SchemaSpan`, there are two new variants:
  - `Properties(Vec)` for plain keys shown together (the mismatch error:
    `prompt` + `sequence`).
  - `SchemaProperties(Vec)` for properties a schema problem names. The plan
    called this `Properties`; it is split out because the per-name rule
    differs. Each name is shown at its exact frontmatter key (when set) and at
    its `$schema` declaration: inline mapping `$schema.x`, or every union arm
    `$schema[N].x`. It is used by `SchemaValidation` (one or many),
    `MissingProperties` (names plus pointer paths), `UnresolvedFileReference`,
    and `UnsupportedInteractiveSchema`. The last was `Property` in S4's table;
    it is a schema problem, and its declaration is what is unsupported.
- **Producers per S4**, with one addition:
  - `FrontmatterParse` with a YAML location becomes `Line(yaml_line + 1)`;
    without a location there is no excerpt.
  - Other `ComposeFailed` variants give no excerpt.
  - `ShellExpansionFailed` is focused only for a `Frontmatter { key }` origin.
    Instead of an ad-hoc `match` on eleven variants in Claudine, Darkmatter
    gained `ShellExpansionError::origin()`, beside the existing `command()`.
  - Absent keys (`PromptPropertyMissing`, `agent` from the CLI) drop out
    naturally: nothing is located, so there is no excerpt.
- **`capture_line`** now accepts an ordinary `---` block as well as the
  `----` near-miss block, because a located YAML parse error lives in a normal
  block.
- **Rendering (N2).** Each region is one `CodeBlock` with
  `with_start_line(region.start_line)` and absolute highlights. The `⋮` line
  is placed in the `│` gutter column of the region below it, measured from
  that block's rendered output, because the gutter width changes with the
  digit count (a 1-digit region followed by a 2-digit one).
- **Known cosmetic limits, not changed:**
  - Each region repeats `CodeBlock`'s `yaml` language label and padding rows.
    Removing them would need a Darkmatter option, and there is no second
    consumer for it.
  - At `ColorDepth::None`, `CodeBlock` degrades to a plain fence with **no
    line numbers**. That was already true of the old appendix.
    `composition.md` now says so.

### Tests (all L1 unless noted)

| Requirement | Test | Regression proof |
|---|---|---|
| R7.8 a mid-file property: ≤7 window lines plus ancestors | `frontmatter_excerpt::tests::mid_file_property_shows_three_lines_either_side_plus_its_ancestor` (spans `(3,3)`, `(7,13)`; `agent`/`tail` absent) | the old model rendered all 17 lines |
| R7.8 `/spec` against the shipped `prompts/clarify.md` (`include_str!`, so CI reruns on edits) highlights the arm-0 `spec` line | `…::caller_input_problem_highlights_its_arm_declaration_in_clarify`, plus end to end through the error producers: `error::tests::caller_input_schema_problem_in_clarify_highlights_the_arm_declaration` (`SchemaValidation`, `UnresolvedFileReference`, `MissingProperties`) | the old code located nothing for `spec` and rendered the whole block unhighlighted (C5) |
| R6 every union arm, plus a same-named top-level key | `…::schema_property_shows_every_arm_and_a_same_named_top_level_key`, `…::schema_property_under_an_inline_mapping_schema_is_located` | — |
| R7.8 unlocatable gives no excerpt | `…::unlocatable_property_gives_no_excerpt`, `error::tests::enrich_omits_the_excerpt_when_nothing_is_locatable` (missing `prompt`, body interpolation, an undeclared schema pointer) | `BlockOnly` rendered the whole block for each |
| R7.8 two problems give two regions with `⋮` | `…::two_problems_give_two_regions_with_an_elision_line`, `error::tests::several_schema_problems_show_the_union_of_their_regions` | multi-problem was `BlockOnly` |
| R7.8 gutter numbers match source lines, and `⋮` sits in the gutter | `…::rendered_gutter_numbers_match_source_lines` (every source line: present with its real number iff selected) | — |
| N4 whole-block exception | `…::a_block_the_window_already_covers_renders_whole`; `biscuit-terminal` `focused_regions_unsafe_yaml_fully_covered_renders_the_whole_block`, `focused_regions_unsafe_yaml_covered_but_for_a_delimiter_is_kept` | — |
| Located `FrontmatterParse` is windowed | `error::tests::enrich_frontmatter_parse_error_windows_the_reported_line` (YAML line 3 gives source line 4) | — |
| Mismatch focuses `prompt` and `sequence`; shell failures focus only frontmatter origins | `error::tests::inline_sequence_mismatch_focuses_prompt_and_sequence`, `…::shell_expansion_failure_is_excerpted_only_for_a_frontmatter_origin` | — |
| Locator sibling fix | `…::item_keys_after_the_first_are_siblings_not_children` | fails on the old parent stack (`$schema[0].doc` was `None`) |
| `focused_line_regions` | `biscuit-terminal` `focused_line_regions_keep_arm_and_parent_ancestors`, `…_find_the_owner_of_a_flush_sequence`, `…_ignore_lines_outside_the_block`, `…_refuse_a_partial_slice_of_anchored_yaml` | — |
| `ShellExpansionError::origin()` | `darkmatter` `shell_expansion::types::tests::origin_names_the_authored_location_or_none_for_unscoped_failures` | — |

The changed existing expectations are in the plan's changed-expectations
table (Phase 4 rows). None was loosened. The S4-flagged
`level1_pty_mismatch_takes_tty_branch_with_yaml_block` is unchanged and
passes, through the delimiter rule above.

### Gates

- `claudine/`: `just test` ran 7567 tests: 7567 passed, 9 skipped. The first
  run failed only `level1_pty_mismatch_takes_tty_branch_with_yaml_block`,
  before the delimiter rule; it is green after it. `just lint` exits 0; its
  only warning is the known macOS `__eh_frame` linker message. `just
  check-tier-coverage claudine` reports 0 stranded.
- `biscuit-terminal/`: `just test` ran 3331 tests: 3331 passed. `just lint`
  is clean.
- `darkmatter/`: `just test` ran 8582 tests: 8582 passed. `just lint` is
  clean.
- L2 (tmux only), `level2_{malformed_frontmatter,inline_compose_mismatch,schema_parse,removed_validation_key,invalid_file_reference}_capture`:
  8 of 8 passed. The WezTerm variants were not run; Phase 5 runs the full
  `just test-l2`.
- Windows: `just cross-check claudine --os windows frontmatter_excerpt
  composition::error::tests` ran 144 tests: 144 passed. This includes the
  `include_str!` of `clarify.md`, where CRLF checkouts are the risk.
- No new `cfg` anywhere.

### Docs

- `claudine/docs/topics/composition.md`: "Frontmatter YAML blocks in errors"
  is rewritten. It covers focused regions with a `clarify.md` example, a
  per-error focus table, the omission rule, the anchor rule, a Mermaid flow,
  and the no-color fence caveat. The mismatch and removed-key paragraphs no
  longer promise the whole block. Phase 5 only needs to review it.
- `biscuit-terminal/README.md` documents `focused_line_regions` and the anchor
  rule. The claudine skill's "Composition diagnostics" row is updated.

## Phase 5

Phase 5 ran the full gates, checked the Windows compile surface, corrected the
remaining docs drift, and filled in the changed-expectations table. No source
or test file changed in this phase.

### Gates (macOS, local)

| Area | Gate | Result |
|---|---|---|
| `claudine/` | `just test` | 7567 run: 7567 passed, 9 skipped |
| `claudine/` | `just lint` | exit 0; the only warning is the known macOS `__eh_frame` linker message |
| `claudine/` | `just test-l2` | `claudine-cli` 251 run: 251 passed; `claudine-gen` 3 run: 3 passed |
| `claudine/` | `just check-tier-coverage claudine` | 0 stranded |
| `biscuit-terminal/` | `just test` / `just lint` | 3331 run: 3331 passed, 55 skipped / exit 0 |
| `darkmatter/` | `just test` / `just lint` | 8582 run: 8582 passed, 12 skipped / exit 0 |

The L2 run includes every capture S4 flagged, on both backends:
`level2_{malformed_frontmatter,inline_compose_mismatch,schema_parse,removed_validation_key,invalid_file_reference}_capture`
(the tmux variants plus the WezTerm `mismatch` and `removed_key` variants that
Phase 4 did not run). All passed.

The skips are the usual tier- and feature-gated tests. There are no failures,
pre-existing or new.

### Windows

`just cross-check <package> --os windows <filter>` builds the package's whole
L1 target set on the native Windows host and runs the filtered tests:

- `claudine-cli`, filter `schema_interactive`: pass (29 tests).
- `biscuit-terminal`, filter `source_context`: pass.
- `darkmatter`, filter `code_block`: pass.

Phase 4 already ran `frontmatter_excerpt` and `composition::error::tests` on
Windows (144 passed), including the `include_str!` of `clarify.md`.

`cfg` audit over every source file this plan touched, compared with the plan's
first commit: the only new gates are three `#[cfg(unix)]` in
`cli/tests/l1/compose_schema_cli.rs`, on `caller_file_fixture` and the two R2/R4
CLI tests. That file's existing tests all carry `#[cfg(unix)]`, because its
provider stubs are `#!/bin/sh` scripts. The lib logic behind those tests has
platform-neutral unit coverage
(`prepare::service::tests::late_caller_file_verdict_names_the_callers_base_directory`
and the `schema::supplied::tests` D1 rows), so the Windows surface keeps its
shape. There is no new `cfg` in production code.

`just ci-local --plan` is the author's step before pushing; the agent does not
push.

### Docs drift

- `claudine/docs/topics/composition.md`:
  - Provider Selection → TTY Mode: the picker renders inline, below the
    cursor, at most 8 rows, with no alternate screen. `Esc` keeps the initial
    selection and `Ctrl-C` cancels. It is the last question before launch, and a
    decidable caller-file failure is reported before it opens (R2, R3, R5). This
    was undocumented.
  - Provided Partial File References: the closing paragraph still said a union
    where several arms match "defers to canonical preparation". That
    contradicts ruling D1, which Phase 3 documented a few paragraphs above it.
    It now says how the arm is chosen (templated values are undecided) and that
    an undecided union completes only through the merged-glob rule. The code
    was checked: `SuppliedArms::Undecided` holds the applicable arms, or every
    arm when none applies.
  - The "Frontmatter YAML blocks in errors" section (Phase 4) was reviewed and
    already states that the excerpt is focused and omitted when unlocatable.
- `.claude/skills/claudine/timeline.md`: a `union-partial-file-completion` entry.
  `SKILL.md` was already current (Phases 3 and 4), and `cli-reference.md` does
  not describe the picker.
- Darkmatter `CodeBlock::with_start_line` / `CodeBlockMeta::start_line`, and
  the `biscuit-terminal` `SourceContext::focused_{yaml,line}_regions` rustdoc
  and README, were reviewed and are current.
- The spec's `packages` now lists `darkmatter` (N1).

### Changed expectations

Phase 5 changed no test or snapshot. The plan's table has a Phase 5 row saying
so. One DoD wording note: "the existing `supplied.rs` tests pass unchanged"
holds with one ruled exception. Phase 3 moved the
`[{spec: **/*.md}, {spec: **/*spec*.md}]` shape out of the "untouched" test,
because ruling D1 now completes it. That move is the first row of the table.

### Requirement-to-test mapping (whole plan)

| Requirement | Test(s) | Tier |
|---|---|---|
| R7.1 reproduce C1 | `level1_provided_partial_file_pty::union_partial_with_templated_file_sibling_reaches_chooser` | L1 PTY |
| R7.2 fixed: chooser, then the stub gets the path | the R7.1 test, `…::union_partial_with_two_matches_opens_the_chooser_and_launches_the_pick` | L1 PTY |
| R7.3 ordering | `…::union_partial_with_zero_matches_fails_before_the_provider_picker`, `…::union_partial_file_dialog_renders_before_the_provider_picker`, `…::union_partial_declined_or_cancelled_fails_before_the_provider_picker`, `compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` | L1 |
| R7.4 D1 | `…::d1_union_chooser_lists_both_trees_and_the_fixes_pick_composes`, `schema::supplied::tests::undecided_union_*` (4) | L1 |
| R7.5 lib arm selection | `schema::supplied::tests::{templated_file_sibling_does_not_rule_out_the_applicable_union_arm, literal_invalid_sibling_still_rules_out_the_union_arm, templated_sibling_does_not_select_conflicting_or_string_only_arms, supplied_files_select_shipped_clarify_spec_arm}` | L1 |
| R7.6 R4 | `compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`, `prepare::service::tests::late_caller_file_verdict_names_the_callers_base_directory` | L1 |
| R7.7 R5 | `level1_provider_picker_pty::level1_pty_provider_picker_is_inline_and_launches_selection`, `schema_interactive::tests::chooser_height_fits_short_lists_and_caps_long_ones` | L1 |
| R7.8 R6 | `frontmatter_excerpt::tests::{mid_file_property_shows_three_lines_either_side_plus_its_ancestor, caller_input_problem_highlights_its_arm_declaration_in_clarify, unlocatable_property_gives_no_excerpt, two_problems_give_two_regions_with_an_elision_line, rendered_gutter_numbers_match_source_lines}`, `error::tests::*` (Phase 4 table), the L2 captures above | L1, L2 |

Each name was checked against the tree in this phase. The PTY files are declared
in `cli/tests/l1/main.rs`. No test path carries a tier-removing segment.
