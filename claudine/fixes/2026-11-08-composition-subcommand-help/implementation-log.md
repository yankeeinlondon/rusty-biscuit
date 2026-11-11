---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/claudine/fixes/2026-11-08-composition-subcommand-help/spec.md"
plan: "claudine/fixes/2026-11-08-composition-subcommand-help/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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
decisions_for_review:
    - |-
        The plan said the shared command factory `build_cli_command` should call clap's
        `Command::build()` itself. I made it return the command unbuilt and have only the
        help renderer build it. Reason: the existing root-help fallback used by ordinary
        parsing adds clap's `ignore_errors` setting afterwards, and clap copies that setting
        to subcommands only while building; building first would silently stop
        `claudine --help mcp alias` (root help with a missing required argument) from
        working. Alternative: build inside the factory and rebuild a fresh model inside the
        fallback, which costs a second model construction on that path. Redoing it means
        moving one `build()` call and adjusting the fallback.
    - |-
        Version precedence: the selector returns "no help" when an exact root `-V` or
        `--version` appears before the composition command and before any help token, so
        `claudine --version compose -h` still prints the version. A help token that comes
        first still wins (`claudine --help --version compose` shows root help). The plan
        did not settle the mixed order; the alternative (version always wins when present
        before the command) would change one branch in `select_composition_help`.
    - |-
        Phase 2 owns the wider test matrix, but three existing tests pinned the old
        behavior and would have failed this phase's gate, so I updated them now: the two
        `argv_normalization.rs` headline tests (now assert the compose usage screen, empty
        stderr, and no root group heading), `provider_tail_ownership.rs`
        `help_opens_no_composition_file` (help before the command expects root help, after
        it the command's own usage), and an allowlist entry in `exit_site_guard.rs` for the
        new early exit. I also added a first set of selector unit tests and the
        `main.rs::composition_help` module. Phase 2 extends rather than replaces these.
    - |-
        The help footer is produced by a small `composition_help_footer!` macro in
        `commands/compose/mod.rs` (clap's `after_help` needs a constant string, and the
        three commands differ only in name and example file). The alternative is three
        literal strings, which would drift independently.
    - |-
        Phase 2 tests place `--perf` after the composition command when proving that help
        ignores it. `--perf` is declared on the composition commands, not on the root, so
        `claudine --perf compose -h` fails as a root-level argument error (exit 2). This was
        already the case before this fix, and the plan did not say where the switch goes. The
        alternative is to make the help selector skip `--perf` before the command (or declare
        it on the root). That widens the change into argument ownership, which the
        specification keeps unchanged. Redoing it means one selector rule plus one test.
    - |-
        Phase 3: the plan asked for the two skill topic pages
        (`.claude/skills/claudine/topics/cli-pre-parsing.md` and `argv-normalization.md`)
        to be brought into agreement with the docs as separate copies. The skill's
        `topics/` directory is a symbolic link to `claudine/docs/topics/`, so they are the
        same files; I edited the docs once and did not create skill-local copies (the
        skill itself forbids them). Nothing needs redoing unless the reviewer wants the
        link replaced by real copies, which the skill advises against.
---

# Implementation Log for 2026-11-08-composition-subcommand-help (3 phases)

## Phase 1

### Route help (Rules 1–7)

- `cli/src/argv/rule4_help_hoist.rs`: added `HelpDestination::{Root, Composition(&'static str)}` and
  `select_composition_help(raw, completion_active)`. It reuses `find_subcommand` (root flags and
  the `--debug` value skipped, first command-shaped token decides, `--` honored), then takes the
  first exact standalone `-h`/`--help` before the first `--`: before the command → `Root`, after →
  `Composition(name)`. An exact root `-V`/`--version` seen before any help token and before the
  command returns `None` so version routing is unchanged. Completion → `None`. Module and
  `hoist_composition_help` docs now describe the hoist as the retained normalization contract.
- `cli/src/argv/mod.rs`: re-exports; Rule 4 module/inline comments corrected.
- `cli/src/main.rs`: `run` selects help immediately after collecting raw argv (before perf
  bootstrap, normalization, partition, request init, tracing, runtime), renders, then
  `shutdown::exit_before_runtime(0)`. `Root` → `log::set_plain` + `commands::help::run`;
  `Composition` → `print_composition_help` (clap `print_help`, `ColorChoice::Never` under
  `--plain`, followed by a newline). Extracted `build_cli_command(is_wrapper)`, now used by
  `parse_cli_from`; added `composition_help_command` (builds the root, extracts the child).
  `root_help_request` doc updated (composition help never reaches it).

### Complete reference (Rules 8–10)

- `SharedComposeArgs`: added `--kilo`, `--pi`, `--antigravity` (group `compose_provider`);
  YOLO doc reduced to one user sentence (env `CLAUDINE_YOLO` still shown by clap);
  `--dry-run` first paragraph now carries the shell-expansion warning; `--dry-run-shell` first
  paragraph states real execution and the `--dry-run` requirement.
- Positional `ARG` docs on the three Args types explain one file reference, setters overriding
  `--set`, and provider arguments. A `composition_help_footer!` macro supplies each command's
  `after_help` example (`claudine <cmd> <file> name=Ken -- --help`) and the forwarding note.
- Explicit struct literals in `commands/compose/prep/tests.rs` and `commands/sequence.rs` tests
  gained the three fields.

### Check command model

- `main.rs::composition_help` module: `shared_model_builds_without_definition_failures`
  (`debug_assert` on wrapper and non-wrapper models) and
  `each_command_renders_its_own_parent_qualified_screen` (usage `claudine <cmd> [OPTIONS] <ARG>...`,
  `--help`, `--plain`, `--verbose`, `--debug`, `--provider`, `--set`, example, no root heading).
  `just test-cli composition_help` → 2 run, 2 passed.
- Added selector unit tests in `rule4_help_hoist.rs` (destination table, invalid arguments vs
  embedded text, completion/version/other-command deferral); Phase 2 extends these.
- Updated the two `argv_normalization.rs` headline tests that asserted the grouped `Composition`
  heading (they would fail now): they assert the compose usage screen, empty stderr, and absence
  of the root heading; regression comments rewritten. Phase 2 still owns the wider CLI matrix.

### Manual evidence (macOS, debug build, empty nonexistent HOME)

`sequence --dry-run-shell --help`, `compose --provider --help`, `compose --set {broken -h`,
`compose --silent --perf -h`, `sequence --help -- --help` → exit 0, stderr empty, command usage;
no home directory created. `--plain compose -h` with `CLICOLOR_FORCE=1` → 0 escape sequences (44
without `--plain`). `--plain --help compose x.md` → root overview, 0 escapes. `--version compose -h`
→ version. `mcp compose --help` → existing clap error (unchanged).

### Further test updates found by the gate

- `exit_site_guard::direct_exits_occur_only_at_allowlisted_sites` flagged the new
  `exit_before_runtime(0)` in `run`; added a reasoned `AllowedExit` entry.
- `provider_tail_ownership::help_opens_no_composition_file` asserted root help for every form;
  it now expects root help when the help token precedes the command and the command's own
  `Usage: claudine <cmd> ` otherwise. The `--`-forwarding and no-launch assertions are unchanged.

### Requirement → test mapping (Phase 1 scope)

| Behavior | Test |
| --- | --- |
| Destination table, first-token rule, `--` boundary | `rule4_help_hoist::tests::selection_follows_the_destination_table` |
| Invalid args don't block help; embedded text isn't help | `…::selection_ignores_invalid_arguments_but_not_embedded_text` |
| Completion, version, other-command, `__complete`, `--debug <v>` | `…::selection_defers_to_completion_version_and_other_commands` |
| Shared model builds; parent-qualified usage, inherited options, footer | `main.rs composition_help::*` |
| CLI wiring: interleaved/trailing help → compose screen, empty stderr | `argv_normalization::headline_compose_with_*_renders_help` |
| Help without/absent/unreadable file, root vs command, `--` forwarding, no launch | `provider_tail_ownership::help_opens_no_composition_file` |

The Input Robustness Matrix does not apply (no file-format or configuration reader changed).

### Gates (macOS, from `claudine/`)

- `just test` → 11864 passed, 15 skipped (after two earlier runs stopped on the two failures above).
- `just lint` → exit 0 (only the pre-existing `__eh_frame` linker warning).
- `just check-tier-coverage claudine` → 0 stranded.
- Dependents: no package depends on `claudine-cli`; nothing further to run.
- Cross-OS check skipped: the change compares exact argv tokens and renders clap help, with no
  path, process, or OS-specific code; CI covers the other environments.

## Phase 2

Three tasks ran in parallel (separate subagents, each owning separate files and running only the tests for what it changed). The full gates ran once afterwards.

### Cover selection (`cli/src/argv/rule4_help_hoist.rs` tests, `cli/src/main.rs::composition_help`)

- Selector table tests (8 new, through an `assert_rows` helper):
  - `selection_covers_every_destination_for_every_command`
  - `selection_skips_root_options_and_their_values` (`--debug trace`, `--debug=trace`)
  - `selection_recognizes_only_the_first_command` (`[]`, bare, `mcp compose --help`, `claude compose --help`, `__complete …`, `-- compose --help`)
  - `selection_takes_the_first_of_repeated_help_tokens` (root help followed by command help → Root)
  - `selection_treats_embedded_and_malformed_help_text_as_data` (`name=--help`, JSON setter, `--help=…`, `-hh`, `--HELP`)
  - `selection_passes_over_opaque_tokens_without_panicking` (per-OS non-UTF-8 token)
  - `selection_yields_to_active_completion` (injected Boolean, no `COMPLETE` mutation)
  - `selection_yields_to_a_root_version_request`
- Command-model tests (8 new):
  - `every_catalog_shortcut_is_a_visible_option_of_every_command`: the expected set is derived like `provider_for_boolean_flag`, 10 entries.
  - `every_visible_option_is_described_and_skipped_fields_are_absent`
  - `enumerated_values_stay_canonical`: provider slugs; `pause`/`abort`/`continue`.
  - `both_help_spellings_select_the_one_compact_renderer`
  - `user_text_carries_no_history_or_internal_paths`
  - `invalid_invocations_without_help_still_fail_validation`: no-help controls through normalize → partition → `build_cli_command(false).try_get_matches_from`. Unknown or missing provider gives `InvalidValue`, `--dry-run-shell` alone gives `MissingRequiredArgument`, and `-i --no-interactive` gives `ArgumentConflict`. `--bogus x.md` gives the partition `SwitchBeforeFile`, compared through `Debug` because `PartitionError` is module-private.
  - `malformed_setter_json_still_fails_without_help`
  - `ordinary_version_parsing_is_untouched`
- Existing hoist tests are unchanged. The printer cannot be captured in process, so "same renderer" is proven by the shared help argument and destination, plus a compact-vs-long difference that the test confirms exists.

### Prove CLI wiring (`cli/tests/l1/argv_normalization.rs`, `cli/tests/l1/sequence_errors_cli.rs`)

- Headline tests assert the compose screen, keeping the original `--gemini name=Ken --help` input and the clap-tip, unexpected-help, and second-file exclusions. Their comments now state the current contract.
- `composition_help_cases!` generates 12 independent tests, `{compose,inline_compose,sequence}_{short,long}_help_{without_file,with_missing_file}`. They run under `fake_only_path()` and also assert that no `config.json` is created.
- One subprocess bypass test each: `help_bypasses_{unknown_provider,missing_provider_value,malformed_setter_json,dry_run_shell_without_dry_run,conflicting_interactive_switches,unknown_switch_before_file}`.
- Root overview with invalid composition arguments, bare invocation, `mcp`/`actions` clap help, and wrapper help. Stdout and stderr are asserted separately.
- Plain mode before and after the command, and for root help. Color is forced with `CLICOLOR_FORCE=1` and `FORCE_COLOR=1`, with `NO_COLOR` removed on the child only. Forced-color controls prove escapes appear without `--plain`.
- `help_ignores_{quiet,silent,perf}`: the switch comes after the command, because `--perf` is a composition option. Before the command it is a root-level clap error, exit 2, which is unchanged existing behavior.
- Feature-gated `inline_compose_{long,short}_help_on_real_document_has_no_effects` use the fake Goose provider, `PROVIDER_MARKER`, an `append_line` initialize action, and no seeded config. They assert no launch, no lifecycle log, unchanged input bytes, no config, and no audio spool. The agent checked by hand that the same document without `--help` does launch and write the log.
- Feature-gated `help_after_separator_is_forwarded_to_provider` runs `compose prompt.md --claude -- --help` for real against `claudine-fake-completion`. The recorded argv has exactly one `--help` and no `--`, and stdout is the provider's answer, not a help screen.
- `removed_fail_fast_is_absent_from_sequence_help` now requires the sequence usage and shared options first, then empty stderr.

### Preserve completions (`cli/src/completion/*`, `cli/tests/l1/completion_ownership.rs`)

- Corrected the stale comment on `engine/mod.rs::run_composition_provider_flag` (and its twin in the engine test). It said kilo/pi/antigravity lacked clap fields; they now have them, and the catalog tokens are de-duplicated against clap.
- `completion::tests::completion_command_exposes_provider_shortcuts_without_injected_help`: all 10 shortcuts appear on every composition child of the legacy tree, and no help argument is injected.
- `composition_provider_flag_completer_is_catalog_derived` is extended: each provider token appears exactly once, and the candidate list has no duplicates.
- `completion_ownership::a_help_token_in_a_complete_payload_completes_instead_of_printing_help` (`__complete` protocol, no `Usage:`, no config).
- `completion_ownership::complete_env_bootstrap_with_composition_help_tokens_registers_without_help` (`COMPLETE=bash` registration, empty stderr, no config).

### Drift fixed outside the task list

- `cli/src/argv/rule1_provider_bool.rs`: the `provider_for_boolean_flag` doc listed seven first aliases. It no longer lists them, because the set is catalog-derived and now has ten.

### Focused checks (macOS, `just test-cli <filter>`)

| Filter | Result |
| --- | --- |
| `composition_help` | 12 passed |
| `rule4_help_hoist` | 18 passed |
| `argv_normalization` | 48 passed (slowest: forwarding case, 2.2 s alone) |
| `sequence_errors_cli` | 52 passed |
| `completion` | 669 passed |
| `rule_1` | 10 passed |

The new subprocess tests use `CliProcessFixture`, `bin_exe!` fixtures, and no shell scripts or `cfg(unix)` gates.

### Requirement → test mapping (Phase 2)

| Requirement | Tests |
| --- | --- |
| Destination selection on original argv, `--` boundary, first command/first help token | `rule4_help_hoist::tests::selection_*` |
| Completion and version precedence | `selection_yields_to_*`, `ordinary_version_parsing_is_untouched`, completion_ownership `__complete`/`COMPLETE=bash` cases |
| All ten shortcuts visible; inventory text accurate | `composition_help::every_catalog_shortcut_*`, `every_visible_option_*`, `enumerated_values_stay_canonical`, `user_text_*`; `completion_command_exposes_provider_shortcuts_*` |
| Help exits 0, stdout only, with or without (missing) file, both spellings, all commands | `argv_normalization::*_help_{without_file,with_missing_file}` |
| Validation bypass by help; validation intact without help | `help_bypasses_*` (subprocess); `invalid_invocations_without_help_*`, `malformed_setter_json_*` (in process) |
| Root overview before the command | `root_help_before_composition_command_renders_root_overview`, `bare_invocation_*` |
| `--plain` disables ANSI; `--quiet`/`--silent`/`--perf` do not suppress | `plain_*`, `forced_color_*` controls, `help_ignores_*` |
| No input read, config, lifecycle, provider, or audio | `inline_compose_*_help_on_real_document_has_no_effects`, `fake_only_path` cases |
| Help after `--` belongs to the provider | `help_after_separator_is_forwarded_to_provider`, selection rows |

The Input Robustness Matrix does not apply: this phase changed no file-format or configuration reader.

### Gates (macOS, from `claudine/`, run once after all three tasks joined, with no subagent running)

- `just test`: 11917 passed, 15 skipped, 0 failed (328 s).
- `just lint`: exit 0.
- `just check-tier-coverage claudine` (repo root): 0 stranded.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gives `packages` = `[claudine-cli, test-toolkit, darkmatter]` (build owners), `source_packages` = `[claudine-cli]`, `reverse_dependencies` = `[]`, and `change_class` = `package`. The gates were therefore required, and there are no dependent packages to test.
- Windows cross-check was attempted with `just cross-check claudine-cli --os windows <the six focused filters>` and **did not run**: SSH to `build-win-native` failed with `Host key verification failed.`, before any build. This is a host trust/configuration problem on the rig connection, not a test failure. It was not worked around, because accepting or changing host keys is not something to do unattended. `os_failing` is not set, because no Windows test ran or failed. CI's nightly Windows/WSL2 legs remain the evidence for those environments. (A first attempt with an `-E '…'` expression failed earlier inside the recipe's shell quoting: `cross-check` does not survive parentheses in its arguments. Use positional nextest filters.)
- Pre-existing and unchanged: `cargo fmt --check` differences across `claudine-cli`. Formatting was not run, per repository rules.

