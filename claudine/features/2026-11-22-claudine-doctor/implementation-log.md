---
spec: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/spec.md
plan: claudine/features/2026-11-22-claudine-doctor/plan.md
implemented-by: claude/opus
started-phase: 1
source_files_during_phase_1:
    - claudine/lib/src/doctor/mod.rs
    - claudine/lib/src/doctor/report.rs
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/tests.rs
    - claudine/lib/src/doctor/config_tests.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/config/load.rs
    - claudine/lib/src/config/json_conversion.rs
    - claudine/lib/src/config/persist.rs
    - claudine/lib/src/config/claudine_config.rs
    - claudine/lib/src/config/setup.rs
    - claudine/lib/src/dispatch/loader.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - darkmatter/lib/src/config/load.rs
    - darkmatter/lib/src/config/init.rs
    - darkmatter/lib/src/markdown/schemas/tests/lazy_positions.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/config_inspect_working_file.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/tests/l1/claudine_editor_parity.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/union_unknown_keys.rs
docs_updated_during_phase_1:
    - claudine/docs/topics/configuration.md
    - darkmatter/docs/topics/configuration.md
    - simplified-schema/docs/topics/language/unions.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_2:
    - claudine/lib/src/composition/registration.rs
    - claudine/lib/src/composition/registration/legacy.rs
    - claudine/lib/src/composition/registration/inspect.rs
    - claudine/lib/src/composition/registration/repair.rs
    - claudine/lib/src/composition/registration/repair_tests.rs
    - claudine/lib/src/config/setup.rs
    - claudine/lib/src/doctor/mod.rs
    - claudine/lib/src/doctor/report.rs
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/config_tests.rs
    - claudine/lib/src/doctor/registration.rs
    - claudine/lib/src/doctor/registration_tests.rs
    - claudine/lib/src/doctor/permissions.rs
    - claudine/lib/src/doctor/permissions_tests.rs
    - claudine/lib/tests/l1/installed_registration.rs
    - darkmatter/lib/src/config/init.rs
    - darkmatter/lib/tests/l1/config_init.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/configuration.md
    - darkmatter/docs/topics/host-registration.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - simplified-schema
    - sniff
