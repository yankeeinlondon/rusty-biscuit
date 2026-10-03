---
fix: 2026-09-27-union-partial-file-completion
deferred_perf_measurement: false
implementation_1: "2026-09-28T16:01:21-07:00"
implementation_2: "2026-09-28T17:18:07-07:00"
---

# Implementation Log: Union Partial File Completion

## Implementation of Review Findings #1

> **started at:** 2026-09-28T16:01:21-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-09-27-union-partial-file-completion/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Root-union file selection can reject a valid caller-owned path' at 16:02:10
        - delegated to a `rust-developer` subagent (skills: `claudine`, `darkmatter`, `rust`, `rust-testing`)
        - defect class: a caller-supplied file value loses its caller origin whenever a step cannot commit to one schema branch, or reuses state built for a different resolution context, and is then judged, rewritten, or displayed as if the document had authored it
        - class sweep: caller file value judged from the document's base instead of its caller origin; sites checked: Darkmatter root-arm projection (`collect_applicable_root_schema_fragments` / `project_root_arm_caller_values`), arm-selection consumers (`resolve_caller_file_overrides`, `classify_caller_overrides`, `ensure_projection_stable`), per-fragment mode selection (`select_file_mode`), eager-file normalization write-back in `run_with_registry`, `rewrite_root_union`, `ValidatorCache` identity, `match` glob enforcement, Claudine `schema::supplied` early resolver, `classify_unresolved_file_reference`, `pre_validate_with_origin`, `drop_invalid_optionals_with_origin`, `build_schema_status_report_for_mode`, `rebase_caller_file_problems`, completion-phase validation, `inline-compose` launch phase, sequence steps and `proxy`; fixed here: root-arm projection, eager-file normalization write-back, `ValidatorCache` identity, `build_schema_status_report_for_mode`; clean: `select_file_mode`, `rewrite_root_union`, `match` (not enforced by validation), `supplied.rs`, `classify.rs`, `pre_validate_with_origin`, `drop_invalid_optionals`, `rebase_caller_file_problems`, completion / `inline-compose` / sequence / `proxy` (all route through the fixed projection)
        - root cause, relative path: when several arms (or none) accepted the value, no root arm was committed, the caller projection was empty, and the raw relative value was resolved from `prompts/`
                - fix: new `caller_file_fragments_shared_by` projects a caller property through an undecided union when every contending arm declares it with exactly one file fragment of the same mode; disagreeing arms still leave it raw
        - root cause, git fixture with an absolute path (each half confirmed by disabling it alone):
                - eager-file normalization rewrote the unprojected absolute caller value to repository-relative `fixes/x/spec.md` as though the document had authored it; the write-back now skips caller-record keys the projection left unclassified
                - `ValidatorCache::validator_for` keyed only on schema, `base_dir`, and fallback, so a validator compiled without a repository root was reused for requests that had one; the key now includes the repository root and package area and a hit requires the full `FileResolutionContext` to match (this also fixed standalone `md compose` for an authored `../fixes/x/spec.md` in a git repo)
        - discovered: `match` is chooser suggestion metadata and validation never enforces it, so picking a file does not by itself select a union arm; `composition.md` claimed otherwise and was corrected (the spec is left as decided)
        - files changed: `darkmatter/lib/src/markdown/compose/schema_validation.rs`, `darkmatter/lib/src/markdown/schemas/validate.rs`, `darkmatter/lib/src/markdown/compose/tests/schema.rs`, `claudine/lib/src/composition/schema/{mod,classify,tests}.rs`, `claudine/cli/tests/l1/compose_schema_cli.rs`, `claudine/cli/tests/l1/level1_provided_partial_file_pty.rs`
        - docs updated: `claudine/docs/topics/composition.md`, `darkmatter/docs/inline/schema-validation.md`, `darkmatter/docs/topics/schemas/definition.md`, `.claude/skills/claudine/SKILL.md`
        - tests added or changed:
                - Darkmatter: `undecided_root_union_resolves_a_caller_file_from_the_launch_area`, `undecided_root_union_leaves_a_caller_value_whose_file_mode_is_disputed_verbatim`, `cache_keeps_validators_for_different_resolution_contexts_apart`
                - Darkmatter: `ambiguous_root_union_does_not_guess_a_file_arm` renamed to `ambiguous_root_union_with_agreeing_file_arms_resolves_from_the_launch_area`, expectation flipped (the old one encoded the defect)
                - Claudine lib: `status_report_judges_a_caller_file_from_the_launch_area`
                - Claudine CLI: `compose_caller_file_resolves_from_the_launch_directory_for_every_schema_shape` walks every review row (single / settled discriminator / undecided union × non-git / git × relative / absolute) on one fixture with the prompt in `prompts/`
                - Claudine CLI: the non-interactive control `compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` now uses the relative path
                - Claudine CLI PTY: `d1_union_chooser_pick_composes_for_a_prompt_outside_the_launch_directory` (git fixture, no `initialize`, chooser pick then compose and launch)
                - the three new or changed CLI tests fail against the old Darkmatter code and pass with the fix
        - results (macOS):
                - `darkmatter/`: `just test` 8637 passed, 12 skipped; `just lint` exit 0
                - `claudine/`: `just test` 7741 passed, 1 failed, 9 skipped; `just lint` exit 0; `just check-tier-coverage claudine` 0 stranded
                - the one failure, `dispatch_inventory::dispatch_inventory_matches_committed_file` (committed 1624 sites, generated 1636), predates this change: it comes from `cli/src` changes merged from `main` (for example 57968bb03) after the inventory was last blessed, and this diff touches no `cli/src` file and no `Provider::` token
        - found, different class: document-authored implicit repository-relative values are judged without the repository-root step by Claudine's context-free validators (`load_effective_schema` builds `DarkmatterSchemas` without the request `FileResolutionContext`); see deferred list below
