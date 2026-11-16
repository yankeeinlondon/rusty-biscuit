---
spec: 2026-11-14-output-flag-not-implemented
implemented_by: claude/opus-5.5 (rust-developer sub-agent)
packages:
    - claudine
    - claudine-cli
---

# Implementation log

## What changed

### The refusal (requirements 1 and 3)

- `cli/src/args.rs`: `refuse_output_option(&Commands) -> Option<String>` and
  `EXIT_USAGE = 2`. It reads the declared `-o`/`--output` of every wrapper
  command and of `compose`, `inline-compose`, and `sequence` (whose shared args
  also carry loop runs) and returns
  `the --output <target> CLI option has not been implemented yet in Claudine`.
- `cli/src/main.rs::dispatch` calls it immediately after `parse_cli_from`,
  before the launch directory, Darkmatter configuration, `ensure_config_exists`
  (and its setup wizard), tracing, and every command entry. On a match it writes
  the line to stderr with `log::message` (raw, no `Error:` decoration) and
  returns exit 2 through the normal `shutdown::finish` path, as `steer` does for
  its usage errors. No new error type was introduced, so no diagnostic code was
  needed.
- The option stays declared (`Option<String>`, `value_name = "TARGET"`,
  `hide = true`) in `WrapperArgs` and `SharedComposeArgs`, so the argv
  ownership partition still treats `-o`/`--output` as a Claudine value option
  and clap never says "unexpected argument". The hand-written wrapper help line
  was removed.

### Dead plumbing removed (requirement 4)

- Library: `composition::OutputFormat` (enum, `Display`, `FromStr`) and its
  re-export; `CompositionExecutionRequest.output` (the spec calls it
  `PrepareOptions.output`; the field lived on `CompositionExecutionRequest`);
  `CommandOutputPolicy.format`; `lifecycle::editor::set_structured_output` and
  its `STRUCTURED_OUTPUT` flag. `service_with` now always reports
  `EditorOutput::Terminal`.
- CLI: the wrapper `profile::OutputFormat` enum with its `Display`, `FromStr`,
  and `From<composition::OutputFormat>`; `WrapperProfile::apply_output_format`
  and `profile/apply.rs::apply_output_format`; the forwarding in
  `wrap/mod.rs` and `wrap/composition/pipeline.rs`; both
  `set_structured_output` calls (`wrap/mod.rs`, `compose/prep.rs`);
  `LaunchPlanInputs.output_format`, its replay block, and
  `LaunchWarningSource::OutputFormat`; the `args.output.is_none()` term of the
  wrapper's structured-mode gate (`wrapper_stages.rs`), which the refusal made
  always true; `output: shared.output` in `prep.rs`, `sequence/iterate.rs`,
  `sequence/task_run.rs`.
- Kept, per the spec: `claudine::provider::OutputFormat`,
  `OutputFormatSupport`, `OutputFormatSelector`, `apply_structured_stream`, and
  `flags::has_explicit_native_output_request`.
- Drifted comments fixed on the symbols touched: `provider/output_format.rs`
  (no longer described as mirroring the universal flag), `launch_plan.rs`
  module docs and `sandbox_requested` (sandbox only; "three stages"), the R8
  comment in `pipeline.rs` (`argv_after_output` renamed
  `argv_before_system_prompt`, since it marks where system-prompt argv begins),
  `editor.rs`.
- `cli/tests/l1/error_guards/transport-allow.toml`: the `formatted_report`
  entry for `run_provider_wrapper_inner` named the deleted
  `OutputFormat::from_str` transport; the guard reported it as matching no live
  site, so it was deleted.
- `docs/providers/dispatch-inventory.json` regenerated (see Gate results).

### Behavior without `--output` (requirement 5)

The only behavioral line touched on the no-`--output` path is the structured
gate in `wrapper_stages.rs`, where the removed term was `true` whenever
`--output` was absent. The composition pipeline's gate never read the option.
Provider-boundary handling (requirement 2) is untouched.

