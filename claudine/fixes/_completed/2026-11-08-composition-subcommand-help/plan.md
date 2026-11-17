---
total_phases: 3
created: 2026-11-10
phase: 4
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1:
- claudine/cli/src/main.rs
- claudine/cli/src/argv/mod.rs
- claudine/cli/src/argv/rule4_help_hoist.rs
- claudine/cli/src/commands/compose/mod.rs
- claudine/cli/src/commands/compose/prep/tests.rs
- claudine/cli/src/commands/sequence.rs
- claudine/cli/tests/l1/argv_normalization.rs
- claudine/cli/tests/l1/provider_tail_ownership.rs
- claudine/cli/tests/l1/exit_site_guard.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- claudine/cli/src/argv/rule1_provider_bool.rs
- claudine/cli/src/argv/rule4_help_hoist.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
- claudine/cli/src/completion/mod.rs
- claudine/cli/src/main.rs
- claudine/cli/tests/l1/argv_normalization.rs
- claudine/cli/tests/l1/completion_ownership.rs
- claudine/cli/tests/l1/sequence_errors_cli.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
- claudine-cli
source_files_during_phase_3: []
docs_updated_during_phase_3:
- claudine/docs/cli/index.md
- claudine/cli/README.md
- claudine/docs/topics/cli-pre-parsing.md
- claudine/docs/topics/argv-normalization.md
docs_created_during_phase_3:
- claudine/fixes/2026-11-08-composition-subcommand-help/implementation.md
skills_files_updated_during_phase_3:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/cli-commands.md
source_code:
- claudine/cli/src/main.rs
- claudine/cli/src/argv/mod.rs
- claudine/cli/src/argv/rule1_provider_bool.rs
- claudine/cli/src/argv/rule4_help_hoist.rs
- claudine/cli/src/commands/compose/mod.rs
- claudine/cli/src/commands/compose/prep/tests.rs
- claudine/cli/src/commands/sequence.rs
- claudine/cli/src/completion/mod.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
- claudine/cli/tests/l1/argv_normalization.rs
- claudine/cli/tests/l1/completion_ownership.rs
- claudine/cli/tests/l1/exit_site_guard.rs
- claudine/cli/tests/l1/provider_tail_ownership.rs
- claudine/cli/tests/l1/sequence_errors_cli.rs
documentation:
- claudine/docs/cli/index.md
- claudine/cli/README.md
- claudine/docs/topics/cli-pre-parsing.md
- claudine/docs/topics/argv-normalization.md
- claudine/fixes/2026-11-08-composition-subcommand-help/implementation.md
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/cli-commands.md
completed_phase: 3
implemented: true
---

# Composition subcommand help implementation plan

## Summary and Definition of Done

Give `compose`, `inline-compose`, and `sequence` their own complete, compact clap help screens. Select help from original argv before normalization and the composition ownership partition, then render the existing command model without parsing a composition request. Complete the shared option descriptions and provider shortcut declarations, preserve provider forwarding and completion ownership, and update the current documentation.

Completion means:

- Each composition command accepts both `-h` and `--help`, with or without a file, and prints its own usage, positional explanation, inherited presentation options, help option, and every public Claudine option. Help exits 0, writes nonempty stdout, and leaves stderr empty even beside invalid arguments, `--silent`, `--quiet`, or `--perf`.
- Root help before the command and bare invocation retain the grouped overview. A help token after the first literal `--` reaches the provider. Wrapper, administrative, version, and completion routing retain their current contracts.
- Help works with an empty isolated home, nonexistent file, and no provider executable. It reads no composition input, initializes no configuration, performs no lifecycle action, and launches no provider. `--plain` produces no ANSI escapes.
- All ten catalog-derived shortcuts are visible. The existing custom completion candidate set and provider-owned tails are preserved; the legacy clap completion model gains the three declarations it currently lacks.
- Focused L1 checks, `just test-cli`, and `just lint` pass from `claudine/`. Current docs, skill guidance, and touched comments describe the implemented behavior. No new dependency, CI cell, terminal test, network test, or performance measurement is introduced.