- work completed for 'Root-union file selection can reject a valid caller-owned path' at 16:36:19
- starting the work on 'The inline provider picker lacks real-terminal verification' at 16:37:05
        - delegated to a `feature-tester-rust` subagent (skills: `claudine`, `rust`, `rust-testing`, `biscuit-test-harness`, `biscuit-tui`)
        - defect class: an interactive prompt whose contract concerns what the terminal shows (inline placement, kept scrollback, no blank full-screen residue, the widget's rows actually drawn) is verified only by raw PTY bytes, which render nothing and keep no scrollback
        - class sweep: interactive prompt rendered-state contract proven only by PTY bytes; sites checked: provider picker via `compose`, provider picker via `inline-compose`, `sequence` (`review_sequence` is full-screen by design and a spec non-goal; `sequence` never calls `prompt_one_shot_provider`), partial-file chooser (`choose_one_file` / `choose_many_files`), `Use this file? (Y/n)` confirmation (`confirm_one_file`), `schema_interactive` required-missing prompts (text, number, boolean, enum, enum[]); fixed here: Level 2 evidence for the provider picker (`compose` and `inline-compose`), the `schema_interactive` prompts (plus a height bug, below), the partial-file chooser, and the confirmation; clean: provider picker rendering, partial-file chooser rendering, confirmation rendering
        - what the real-terminal captures showed:
                - the provider picker draws inline below prior output (4 rows), and after submit or Ctrl-C the viewport is cleared and all prior output stays in history in order; no code change needed
                - bug found and fixed: `biscuit-tui`'s default standalone chrome pads the widget one row top and bottom with the hint on the bottom row, and Claudine's height math omitted the top row
                        - text and boolean prompts (height 2, already wrong on `main`) drew no widget rows, so typed input was invisible
                        - the number prompt's retry error could not draw
                        - labeled enum choosers (`chooser_height`, introduced on this branch) hid their last option
                        - fix: `STANDALONE_CHROME_ROWS = 2`; `inline_height` now takes content rows; `chooser_height(option_count, labeled)` adds the label row, still capped at 8
                - discovered: the picker's hint says `Esc=Cancel`, but `biscuit-tui`'s `ChooseOne` treats Esc as "restore the default and submit"; only Ctrl-C cancels (reported to the author, not changed; this is `biscuit-tui` design rather than a requirement of this spec)
        - proof the tests are load-bearing: with the padding allowance at 0 the schema and picker tests fail; with the picker forced full-screen the picker tests fail because the line above the prompt is not on screen
        - files changed: `claudine/cli/src/commands/schema_interactive/{mod,tests}.rs`, `claudine/cli/src/commands/wrap/selection_ui.rs`, `claudine/cli/tests/level2/level2_inline_prompt_scrollback.rs` (new), `claudine/cli/tests/level2/main.rs`, `claudine/docs/topics/composition.md`, `claudine/docs/topics/testing.md`
        - tests added in `level2_inline_prompt_scrollback`, each under `tmux_backend::` and `wezterm_backend::` (14 total, WezTerm spawned in the background so it never takes focus):
                - `compose_provider_picker_submit_keeps_scrollback`
                - `compose_provider_picker_cancel_keeps_scrollback`
                - `inline_compose_provider_picker_submit_keeps_scrollback`
                - `inline_compose_provider_picker_cancel_keeps_scrollback`
                - `schema_required_prompts_keep_scrollback`
                - `partial_file_chooser_keeps_scrollback`
                - `partial_file_confirmation_keeps_scrollback`
                - each seeds a history sentinel, 60 filler lines, and an above-prompt sentinel; asserts the sentinel is visible while the prompt is open, and after close that full history is intact in order, no hint residue remains, and no blank run exceeds 2 rows; on submit the chosen stub launches, on cancel none does; the audio spool is never created
        - updated: `chooser_height_fits_short_lists_and_caps_long_ones` for labeled and unlabeled cases
        - results (macOS, `claudine/`):
                - `just test-l2`: claudine-cli 266 passed, claudine-gen 3 passed; the new file 14/14
                - `just test`: 7741 passed, 1 failed (the pre-existing `dispatch_inventory` failure noted above), 9 skipped
                - `just lint`: exit 0; `just check-tier-coverage claudine`: 0 stranded
        - found, pre-existing and unrelated: `cargo clippy -p claudine-cli --features terminal-tests --test level2` reports `needless_lifetimes` in `level2_dry_run_metadata_capture.rs:283`, a file this cycle did not touch and that `just lint` does not build
- work completed for 'The inline provider picker lacks real-terminal verification' at 17:00:12
- starting the work on 'native Windows verification' at 17:00:30
        - reason: the Darkmatter fix compares file-resolution contexts and resolved paths, which is a Windows path-spelling risk
        - `just cross-check darkmatter --os windows` filtered to `schema`: 1339 passed, 1 failed: `undecided_root_union_resolves_a_caller_file_from_the_launch_area`
                - cause: the test built its expectation with `join("fixes/x/spec.md")`, which keeps forward slashes on Windows, while the code correctly returns native separators; production behavior was right
                - fixed: the expectation now joins by component; the sibling `undecided_root_union_leaves_a_caller_value_whose_file_mode_is_disputed_verbatim` compares the caller value verbatim and was already clean
        - rerun on Windows: `darkmatter` filtered to `root_union` 60/60 passed; `claudine` filtered to `status_report` 11/11 passed
        - the new Claudine CLI and PTY tests are `#[cfg(unix)]` (the file's `#!/bin/sh` stub convention), and the Level 2 file is `#[cfg(unix)]` (tmux and WezTerm harness)
- work completed for 'native Windows verification' at 17:12:27

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 h 11 min. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no review finding was deferred
- adjacent defects found during the class sweeps, outside this spec's requirements and not fixed in this cycle:
        - **deferred:** document-authored implicit repository-relative file values are judged without the repository-root step by Claudine's context-free validators (`load_effective_schema` builds `DarkmatterSchemas` without the request `FileResolutionContext`), so a document in `prompts/` authoring `spec: fixes/x/spec.md` in a git repo fails `claudine compose` while `md compose` passes
                - why deferred: the value belongs to the document, not the caller (R4 and finding 1 concern caller values), and the fix threads the request context through public `claudine::composition` APIs used by `schema_interactive`, `sequence`/`jit`, and `phase1c`; it deserves its own fix spec
        - **deferred:** `biscuit-tui`'s `ChooseOne` shows `Esc=Cancel` but Esc restores the default and submits, so Esc in the provider picker launches the default provider; only Ctrl-C cancels
                - why deferred: this is a `biscuit-tui` design decision affecting every chooser, not a requirement of this spec; it needs an author ruling
        - pre-existing, unrelated: `dispatch_inventory::dispatch_inventory_matches_committed_file` fails in `claudine/` `just test` because `cli/src` changes merged from `main` were never re-blessed (`CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`)
        - pre-existing, unrelated: clippy `needless_lifetimes` in `claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs:283` under `--features terminal-tests`

## Implementation of Review Findings #2

> **started at:** 2026-09-28T17:18:07-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-09-27-union-partial-file-completion/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Union arm selection ignores file match patterns' at 17:18:17
        - defect class: a `file(match(...))` glob that discriminates between root-union arms is used to offer completion candidates but discarded whenever anything decides which arm accepts the resulting path (early arm selection, caller-file projection, final validation)
        - design decision: `match` becomes an arm constraint in Darkmatter's schema conversion, only where it can discriminate: a root union with two or more arms, on a top-level property that another arm also declares, where the declaring arms do not all carry the same globs; a single schema, and a union property whose `match` no other arm contests (`prompts/review.md`, `prompts/implement.md`, `plan` in `prompts/plan.md`), keep suggestion-only semantics so shipped routers do not start rejecting paths the glob never discriminated
        - discovered: root unions are converted arm by arm in `darkmatter/lib/src/markdown/schemas/resolve.rs` (`resolve_root_union`, `resolve_standalone_root_union`), not only in `convert::to_json_schema`, so the arm constraint is attached once every arm is known, at all three sites
        - implemented in Darkmatter: new `schemas::file_match` (the `x-darkmatter-match` keyword, `contested_match_patterns`, `file_match_admits`, and `FileMatchGlobs`, the glob comparison moved out of `claudine-cli` so candidate discovery and validation share one implementation); the validator registers the keyword; a root union reports a match-ruled-out arm only when every arm was ruled out, and surfaces the glob message through an optional file's `null`/empty-string wrapper; caller-file mode selection ignores the keyword so a ruled-out arm does not cost a caller value its launch origin
        - decisions: a value is judged only when it names an existing file (a partial, a lazy output that does not exist yet, and template or shell syntax never rule an arm out); a document-authored value and a caller value get the same rule, each resolved from its own origin; each element of a contested `file[]` must match; the comparison is `/`-separated and relative to the launch directory first (then the document directory, then the repository root), which is where candidate discovery walks; with no document anchor (coercion's validator) nothing is judged
        - implemented in Claudine: `schema::supplied` arm selection rules out an arm whose contested glob rejects an existing caller file (each arm is validated alone there, so it applies `contested_match_patterns` / `file_match_admits` directly); `file_reference_target` (late verdict chooser) reads only the arm a literal discriminant settles, since the other arm's candidates would now be rejected; `claudine-cli` `MatchGlobs` is now Darkmatter's `FileMatchGlobs` and the unused `globset` dependency was dropped
        - discovered: an arm-specific `required` property is not a usable observable for chooser tests, because a literal missing requirement already rules its arm out in the early supplied-file pass (by design); arm-specific coercion (`count: string` vs `count: number`) is, and it exposed one more sibling site: coercion's root-union arm probe compiles validators with no document anchor, so the glob keyword now resolves from the process directory there, the same fallback the `darkmatter-file` format uses
        - tests added: Darkmatter `file_match::tests` (5), compose `single_schema_match_only_suggests_files`, `settled_union_arm_rejects_an_existing_file_outside_its_match`, `undecided_union_selects_the_arm_whose_match_admits_the_file`, `optional_contested_file_is_rejected_by_its_glob_from_the_callers_origin`, `contested_glob_decides_which_arm_coerces_siblings`, `document_authored_file_selects_its_arm_by_match_too`, `every_element_of_a_contested_file_array_must_match_its_arm`; `md schema validate` CLI `schema_validate_contested_file_match_selects_and_rejects_union_arms` (all six review rows); Claudine lib `an_existing_file_selects_the_arm_whose_contested_glob_admits_it`, `a_partial_never_rules_a_contested_arm_out`, `late_verdict_on_a_settled_union_completes_against_that_arm_alone`; Claudine CLI `compose_contested_file_match_selects_and_rejects_union_arms` (all six rows through `claudine compose`)
        - tests changed: both D1 PTY chooser tests (with and without `initialize`) now type `count` differently per arm and assert the stub receives `Count is a number: true`, proving only the `fixes` arm applied; before, they asserted only the chosen path, which the defect also satisfied
        - each new or changed Darkmatter, Claudine CLI, and PTY test was confirmed to fail with the arm constraint disabled, and the two Claudine lib selection tests with their Claudine-side check disabled
        - docs updated: `claudine/docs/topics/composition.md` (the iteration-1 statement that `match` is not enforced is replaced by the contested-glob rule with a table), `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/docs/inline/schema-validation.md`, the `convert.rs` / `format.rs` / `validate.rs` / `schemas/mod.rs` module docs, `md schema about` descriptors in `about.rs`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/schema.md`
        - class sweep: a `file(match(...))` glob that discriminates between root-union arms is used to offer candidates but discarded when deciding which arm accepts the path; sites checked: Claudine `schema::supplied` arm selection (`supplied_file_arms`), `file_reference_target` / `classify_unresolved_file_reference` (late-verdict chooser), `pre_validate_with_origin` / `drop_invalid_optionals` / `build_schema_status_report_for_mode`, `schema_interactive` candidate walk (`MatchGlobs`, `provided_partial_candidates`), TAB completion candidates, `inline-compose` / `sequence` / `proxy` preparation, Darkmatter schema conversion (`convert::union_to_root_schema`, `resolve::resolve_root_union`, `resolve::resolve_standalone_root_union`), root-arm selection (`collect_applicable_root_schema_fragments` / `root_schema_arm_applies`), caller-file mode selection (`select_file_mode`), arm projection (`caller_file_fragments_shared_by`, `project_root_arm_caller_values`), coercion's root-union arm probe, final validator and root-union problem ranking (`collect_root_union_problems_with_anchors`), `md schema validate`, DMLS diagnostics; fixed here: all three conversion sites, the validator (keyword registration), root-union problem ranking and optional-wrapper message drill, `select_file_mode` (must ignore the glob), coercion's anchorless probe, Claudine `supplied_file_arms`, `file_reference_target`, and the `claudine-cli` glob comparison (now Darkmatter's `FileMatchGlobs`); clean: root-arm selection, arm projection, `md schema validate`, DMLS, status report, pre-validator, `drop_invalid_optionals`, `inline-compose` / `sequence` / `proxy` (all judge through the fixed validator and needed no code change), TAB completion and `provided_partial_candidates` (offer only; share the moved glob comparison)
        - results (macOS):
                - `darkmatter/`: `just test` 8650 passed, 12 skipped; `just lint` exit 0
                - `claudine/`: `just test` 7746 passed, 9 skipped; `just lint` exit 0; `just check-tier-coverage claudine` 0 stranded; `just test-l2 inline_prompt_scrollback` 14/14 (the chooser's glob comparison moved)
                - re-blessed `claudine/docs/providers/dispatch-inventory.json` with `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`: the only drift was 12 sites in `cli/src/commands/wrap/profile/tests/positional.rs`, merged from `main` and untouched here (the same 1624 vs 1636 failure recorded in iteration 1)
        - native Windows (`just cross-check … --os windows`, filtered): `darkmatter` 77/77 (every new Darkmatter test and the `root_union` suite), `darkmatter-cli` 42/42 (including the `md schema validate` table test), `claudine` 38/38 (`supplied`, `late_verdict`, `status_report`), `claudine-cli` 36/36 (`schema_completion`, `provided_partial`); the new Claudine CLI and PTY tests are `#[cfg(unix)]` by their files' stub convention
- work completed for 'Union arm selection ignores file match patterns' at 18:01:48

### Successful Completion

The implementation of review cycle 2 has completed successfully in 44 min. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no review finding was deferred
- a decision for the author to confirm: a `match` glob rules out an arm only when two or more root-union arms declare the same property with different globs, and only for a value that resolves to an existing file
        - enforcing every glob in every union arm would start rejecting paths in shipped prompts where the glob never tells arms apart (`prompts/review.md`, `prompts/implement.md`, and `plan` in `prompts/plan.md`)
        - a single schema keeps the rule that `match` only suggests files
- the files changed in this cycle are listed in the sub-bullets of the finding above; the new Darkmatter module is `darkmatter/lib/src/markdown/schemas/file_match.rs`, and `globset` was removed from `claudine-cli`, which now uses Darkmatter's `FileMatchGlobs`