## Requirement-to-test mapping

| Requirement | Test |
|---|---|
| R1 wrapper, three spellings, four values, `--dry-run` | `lifecycle_edit_action::wrapper_output_is_refused::{short_spelling,long_spelling,attached_spelling,under_dry_run}`, `wrapper_output_is_refused_before_a_prompt` |
| R1 `compose` | `lifecycle_edit_action::compose_output_is_refused::{…}`, `compose_output_after_the_file_is_refused` |
| R1 `inline-compose` | `lifecycle_edit_action::inline_compose_output_is_refused::{…}` |
| R1 `sequence` | `lifecycle_edit_action::sequence_output_is_refused::{…}` |
| R1 loop runs | `lifecycle_edit_action::a_loop_run_output_is_refused` |
| R3 help | `argv_normalization::help_does_not_advertise_the_output_option` (`claude`, `compose`, `inline-compose`, `sequence`) |
| R4 / edit eligibility | `composition::lifecycle::editor::tests::the_output_is_always_the_terminal_and_stdin_is_never_reserved` (replaces `the_output_mode_follows_the_invocation_…`) |
| R2, R5 | existing suites, unchanged: `wrap::profile::tests::native_output`, `positional`, `launch_plan::tests`, the full L1 and L2 runs |

Each refusal test loops over `json`, `text`, `stream`, and `bogus` and asserts
exit 2, stderr equal to the exact message, empty stdout, an unchanged workspace
file listing (home, cwd, tools), no fake-editor launch, and no fake-provider
launch. The documents carry `initialize: [ensure_file, edit]`; the sequence
document also has a shell task. A mutation check (refusal disabled) made them
fail.

### Tests removed or rewritten

- Removed: `lifecycle_edit_action::structured_output_refuses_a_terminal_editor_and_no_error_continues`,
  `structured_output_runs_a_gui_editor_and_keeps_its_output_off_stdout` (GUI
  editors stay covered by `compose_runs_edit_in_order_…`,
  `a_parallel_member_runs_a_gui_editor`, and the other `code` rows; terminal
  refusal by `a_parallel_member_refuses_a_terminal_editor_before_creating_anything`).