The phases run consecutively without a human gate. Waves identify concurrent tasks within a phase and have unique indices across this plan. Every delegated session is non-interactive, stays in this worktree, and receives a bounded file ownership assignment. Do not commit, run a formatting mutation, move the fix to `_completed`, or run `just complete`.

## Phase 1: Implement help selection and the command reference

### Necessary Rules

1. **Select on original argv.** Add `HelpDestination::{Root, Composition(&'static str)}` and `select_composition_help(raw: &[OsString], completion_active: bool) -> Option<HelpDestination>` in `cli/src/argv/rule4_help_hoist.rs`, re-exported by `argv/mod.rs`. Call `find_subcommand(raw, COMPOSITION_SUBCOMMANDS)` and `first_dash_dash_index`, rather than searching anywhere for command names. Select the first exact standalone `-h`/`--help` before the boundary: before the recognized command means `Root`; after it means that composition command. No matching command or no help token means `None`. An empty token list is `None`. Evidence: `argv::find_subcommand` already skips root flags and the value of `--debug`, stops at the first command-shaped token, and honors `--`; the specification requires preserving that recognition.

2. **Keep help token-based and validation-free.** Scan exact tokens without attempting to validate or consume composition option values. Consequently `compose --provider --help`, unknown providers, malformed `--set`, conflicting flags, missing files, unknown flags before the file, and repeated help tokens all select one screen. `name=--help`, `--help=…`, `-hh`, and JSON containing `--help` do not match. The first help token wins, including root help followed by command help. Opaque non-UTF-8 tokens remain untouched by selection. Evidence: the specification's selection table and standalone-token rule; `partition_composition_tail` can fail before `parse_cli_from`, so catching clap errors is too late.

3. **Preserve completion and version precedence.** `completion_active` makes selection return `None`. Keep `completion::maybe_complete()` first in `run`; `__complete` remains excluded naturally by first-command recognition, even when its trailing payload contains composition commands and help. An exact root `-V`/`--version` before the composition command leaves selection to the existing parser instead of taking the early help branch. Keep existing version declarations and routing untouched. Evidence: `completion/mod.rs::maybe_complete`, `argv::normalize_inner`'s `COMPLETE` and `__complete` bypasses, `args.rs::Cli`'s clap version declaration, and the specification's unchanged-version requirement.

4. **Exit before composition bootstrap.** In `main.rs::run`, select immediately after collecting `raw_argv`, before `perf::scan_perf_bootstrap`, `argv::normalize`, `partition_composition_tail`, request initialization, tracing, or the ordinary execution runtime. For a selected destination, render, then call `shutdown::exit_before_runtime(0)`; propagate rendering I/O errors through the existing `Result` path. Do not introduce direct process exits or fabricate a `Cli`, required file, or composition request. For `Root`, use the existing `commands::help::run`; for `Composition`, use the clap renderer described below. Evidence: `run` currently partitions before creating its execution runtime; `dispatch` initializes request state and tracing before its help branch; `shutdown::exit_before_runtime` is the existing flush-and-exit API and `exit_site_guard` forbids new direct exits. No deliveries can have started on this new branch.

5. **Share parsing's command construction, keep completion's tree separate.** Extract `build_cli_command(is_wrapper: bool) -> clap::Command` in `main.rs`. It performs the existing `Cli::command`, `inject_subcommand_help_flag`, and wrapper `ignore_errors(true)` adjustments, then calls `Command::build()` on the root. `parse_cli_from` uses it with its existing wrapper detection; composition help uses it with `false`. Add `print_composition_help(command_name: &str, plain: bool) -> std::io::Result<()>` in the same file; locate the selected child on the built root and call its standard compact `print_help`, followed by a newline. Use an internal invariant failure for an absent child, since selection comes only from `COMPOSITION_SUBCOMMANDS`. Keep `root_help_request` for existing parse-fallback callers. Do not move command construction into the completion engine or change `completion::completion_command`. Evidence: `parse_cli_from` owns the injected non-wrapper help model, whereas `completion_command` deliberately derives a tree without that injection. The locked clap_builder 4.6.0 `Command::build` recursively builds children, propagates globals, and computes parent-qualified binary names; building before child extraction supplies `Usage: claudine <command> …` and inherited arguments.