decisions-for-review:
    - |-
        The plan leaves wiring the seven production check callbacks to Phase 4. I wired the three that Phase 1 builds (`config`, `repo-config`, `stray-config-keys`) into the registry now and gave the other four a placeholder callback that returns an `info` row ("this check is not available yet"). Leaving the three unwired would have made their code unused, which fails the lint gate, or would have needed lint suppressions that hide real dead code. The alternative is lint allowances now and wiring everything in Phase 4. To reverse: point the three entries back at the placeholder. Phase 4 still replaces the four placeholders.
    - |-
        A missing `~/.darkmatter.yaml` is an `error` under `config` with the remedy `claudine setup`, the same as a missing `~/.claudine/config.yaml`. The spec only says "missing required user configuration". Normal Darkmatter startup creates the file, but doctor never does, and the registration repair needs it. The alternative is `warn` or `info` for the Darkmatter file. Reversing it changes one severity in `doctor/config.rs::darkmatter_findings` and one test.
    - |-
        Doctor never shows a raw validator message, because those messages can quote authored values. An invalid value is shown by its kind and location ("has an invalid value at `/tts` (line 2, column 6)"). A pointer segment is shown only if the scope's schema declares that name somewhere, or if it is a list index; any other segment shows as `*`, so a messenger route's name is hidden. A semantic failure names its setting only when the message starts with a plain setting path. The remedy tells the user that any other `claudine` command prints the full message. The alternative is to show filtered messages, which keeps more detail but needs a per-message allowlist that can drift. Reversing it means changing the wording helpers in `doctor/config.rs`.
    - |-
        `repo-config` reports a repository file that exists but is invalid or unreadable as `ok` ("present; its problems are reported under `config`"). The spec says the row reports whether the file exists, so its own check passes, and the errors are already counted once under `config`. The alternative is `warn`, which would count the same problem in two rows. Reversing it changes one arm of `check_repo_config`.
    - |-
        SimplifiedSchema now reports an unknown key inside a union (for example `tts: {provider: say, pitch: 3}` or a misspelled lifecycle verb) by name, at the key, with the `UnknownKey` code, instead of the generic "not valid under any of the schemas" failure. The plan asked for this (Rule 9). It also changes two behaviors outside doctor. (1) DMLS grades unknown keys as warnings unless `[schema] strict` is set, so in the editor a misspelled lifecycle verb or a `shell` under `initialize` drops from error to warning, as every other unknown key already is. `md schema validate`, composition, and Claudine still refuse the document. (2) `md set` refuses to edit a key that an object-union property does not declare (it used to allow it, because the old problem named no key). One Darkmatter test fixture relied on the old behavior and now edits a declared key. The alternatives are to keep union-derived unknown keys at `ConstraintViolation` so DMLS keeps showing an error, or to have DMLS grade union-member unknown keys as errors. To reverse: change the `code` in `simplified-schema/lib/src/validate.rs::union_unknown_key_problem`, or change DMLS's classifier, and update the regenerated A3 golden.
    - |-
        A flat (pre-2026) action or route entry still reports only its migration message. The union's unknown-key problems below the entry are dropped as before, because every key of a flat entry (`type`, `command`) is unknown to the keyed arms and would otherwise show up as a stray key. Rule 8 asks to keep a flat entry's "genuine" unknown keys for `stray-config-keys`, but separating those needs the keyed conversion's view of the entry. I left that undone. The alternative is to run `wire::upgrade_flat` on the entry and validate the result to find the keys that are still unknown. Doing it later is additive to `config/load.rs::project`.
    - |-
        Darkmatter places an unknown key in `~/.darkmatter.yaml` at its value (for example column 8 of `colour: red`), not at the key, as Claudine now does. Doctor shows Darkmatter's position as it is, because changing it would change every `md` message for that file. The alternative is to make Darkmatter's config loader locate unknown keys at the key token, matching Claudine. Doing that later changes the position only, not which keys are found.
    - |-
        `darkmatter::config::inspect_working_file` returns a `Read` error when there is no home directory, instead of a separate "no home" error kind. Doctor reports "no home directory" before it asks, so the case never reaches the user through doctor. The alternative is a new `ConfigErrorKind` variant, which every match on that type would need to handle.
    - |-
        When SimplifiedSchema names unknown keys inside a union, it ignores arms that are not objects (a literal such as `true`, or an enum of words). Only object arms say which keys they accept, so the rejected keys are those that no object arm declares. A union with no object arm keeps its old message.
    - |-
        `file-read: []` (an empty list) is accepted by both scopes, so doctor treats it as valid. The plan expected it to be refused. I tested what the schema does and listed it with the valid cases; the scope-refusal case uses an invalid glob instead. If an empty list should be refused, the change goes in the permissions schema, not in doctor.
    - |-
        A changed registration manifest is judged by Darkmatter's manifest admission (`bundle::admit_manifest`) plus checks that the host is `claudine`, every member path is one Claudine publishes, and every member's bytes match their digest. The plan asked for Darkmatter's full v2 loader (`load_entries`). An existing Claudine guard test forbids calling the installed-bundle loaders anywhere in Claudine's source, so using the loader would have meant loosening that guard. The difference: the full loader also prepares the schema payloads, so it would also catch a manifest whose files are digest-correct but whose schemas fail to prepare; that cannot happen for files Claudine itself published. To switch: allow `load_entries` in `composition/registration/inspect.rs` in `cli/tests/l1/registrations_command.rs` and replace `inspect.rs::manifest_load` (about 15 lines, no test changes).
    - |-
        The registration repair refuses to start when the user `~/.darkmatter.yaml` has any invalid `register` entry, including another host's. The plan says the repair needs a valid `register` field. The alternative is to require only that `register` is a list and let Darkmatter's writer refuse an invalid result; that would repair more homes but edits a file Darkmatter already reports as invalid. Reversing it changes `registration/inspect.rs::entry_precondition` and one test.
    - |-
        Doctor's two repairs refuse to write a configuration file that is itself a symbolic link (for example a dotfiles-managed `~/.darkmatter.yaml` or `~/.claudine/config.yaml`), or whose writer lock sidecar is one, and report it as a manual finding. The plan asks for the no-follow guard at write destinations. The alternative is to resolve the link and write its target, as `claudine setup` and `md init` do; that repairs linked dotfiles but writes outside the paths doctor names. Reversing it means dropping the destination itself from `config_target_blockers` (keeping the sidecar check).
    - |-
        Setup's own `permissions` handling is unchanged: a Darkmatter `permissions: null` is still copied into Claudine's file as `null`, a Claudine `permissions: null` still counts as a block, and setup still lets the writer guess the Darkmatter file's scope (wrong only when the home is the repository root). Doctor handles all three as the plan specifies. The plan asked to keep setup's behavior while sharing the planner, so setup was not changed. Fixing setup means calling the shared planner with doctor's null handling and the explicit-scope validator in setup mode; it is small but changes what `claudine setup` writes.
    - |-
        The registration and permissions `--fix` tests run with a registry of only the row under test (plus `config`), so one repair never edits the other's fixture. Interaction of the two repairs in one run (registration first, then permissions on the same `~/.darkmatter.yaml`) is covered only by the existing fresh-reading order of `run_with` and the writer's observed-bytes check, not by an end-to-end test; Phase 4's CLI repair-and-rerun scenario will exercise both together. The alternative is a library test with the full registry now.
    - |-
        Sniff's shell probe takes any plain program name, not only `claudine`: `windows_shells::probe(shell, program, dir)` accepts a name of ASCII letters, digits, `.`, `_`, `-` (no leading `-` or `.`), refuses anything else before starting a shell, and passes the name to Bash as `$1`, never as script text. The plan described a fixed script for `claudine`. A fixed name would be an odd API for a library, and the check keeps the script fixed in every other way. The alternative is a `claudine`-only function. To reverse: make the name a constant in `windows_shells.rs` and drop `InvalidProgramName`; Claudine's call does not change.
    - |-
        Shell discovery picks the first installed candidate of each family in preference order, so there is no separate "deduplicate by path identity" pass. Because a family yields one shell and a Bash belongs to only one family (by the DLL beside it), a candidate that is the same file as another can never be reported twice. An indexed `bash` (Sniff's full index: `PATH` first, then App Paths and install roots) counts as Git Bash when its installation also holds `cmd\git.exe`, and as MSYS2 otherwise. It wins over the install layouts of its kind. The alternatives are to treat only a `PATH` hit as preferred, or to classify Git Bash by another marker such as `mingw64`. Reversing either changes `windows_shells.rs::discover` and its fake-layout tests.
    - |-
        In the `path` row, every other-shell outcome is a `warn` and a `Found` is an `ok` finding listed in the detail, so a healthy Windows report still names each shell it asked. Shell findings are listed in path order, as the report foundation orders every row's findings. The alternative is to list only problems, which shortens the detail but hides which shells were checked. Reversing it drops the `Found` arm's finding in `doctor/path.rs::shell_finding`.
    - |-
        In the `tts` row, a pinned provider with low quality says "the configured speech provider, X, has low quality" and suggests changing `tts.provider`, without naming other installed providers. Only automatic selection names the provider runtime tries first. Each row also carries an `ok` note that detection checks only local prerequisites, and a disabled row adds that lifecycle `say` can still speak. The alternative is to put those notes only in the documentation. Reversing it removes the trailing note finding in `doctor/tts.rs::assess`.
    - |-
        The human report prints each row as a bullet `[severity] Title (id): summary`, its detail lines as a nested list, and a closing count ("2 errors, 1 warning."). Severity is bracketed text, colored only on a terminal, so the report reads the same without color. The spec asks only for biscuit-terminal list and prose components. The alternatives are a table, or glyphs such as ✓/✗; a table wraps long paths badly and glyphs need a Unicode fallback. Changing it touches only `cli/src/commands/doctor.rs::render` and two CLI tests that look for the row heads.
    - |-
        Redirected output is always plain, even with `FORCE_COLOR` set: doctor calls `log::set_plain(true)` when stdout is not a terminal. The plan said "redirect detection before rendering". Elsewhere Claudine lets `FORCE_COLOR` style redirected output. The alternative is to honor `FORCE_COLOR`, which helps CI logs that render ANSI but breaks the "no ANSI on redirect" contract. Reversing it means checking `FORCE_COLOR` in `commands/doctor.rs::run` and changing the warnings-only CLI test.
    - |-
        Doctor writes its report to stdout directly (as `claudine completions` does) instead of through `log::data`, because `log::data` drops write errors and the plan requires a failed write to reach the CLI's error path (exit 1). The cost is that doctor bypasses the terminal gate; that gate only matters during a wrapped run, which doctor never starts. The alternative is to make `log::data` return the error, which changes every caller.
    - |-
        On native Windows, the `path` row's PowerShell probe makes PowerShell refresh its own startup cache, `%LOCALAPPDATA%\Microsoft\PowerShell\StartupProfileData-NonInteractive`, even with `-NoProfile`. The spec says an inspection leaves the filesystem as found. I treated this as PowerShell's write, not doctor's, and documented it in `docs/cli/doctor.md`. Real homes almost always have this file already. The alternative is to give each PowerShell probe a private `LOCALAPPDATA` (a temporary directory). That leaves the user's profile untouched, but the probe would no longer run in the environment the user's shell sees, and the probe's environment policy is Sniff's (`windows_shells::probe`). Doing that later changes Sniff's probe environment and its argv/environment test.
    - |-
        Spawned `claudine doctor` CLI tests skip the real Windows shell probes through a new `test-fixtures`-only seam, `CLAUDINE_TEST_DOCTOR_SKIP_SHELL_PROBES`. The healthy-home test is the exception: it keeps the real probes and tolerates `ok` or `warn` on `path`. The plan said spawned cases should tolerate the installed shell roster. With real probes in every spawn, multi-spawn tests took over 5 s on Windows (about 2.5 s per spawn), and each spawn wrote PowerShell's cache into the fixture home. Exact probe outcomes stay covered by the library's injected `path::assess` tests. The alternative is real probes everywhere, with each multi-spawn test split into one-spawn tests and `AppData` excluded from every tree comparison. To reverse: remove the seam from `doctor/path.rs` and the guard allowance, and the `SKIP_SHELL_PROBES` env from `cli/tests/l1/doctor.rs`.
    - |-
        The human report keeps biscuit-terminal's default word wrap, which may break a long path at a hyphen (`/tmp/p5-doctor-` then `i85y/…` on the next line), because `WordWrap::WrapProse` treats `-` as a break character. A wrapped path cannot be copied in one piece; `--json` is unaffected. The alternatives are to give doctor's detail lines a wrap policy that breaks only at spaces (the list component applies its own wrap to items, so this needs a list option or a change in biscuit-terminal), or to stop wrapping paths in every Prose user. Changing it later touches `cli/src/commands/doctor.rs::render` or biscuit-terminal's list wrapping, and no test asserts the current breaks.
source_files_during_phase_3:
    - sniff/lib/src/process.rs
    - sniff/lib/src/programs/mod.rs
    - sniff/lib/src/programs/git_program.rs
    - sniff/lib/src/programs/windows_shells.rs
    - sniff/lib/src/programs/windows_shells_tests.rs
    - claudine/lib/src/doctor/mod.rs
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/config_tests.rs
    - claudine/lib/src/doctor/registration_tests.rs
    - claudine/lib/src/doctor/permissions_tests.rs
    - claudine/lib/src/doctor/tts.rs
    - claudine/lib/src/doctor/tts_tests.rs
    - claudine/lib/src/doctor/path.rs
    - claudine/lib/src/doctor/path_tests.rs
docs_updated_during_phase_3:
    - sniff/README.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_3:
    - sniff/docs/topics/windows-shells.md
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/sniff/programs.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - claudine/cli/src/commands/doctor.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/help.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/log.rs
    - claudine/cli/src/completion/root_menu.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/tests/l1/doctor.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/completion_cli.rs
    - claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
    - claudine/lib/src/doctor/path.rs
    - claudine/lib/src/doctor/permissions_tests.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
docs_updated_during_phase_4:
    - claudine/README.md
    - claudine/docs/cli/index.md
    - claudine/docs/topics/configuration.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/host-registration.md
docs_created_during_phase_4:
    - claudine/docs/cli/doctor.md
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/os/windows.md
    - claudine/docs/use-skills/use-claudine/cli.md
source_files_during_phase_5:
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/config_tests.rs
docs_updated_during_phase_5:
    - claudine/docs/cli/doctor.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source-code:
    - claudine/lib/src/doctor/mod.rs
    - claudine/lib/src/doctor/report.rs
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/tests.rs
    - claudine/lib/src/doctor/config_tests.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/config/load.rs
    - claudine/lib/src/config/json_conversion.rs
    - claudine/lib/src/config/persist.rs
    - claudine/lib/src/config/claudine_config.rs
    - claudine/lib/src/config/setup.rs
    - claudine/lib/src/dispatch/loader.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - darkmatter/lib/src/config/load.rs
    - darkmatter/lib/src/config/init.rs
    - darkmatter/lib/src/markdown/schemas/tests/lazy_positions.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/config_inspect_working_file.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/tests/l1/claudine_editor_parity.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/union_unknown_keys.rs
    - claudine/lib/src/composition/registration.rs
    - claudine/lib/src/composition/registration/legacy.rs
    - claudine/lib/src/composition/registration/inspect.rs
    - claudine/lib/src/composition/registration/repair.rs
    - claudine/lib/src/composition/registration/repair_tests.rs
    - claudine/lib/src/doctor/registration.rs
    - claudine/lib/src/doctor/registration_tests.rs
    - claudine/lib/src/doctor/permissions.rs
    - claudine/lib/src/doctor/permissions_tests.rs
    - claudine/lib/tests/l1/installed_registration.rs
    - darkmatter/lib/tests/l1/config_init.rs
    - sniff/lib/src/process.rs
    - sniff/lib/src/programs/mod.rs
    - sniff/lib/src/programs/git_program.rs
    - sniff/lib/src/programs/windows_shells.rs
    - sniff/lib/src/programs/windows_shells_tests.rs
    - claudine/lib/src/doctor/tts.rs
    - claudine/lib/src/doctor/tts_tests.rs
    - claudine/lib/src/doctor/path.rs
    - claudine/lib/src/doctor/path_tests.rs
    - claudine/cli/src/commands/doctor.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/help.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/log.rs
    - claudine/cli/src/completion/root_menu.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/tests/l1/doctor.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/completion_cli.rs
    - claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
    - claudine/lib/tests/l1/context_construction_guard.rs
documentation:
    - claudine/docs/topics/configuration.md
    - darkmatter/docs/topics/configuration.md
    - simplified-schema/docs/topics/language/unions.md
    - .claude/skills/claudine/SKILL.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/host-registration.md
    - .claude/skills/darkmatter/SKILL.md
    - sniff/README.md
    - sniff/docs/sniff-library-architecture.md
    - sniff/docs/topics/windows-shells.md
    - .claude/skills/sniff/programs.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/os/windows.md
    - claudine/README.md
    - claudine/docs/cli/index.md
    - claudine/docs/cli/doctor.md
    - .claude/skills/claudine/cli-commands.md
    - claudine/docs/use-skills/use-claudine/cli.md
completed-phase: 5
implemented: true
---

# Implementation Log for 2026-11-22-claudine-doctor (5 phases)

## Phase 1

- started 2026-11-23; the host-registration v2 code has landed in this tree
  (no `HostRevision`/`refresh_host_registration` remain in `claudine/lib/src`),
  so the phase builds on v2 directly as Rule 18 requires.
- while the phase ran, the author's pipeline committed the host-registration
  v2 work that was uncommitted at session start (`ce553e706`, `ebdff458b`,
  `33c412d45`); nothing in this phase depends on those commits beyond v2
  itself.

### Report Foundation (Rules 1–4)

- `claudine/lib/src/doctor/mod.rs`: public `Severity` (`ok < info < warn <
  error`, serialized lowercase), `DoctorRow { id, severity, summary, detail,
  fixable }` (serializes to exactly the five stable JSON fields),
  `run(snapshot, fix)`, and `title_of(id)` for the CLI's headings. The static
  `REGISTRY` lists the seven checks in report order, each a
  `CheckDescriptor { id, title, prerequisites, check, repair }`.
  `Environment` captures the launch context once with
  `composition::capture_file_resolution_context` (its failure is kept as a
  reason-free `ContextUnavailable`, since the error text can name authored
  values); `Pass` is one reading of the disk and caches the configuration
  observations on first use.
- `run_with` inspects every check, then with `fix` runs each repair in
  registry order, each from a fresh `Pass`, only when that fresh reading is
  fixable; afterwards every attempted row and every row an attempted check
  names as a prerequisite are read again from one new pass.
- `doctor/report.rs`: `Finding` (severity, scope, path, summary, notes,
  remedy, fixable), `CheckOutcome` (findings + the ok summary), and
  `RepairOutcome { changes, failure }`. A row's severity is its worst
  finding; the summary is the worst finding's (`(+N more)` when several
  share that severity); the detail lists findings ordered general → user →
  repository, then by path, stable within a path, with `Remedy:` lines and
  `Repaired:` / `Repair failed:` / `Already changed:` lines from the attempt.
  A failed attempt keeps the row at `error` even when the disk now reads
  healthy; `fixable` is always the last reading's. Every dynamic string
  passes through `display_text`, which escapes control and bidirectional
  formatting characters (`\u{1b}`), so no file name or key can move the
  cursor or break a row. Prose markup escaping is left to the Phase 4
  renderer, which owns Prose.