- Removed: `wrap/profile/tests/apply_output_format.rs` (whole module);
  `launch_plan::tests::a_goose_to_gemini_retry_re_renders_the_requested_output_format`,
  `a_gemini_to_goose_retry_drops_the_gemini_output_encoding`,
  `a_provider_move_keeps_the_rebuilt_output_format_warning`, and the unused
  `adjacent_pair` helper; L2
  `level2_lifecycle_retry_drops_the_opening_providers_flag_encoding` (its only
  subject was Gemini's `--output-format` bytes) and the unused `has_pair`
  helper.
- Rewritten to `--sandbox` only:
  `launch_plan::tests::a_replay_keeps_the_invocation_fixed_material_in_order`;
  L2 `level2_lifecycle_retry_re_renders_output_and_sandbox_for_the_refreshed_provider`
  is now `level2_lifecycle_retry_re_renders_sandbox_for_the_refreshed_provider`.
- `wrap/profile/tests/positional.rs`: the pipeline simulation no longer has an
  output-format step.

## Documentation

- New `docs/topics/output.md` (stdout/stderr, richest stream, no-provider
  failures, exit statuses, the refusal, and the `md --output` comparison the
  author added to the spec).
- Flag lists: `docs/getting-started/index.md`, `docs/topics/execution-flow.md`,
  `docs/cli/sequence.md`, `docs/pipeline.md` (C4.7 removed; a new A6 step names
  the refusal, dispatch renumbered A7).
- `docs/topics/flow-control/lifecycle.md`: the `--output` row of the edit host
  table and the `--output json` sentence.
- Also drifted and fixed: `docs/topics/provider-metadata.md` and
  `docs/topics/building-an-agent-wrapper.md` listed `apply_output_format`.
- Skills: `.claude/skills/claudine/cli-reference.md`, `architecture.md`;
  user-scope `~/.claude/skills/use-claudine/cli.md` (switch row removed, a
  refusal note added) and `lifecycle.md` (the `--output json` editor sentence).
  The use-claudine `SKILL.md` description never listed `--output`.

## Departures and decisions

- No departure from the rulings.
- Round 1 made two decisions that the author reversed in review round 1 (see
  below): that a wrapper `--output` after the prompt still went to the
  provider, and that the refusal ran before a wrapper's `--help`.
- The spec's `PrepareOptions.output` is `CompositionExecutionRequest.output` in
  the code; that field was removed.

## Review round 1 (2026-11-14)

The author's rulings, review round 1, reversed both decisions above: argument
position never changes meaning, and help before `--` always wins.

### What changed

- **Wrapper option ownership.** New `cli/src/argv/wrapper_options.rs`:
  `hoist_wrapper_options` runs in `main.rs::run` after the composition
  partition and before clap. For a wrapper subcommand it moves every option
  the wrapper declares (space, `=`, attached, and short-cluster spellings),
  with its value, ahead of the first positional, keeping their written order;
  every other token keeps its order; nothing at or after the first `--` is
  read. A value option with no value before `--` is
  `missing value for '<option>'` (exit 1, as the old `--operation` check was).
- **Owned surface from clap.** `OwnedFlags::for_wrapper` (in
  `argv/partition.rs`, beside `for_composition`, sharing its classifier) reads
  `WrapperArgs` plus the root globals (`-v`, `--debug`, `--plain`); the
  non-global root `--version` is not included and stays with the provider.
- **Hand-written extractor removed.** `extract_wrapper_flags_from_passthrough`
  and `ExtractedWrapperFlags` (the `-y/-i/--edit/--repo/-q/--silent/-v/--perf/
  --operation` list and its OR-merge in `wrap/mod.rs`) are gone. What remains
  is `flags::consume_passthrough_separator`, which only removes the user's
  first `--` and reports where the tail starts
  (`find_passthrough_dash_boundary_with_raw` is unchanged).
- **Help precedence.** `argv::select_composition_help` is now
  `argv::select_help` and also recognizes wrapper subcommands
  (`HelpDestination::Wrapper`), answered by `main.rs::answer_selected_help`
  with `print_wrapper_help` before normalization, the `--output` refusal, and
  any configuration loading. The provider is looked up through the same
  `Commands` mapping dispatch uses (`main.rs::wrapper_provider`).
- `claudine compose -o json --help` exited 0 on this host before and after the
  change; the reported exit 1 did not reproduce. It is now covered by
  `help_before_the_separator_wins_in_every_position`.

### Tests

- Unit, `argv::wrapper_options::tests`: `the_wrapper_surface_is_the_clap_surface`
  (drift test: every value and boolean spelling listed and compared with
  `OwnedFlags::for_wrapper`), every value spelling after the prompt in space
  and attached form, every boolean spelling, short clusters, ordering,
  nothing read after `--`, missing value, `--version` left alone, non-wrapper
  argv unchanged.
- Unit, `wrap::flags::tests`: extractor tests replaced by
  `options_after_the_prompt_mean_what_they_mean_before_it`,
  `a_tail_after_the_separator_is_never_read_as_a_wrapper_option`, the
  separator tests, a proptest that wrapper options anywhere are Claudine's and
  other tokens keep their order, and
  `the_consumed_separator_reports_where_the_tail_starts`.
- `rule4_help_hoist` tests: `claudine claude compose --help` now selects the
  claude wrapper's help (it was "no help").
- L1 `lifecycle_edit_action`: the refusal matrix is now command
  (wrapper, `compose`, `inline-compose`, `sequence`) × position
  (`BeforeTarget`, `BetweenOptions`, `AfterTarget`) × spelling (short, long,
  attached, plus `--dry-run`), each over `json`/`text`/`stream`/`bogus`;
  plus `a_loop_run_output_is_refused` (before and after the file) and
  `an_interactive_wrapper_output_is_refused`. The earlier
  `wrapper_output_is_refused_before_a_prompt` and
  `compose_output_after_the_file_is_refused` are subsumed.
- L1 `argv_normalization`: `help_before_the_separator_wins_in_every_position`
  (first, middle, last; beside `-o json`, `--output=json`, an unknown option,
  a missing value; claude, compose, inline-compose, sequence),
  `help_after_the_separator_is_not_claudines`,
  `help_neither_loads_nor_converts_configuration` (a temp HOME whose legacy
  `config.json` cannot be converted: a run fails with `config.invalid`, help
  exits 0 and leaves the files unchanged).
- L1 new `wrapper_option_position` (Unix, recording stub provider):
  `a_value_option_after_the_prompt_is_applied_by_claudine` (`-m opus` after the
  prompt launches with `--model opus`, no `-m`),
  `dry_run_after_the_prompt_launches_nothing`,
  `options_after_the_separator_reach_the_provider_verbatim`
  (`--output json --help -m x` after `--`).

### Behavior a user could have relied on

These change for a token written after the prompt (and before any `--`) that
used to reach the provider:

- A provider-native flag that shares a Claudine spelling is now Claudine's.
  Checked against the catalog: Codex `-o <file>` (its `--output-last-message`
  alias) and Gemini `-o <format>` (its `--output-format` alias) are now
  refused; Codex `--sandbox <mode>` becomes Claudine's boolean `--sandbox`,
  leaving `<mode>` as a provider token; Gemini's boolean `--debug` becomes
  Claudine's value-taking `--debug`, so `claudine gemini hi --debug x` reads
  `x` as Claudine's tracing level (and `--debug` last is a missing-value
  error). Any other provider flag spelled like a wrapper option (`-m`, `-y`,
  `-i`, `-t`, `-q`, `-v`, `--model`, `--include`, `--use`, …) is likewise
  Claudine's after the prompt.