6. **Use clap's presentation contract.** Derive `plain` from exact `--plain` tokens before the first literal `--` on a selected help request, whether before or after the command. Set `log::set_plain(plain)` for grouped help and set the selected clap command's color policy to `ColorChoice::Never` for plain command help. Both help spellings use `print_help`, not `print_long_help`. Print directly to stdout through clap; do not pass help through quiet/silent logging filters or enable telemetry/perf. Evidence: the existing normal path prescans exact `--plain` tokens, `log::set_plain` controls root rendering, and the spec explicitly chooses conventional clap help. Root help continues using its existing `Prose`/`TerminalRenderable` implementation in `commands/help.rs`; no new custom terminal component is needed.

7. **Retain the existing rewrite and parser contracts.** Leave rule 4's hoisting algorithm and normalization tests intact. The executable's newly selected help requests exit before it; normalization remains a reusable syntactic API used by completion ownership. Update its comments to distinguish the retained rewrite from the executable's earlier help destination selection. Preserve `Vec<String>` greedy positionals, their `num_args = 1..`, required status, and `ARG` value name in `ComposeArgs`, `InlineComposeArgs`, and `SequenceArgs`. Do not change partition or provider-tail execution. Evidence: `argv::normalize_for_completion` and `completion/engine/ownership.rs` reuse normalization, and the spec expressly preserves the collector and the authored `--` boundary.

8. **Describe the actual positional grammar.** Set the three positional help descriptions to explain exactly one required file reference, optional `key=value` setters, and provider arguments following the file; inline setters override `--set`. Add command-specific `after_help` examples on the three Args types: `claudine compose prompt.md name=Ken -- --help`, `claudine inline-compose prompt.md name=Ken -- --help`, and `claudine sequence sequence.md name=Ken -- --help`. Explain that `--` explicitly forwards subsequent tokens and that provider-native switches are not listed. Keep the generated option inventory as the sole option reference. Evidence: `parse_composition_positionals`, `SharedComposeArgs::caller_arguments`, and the three current Args declarations collect these concepts together rather than defining separate clap positionals.

9. **Declare existing shortcuts without changing selection.** Add `kilo`, `pi`, and `antigravity` Boolean fields to `SharedComposeArgs`, each with `#[arg(long, group = "compose_provider")]` and a concise provider description. Initialize all three to `false` in the explicit literals in `commands/compose/prep/tests.rs::shared_args` and `commands/sequence.rs::tests::shared_args`. `commands/compose/tests.rs::shared_from` and `commands/wrap/sequence/jit/tests.rs::shared_args` construct via parsing and need no literal fields. Leave `explicit_provider` catalog normalization unchanged. Evidence: `rule1_provider_bool::provider_for_boolean_flag` derives accepted switches from `PROVIDERS_DISPLAY_ORDER` and each provider's first `cli_aliases` entry; only seven shortcut fields currently exist. Repository-wide search found only those two explicit initializer bodies.

10. **Expose concise, accurate user text.** Review every non-skipped field of `SharedComposeArgs`, the three positional docs, and rendered enum descriptions. Remove the YOLO migration/history paragraphs; retain `CLAUDINE_YOLO` as the supported environment setting through clap's existing declaration. Put the composition-shell warning in the first paragraph of `dry_run`'s help and the real shell-task execution/`--dry-run` requirement in the first paragraph of `dry_run_shell`'s help, so compact help displays them. Keep `--asp`, `--rsp`, `--op`, provider possible values from `provider_value_parser`, and `pause`, `abort`, `continue` unchanged. Keep skipped bookkeeping fields hidden and preserve argument IDs, defaults, conflicts, and requirements. Evidence: these are existing clap attributes in `SharedComposeArgs`; `OnRateLimitArg` is already a `ValueEnum`. This fix documents existing semantics and changes no loader or option validator.