- tests: `doctor/tests.rs` (13 tests) drive fake checks whose state is a
  marker file in a temp home, so repairs are real writes and re-reads real
  reads: aggregation and ordering, blocked → info, fixable without the flag
  and no writes, verified post-repair state, second run is a no-op (mtime
  kept), fresh-evidence ordering between repairs and prerequisite re-reads,
  partial failure with recorded changes and remaining fixability, a failed
  attempt over a healed disk, a no-change failure beside an unrelated repair,
  control/bidi escaping, the JSON shape, and the registry order.

### Union Diagnostics (Rule 9)

- delegated to a subagent. `simplified-schema/lib/src/validate.rs`:
  `build_problems`'s `AnyOf` branch now asks `union_unknown_key_problem`
  for the keys every type-applicable arm rejects at the union's own node
  (`union_rejected_keys` / `arm_rejected_keys`: an arm failing `type`,
  `const`, or `enum` there is left out; `additionalProperties` /
  `unevaluatedProperties` errors add keys; a same-node nested union is
  resolved recursively; an applicable arm with no rejection empties the
  intersection). The result is shaped exactly like an ordinary closed-object
  unknown-key problem (instance path at the object, `offending_properties`,
  jsonschema's wording), listed ahead of the existing deeper / file /
  nullable problems, deduplicated by (path, message); the generic union
  failure remains the fallback. Side effect: a plain nullable object now
  reports an extra key beside a deeper failure, which it used to drop.
- tests: `simplified-schema/lib/tests/l1/union_unknown_keys.rs` (10 tests:
  scalar-or-object union, shape parity with a closed-object problem,
  nullable object, deeper failure beside a same-node key, another arm's
  known key never stray, a key every arm rejects, a literal arm does not
  veto, a union reached through a named union type, a valid control, an
  open mapping arm). 6 of the 10 fail against the old validator.
- ran: simplified-schema focused filters (92 + 88 passed),
  `cargo nextest run -p simplified-schema --features work-counters,terminal
  -E 'test(/validat|union|unknown|additional|anyof|any_of|nullable|soft|coerc|default/)'`
  (616 passed), simplified-schema `just lint` clean, Darkmatter
  `-E 'test(/config/)'` (194 passed), Claudine config/load/schema filters
  (161 passed). No downstream expectation changed. The subagent added an
  "Unknown Keys in a Union" section to
  `simplified-schema/docs/topics/language/unions.md` and corrected its claim
  that discriminant-free unions keep byte-identical diagnostics.
- note: `simplified-schema`'s `just test <filter>` runs its
  no-default-features leg (`test-minimal`) unfiltered, so a filtered run
  there still executes ~2,200 tests; the subagent ran that leg by hand
  with `-E` instead. The recipe was not changed.

### Darkmatter Observation (Rules 5, 7)

- delegated to a subagent: `darkmatter::config::load::inspect_working_file
  (context, scope) -> Result<ConfigFile, ConfigError>` and crate-private
  `read_working_file`, `working_path`, `working_revision`,
  `read_working_bytes` in `darkmatter/lib/src/config/load.rs`;
  `init::config_path` and `init::observe` now share the path rule, byte
  reader, revision, and judge with inspection (observe keeps its own call of
  the shared byte reader so `InitError::Read` still carries the
  `io::Error`). Whole-file failures are `Err`; invalid `permissions` and
  `register` entries stay inside the successful `ConfigFile`. No accessor was
  missing. The subagent also added a "Checking one file without loading"
  section to `darkmatter/docs/topics/configuration.md`.
- tests: `darkmatter/lib/tests/l1/config_inspect_working_file.rs` (8 tests:
  absent, valid in each scope, invalid permissions kept inside, invalid
  register entry kept as a finding, whole-file failures with locations,
  writes nothing with a legacy whitelist present and the user file missing,
  a reference is not followed, a context without the scope is refused).
  Ran: `cargo nextest run -p darkmatter --test l1 -E 'test(config_inspect_working_file)'`
  (8 passed), `-E 'test(/^config_/)'` (172 passed), `darkmatter-cli`
  `init_command` (4 passed), Claudine `darkmatter_setup` /
  `installed_registration` / `config_indentation` (57 passed), Darkmatter
  `just lint` clean.

### Claudine Observation and Observation Integration (Rules 5, 6, 8)

- `config/load.rs`: `ConfigProblem` gained `kind: ConfigProblemKind`
  (`UnknownKey`, `Syntax`, `InvalidSetting`) and `unknown_key()`; every
  initializer (load, json_conversion, persist, claudine_config) sets it.
  An unknown key's problem is now located at the key token (a locating
  problem with `ValidationProblemCode::UnknownKey` and the key as
  `offending_property`, so `PositionMap` selects the key), and is
  recognized by the validator's `UnknownKey` code as well as the old
  message. Duplicates are removed by (kind, pointer, message) rather than
  only adjacent ones. `Display` is unchanged for established callers.
- `dispatch/loader.rs::refused_action_mappers`: the passive half of
  `compile_canonical_runtime`, sharing its `compile_canonical_action_mapper`;
  matchers stay tolerated.
- `config/setup.rs::same_file` is now `pub(crate)` so doctor uses setup's
  `PathIdentity` + `canonicalize_simplified` comparison for aliases.
- `doctor/config.rs`: `observe(environment)` builds the shared observation
  record (`ConfigObservations`: the four `ObservedFile`s with
  `ClaudineState` / `DarkmatterState`, `user_config` = the user file alone
  typed and semantically judged, `effective_config` = merged with usable
  repository overrides and judged again, plus availability flags). A path
  is probed with no-follow metadata first, so a dangling link and a
  non-file are unreadable rather than absent; a missing user YAML with a
  `config.json` beside it gets the legacy remedy and the JSON is never
  opened. Claudine files are read with `config::load::read_document`;
  Darkmatter files with `inspect_working_file`. Semantic checks follow the
  normal load's order (typed read, merge, `validate`) plus the mapper
  check; a merged failure is attributed to the user file when the user
  file fails alone, otherwise to the repository file with a note naming the
  user file; a user-only failure superseded by a valid merged pair is not a
  `config` finding (it stays in `user_config` for speech and the handoff).
- the three checks: `config` (home/context availability, each file's
  state, located categories, semantic findings, ok notes per file),
  `repo-config` (info outside a repository or when the file is absent; ok
  when present, aliased, or present with problems, which are under
  `config`), `stray-config-keys` (one error per unknown key, sorted by
  pointer, with line/column; a file whose syntax prevents key validation is
  an info naming it; a user setting written in the repository file gets a
  move remedy).
- redaction: a Claudine problem is shown by kind and location; an invalid
  value's pointer keeps only segments the scope's lowered schema declares
  somewhere (and list indices), every other segment shows as `*` (a
  messenger route's name, say); a semantic failure names its setting only
  when the validation message leads with a plain setting path, and never
  keeps the rest. Darkmatter problems are shown by `ConfigErrorKind` with
  the same pointer rule (`register` added). Unknown key names are the only
  authored text shown.
- tests: `doctor/config_tests.rs` (30 tests) over temp homes and
  repositories through the built-in registry: the control; no writes with a
  legacy file present; missing user files; outside a repository; no home /
  no context; top-level unknown key located at the key and named only in
  the stray row; nested unknown keys in the `tts` mapping, the imported
  `permissions` mapping, and a keyed action union (needs Rule 9); a known
  key of another union arm not reported; a user setting in the repository
  file; the shared YAML loader's document shapes once (duplicate key,
  trailing nonsense, malformed, second document, non-string key, empty
  document; BOM, comments, `key:` null, `{}` accepted); invalid values
  without their text; route-name redaction; the `tts` field shapes; repo
  override null/empty and event list shapes (mixed and all-invalid
  arrays); semantic rule naming without value; a refused regex mapper; a
  merged-only `active_messenger` interaction naming both files; a valid
  merged pair superseding a user-only problem; a repository at the home
  judged once; dangling link and directory (Unix); an invalid repository
  file; Darkmatter field shapes (wrong scalar, blank reference, wrong
  collection, mixed and all-invalid `register`, `shell-commands` invalid
  element, scope-invalid `file-read` globs, unknown key, duplicate key) and
  their valid defaults (`{}`, null, empty mapping, `[]`, a third-party
  register entry). Every redaction case asserts the sentinel `SECRET` is
  absent from the serialized rows.
- ran: `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` (43
  passed); `cargo nextest run -p claudine -E 'test(/config|setup|registration/)'`
  (727 passed) to check the location change against existing callers.
- departures found while testing: `file-read: []` is accepted by both scope
  projections (only the shared, scope-less shape says `not-empty`), so the
  plan's expected refusal does not exist; the test asserts acceptance.
  Darkmatter locates an unknown key at its value, not the key; doctor
  reports Darkmatter's own location.
- the robustness matrix's process-proof cases (`CliProcessFixture`) need the
  command, which Phase 4 wires; they belong to Phase 4's CLI Contracts.

### Downstream expectations changed by the union diagnostics

The full suites found tests that pinned the old generic union message where
the validator now names the rejected key. Each new diagnostic is the more
precise and correct one; the tests were updated, not the behavior:

- Claudine `lib/tests/l1/claudine_schemas.rs` (`ac5_shell_placement`,
  `ac5_single_action_dictionaries`): `shell` under `initialize` and `then`
  in a root dictionary are now `'shell' was unexpected` /
  `'then' was unexpected` at the same pointers.
- Claudine `lib/tests/l1/claudine_schemas_embedded.rs`
  (`a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written`):
  a misspelled verb (`inf0`) and a misplaced `shell` are now `UnknownKey`
  problems with no arm description; beside a known verb, the position is the
  unknown key's own line (3:5, was 2:5). The test now pins the code per case.
- Claudine `lib/tests/l1/sequence_schema.rs` and Darkmatter
  `cli/tests/l1/claudine_schema_bundle.rs`: group `execution` /
  `max_paralel` and the same lifecycle cases, by name.
- Darkmatter `cli/tests/l1/schema_validate.rs`: an unknown lifecycle verb is
  `'bogus' was unexpected`, not the `anyOf` text.
- Darkmatter `lib/src/markdown/schemas/tests/lazy_positions.rs`: a nullable
  inline object with a deeper failure now also reports its unknown key
  (4 problems, was 3: the old validator silently dropped it).
- Darkmatter `lib/tests/content_policy_fixtures/mod.rs`: an extra long-form
  field is located at its key (new `Anchor::Key`), not the item start.