- `--help` after the prompt shows Claudine's wrapper help instead of the
  provider's.
- A repeated boolean across the prompt (`claudine claude -y hi -y`) used to be
  OR-merged; it is now clap's "cannot be used multiple times" error (exit 2),
  as it already was with both copies before the prompt.
- A value option at the end with no value (`claudine claude hi -m`) is now an
  error instead of being forwarded.

Each keeps its old meaning when written after `--`.

## Gate results

All run on macOS (2026-11-14) from `claudine/`:

- `just lint`: pass (clippy for all six claudine packages, including
  `claudine-contract`, the only other dependent of `claudine`; no other
  workspace package depends on it).
- `just test`: 12075 tests run, 12075 passed, 15 skipped. The first run failed
  one test, `dispatch_inventory::dispatch_inventory_matches_committed_file`,
  because the removed code and tests dropped 12 provider-dispatch sites;
  `docs/providers/dispatch-inventory.json` was regenerated with
  `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::` (sites
  1875 → 1863, line shifts) and the rerun passed. None of the added or changed
  tests is slow; the refusal tests run in 0.05–0.25 s each.
- `just test-l2`: 289 passed (plus 3 in the second pass), none failed.
- Windows and WSL2 were not run; nothing here is OS-specific beyond the
  existing fixtures.

### Review round 1 gate results (macOS, 2026-11-14)

- `just lint`: pass.
- `just test`: 12110 tests run, 12110 passed, 15 skipped. No new or changed
  test is slow (the help tests take 0.16–0.41 s, each refusal test about
  0.1 s).
- `just test-l2`: 289 + 3 passed.

## Review round 1, follow-up (2026-11-14)

The coordinator's check of a fresh binary found two defects in the round-1
change.

