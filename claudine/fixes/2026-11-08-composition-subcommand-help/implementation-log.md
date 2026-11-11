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
packages:
    - claudine-cli
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
message_to_agent: |-
    Phase 1 is in place and `just test` (11864 passed), `just lint`, and the tier audit are
    green on macOS. Things you need to know for Phase 2:
    - Selector: `argv::select_composition_help(&raw, completion_active)` in
      `cli/src/argv/rule4_help_hoist.rs`, with a first set of unit tests there
      (`selection_*`). Extend them rather than duplicating.
    - Renderer: `main.rs::composition_help_command(name, plain) -> clap::Command` returns the
      built child (use `.render_help()` in tests); `build_cli_command(is_wrapper)` returns the
      model UNBUILT on purpose (see decisions_for_review). The test module is
      `main.rs::composition_help`; run with `just test-cli composition_help`.
    - Already updated to the new behavior: the two headline tests in
      `tests/l1/argv_normalization.rs`, `help_opens_no_composition_file` in
      `tests/l1/provider_tail_ownership.rs` (Unix-only file), and an `exit_site_guard.rs`
      allowlist entry for the new `exit_before_runtime` in `run`. If you move that exit call
      or change its enclosing `if let` line, update the allowlist `position` string too.
    - Not yet done (Phase 2 "Preserve completions"): the stale comment on
      `completion/engine/mod.rs::run_composition_provider_flag` claiming kilo/pi/antigravity
      lack clap fields.
    - Not yet done (Phase 3): docs and the claudine skill still describe composition help as
      the grouped root screen.
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