- Darkmatter `cli/tests/l1/get_set_rm.rs`
  (`a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`): the
  fixture edited the undeclared `style.width`. `md set` blocks problems at
  or under the edited path; the old union failure at `/style` named no key,
  so the edit slipped through, and now the unknown-key problem names
  `width` and refuses it. The test's subject is comment preservation, so it
  now edits the declared `style.margin`.
- DMLS `tests/l1/core_schema_types.rs`, `claudine_editor_parity.rs`, and the
  regenerated golden `tests/fixtures/response_contracts/a3_lifecycle_faults.json`:
  the same items are `dm.schema.unknown_key` at the key token. DMLS grades
  unknown keys as warnings outside `[schema] strict`, so an unknown verb in a
  lifecycle stack is now an editor warning (an error in strict mode);
  `md schema validate` still exits 1 for it (see decisions-for-review).

### Gates and verification (all serial, after both subagents finished)

| Command | Result |
| --- | --- |
| `cd claudine && just test` | 12768 passed, 15 skipped; `🐢 none at or above 5.0s`; `just slow-tests fresh` exit 0 |
| `cd claudine && just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `cd simplified-schema && just test` | 2592 + 2172 passed, none slow |
| `cd simplified-schema && just lint`, `just check-tier-coverage simplified-schema` | clean, 0 stranded |
| `cd darkmatter && just test` | 17855 passed, 10 skipped, none slow |
| `cd darkmatter && just lint`, `just check-tier-coverage darkmatter` | clean, 0 stranded |

The earlier `just test` runs that failed were the expectation changes listed
above; each was then rerun alone, fixed, and the full suites rerun green.
Host load during the runs was 4–15 on this host; no test was slow.

Dependents: the other packages that depend on Darkmatter or SimplifiedSchema
(`biscuit-icon`, `biscuit-speaks`, `biscuit-terminal`, `messenger`, `playa`,
`research`, `sniff` CLIs/libs) were searched for assertions on union
messages, `init::observe`, or `inspect_working_file`; none exist, so their
suites were not run.

Cross-OS: the only OS-specific code is the Unix-gated dangling-link test;
path identity reuses setup's `PathIdentity`/`canonicalize_simplified`. The
plan (Rule 18) leaves the local cross-check out because it creates a
throwaway commit; remote coverage is left to the regular CI schedule.

### Requirement-to-test mapping (Phase 1)

| Requirement | Tests |
| --- | --- |
| Rules 1–4 registry, ordering, aggregation, blocked, fix/no-fix, partial failure, redaction | `doctor::tests::*` (13) |
| Rule 5 four files, absence vs failure, aliases, legacy existence-only | `doctor::config::tests::{the_valid_control…, an_inspection_writes_nothing…, a_missing_user_file…, outside_a_repository…, without_a_home_or_a_context…, a_repository_at_the_home…, a_dangling_link_and_a_directory…, an_invalid_repository_file…}` |
| Rule 6 semantic checks, merge attribution, mapper check | `a_semantic_rule_names_the_setting…`, `a_mapper_the_runtime_compiler_refuses…`, `a_repository_selection_with_no_user_route…`, `a_valid_merged_pair_supersedes…` |
| Rule 7 Darkmatter passive inspection | `darkmatter::l1 config_inspect_working_file::*` (8); `doctor::config::tests::darkmatter_*` |
| Rule 8 structured unknown keys at the key token | `a_top_level_unknown_key…`, `nested_unknown_keys…`, `a_user_setting_in_the_repository_file…` |
| Rule 9 union unknown keys | `simplified-schema::l1 union_unknown_keys::*` (10); `a_known_key_of_another_arm…` |
| Robustness matrix: shared YAML loader shapes | `syntax_cases!` (6), `a_duplicate_key_names_the_repeated_key_line`, `a_bom_comments_explicit_null…` |
| Matrix: TTS field shapes | `tts_shapes_are_judged_by_the_user_schema` |
| Matrix: settings and overrides | `override_null_inherits_and_empty_mappings…`, `an_invalid_value_is_located_without_its_text`, `a_route_name_the_user_chose…` |
| Matrix: Darkmatter and imported permissions, registration | `darkmatter_files_are_judged_by_their_scope_schema`, `darkmatter_defaults_and_empty_values_are_valid`, `nested_unknown_keys…` (imported `permissions`) |
| Matrix: process proof through `CliProcessFixture` | deferred to Phase 4 (the command is wired there) |

Code-smell review of the new load-bearing parse paths: no new
`#[serde(default)]`, no `Option<T>` collapsing null and absence (the typed
reads reuse the existing schema-default projection), and no
`filter_map`/`unwrap_or_default()`/`.ok()` on a load-bearing parse; the one
`unwrap_or_default()` (an unknown key's display name) runs only on a
problem already known to be `UnknownKey`, and the legacy `config.json`
probe is existence-only by design.

## Phase 2

- started 2026-11-23 on the committed Phase 1 tree (`a347c45c8`).
- scaffolding (orchestrator): `doctor/registration.rs` and
  `doctor/permissions.rs` created with their check and repair callbacks, and
  wired into `REGISTRY` (`registration` and `darkmatter-permissions` now
  declare repairs), so the two Wave 2 tasks each own one file.
- Darkmatter `config/init.rs` (orchestrator, before Wave 2, so neither
  parallel task edits it):
  - `replace_manifest_registration_existing(context, scope, reference,
    old_path)`: the doctor's register writer. It never creates the file
    (`InitError::Read` with `NotFound`), shares `replace_manifest_change`
    with setup, and edits only while the file holds the observed bytes.
  - `check_referenced_permissions(text, path, scope, context)`: the public
    form of the loader's reference-target judgment (`referenced_block`
    now shares `referenced_block_text`), so a handoff can judge Claudine's
    candidate block with Darkmatter's scoped runtime rules.
  - tests in `darkmatter/lib/tests/l1/config_init.rs` (4): never creates,
    edits in place keeping comments and the other host's entry and then
    converges, refuses an invalid `register` writing nothing, and the
    referenced-mapping verdicts (`{}` and a valid mapping pass; missing,
    `null`, a reference, and a wrong-typed entry fail). 4 passed.

### Publisher Repair (Rules 10–11), delegated to a subagent

- `composition/registration.rs`: `publish`/`publish_with` now delegate to a
  crate-private `Publisher` that owns the lock and the layout
  (`acquire`, `publish`, `finalize_legacy`); each write and removal repeats
  the no-follow check just before it happens. Plain `publish` behavior is
  unchanged (unknown files are kept); the existing publisher tests pass
  unchanged.
- `registration/legacy.rs`: the no-write examination is its own function
  (`legacy_plan`), shared by inspection and the repair's preflight; each entry
  is examined again just before removal, and an entry another caller already
  removed is tolerated.
- new `registration/inspect.rs` (read-only, no-follow inspection:
  `inspect_installation`, `ExpectedState`, `ManifestLoad`, `Blocker`,
  `entry_precondition`, `entry_registered`, `config_target_blockers`) and
  `registration/repair.rs` (`repair_registration`, `repair_registration_with`
  with a lock deadline and an observer; `RegistrationRepair { operations,
  failure }` with typed `RepairOperation`/`RepairFailure`/`EditFailure`).
  Repair order: user `.darkmatter.yaml` must be valid with a valid `register`
  → preflight everything it will touch → lock, preflight again → publish →
  delete extras deepest first → verify the tree → recheck the config file
  and its lock sidecar → `replace_manifest_registration_existing` → re-read
  and verify the entry → `finalize_legacy`.
- `doctor/registration.rs`: the `registration` row and its repair, with
  wording that names paths, operations, and I/O kinds only.
- departure: a changed manifest is judged with `bundle::admit_manifest`
  plus host, path, and digest checks, not `load_entries`, because the CLI
  guard `registrations_command.rs::only_the_registration_module_locates_or_loads_an_installed_bundle`
  forbids the installed-bundle loaders in Claudine source (see
  decisions-for-review).
- tests: `composition::registration::repair::tests` (9) and
  `doctor::registration::tests` (14 + 4 Unix-only obstruction cases).
  `doctor/config_tests.rs::the_valid_control_passes_every_configuration_row`
  now asserts unfixability only for the three config rows, since its
  fixture publishes no registration and the `registration` row is rightly
  fixable there.

### Permissions Repair (Rules 12–13), delegated to a subagent

- `config/setup.rs`: `setup_with_darkmatter` now calls the shared crate-private
  `plan_permissions_handoff(context, scope, mode, claudine, observation)` and
  `commit_permissions_handoff(context, plan)`; `HandoffMode::Setup` decides
  exactly as before (the `darkmatter_setup.rs` tests pass unchanged),
  `HandoffMode::Doctor` follows Rules 12–13: existing valid distinct files
  only; omitted and `null` Darkmatter `permissions` are handoff requests;
  `null`/omitted Claudine `permissions` is no authored block; a different
  effective block or a Claudine-side reference is a conflict; Claudine's
  selected mapping must pass `check_referenced_permissions`; edits are
  `permissions` only, against the observed bytes, the Darkmatter edit
  `published_last`, and the Darkmatter candidate is judged for the plan's own
  scope.
- `doctor/permissions.rs`: the `darkmatter-permissions` row and its repair;
  after a commit the files are observed again to verify the reference
  resolves to the mapping written. A repository-scope finding carries the
  note that grants still depend on Darkmatter's user, committed, and
  working-copy policy.
- tests: `doctor::permissions::tests` (23).

### Setup Integration (orchestrator)

- `setup_claudine` now holds one `Publisher` from the publish through the
  legacy cleanup (`Publisher::acquire` → `publish` → handshake →
  `replace_manifest_registration` → `finalize_legacy` → drop). Lock order is
  the publisher lock, then each configuration transaction's own locks, one
  transaction at a time (each releases its locks when it commits).
- the permissions writer's local symlink check was replaced by the
  publisher's shared no-follow guard, `registration::config_target_blockers`
  (now `pub(crate)`), for both destinations and their `LOCK_SUFFIX`
  sidecars.
- regression test:
  `lib/tests/l1/installed_registration.rs::installed_registration_setup_holds_the_publisher_lock_through_the_handoff`:
  a publisher started from inside setup's Claudine write (between the
  publish and the cleanup) times out on the lock; after setup returns, a
  publish succeeds and finds the tree current. It fails against the old
  `setup_claudine`, which released the lock after publishing.
- docs: `claudine/docs/topics/schemas.md` (setup holds the publisher lock
  through its four steps), `darkmatter/docs/topics/host-registration.md`
  (`replace_manifest_registration_existing`),
  `darkmatter/docs/topics/configuration.md` (`check_referenced_permissions`).
  The doctor command reference, the configuration topic's repair section,
  and the caller skill are Phase 4's Reference Updates.
- skills: `.claude/skills/claudine/SKILL.md` (publisher lock, shared
  handoff planner, repair entry points, the loader guard),
  `.claude/skills/darkmatter/SKILL.md` (the two new `init` functions).