## Phase 3

- Docs: `claudine/docs/cli/index.md` replaces the **planned** marker with a
  "Help for the composition commands" section (placement table, no-file and
  plain examples, validation bypass, `--` forwarding, completion note).
  `claudine/cli/README.md` gains one overview paragraph linking there.
- `claudine/docs/topics/cli-pre-parsing.md`: pipeline diagram now shows
  completion intercept → original-argv help selection → help/exit, else
  normalize → partition → clap → dispatch; new "Composition help is selected
  first" section; rule 4 table row, partition paragraph, Root Cli quirks
  (removed the "root handler" and "seven provider booleans only as help
  entries" claims), and the motivating-bug resolution corrected.
- `claudine/docs/topics/argv-normalization.md`: same diagram change; rule 4
  rewritten as selection first plus a retained normalization contract for
  completion ownership; testing paragraph mentions the help screens.
- Skill: `SKILL.md` pointer paragraph; `cli-commands.md` argv pre-parsing row.
  `topics/` is a symlink to `claudine/docs/topics/`, so the topic pages were
  edited once (see `decisions_for_review`).
- `implementation.md` created with commands, results, covered behavior,
  deviations, and the three captured `--plain --help` screens (debug build,
  each exit 0 with empty stderr).
- Verified across all phases: no `Cargo.toml`/`Cargo.lock`, `.github`, or
  `scripts` changes (`git diff --stat c26823f69`).
- Gates (macOS, run once after all edits, no subagents): `just test` in
  `claudine/` 11917 passed, 15 skipped, 618.7 s (host load ~200 from other
  work; no failures or timeouts); `just lint` exit 0;
  `just check-tier-coverage claudine` 0 stranded. The scope-calculator skip
  was not used; the gates were run.
- Dependents: `claudine-cli` is a binary crate with no dependent packages, and
  this phase changed no source, so no downstream run. No cross-OS run: this
  phase changed only Markdown; earlier phases' code has no path or
  OS-specific logic.
