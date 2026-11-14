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
- Decided here: a wrapper `--output` written after the prompt was never parsed
  by Claudine (wrapper value options are read only before the first
  positional); it still reaches the provider, as before. Only a declared
  Claudine `--output` is refused.
- Decided here: the refusal runs before `--help` handling of wrappers, so
  `claudine goose -o json --help` is refused rather than showing help.
- The spec's `PrepareOptions.output` is `CompositionExecutionRequest.output` in
  the code; that field was removed.

## Gate results

GATES_PLACEHOLDER