### Gates and verification (serial, after both subagents finished)

| Command | Result |
| --- | --- |
| `cargo nextest run -p claudine --lib -E 'test(/doctor::\|composition::registration\|config::setup/)'` | 97 passed |
| `cargo nextest run -p claudine --test l1 -E 'test(/installed_registration\|darkmatter_setup\|host_publication\|registration_inventory/)'` | 88 passed (incl. the new lock test) |
| `cd claudine && just test` | 12819 passed, 15 skipped; `🐢 none at or above 5.0s`; `just slow-tests fresh` exit 0 |
| `cd claudine && just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `cargo nextest run -p darkmatter -E 'test(/config\|init\|regist\|permission/)'` | 725 passed |
| `cargo nextest run -p darkmatter-cli -E 'test(/init\|regist\|config/)'` | 100 passed |
| `cd darkmatter && just lint` | clean |

Host load stayed about 4–6 during the runs. The Darkmatter change is additive
(two public functions, one internal helper split), so the config-scoped
Darkmatter and Darkmatter-CLI selections stand in for their full suites;
Claudine's suite is the other consumer of the changed code. The subagents saw
no failing test; the one earlier red (`the_valid_control_passes_every_configuration_row`)
was a correct expectation change, described above.

Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}'
| xargs python3 scripts/ci/affected_scope.py` → `packages: [claudine,
darkmatter, test-toolkit]`, `change_class: package`, so no gate was skipped.