- **Attached short spellings on wrappers were not owned.** `-ojson`, `-mopus`,
  and clusters such as `-yojson` were hoisted as written, but the wrapper's
  lenient clap pass (`ignore_errors(true)`) drops an attached short value into
  the passthrough instead of setting the option, so `claudine claude -ojson hi`
  ran past the refusal in every position. The unit tests only checked the
  hoisted token order and parsed through a strict probe, and the L1 refusal
  matrix covered `-o v`, `--output v`, and `--output=v` but not `-ov`, which is
  why everything was green. Fix: `hoist_wrapper_options` now splits an owned
  short cluster or attached short value into separate tokens
  (`split_short_cluster`: `-yq` → `-y -q`, `-yojson` → `-y -o json`).
- **A missing option value exited 1.** `claudine claude hi -m` reported
  `missing value for '-m'` with exit 1. `main.rs::exit_code_before_runtime`
  now exits 2 (`args::EXIT_USAGE`) for `MissingWrapperOptionValue` and for the
  composition partition's `PartitionError` (a switch or `--` before the file,
  `--fail-fast`, non-UTF-8 after the file), matching clap's own usage errors
  (`claudine compose x.md -m` already exited 2). The message text is unchanged.

Tests added or changed:

- Unit: attached short values and clusters expect the split form, before and
  after the prompt; `a_short_cluster_of_owned_options_is_split_into_its_options`.
- L1 `lifecycle_edit_action`: a `short_attached_spelling` row (`-o<value>`) in
  every command and position.
- L1 `argv_normalization`:
  `the_output_refusal_precedes_configuration_in_every_spelling_and_position`
  (a temp HOME whose configuration cannot be converted; claude, compose,
  inline-compose, sequence; `-o json`, `-ojson`, `--output json`,
  `--output=json`, `-yojson`, `-yo json`; before, between, after; all exit 2
  with the exact refusal) and `an_option_missing_its_value_is_a_usage_error`
  (wrapper and composition, exit 2, no `config.invalid`).
- L1 `wrapper_option_position::a_value_option_after_the_prompt_is_applied_by_claudine`
  now covers `-m opus`, `-mopus`, `--model=opus`, and `-ymopus`.

Behavior change to note: composition partition errors (a provider switch or
`--` before the file, and the others above) now exit 2 instead of 1.


### Follow-up gate results (macOS, 2026-11-14)

The full `just test` after the follow-up found one stale assertion:
`sequence_errors_cli::the_removed_ledger_option_is_an_unknown_option` expected
exit 1 for an unknown `sequence` option, which is now a usage error (exit 2).
The assertion was updated. Because that was the only change after the full
run (test-only, one line), it was verified with a targeted run rather than a
second full run:

- `cargo nextest run -p claudine-cli` over that test, `wrapper_options::`,
  and the L1 `wrapper_option_position::`, `argv_normalization::`, and
  `--output` refusal tests: 64 passed.
- `cargo clippy -p claudine-cli --all-targets -- -D warnings`: clean.
- Built binary: `claudine claude -ojson hi`, `claude hi -ojson`, and
  `claude --dry-run hi -ojson` exit 2 with the refusal; `claude hi -m`,
  `claude -m`, `compose x.md -m`, and `sequence x.md --model` exit 2;
  `claude hi -- -o json --help` passes both tokens to the provider.

## Help precedence follow-up (2026-11-16)

Review found that the early help selector still yielded to a root `-V` or
`--version` written before help. The author requested removing that exception.
The selector now answers help in either order, so
`claudine --version compose --help` shows composition help and exits 0.
Updated the selector's existing version cases, the CLI help-precedence matrix
(both version spellings across wrappers and composition commands), and both
pre-parsing topic pages.

Focused macOS validation, run sequentially to limit load while another agent
was working:

- `just test-cli selection_prioritizes_help_over_a_root_version_request`:
  1 passed.
- `CARGO_BUILD_JOBS=2 just test-cli help_before_the_separator_wins_in_every_position --test-threads 2`:
  1 passed.
- `git diff --check`: clean.