11. **Limit completion changes to existing shortcuts.** Preserve `completion::completion_command` construction and custom engine logic. Extend their tests; correct the now-stale comment on `engine/mod.rs::run_composition_provider_flag` claiming the three providers lack clap fields. The custom engine already emits the three flags, so its set must remain identical and contain no duplicates; the legacy tree newly exposes those declarations. Evidence: `completion/engine/tests.rs::composition_provider_flag_completer_is_catalog_derived` already checks all three; the engine merges catalog tokens with clap fallback and de-duplicates them.

12. **Use portable existing fixtures.** Keep new subprocess help tests in `tests/l1/argv_normalization.rs`, with `CliProcessFixture`, and feature-gate only tests that copy compiled fixture binaries. For the no-effects trap, copy `claudine-fake-goose` to `goose{EXE_SUFFIX}`, set `PROVIDER_MARKER`, and use `initialize: [{ append_line: [help-lifecycle.log, ran] }]` in the document. For exact forwarding, copy `claudine-fake-completion` to `claude{EXE_SUFFIX}`, set `FAKE_COMPLETION_PROVIDER=claude` and `FAKE_COMPLETION_DIR` to a fixture-owned record directory; inspect its `argv.json`. Use `bin_exe!`, no shell scripts or real providers. Evidence: `launch_trigger_schemas.rs::marker_fixture` demonstrates the portable Goose pattern; `tests/bin/fake_completion/main.rs` records exact argv and emits a successful Claude stream, while `common::launch_recorder` and `provider_tail_launch.rs` are Unix-only. No new fixture capability or platform-specific path logic is needed.

13. **No spike or configuration matrix.** All questions here are answered by the specification and the caller/type trace. Use L1 behavioral tests, not a runtime spike. The Input Robustness Matrix does not apply: this work selects argv and renders help without adding or changing file-format readers, configuration loaders, or deserializers. Malformed setter text is used only to prove help bypasses existing validation. Evidence: the planned production edits are restricted to argv selection, command construction, clap declarations, and rendered descriptions.

### Wave 1

- [x] **Route help**
  - Own `cli/src/main.rs`, `cli/src/argv/mod.rs`, and `cli/src/argv/rule4_help_hoist.rs`. Implement Rules 1–7, including the pure selector, shared command builder, compact renderer, early branch, plain mode, and shutdown path.
  - Update the touched module/function comments in the same edit. Retain wrapper help injection exclusions and `root_help_request` for unaffected parsing paths.
  - Risk: selecting after partition would retain the defect; extracting an unbuilt child would omit inherited globals or parent-qualified usage. The order and root `build()` call above are mandatory.

- [x] **Complete reference**
  - Own `cli/src/commands/compose/mod.rs`, `cli/src/commands/sequence.rs`, and `cli/src/commands/compose/prep/tests.rs`. Implement Rules 8–10, preserving the actual positional parser and all existing validation attributes.
  - Add the three shortcut fields and update the two explicit struct literals together so test targets compile.
  - This task runs concurrently with Route help; it does not edit `main.rs` or argv modules.

### Validation checkpoint

- [x] **Check command model**
  - After Wave 1 joins, add an in-process test beside the new main helpers that builds the shared model and renders each selected command. Assert parent-qualified usage, `--help`, `--plain`, `--verbose`, and `--debug`, and that the model builds without duplicate-argument/group failures.
  - Name the new test module `composition_help` and run `just test-cli composition_help`; verify nextest reports a nonzero test count.
  - Confirm normal parsing still uses the shared factory and the completion factory remains unchanged. Continue directly to Phase 2.

## Phase 2: Prove selection, isolation, and forwarding

### Wave 2