Cross-OS: the new code has Windows branches (reparse-point attribute checks
in the shared no-follow guard, which Phase 1's publisher already used) and
Unix-only symlink tests. As Rule 18 directs, the local cross-check (which
makes a throwaway commit) was not run; the regular CI schedule covers
Windows and WSL2.

### Requirement-to-test mapping (Phase 2)

| Requirement | Tests |
| --- | --- |
| Rule 10: control, no home, missing tree/member, stale member, changed manifest loads vs refused | `doctor::registration::tests::{the_state_setup_leaves_passes, without_a_home_the_row_is_informational, a_missing_tree_is_a_fixable_error_and_the_fix_publishes_it, a_missing_member_is_a_fixable_error, a_stale_member_is_a_fixable_warning_that_explains_the_digest, a_changed_manifest_darkmatter_still_loads_is_a_warning_and_one_it_refuses_an_error}` |
| Rule 10: extras, legacy paths, equivalent entry, missing entry, other hosts kept, blocked entry check | `extras_are_removable_warnings_and_the_fix_removes_only_them`, `the_retired_layout_is_a_warning_and_the_fix_removes_it_after_moving_the_entry`, `an_equivalent_spelling_of_the_entry_passes`, `a_missing_entry_is_a_fixable_error_and_other_hosts_entries_survive_the_fix`, `a_missing_or_invalid_user_file_blocks_the_repair_and_nothing_is_written`, `a_current_tree_with_a_missing_user_file_reports_the_entry_as_blocked` |
| Rule 11: order, idempotence, extras deepest first, failed publish withholds the edit, failed register write keeps tree and legacy paths, retry converges, concurrency, obstructions | `composition::registration::repair::tests::*` (9), `doctor::registration::tests::{a_second_fix_changes_nothing, obstructions::*}` |
| Rule 11 / Setup Integration: one publisher lock through setup | `installed_registration::installed_registration_setup_holds_the_publisher_lock_through_the_handoff`; existing setup migration, conflict, failed-publication, and link-refusal tests unchanged and green |
| Rule 12: references in every spelling, third-party, broken, prerequisites, aliases, conflicts, Claudine-side reference, scoped mapping | `doctor::permissions::tests::{a_reference_to_claudine_passes_in_every_spelling, a_valid_third_party_reference_is_information, every_broken_reference_is_an_error_without_a_repair, a_missing_prerequisite_skips_its_scope_with_information, a_missing_home_skips_the_user_scope, a_repository_without_its_claudine_file_skips_only_that_scope, a_repository_that_is_the_home_is_assessed_once, a_repository_darkmatter_file_aliasing_the_user_one_skips_the_repository_scope, different_inline_blocks_conflict_and_nothing_is_written, a_claudine_side_reference_is_manual_in_every_darkmatter_state, a_mapping_that_fails_the_scoped_rules_is_manual, the_doctor_planner_*}` |
| Rule 13: omitted / null / equal / transfer writes, comments kept, no-op rerun, interrupted transfer + retry, source-byte race, linked destination | `an_omitted_setting_is_repaired_with_the_default_block_and_the_reference`, `a_null_setting_reuses_claudines_mapping_and_writes_only_the_reference`, `an_effectively_equal_block_is_repaired_by_writing_only_the_reference`, `a_transfer_preserves_every_other_setting_and_comment_and_converges`, `an_interrupted_transfer_keeps_the_inline_block_and_a_retry_completes_it`, `a_file_changed_between_plan_and_commit_is_refused_without_an_overwrite`, `a_linked_destination_or_lock_sidecar_is_manual_and_nothing_is_written`, `the_repository_scope_is_repaired_with_its_relative_reference` |
| Rule 4: redaction | `doctor::registration::tests::rows_never_show_an_authored_reference`, `doctor::permissions::tests::no_reference_or_block_value_reaches_the_report` |
| Darkmatter writer and referenced-mapping check | `darkmatter::l1 config_init::{replace_manifest_registration_existing_*, check_referenced_permissions_needs_a_valid_mapping_of_the_scope}` |
| Robustness matrix, Darkmatter and imported permissions: omitted and null are repairable handoffs on a valid file; `{}` valid; referenced `{}` uses defaults, missing/null/non-mapping target is broken | `the_doctor_planner_reads_null_and_omitted_as_handoff_requests`, `every_broken_reference_is_an_error_without_a_repair`, `check_referenced_permissions_needs_a_valid_mapping_of_the_scope` |
| Robustness matrix, registration: absent/null/`[]` `register` leave the entry unregistered (fixable); invalid `register` forbids updates to that file | `a_missing_entry_is_a_fixable_error…`, `a_missing_or_invalid_user_file_refuses_before_anything_is_written`, `replace_manifest_registration_existing_refuses_an_invalid_register_and_writes_nothing` |
| Process proof through `CliProcessFixture` | Phase 4 (the command is wired there) |

Code-smell review of the new load-bearing reads: no new `#[serde(default)]`;
`permissions: null` and an omitted `permissions` are told apart from the
authored document in the planner; no `unwrap_or_default()` on a
load-bearing read. The `.ok()` uses in the new inspection and repair code
(`inspect.rs::manifest_load`, `entry_registered`, the repair's entry
verification) and setup's existing `read_document` all fail closed: a
failed read makes the manifest refused, the entry unverified, or the file
reported unreadable, never silently accepted. The two `filter_map` calls
collect blockers, not parsed values. Darkmatter's existing
`replace_manifest_change` filters non-string `register` entries, which its
schema already refuses before the repair can start.

## Phase 3

Phase 3 builds the two host-probing rows, `tts` and `path`, and the Sniff
support the `path` row needs on native Windows. Both rows are wired into
`doctor::REGISTRY`, and the `pending_check` placeholder is deleted. That
finishes the move Phase 1 started of wiring each callback as soon as it
exists; Phase 4 still adds the clap command, dispatch, and rendering.

### Speech (`doctor/tts.rs`)

- `check_tts` passes the pass's `user_config` and
  `biscuit_speaks::get_available_providers()` to
  `assess(user_config, candidates)`. The candidate slice is the injection
  seam, and tests pass their own slices in runtime order.
- Enablement and the provider string come from
  `dispatch::loader::bridge_tts_settings`, the same projection hook `speak`
  reads. `tts.provider` goes through `parse_provider_name`. A recognized
  name keeps only that candidate (runtime `SpecificProvider` returns `[p]`
  or `[]` and never falls back). An unrecognized name adds a `warn` floor
  and judges the automatic candidates.
- "Highest rated" is the lowest `VoiceQuality::rank`, and a tie goes to
  the earlier candidate in runtime order. When the first runtime candidate
  differs from the highest-rated one, the detail says which candidate
  automatic selection tries first.
- Severities follow the spec table. When the user configuration is not
  usable, the row is `info` ("speech settings could not be determined").
  Every row ends with the note that detection checks only local
  prerequisites. A disabled row also says that lifecycle `say` can still
  speak.
- The unrecognized name is never shown. Provider names come from a fixed
  table in `tts.rs`.
- No engine is built, and nothing is prewarmed, synthesized, or played.

### Executable path (`doctor/path.rs`) and Sniff

- The current process uses `ExecutableIndex::build_path_only().find("claudine")`.
  A miss is `error`.
- On native Windows, `native_shell_lookups` runs
  `sniff::programs::windows_shells::detect()`, then `probe()` for each shell,
  in `Environment::launch_dir()`. That field is new and is captured from
  `RequestSnapshot::request_dir`. On other OSes the list is empty and the
  row has no shell detail.
- Every shell outcome is a `warn`, except `Found`, which is `ok`. Each
  outcome has its own summary and remedy. The finding is placed at the
  shell's executable and carries the startup-free note.
- New Sniff module `programs/windows_shells.rs`:
    - Public: `ShellFamily`, `ShellKind`, `InstalledShell`, `ShellLookup`
      (`Found`, `Missing`, `TimedOut`, `SpawnFailed { kind }`,
      `Failed { exit_code }`), `InvalidProgramName`, `detect()`, and
      `probe()`.
    - The selection rules are a pure function over a private
      `DiscoveryInputs`. Only `DiscoveryInputs::from_host` (the full
      `ExecutableIndex`, `SystemRoot`/`ProgramFiles`/`SystemDrive`, and the
      read-only Cygwin registry keys in both views) is `cfg(windows)`.
- Sniff process changes:
    - `run_command_with_timeout` now delegates to a new crate-private
      `run_command_in_tree(command, timeout, TreeConfig)`. Existing callers
      keep `TreeConfig::process_group()`.
    - The probe uses `process_group().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW)`
      and the new `timeouts::SHELL_LOOKUP` (2 s).
- `programs/git_program.rs`:
    - The name-only part of `launcher_layout` is now `installation_top`.
    - The new `installation_root` exposes it, so Git Bash is found from the
      same launcher layouts Sniff's Git hint uses.
    - `launcher_layout`'s behavior is unchanged, and its existing layout
      tests pass.

### Tests

- Sniff: `programs::windows_shells::tests` (25 tests).
    - Fake layout trees in a temporary directory cover every discovery rule
      on every OS.
    - The probe's argv, working directory, and environment removals are
      read back from the built `Command`.
    - Outcomes come from child fixtures. The test binary is re-executed with
      `--exact --ignored`, so no real shell runs. The fixtures cover exit 0,
      exit 3, exits 1 and 2, death by signal (Unix), a missing executable,
      and a 1.5 s injected timeout. For the timeout, the child's recorded
      PID is checked to be gone afterward.
- Claudine:
    - `doctor::tts::tests` (10) reads real `config.yaml` files through
      `config::observe`.
    - `doctor::path::tests` (7): injected shell answers for every outcome,
      plus the current-process lookup in a re-executed child whose `PATH` is
      only a temporary directory. On Unix this covers an executable, a
      non-executable file, and an empty directory. On Windows it covers a
      suffix-less `claudine` against `claudine.exe`.
- `config_tests.rs` and `registration_tests.rs` used to run the full
  `REGISTRY`. They now run `doctor::file_checks()`, the registry without
  `tts` and `path`. Otherwise every file-row test would detect speech
  engines and, on Windows, start the real shells. A `debug_assert!` pins
  the two dropped rows.
- `ConfigObservations`: the struct-level `expect(dead_code)` moved to
  `effective_config`, the only field that production code still does not
  read.

### Cross-OS check

The plan's Rule 18 leaves the local cross-check out because the recipe
makes a throwaway commit. This session's instructions require it when the
work has notable cross-OS risk. The recipe ships that commit as a bundle
and leaves the branch, index, and stash untouched. This phase adds
Windows-only code that Linux never compiles: the registry reads, the host
input gatherer, `native_shell_lookups`, and the suffix test. So I ran
`just cross-check sniff --os windows` and
`just cross-check claudine --os windows`, each filtered to the touched
modules. Nextest's positional filters were needed, because a `-E '…'`
expression's parentheses break the recipe's re-quoting.

Windows results:

- `sniff`: 86 of 86 pass, covering `windows_shells`, `process::`, and
  `git_program`.
- `claudine`, `doctor::`: the first run failed 19 tests. None of them is a
  Phase 3 test. Every new `tts`, `path`, and Sniff test passed there,
  including `a_native_suffix_is_required_on_windows`. The 19 were Phase 1–2
  config, permissions, and registration tests, and all had the same cause in
  the test code: expected paths were built by joining a `/`-separated
  literal (`.claudine/config.yaml`) onto a Windows path. Production prints
  native separators. The fix was a test-only `doctor::native_path`, which
  splits a `/` literal into native components, applied in
  `config_tests.rs`, `permissions_tests.rs`, and `registration_tests.rs`.
  On the second attempt all 93 tests pass on Windows. Linux runs 100,
  because seven tests there are Unix-only.
- I also checked the two probe scripts against the real shells on
  build-win over SSH, using read-only commands. `pwsh`, Windows PowerShell,
  and Git Bash each exit 0 for `git` and 3 for a name that does not exist.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  reports `packages: [claudine, darkmatter, sniff, test-toolkit]` and
  `change_class: package`. No gate was skipped.
- Claudine `just test`: 12835 passed, 16 skipped, none at or above 5 s.
  `just lint` is clean. `just check-tier-coverage claudine` reports nothing
  stranded. The Windows path fix touched only tests, and afterward the
  focused `doctor::` run passes 100 of 100.
- Sniff `just test`: 3904 passed, none slow. `just lint` is clean.
  `just check-tier-coverage sniff` reports nothing stranded.
- Dependents of Sniff: the Sniff changes keep existing behavior.
  `run_command_with_timeout` delegates with the same `TreeConfig`, and
  `launcher_layout` gives the same result through the extracted
  `installation_top`; Sniff's own process and git-layout tests cover both.
  The rest is additive. The one dependent that uses the new API is
  Claudine, and its whole area passed. I ran no other dependent package.
- Failures: the 19 Windows-only test failures above, now fixed. There were
  no failures on Linux.

### Requirement-to-test mapping (Phase 3)

| Requirement | Tests |
| --- | --- |
| Rule 14: automatic ranking and the runtime-first note | `doctor::tts::tests::automatic_selection_reports_the_highest_rated_and_the_runtime_first_candidate` |
| Rule 14: equal ratings | `equal_ratings_select_the_candidate_runtime_tries_first` |
| Rule 14: enabled with no candidate (`true`, `{}`, `provider:` null) | `every_enabled_automatic_shape_without_a_candidate_is_an_error` |
| Rule 14: low, moderate, good quality | `quality_below_good_warns_and_good_or_better_passes_when_enabled` |
| Rule 14: a recognized pinned provider is assessed alone | `a_recognized_provider_is_assessed_alone` |
| Rule 14: a missing pinned provider despite a fallback | `a_missing_pinned_provider_is_an_error_despite_an_available_fallback` |
| Rule 14: an unrecognized name (warn floor, no-candidate error, raw name hidden) | `an_unrecognized_provider_warns_at_least_and_uses_automatic_candidates` |
| Rule 14: every disabled shape (absent, `tts:`, null, false) × none, low, good | `disabled_hook_speech_is_informational_and_names_lifecycle_say` |
| Rule 14: an invalid or missing user configuration | `an_unusable_user_configuration_leaves_the_settings_undetermined` |
| Rule 14: the registered callback over real detection | `the_registered_check_reports_one_row_from_host_detection` |
| Rule 15: current-process lookup, found and missing (F2, a child with a controlled `PATH`) | `doctor::path::tests::{an_empty_path_directory_is_a_miss, an_executable_on_path_is_found_and_a_non_executable_file_is_not (unix), a_native_suffix_is_required_on_windows (windows)}` |
| Rule 15: row severities, every shell outcome, ordering | `a_found_executable_passes_and_names_where_it_is`, `a_miss_in_this_process_is_an_error_with_a_remedy`, `each_shell_outcome_is_distinct_and_never_more_than_a_warning`, `shell_findings_follow_path_order_and_a_current_miss_still_leads` |
| Rule 15: PowerShell preference and fallback | `sniff programs::windows_shells::tests::{pwsh_is_preferred_over_windows_powershell, indexed_windows_powershell_is_used_without_pwsh, windows_powershell_falls_back_to_its_system_root_layout}` |
| Rule 15: both Bash families, DLL classification, WSL launcher excluded, preference order, `SystemDrive` root | `git_bash_is_found_from_the_git_launcher_root`, `git_bash_is_found_under_program_files`, `git_bash_is_preferred_over_a_standalone_msys2`, `standalone_msys2_layouts_are_checked_in_order`, `the_indexed_bash_is_preferred_within_its_kind`, `a_bash_without_an_msys_or_cygwin_runtime_is_not_a_shell`, `cygwin_registry_roots_precede_the_drive_layouts`, `the_indexed_cygwin_bash_is_preferred_and_needs_its_runtime`, `every_family_is_reported_once_in_family_order`, `a_system_drive_is_joined_as_a_volume_root`, `nothing_installed_finds_no_shell`, `detect_finds_nothing_off_native_windows` |
| Rule 16: argument and environment policy, window flags, deadline | `a_powershell_probe_runs_no_profile_and_no_module_auto_loading`, `a_bash_probe_reads_no_startup_file_and_passes_the_name_as_an_argument`, `a_probe_runs_windowless_in_its_own_process_group`, `a_probe_refuses_anything_but_a_plain_program_name` |
| Rule 16: exit 0, exit 3, other exits, termination, spawn failure, timeout with child cleanup | `exit_zero_is_found`, `exit_three_is_missing`, `any_other_exit_is_a_failed_probe`, `a_terminated_shell_is_a_failed_probe_without_an_exit_code` (unix), `a_shell_that_cannot_start_is_a_spawn_failure`, `a_hung_shell_times_out_and_is_terminated` |
| Robustness matrix, Claudine TTS shapes through the doctor row | the disabled, enabled, unrecognized, and unusable tests above. Wrong-type, empty-string, and `[true]` shapes give `info` here, and `config` reports them as errors (`config_tests::tts_shapes_are_judged_by_the_user_schema`) |

Code-smell review: the new code adds no `#[serde(default)]`. The speech row
reads `Option<String>` provider through the bridge, where absent and null
are the same by contract. No `unwrap_or_default()` sits on a load-bearing
read. The `.ok()` in `native_shell_lookups` drops only an
`InvalidProgramName`, which cannot happen for the constant `claudine`. The
`.ok()` in `cygwin_registry_roots` reads a missing key or value as "no
root", the meaning absence has.

## Phase 4

Phase 4 wires `claudine doctor [--fix] [--json]` into the CLI, proves the
command end to end through the shipped binary, and writes the user
reference. The seven callbacks were already in `doctor::REGISTRY` in
registry order (Phase 3), so wiring meant the CLI adapter only.

### Doctor Dispatch

- New `claudine/cli/src/commands/doctor.rs` (`DoctorArgs { fix, json }`).
  It calls `claudine::doctor::run(request::snapshot(), fix)` and renders the
  rows. Exit status is `1` when any row is `error`, else `0`. Clap usage
  errors keep clap's status `2`.
- `main.rs::dispatch` returns doctor right after `registrations`. That is
  after `request::initialize` and `log::set_plain`, but before
  `request::darkmatter_config()` (which can create `~/.darkmatter.yaml`) and
  before `ensure_config_exists` (setup, conversion). The exhaustive terminal
  match has `Commands::Doctor(_) => unreachable!(…)`, and `telemetry.rs` names
  the command. The ordinary routes and the hidden `registrations` route are
  unchanged.
- Human output is built with biscuit-terminal: one `UnorderedList` line per
  row (`[severity] Title (id): summary`), its detail lines as a nested list,
  and a closing tally. Text goes through `Prose::escape_text_outside_code_spans`,
  so authored path characters are never read as markup. When stdout is not a
  terminal, `log::set_plain(true)` runs before rendering, so redirected
  output has no ANSI codes even with `FORCE_COLOR`. A plain terminal honors
  `COLUMNS`/`TERM_WIDTH` (`log::forced_width` is now `pub(crate)`).
- stdout is written directly with `?` (as `completions` does), not through
  `log::data`, which drops write errors. A failed write therefore reaches
  `render_top_level_error`, with exit 1.
- Grouped help (`help.rs`, Administration, after `setup`) and the root
  completion menu (`root_menu.rs`, Administration, after `config`) list
  `doctor`. Its options complete through the existing clap fallback.

### CLI Contracts (`cli/tests/l1/doctor.rs`, 10 tests, declared in `tests/l1/main.rs`)

- Help (`doctor --help` with no configuration, plus the grouped overview)
  and usage (`--repair` → 2, empty stdout).
- Healthy home inside a repository: seven rows, exit 0, home tree unchanged.
  This is the one spawned case that keeps the real Windows shell probes;
  it tolerates `ok`/`warn` on `path`.
- Warnings only (format 1 leftover) → exit 0. The JSON shape is checked
  (exactly five keys, types, single-line summary, id order). The five file
  rows equal the in-process `claudine::doctor::run` rows for the same home.
  The human report is checked twice, redirected with `FORCE_COLOR=1` and
  with `--plain`: no ESC byte, row heads in JSON order, every registration
  detail line, and the tally.
- Missing home with `fake_only_path()`: `config`, `registration`, and
  `path` are errors, exit 1. Nothing is created in the home, no audio spool
  appears, and no setup text is on stderr.
- Broken home (legacy `config.json` with no YAML, an invalid
  `~/.darkmatter.yaml`): exit 1, the `config` row names `config.json`, the
  tree is unchanged, and nothing is published.
- Redaction: one case per sentinel (a schema error quoting its value, and a
  broken `permissions` reference). Neither sentinel is on stdout or stderr,
  in JSON or human output, and the targeted row is an error.
- Repair and rerun: both repairs in one run (unpublished, unregistered
  registration plus an inline Darkmatter block). Inspection is exit 1 with
  both rows fixable. `--fix` is exit 0 with `Repaired:` lines, comments
  kept, block moved, and the entry written. A second `--fix` repairs nothing
  and leaves the tree unchanged.
- Failed repair: the parent holds `~/.darkmatter.yaml.darkmatter-lock`
  (`File::lock`) while the child runs `--fix` for a registration-only edit
  (`register: []`, legacy files present). Exit 1, `the repair failed: …lock…`,
  `fixable: true`, and the tree (entry and legacy paths) unchanged. The lock
  is released after the child exits. This test waits out Darkmatter's fixed
  2 s `LOCK_DEADLINE` (2.1 s on Linux, 3.2 s on Windows; mostly sleep). No
  CLI seam exists to shorten it, and the plan asks for this exact case.
- stdout write error (Linux only, `/dev/full`): exit 1 with an `Error:`
  block on stderr.
- Library, in-process report boundary:
  `doctor::permissions::tests::a_refused_candidate_reaches_the_report_without_its_refusal_text`
  passes a refused Claudine candidate and a refused Darkmatter candidate,
  each with a sentinel in its refusal text, through `failure` and
  `report::row`. The row is an error, says "would be invalid; nothing was
  saved", and contains no sentinel.
- Completion: `completion::engine::tests::doctor_is_offered_at_the_root_and_its_options_after_it`
  (unit) and `completion_cli::doctor_and_both_of_its_options_complete_through_the_binary`
  (spawned). The root menu test and the help snapshot
  (`l1__wrap_basics__help_lists_wrapper_subcommands.snap`) were updated.
- Renderer unit tests in `commands/doctor.rs`: exit status, row and detail
  order without ANSI, and the no-problem tally.

### Windows (cross-check, two attempts)

`just cross-check claudine-cli --os windows doctor`:

- **First attempt: 5 of 14 failed.** Each spawned `doctor` started the
  host's real PowerShell, Git Bash, and Cygwin probes. PowerShell writes
  `AppData\Local\Microsoft\PowerShell\StartupProfileData-NonInteractive`
  under `%LOCALAPPDATA%`, even with `-NoProfile`, and `CliProcessFixture`
  points that at the fixture home, so every "home unchanged" assertion
  failed. The probes also cost about 2.5 s per spawn, which put two
  multi-spawn tests over 5 s.
- **Fix.**
    - A `test-fixtures`-only seam, `CLAUDINE_TEST_DOCTOR_SKIP_SHELL_PROBES`,
      read in `doctor/path.rs::native_shell_lookups`, Windows only. It has a
      `context_construction_guard` allowance.
    - Every spawned case sets it except the healthy-home case. That case
      keeps the real probes and leaves `AppData` out of its tree comparison.
    - `tree()` lists a lock sidecar without reading it, because Windows
      refuses to read a file another handle holds locked.
- **Second attempt: 14 of 14 pass** (slowest 3.2 s). The new fact is
  recorded in `.claude/skills/os/windows.md`, and the PowerShell cache
  behavior is documented in `docs/cli/doctor.md`. `os-failing` is not set.

### Reference Updates

- New `claudine/docs/cli/doctor.md`. It covers:
    - options and runnable examples with expected results;
    - what an inspection never does, and redaction;
    - exit statuses and row order, severities, and the human and JSON
      reports, with one JSON example;
    - each check's severity table, including registration findings, the
      permissions handoff (with the working-copy meaning), `tts` limits, and
      the `path` and Windows probes (no startup files, no window, 2 s, never
      above `warn`, a link to Sniff's `windows-shells.md`);
    - repair order and limits, with a Mermaid flow for links → publish →
      remove extras → verify → register → verify → legacy cleanup.
- Updated:
    - `docs/cli/index.md` (listing and paragraph) and `README.md` (new
      Troubleshooting section);
    - `docs/topics/configuration.md`: a doctor paragraph under "How a file
      is read", and doctor's handoff repair under "Darkmatter's permissions";
    - `docs/topics/schemas.md`: doctor in recovery, a table row, and extras
      being removed by `--fix`;
    - `darkmatter/docs/topics/host-registration.md`: links the doctor repair.
- Skills:
    - `.claude/skills/claudine/SKILL.md`: the CLI adapter, early dispatch,
      direct stdout, test seams, and `doctor --fix` as a publisher;
    - `.claude/skills/claudine/cli-commands.md`: a command row, and doctor
      added to the list of publication routes;
    - `claudine/docs/use-skills/use-claudine/cli.md`: an inspection table
      row and a "Checking the setup" section. It is a summary, not a
      transclusion, because the canonical page is too long to transclude
      whole. `just publish-skills` is Phase 5's task and was not run here.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  reports `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`. No gate was skipped.
- Claudine `just test`: run twice, the second time after the Windows fix.
  The final run passed 12851 tests, skipped 16, and recorded none at or
  above 5 s. `just slow-tests fresh` exits 0.
- `just lint` is clean, and `just check-tier-coverage claudine` reports
  nothing stranded.
- Dependents:
    - The calculator lists `darkmatter` because three edited docs are read
      by Darkmatter tests. I ran `trigger_spelling_guard`,
      `shipped_schema_corpus`, `meta_schema_phase1`/`phase4`, and
      `base_schema_end_to_end` (55 passed).
    - `just test test-toolkit`: 591 passed, none slow. A bare
      `cargo nextest -p test-toolkit` also runs the `real_` test
      `real_kache_config_selector_bypasses_the_managed_pin`, which fails on
      this host. L1 excludes it, and this phase did not touch it.
    - No public library API changed, and nothing depends on `claudine-cli`.
- Failures: none on Linux. On Windows, the 5 failures above were fixed on
  the second attempt.

### Requirement-to-test mapping (Phase 4)

| Requirement | Tests |
| --- | --- |
| Early dispatch: no setup, conversion, creation, refresh, or worker from missing and invalid homes | `doctor::inspecting_a_missing_home_reports_errors_and_creates_nothing`, `doctor::inspecting_a_broken_home_leaves_every_file_as_found` |
| Happy path, all seven rows, clean temp files | `doctor::a_healthy_home_reports_every_row_and_exits_zero` |
| Help, both options, grouped Administration help | `doctor::help_names_both_options_and_needs_no_configuration`, `wrap_basics::help_lists_wrapper_subcommands` (snapshot) |
| Unknown option exits 2 | `doctor::an_unknown_option_is_a_usage_error_with_nothing_reported` |
| Completion of `doctor`, `--fix`, `--json` | `completion::engine::tests::doctor_is_offered_at_the_root_and_its_options_after_it`, `completion_cli::doctor_and_both_of_its_options_complete_through_the_binary`, `root_menu::tests::full_menu_has_administration_in_spec_order` |
| Warnings only exit 0; JSON shape and order match the library; human report uses the same rows, with no ANSI when redirected or under `--plain` | `doctor::warnings_alone_exit_zero_and_both_formats_show_the_library_rows`, `commands::doctor::tests::*` |
| Unresolved errors exit 1; deliberate `PATH` miss with fake-only `PATH` | `doctor::inspecting_a_missing_home_reports_errors_and_creates_nothing` |
| Repaired errors exit 0; both repairs in one run; second run is a no-op | `doctor::a_repaired_home_exits_zero_and_a_second_run_changes_nothing` |
| Failed repair exit 1 under a held Darkmatter lock; entry and legacy kept; still fixable | `doctor::a_repair_blocked_by_a_held_lock_fails_and_keeps_the_entry_and_the_legacy_files` |
| Secret sentinels out of stdout and stderr (schema error, invalid reference) | `doctor::no_authored_value_reaches_either_report_or_stderr` |
| Refused candidate with a sentinel at the in-process report boundary | `doctor::permissions::tests::a_refused_candidate_reaches_the_report_without_its_refusal_text` |
| stdout write error goes through the CLI error path | `doctor::a_failed_write_to_stdout_is_an_error` (Linux) |

The input robustness matrix does not apply to this phase: no parser or
loader changed. The CLI only renders rows the library computes, and each
row's field and document shapes are the Phase 1–3 tests.

## Phase 5

Verification and delivery. Every command ran non-interactively; no
terminal-harness (L2/L3) test ran, so nothing could take focus.

### Audit

- `git diff --check` over the working tree, and over the committed Phases
  1–3 (`d6110dd40~4..d6110dd40`), reports nothing. The three untracked
  Phase 4 files have no trailing whitespace.
- Robustness smells (`unwrap_or_default()`, `.ok()`, `filter_map(.. as_str)`,
  `#[serde(default)]`) in `lib/src/doctor/`, `composition/registration/
  {inspect,repair}.rs`, and `cli/src/commands/doctor.rs`: each hit is a
  display fallback or an `Option` the caller already treats as "not
  verified" (`repair.rs` turns `None` into `EntryUnverified`). The one
  `.ok()?` that drops a shell (`doctor/path.rs`) is on `windows_shells::probe`,
  whose only error is `InvalidProgramName`, impossible for the constant
  `claudine`; timeouts and crashes are `ShellLookup` variants, so the three
  outcomes stay distinct.
- Row contracts against the spec: each of the seven rows has library tests
  (`doctor/{tests,config_tests,registration_tests,permissions_tests,tts_tests,path_tests}.rs`,
  `composition/registration/repair_tests.rs`, 107 tests) and the CLI
  contract in `cli/tests/l1/doctor.rs` (help without configuration, usage
  exit 2, healthy home, warnings only, missing home creates nothing, broken
  home left as found, no authored value in either report or stderr, repair
  then a no-op second run, failed repair under a held lock, stdout write
  error).
- Found and fixed: `stray-config-keys` listed a file's unknown keys sorted by
  JSON pointer, so `/actions/…` came before `/tts` even when `tts` was on
  line 1, and the row summary named whichever key sorted first. They are now
  listed by line and column (`doctor/config.rs::check_stray_keys`); the
  existing nested-key test now asserts the order and the summary
  (`config_tests.rs::nested_unknown_keys_are_found_in_the_tts_mapping_the_imported_permissions_and_a_keyed_action`),
  and `docs/cli/doctor.md` says so.
- Observed, not changed: the human report's Prose word wrap breaks a long
  path at a hyphen (`/tmp/p5-doctor-` / `i85y/…`), because biscuit-terminal's
  `WordWrap::WrapProse` treats `-` as a break character. `--json` is
  unaffected. See `decisions-for-review`.

### Representative output

Produced by the `target/debug/claudine` that the suite built, in temp homes
under `/tmp/p5-doctor-i85y/` (`env -i HOME=… PATH=<fixture bin>:/usr/bin:/bin
COLUMNS=100`, launch directory outside any repository unless noted). Every
run's stdout was a pipe, so each human report is the redirected (plain)
form: no ANSI, severity as bracketed text. The `tts` row reads this host
(eSpeak NG only, hook speech off, so `info`).

**Warning only, redirected human report** (published registration, a
leftover `~/.claudine/darkmatter-host.yaml`), exit 0:

```text
• [ok] Configuration files (config): every configuration file is valid
    - /tmp/p5-doctor-i85y/warn/home/.claudine/config.yaml: valid
    - /tmp/p5-doctor-i85y/warn/home/.darkmatter.yaml: valid
• [warn] Darkmatter host registration (registration): is left over from the retired format 1
  registration
    - /tmp/p5-doctor-i85y/warn/home/.claudine/darkmatter-host.yaml: is left over from the retired
      format 1 registration
    - the repair removes it once the `register` entry names the published manifest
    - Remedy: run `claudine doctor --fix`
• [ok] Darkmatter permissions handoff (darkmatter-permissions): Darkmatter takes its permissions
  from Claudine's configuration
    - /tmp/p5-doctor-i85y/warn/home/.darkmatter.yaml: the user `permissions` refers to Claudine's
      configuration
• [info] Repository configuration (repo-config): not inside a repository
    - not inside a repository
• [ok] Unknown configuration keys (stray-config-keys): no unknown Claudine configuration keys
    - /tmp/p5-doctor-i85y/warn/home/.claudine/config.yaml: no unknown keys
• [info] Text-to-speech (tts): the best available speech provider, eSpeak NG, has low quality
    - the best available speech provider, eSpeak NG, has low quality
    - Remedy: install a higher-quality provider such as Kokoro, Echogarden, or Piper
    - provider detection checks local prerequisites only
    - an executable or a credential being present does not prove a working voice, network access,
      or audio playback
    - hook speech is off because `tts` is absent, null, or false; lifecycle `say` can still speak
• [ok] Executable path (path): this process finds `claudine` at /tmp/p5-doctor-
  i85y/warn/bin/claudine

0 errors, 1 warning.
```

**The same home, `--json`** (first two of the seven rows), exit 0:

```json
[
  {
    "id": "config",
    "severity": "ok",
    "summary": "every configuration file is valid",
    "detail": "/tmp/p5-doctor-i85y/warn/home/.claudine/config.yaml: valid\n/tmp/p5-doctor-i85y/warn/home/.darkmatter.yaml: valid",
    "fixable": false
  },
  {
    "id": "registration",
    "severity": "warn",
    "summary": "is left over from the retired format 1 registration",
    "detail": "/tmp/p5-doctor-i85y/warn/home/.claudine/darkmatter-host.yaml: is left over from the retired format 1 registration\nthe repair removes it once the `register` entry names the published manifest\nRemedy: run `claudine doctor --fix`",
    "fixable": true
  }
]
```

**Repair** (no published registration, `register: []`, and an inline
`permissions` block in `~/.darkmatter.yaml`). Before, exit 1, the two
fixable rows:

```text
• [error] Darkmatter host registration (registration): the published registration is missing
    - /tmp/p5-doctor-i85y/fix/home/.claudine/registrations/darkmatter: the published registration
      is missing
    - Remedy: run `claudine doctor --fix`
    - /tmp/p5-doctor-i85y/fix/home/.darkmatter.yaml: the `register` entry is verified once the
      manifest is published
• [error] Darkmatter permissions handoff (darkmatter-permissions): the user `permissions` is an
  inline block that Claudine's configuration should hold
    - /tmp/p5-doctor-i85y/fix/home/.darkmatter.yaml: the user `permissions` is an inline block that
      Claudine's configuration should hold
    - the repair writes the block into /tmp/p5-doctor-i85y/fix/home/.claudine/config.yaml, then
      refers to it
    - Remedy: run `claudine doctor --fix`, or `claudine setup`
…
2 errors, 0 warnings.
```

`--fix`, exit 0 (the run of `Repaired: wrote …` lines is cut to its first
two and the manifest):

```text
• [ok] Darkmatter host registration (registration): the registration is published and registered
    - Repaired: wrote /tmp/p5-doctor-
      i85y/fix/home/.claudine/registrations/darkmatter/globals.lifecycle.json
    - Repaired: wrote /tmp/p5-doctor-
      i85y/fix/home/.claudine/registrations/darkmatter/schemas/action.yaml
    - Repaired: wrote /tmp/p5-doctor-i85y/fix/home/.claudine/registrations/darkmatter/manifest.yaml
    - Repaired: updated the `register` entry in /tmp/p5-doctor-i85y/fix/home/.darkmatter.yaml
• [ok] Darkmatter permissions handoff (darkmatter-permissions): Darkmatter takes its permissions
  from Claudine's configuration
    - /tmp/p5-doctor-i85y/fix/home/.darkmatter.yaml: the user `permissions` refers to Claudine's
      configuration
    - Repaired: /tmp/p5-doctor-i85y/fix/home/.claudine/config.yaml: set `permissions`
    - Repaired: /tmp/p5-doctor-i85y/fix/home/.darkmatter.yaml: set `permissions`
…
No problems found.
```

A second `--fix --json` reported no `Repaired:` line and exit 0.

**Failed repair** (registration published but `register: []`, a legacy
manifest present, and Darkmatter's writer lock on `~/.darkmatter.yaml` held
by `flock` for the whole run), exit 1 after Darkmatter's 2 s lock deadline
(2.09 s wall time); the entry and the legacy file were left as found:

```text
• [error] Darkmatter host registration (registration): the repair failed: could not update the
  `register` entry in /tmp/p5-doctor-i85y/locked/home/.darkmatter.yaml (another writer held its
  lock); the published registration is kept and the retired one left in place
    - /tmp/p5-doctor-i85y/locked/home/.claudine/darkmatter-host.yaml: is left over from the retired
      format 1 registration
    - the repair removes it once the `register` entry names the published manifest
    - Remedy: run `claudine doctor --fix`
    - /tmp/p5-doctor-i85y/locked/home/.darkmatter.yaml: no `register` entry names Claudine's
      manifest
    - Remedy: run `claudine doctor --fix`
    - Repair failed: could not update the `register` entry in /tmp/p5-doctor-
      i85y/locked/home/.darkmatter.yaml (another writer held its lock); the published
      registration is kept and the retired one left in place
    - No change was made before the failure
…
1 error, 0 warnings.
```

**Repository with stray keys** (`{repo}/.claudine/config.yaml` holding
`tts: true` then `loging: false`), `--json`, exit 1, after this phase's
source-order fix:

```json
[
  {
    "id": "stray-config-keys",
    "severity": "error",
    "summary": "unknown key `tts` at the top level (line 1, column 1) (+1 more)",
    "detail": "/tmp/p5-doctor-i85y/warn/home/.claudine/config.yaml: no unknown keys\n/tmp/p5-doctor-i85y/warn/work/repo/.claudine/config.yaml: unknown key `tts` at the top level (line 1, column 1)\nRemedy: it is a user setting; move it to `~/.claudine/config.yaml` or remove it\n/tmp/p5-doctor-i85y/warn/work/repo/.claudine/config.yaml: unknown key `loging` at the top level (line 2, column 1)\nRemedy: remove it, or correct its spelling",
    "fixable": false
  }
]
```

### Gates

All ran on Linux, serially, with no subagents (load average 4–10 on this
host). Each ran on the final tree except the first Claudine `just test`,
which started before the `stray-config-keys` sort fix; the 117 tests that
cover doctor ran again afterwards.

| Area | `just build` | `just test` (L1) | `just lint` |
|---|---|---|---|
| claudine | pass | 12851 passed, 16 skipped, none ≥ 5 s; after the fix, `just test doctor` 117 passed, none ≥ 5 s | pass, no warnings |
| darkmatter | pass | 17859 passed, 10 skipped, none ≥ 5 s | pass, no warnings |
| sniff | pass | 3904 passed, 38 skipped, none ≥ 5 s | pass |
| simplified-schema | pass | 2592 (+2172) passed, none ≥ 5 s | pass |

- `just check-tier-coverage claudine`: nothing stranded.
- Scope calculator over the uncommitted tree (Phases 4 and 5):
  `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates could not be skipped.
  `just test test-toolkit`: 591 passed, none ≥ 5 s.
- Consumers: Darkmatter's and Claudine's full suites above are the consumers
  of the Sniff (`windows_shells`) and SimplifiedSchema (union unknown keys)
  public changes. Nothing in this phase changed a public API, so no other
  dependent package was run.
- Cross-OS: not run this phase. The only code change is the sort order of
  findings inside one function, with no path, process, or OS-specific code.
  Phase 4's `just cross-check claudine-cli --os windows doctor` (14/14) is
  the latest Windows evidence; macOS, native Windows, and WSL2 are left to
  CI as the repository's event schedule decides.

### Skill publication

`just publish-skills` (from `claudine/`) passed and installed
`~/.claude/skills/use-claudine/`. Its `cli.md` carries the doctor row in the
command table and the "Checking the setup: `claudine doctor`" section with
the seven row IDs in order, the exit statuses (0/1/2), the JSON object
shape, the two repair scopes, and the `jq` example, word for word from
`claudine/docs/use-skills/use-claudine/cli.md`; the section has no links or
transclusions to break. The 2 s shell limit it states matches Sniff's
`timeouts::SHELL_LOOKUP`. The research summaries the recipe also writes
(`.claude/skills/claudine/summaries/`) came out unchanged. One
pre-existing warning printed (`err` undeclared in
`docs/research/summary/plugins.md`), unrelated to doctor.

### Requirement-to-test mapping (this phase)

- Unknown keys are listed in file order, and the summary names the first one
  in the file: `doctor::config::tests::nested_unknown_keys_are_found_in_the_tts_mapping_the_imported_permissions_and_a_keyed_action`
  (order asserted by position; the summary now `pitch` at line 3, which
  sorted after `/actions/…` before). F1 at the library's public `run` result;
  the CLI only renders those rows (`warnings_alone_exit_zero_and_both_formats_show_the_library_rows`
  proves the CLI rows equal the library rows).
- No spec departures in this phase. Skipped or pre-existing failures: none.
