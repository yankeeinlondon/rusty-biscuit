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
