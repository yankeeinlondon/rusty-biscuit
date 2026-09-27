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