- [x] **Cover selection**
  - Own unit tests in `argv/rule4_help_hoist.rs` and the `main.rs::composition_help` test module. Table-test all specification destinations, all three commands, root options with values (`--debug trace`, `--debug=trace`), and first-command recognition (`mcp compose --help` must not select composition help).
  - Add empty/bare argv, repeated mixed help spellings, embedded setter/JSON text, `--help=…`, `-hh`, help after `--`, help on both sides of `--`, and opaque-token pass-through cases. Test completion with the injected Boolean rather than mutating `COMPLETE` in unit tests. Cover root version precedence without changing ordinary version parsing.
  - Render the model in process for option inventory checks. For each composition child, compare visible shortcut declarations against every provider's first catalog alias. Assert every visible option has nonempty help, skipped field IDs are absent, visible aliases remain, provider values remain canonical, and rate-limit values are exactly `pause`, `abort`, `continue`.
  - Verify both compact spellings share the same renderer and user text contains no retired YOLO discussion or internal Rust paths. Preserve the existing hoist tests as syntactic API coverage.
  - Add no-help controls through `partition_composition_tail` and `build_cli_command(false).try_get_matches_from`: an unknown provider, missing provider value, `--dry-run-shell` alone, and conflicting interactive switches produce clap errors; an unknown switch before the file produces the existing partition error. Assert malformed setter JSON still fails through `commands::compose::merge_set_overrides(Some("{broken"), Default::default(), Vec::new())`. These controls never call the exit-on-error `parse_cli_from`, initialize config, or execute a command.

- [x] **Prove CLI wiring**
  - Own `cli/tests/l1/argv_normalization.rs` and `cli/tests/l1/sequence_errors_cli.rs`. Keep these modules in the existing consolidated L1 target; create no new target or module registration.
  - Replace the grouped-heading assertions in `headline_compose_with_interleaved_flag_renders_help` and `headline_compose_with_trailing_help_renders_help` with compose-specific usage and option assertions. Preserve the original interleaved `--gemini name=Ken --help` input and checks excluding the misleading clap tip, unexpected-help error, and second-file-reference error. Replace history-heavy/drifted comments with the current regression contract.
  - Use four independently scheduled cases per command: `-h` without a file, `--help` without a file, `-h` with a nonexistent file, and `--help` with a nonexistent file. Each checks exit 0, empty stderr, nonempty stdout, matching `Usage: claudine <command>`, positional/forwarding explanations, and `--help`, `--provider`, `--set`, `--dry-run`, `--dry-run-shell`, `--max-iterations`, `--on-rate-limit`. Exclude root group headings `Shared Resources:` and `Composition:`. Use a small assertion helper, not twelve duplicated bodies or one long test loop.
  - Add one subprocess case per validation bypass: unknown provider, missing provider value, malformed setter JSON, `--dry-run-shell` without `--dry-run`, conflicting interactive switches, and unknown switch before the file. Each includes a recognized help token and checks success/stdout/empty stderr. Cover selection owns the corresponding no-help in-process controls, so this task adds no duplicated subprocess failure matrix.
  - Add explicit root-help-before-command coverage with invalid composition arguments; retain bare/root help, representative `mcp`/`actions` help, wrapper help, and root version regression coverage. All help channels are asserted separately, never only as combined output.
  - Add plain-mode cases before and after the command, with forced color enabled and inherited `NO_COLOR` removed only on that child; assert `stdout == common::strip_ansi(stdout)`. Include `--quiet`, `--silent`, and `--perf` help cases proving no suppression or stderr report.
  - Add a feature-gated portable trap case with the Goose fixture from Rule 12, no seeded config, and a real document containing the portable lifecycle action. Assert no provider marker, no lifecycle marker, unchanged input bytes, absent `home/.claudine/config.json`, and no audio publication. Give `inline-compose` this trap case because it would mutate its input during ordinary execution. Keep the fixture's audio and rendezvous defaults. The nonexistent-file help cases separately prove no provider installation is required; use `fake_only_path()` there with the required absence-proof comment.
  - Add a feature-gated portable `compose prompt.md --claude -- --help` execution case using the compiled Claude stream fixture from Rule 12 and a seeded user config. Assert the recorded provider argv contains exactly one `--help` and the authored separator is consumed. Assert this output is provider output rather than a composition help screen. This invocation deliberately executes; do not use dry run as a forwarding proof.
  - Strengthen `removed_fail_fast_is_absent_from_sequence_help`: first require sequence-specific usage and shared public options, then assert `--fail-fast` is absent and stderr empty.

- [x] **Preserve completions**
  - Own `cli/src/completion/mod.rs`, `cli/src/completion/engine/mod.rs`, `cli/src/completion/engine/tests.rs`, and `cli/tests/l1/completion_ownership.rs`. Keep production completion algorithms unchanged; update only the drifted comment and tests.
  - Check `completion_command` exposes all catalog-derived shortcuts for every composition child, including the three newly declared flags, without adding injected help candidates. Extend the existing custom engine catalog test to assert no duplicate provider tokens.
  - Retain representative admin/wrapper completion and provider-owned-tail cases. Add a generated `__complete` invocation whose payload contains `compose` and `--help`; assert candidate protocol output rather than a usage screen. Extend the existing `COMPLETE=bash` bootstrap proof to carry composition help tokens and assert registration output with empty stderr, no help usage, and no config creation.
  - Avoid touching `argv_normalization.rs`, which belongs to Prove CLI wiring; put the added legacy-bootstrap subprocess case in `completion_ownership.rs` using its shared fixture conventions.

### Validation checkpoint

- [x] **Run focused checks**
  - After all Wave 2 tasks join, run the affected filters through `just test-cli`: `composition_help`, `rule4_help_hoist`, `argv_normalization`, `sequence_errors_cli`, and `completion`. Inspect nextest counts to establish that new cases actually ran.
  - Confirm new subprocess tests use the area builder, fixture paths, and compiled binaries, contain no Unix-only shell dependency, and each remains within the L1 runtime budget. Resolve failures before continuing; do not add sleeps, retries, or timeout increases.

## Phase 3: Document and validate the completed behavior

### Wave 3

- [x] **Update current docs**
  - Own `claudine/docs/cli/index.md`, `claudine/cli/README.md`, `claudine/docs/topics/argv-normalization.md`, and `claudine/docs/topics/cli-pre-parsing.md`.
  - Replace the CLI guide's planned marker with the completed behavior. Include root overview, command reference, help without an existing file, plain mode, and explicit provider forwarding examples. Keep the README help change to a compact overview with a link to the CLI guide; add no option inventory or implementation narrative there.
  - Correct both topic pages' rule 4 explanations and Mermaid pipeline diagrams: completion intercept → original-argv help selection → help renderer/exit, or normal normalization → partition → clap → execution. Describe retained hoisting as a normalization contract rather than the command-help destination mechanism.
  - Explain why a trailing command-help request bypasses validation while a token after `--` belongs to the provider. Examples must show actual command-specific help, not an internal hoisted root invocation as the displayed outcome. No docs link to or name this fix directory.

- [x] **Update skill guidance**
  - Own `.claude/skills/claudine/SKILL.md`, `.claude/skills/claudine/cli-commands.md`, `.claude/skills/claudine/topics/cli-pre-parsing.md`, and `.claude/skills/claudine/topics/argv-normalization.md`.
  - Add a concise pointer in the main skill to early help selection and the shared built command model; correct the `cli-commands.md` argv row. Bring the two skill topic copies into agreement with the corresponding current docs without copying unrelated changes.
  - Remove claims that composition help always reaches grouped root help or that accepted provider shortcuts are missing from the clap reference. Preserve the distinction between custom wrapper help and generated non-wrapper help.
  - This task runs concurrently with Update current docs. Both follow the already settled Rules 1–13, and final validation compares their descriptions after the wave joins.

### Final validation

- [x] **Verify final behavior**
  - From `claudine/`, run `just test-cli` once over the integrated change, then `just lint`. These are the specification's canonical gates; nextest runs L1, and lint may check formatting but must not mutate it. Fix affected source manually, with no `cargo fmt` or `just lint-fix`.
  - Review the diff for unrelated changes, all touched rendered comments, docs/skill agreement, and exact shortcut coverage. Confirm no dependency manifest, configuration parser, CI schedule, wrapper-help layout, or provider argument-ownership algorithm changed.
  - Record the commands, results, covered behavior, and any deviation from the specification in `claudine/fixes/2026-11-08-composition-subcommand-help/implementation.md`. Include one captured compact help output per composition command for the final review. Do not modify the finalized specification to match implementation.
  - Mark completed task boxes and report implementation complete, ready for review. All phases finish without a human checkpoint; review follows this terminal state. Report host-specific evidence accurately and make no claim that a local macOS run proved other environments.