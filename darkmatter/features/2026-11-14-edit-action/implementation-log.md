---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/spec.md"
plan: "darkmatter/features/2026-11-14-edit-action/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - sniff/lib/src/os/display.rs
  - sniff/lib/src/os/mod.rs
  - sniff/lib/src/programs/enums/editor_traits.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/mod.rs
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/src/lib.rs
  - tools/test-toolkit/src/fake_editor.rs
  - tools/test-toolkit/src/bin/test-toolkit-fake-editor.rs
  - tools/test-toolkit/tests/fake_editor.rs
  - darkmatter/cli/Cargo.toml
  - darkmatter/cli/tests/bin/fake_editor/main.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/tests/bin/fake_editor/main.rs
  - Cargo.lock
docs_updated_during_phase_1:
  - sniff/docs/sniff-library-architecture.md
  - tools/test-toolkit/README.md
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
  - claudine/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/programs.md
  - .claude/skills/darkmatter/testing.md
packages:
  - sniff
  - test-toolkit
  - darkmatter-cli
  - claudine-cli
  - darkmatter
  - claudine
decisions_for_review:
  - |-
    `Editor::identify` treats its whole argument as one program path and compares only its final component; it does not split on whitespace. The plan (R7) said it "takes the first whitespace-separated token's path", but also asked for a test with "a path with spaces in the directory", which cannot be recognized if the input is split on spaces. Splitting the `$EDITOR` command line is left to the caller (the Phase 2 resolver, which already takes the first whitespace token). Alternative: split inside `identify`, which makes `identify("code --wait")` work but makes `/opt/My Editors/bin/subl` unrecognizable. Redoing it means changing one line in `identify` and two test assertions.
  - |-
    The test toolkit got its own copy of the fake editor binary (`test-toolkit-fake-editor`, ungated) so its tests can launch the real program, plus a Unix-only `libc` dependency (already in the graph through `biscuit-test-harness`) for ignoring `SIGTERM`. The plan named only the Darkmatter and Claudine binaries. Alternative: test the body in-process only, which cannot prove the spawn-child or ignore-SIGTERM behaviors. Reversing means deleting one `[[bin]]` and the integration test file.
  - |-
    Fake editor details the plan left open: `FAKE_EDITOR_REPLACE` names a content file (like `FAKE_EDITOR_WRITE`) that is saved through a temporary file and a rename; `FAKE_EDITOR_READ_LINE` runs before `FAKE_EDITOR_STDIO` so the report can show the line read; the stdio report is a small JSON file read back with `read_stdio_report`; a held or spawned process exits on its own after 120 seconds so a failed test cannot leak it; a spawned child reports readiness over a private pipe before its PID is written, so a test never races its `SIGTERM` setup. Each is a local change in `tools/test-toolkit/src/fake_editor.rs` if a reviewer prefers otherwise.
  - |-
    On Windows the editor's Job Object is created with kill-on-close, as planned, but a successful edit clears that limit before closing the Job instead of letting the close kill whatever is left. The plan said "Job object (kill-on-close)" without saying what happens after a normal exit. With plain kill-on-close, closing a file in VS Code would also kill a VS Code window that its waiting client had started, because that window is in the Job. Alternatives: no kill-on-close at all (simpler, but an editor survives if the host crashes mid-edit), or keep plain kill-on-close (kills a GUI editor's window after every successful edit). Reversing it means removing `ProcessTree::release` in `darkmatter/lib/src/process_tree.rs` and its one call in `editor/launch.rs`.
  - |-
    Resolution's order and filtering are unit-tested in-process over a private core (`resolve_editor_from`) that takes the two variables, a program lookup, and a probe lookup as inputs, and the CLI tests cover only cases where a variable decides the outcome. The plan asked for environment-variable tests plus portable stubs; tests that reach the built-in probe list through the real environment would depend on which editors the test host has installed, so they cannot be deterministic. Alternative: control `PATH` in the CLI tests so the probes see only installed fakes (much more fixture code, and `PATH` lookups differ per OS). Reversing means adding such a fixture; the core stays either way.
  - |-
    The plan asked for a "test hook creating the file between the existence check and the call" to prove a concurrently created file is opened, not replaced. The service has no separate existence check: it makes one exclusive create, so the race window the hook would target does not exist. The evidence instead is a two-thread race on the exclusive create (exactly one creator wins) plus a service test where the name is already held by another writer's bytes (opened unchanged; refused under `create_new`). Alternative: add a test-only seam inside `create_new_guarded`, which would test a seam rather than a real window. Nothing to redo unless a reviewer wants the seam.
  - |-
    The fake editor gained one variable, `FAKE_EDITOR_PAUSE` (create a marker, wait until the test removes it, then exit normally), which the plan did not list. Without it no test can change the target while the editor is "open" and still see a normal exit, so the post-exit directory and containment checks could not be tested. Alternative: leave those two checks untested. Reversing means deleting the variable from `tools/test-toolkit/src/fake_editor.rs` and the one test that uses it.
  - |-
    The plan's `EditError` list had no cancellation variant, so a cancellation (before the editor starts or while it is open) is reported as `EditError::Editor(EditorError::Cancelled { path })`, with `EditError::is_cancelled()` for callers. A file deleted by the editor is `EditError::Editor(EditorError::Missing)`, the existing "edited file was deleted" error, rather than a new variant. Alternative: dedicated `EditError::Cancelled` and `EditError::Deleted` variants (clearer matching, two more names). Reversing touches `editor/service.rs` and the matching assertions in `cli/tests/l1/editor_service.rs`; Phase 4 maps these to `ActionFailure`, so decide before then.
  - |-
    `edit`'s literal-boolean errors are located at the flag's own key (`initialize[0].else[0].edit.gui`), and a long-form `file` that is a number, a boolean, or blank at `…edit.file`; the short form's operand error stays at `…edit`. The plan asked for exactly this location, but the nearest precedent, `break.code`, reports at the action (`…break`) and names the key only in the message. Alternative: report every `edit` error at the action for consistency with `break` (coarser DMLS squiggles). Reversing means changing two `property:` expressions (`parse.rs` flag check, `action_shape.rs` long-form `edit_file` call) and the expected paths in `darkmatter/lib/tests/l1/lifecycle_edit_action.rs`.
  - |-
    Two schema-agreement tests compare the parser's verbs with the shipped schemas, which do not type `edit` until the schema work later in the plan: `darkmatter/lib/tests/l1/lifecycle_schema_parity.rs::the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb` and `claudine/lib/tests/l1/claudine_schemas.rs::hand_verbs_equal_the_parser_inventory`. Each now exempts `LIFECYCLE_ONLY_VERBS` and also asserts the verb is still untyped, so the test fails (with a message saying to drop the exemption) as soon as a schema types `edit`; the exemption cannot outlive the gap. The plan expected these checks to be updated now without saying how. Alternative: write the `edit` schema types now and regenerate the shipped artifacts, moving part of the schema phase forward. Until the schemas land, `md schema validate` and DMLS accept any `edit` value through the md open arm and reject it in Claudine's closed action types.
  - |-
    The schema projection's collision check now treats a lifecycle-only verb like a control verb: a catalog function or effect named `edit` fails generation ("a lifecycle-only verb"). The plan only asked for a test that no descriptor is named `edit`; this makes the generator refuse it too. Reversing means removing one branch in `schema_projection.rs::hand_owner` and one row of `lifecycle_schema_generation.rs`.
message_to_agent: |-
  Phase 3 is in place: `LifecycleActionKind::Edit(EditAction { file: Expr, create_new: bool, gui: Option<bool> })` parses from the short, one-element-list, and long forms (`darkmatter/lib/src/lifecycle/action_shape.rs`, `parse.rs`), is an `Expression` action, and `LIFECYCLE_ONLY_VERBS = ["edit"]` lives in `lifecycle/signatures.rs`. Things later phases need:
  - Phase 4: `execute_action_inner` (`lifecycle/executor.rs`) has a placeholder `Edit` arm that refuses with an evaluation failure ("this host cannot run `edit`"); replace it with `dispatch_edit` (R14/R17/R18/R19). `dispatch_task_side_effect_inner` already rejects `Edit` through its wildcard. A literal `file` is guaranteed a non-blank `Expr::StringLiteral` (bytes kept, never trimmed); `{{ fqn }}` arrives as an expression (`Expr::Variable`), mixed text as a `StringLiteral` template, so the runtime string/blank check (R14) is still needed for interpolated values.
  - Phase 5: Claudine's `describe_action` (`claudine/lib/src/composition/sequence/task/mod.rs`) already has the R22 `Edit` arm and message; only the refusal test is left.
  - Phase 6: two tests exempt `LIFECYCLE_ONLY_VERBS` until the schemas type `edit`, and each fails on purpose once a schema does: `darkmatter/lib/tests/l1/lifecycle_schema_parity.rs::the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb` (remove the `provided` filter and the `untyped` row once `md-unknown-action` excludes `edit`) and `claudine/lib/tests/l1/claudine_schemas.rs::hand_verbs_equal_the_parser_inventory` (remove the `untyped`/`missing` split). `template_operands` already reads `edit` (`file` only; `create_new`/`gui` excluded), and the schema projection refuses a catalog verb named `edit`.
  - Error locations: flag errors are `LifecycleInvalidArgs` at `…edit.<flag>`; long-form `file` type/blank errors at `…edit.file`; short-form and arity errors at `…edit`. Phase 6's schema/DMLS wiring cases should agree with those paths.
source_files_during_phase_2:
  - darkmatter/lib/src/process_tree.rs
  - darkmatter/lib/src/lib.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
  - darkmatter/lib/src/effects/mod.rs
  - darkmatter/lib/src/effects/fs_write.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/editor/resolve.rs
  - darkmatter/lib/src/editor/launch.rs
  - darkmatter/lib/src/editor/surface.rs
  - darkmatter/lib/src/editor/service.rs
  - darkmatter/lib/tests/l1/context_construction_guard.rs
  - darkmatter/lib/tests/l1/error_snapshots/editor.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/editor_support.rs
  - darkmatter/cli/tests/l1/editor_resolution.rs
  - darkmatter/cli/tests/l1/editor_launcher.rs
  - darkmatter/cli/tests/l1/editor_service.rs
  - tools/test-toolkit/src/fake_editor.rs
  - tools/test-toolkit/tests/fake_editor.rs
docs_updated_during_phase_2:
  - darkmatter/docs/cli/edit.md
  - tools/test-toolkit/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/testing.md
  - .claude/skills/darkmatter/library-surfaces.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/lifecycle/actions.rs
  - darkmatter/lib/src/lifecycle/signatures.rs
  - darkmatter/lib/src/lifecycle/mod.rs
  - darkmatter/lib/src/lifecycle/action_shape.rs
  - darkmatter/lib/src/lifecycle/parse.rs
  - darkmatter/lib/src/lifecycle/validate.rs
  - darkmatter/lib/src/lifecycle/operands.rs
  - darkmatter/lib/src/lifecycle/source_map.rs
  - darkmatter/lib/src/lifecycle/schema_projection.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/tests/l1/lifecycle_edit_action.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
  - claudine/lib/src/composition/sequence/task/mod.rs
  - claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/lifecycle.md
---

# Implementation Log for 2026-11-14-edit-action (7 phases)

## Phase 1

### What was built

- **Sniff editor facts** (R5–R7): `sniff/lib/src/programs/enums/editor_traits.rs` adds
  `EditorSurface` (`terminal`/`gui`/`both`, serde and `Display` snake_case),
  `EditorTraits`, and the `EDITOR_TRAITS` table indexed like `EDITOR_INFO`;
  `Editor::{traits, surface, wait_args, can_block, identify}`. Re-exported from
  `sniff::programs`. `ProgramInfo` and every serialized program shape are unchanged
  (`program_serialization` tests pass unchanged within the full sniff run).
- **Sniff display session** (R8): `sniff/lib/src/os/display.rs` with `DisplaySession`,
  `detect_display_session()`, and the pure `display_session_from(os, runtime, var)`.
  Not in `OsInfo` or any cache.
- **Fake editor** (R21): `tools/test-toolkit/src/fake_editor.rs` (`main`,
  `read_stdio_report`, `read_pids`, the variable-name constants, `SLEEP_CEILING`), thin
  bins `darkmatter-fake-editor` (darkmatter-cli) and `claudine-fake-editor`
  (claudine-cli), both `required-features = ["test-fixtures"]`, plus the toolkit's own
  `test-toolkit-fake-editor`. `rg darkmatter-argv-recorder` found no justfile, CI
  manifest, or archive list naming the recorder, so only Cargo manifests and the
  dependency docs needed the new names.
- **Docs and skills**: a new "Editor Launch Facts and the Display Session" section in
  `sniff/docs/sniff-library-architecture.md` (the existing page `rg -l Editor sniff/docs`
  found), a new item in the toolkit README, dependency notes, Sniff skill
  (`SKILL.md`, `programs.md`), and a "Launching the user's editor" section in the
  darkmatter skill's `testing.md`. Darkmatter's own `wait_args_for_editor` table is still
  present; Phase 2 removes it, so the docs say only that Sniff is the source of these
  facts.

### Requirement-to-test mapping

All are L1, F1 unless noted (pure in-process logic at Sniff's public API; the fake
editor crosses a process boundary, so its tests spawn the real binary: F3 at L1).

| Requirement | Test(s) |
|---|---|
| R5 table covers every variant | `sniff programs::enums::editor_traits::tests::traits_table_covers_every_editor` |
| R6 surface + wait args per variant (wildcard-free match) | `...::every_editor_has_its_documented_surface_and_wait_args` |
| R5/R6 `can_block` | `...::non_blocking_gui_editors_cannot_block`, `...::terminal_dual_and_waiting_gui_editors_can_block` |
| R7 identify by path, spaces in directory | `...::identify_reads_the_final_path_component` |
| R7 launcher-extension stripping only (`code.cmd`, `nvim.appimage`) | `...::identify_strips_windows_launcher_extensions_only` |
| R7 `code-insiders`, `helix` | `...::identify_uses_extra_identifying_names` |
| R7 case by platform (`Code.exe`, `NVIM`) | `...::identify_compares_case_by_platform` (both branches proved: macOS here, Windows via cross-check) |
| R7 unknown wrapper → `None` | `...::identify_does_not_recognize_unknown_commands` |
| `EditorSurface` wire form | `...::editor_surface_serializes_in_snake_case` |
| R8 macOS/Windows, Linux X11/Wayland/empty/neither/mixed, WSL1/WSL2 with and without vars, other OS | `sniff os::display::tests::*` (9 tests) |
| R8 `detect_display_session` does not panic | `os::display::tests::detecting_the_process_display_session_does_not_panic` |
| R21 no variables = no-op | `test-toolkit::fake_editor no_variables_exits_zero_and_leaves_the_target_untouched` |
| R21 RECORD (argv_recorder format, repeated runs) | `record_appends_one_argv_record_per_run` |
| R21 WRITE / REPLACE / DELETE / EXIT | `write_copies_content_into_the_target_in_place`, `replace_saves_through_a_rename_and_leaves_no_temporary`, `delete_removes_the_target`, `exit_sets_the_exit_code` |
| R21 fixture failure is loud (exit 2) | `an_unreadable_content_file_fails_loudly_with_exit_two` |
| R21 OUT_TEXT / ERR_TEXT | `out_and_err_text_reach_their_streams` |
| R21 STDIO (non-terminal streams, piped stdin) and report round trip | `stdio_reports_non_terminal_streams_and_piped_input`, `fake_editor::tests::stdio_reports_round_trip`, `a_malformed_stdio_report_is_invalid_data` |
| R21 READ_LINE | `read_line_writes_one_stdin_line_into_the_target` |
| R21 HOLD | `hold_creates_its_marker_and_stays_running_until_killed` |
| R21 SPAWN_CHILD + PIDS | `spawn_child_records_a_live_child_pid` |
| R21 IGNORE_TERM (Unix), editor and child | `ignore_term_keeps_a_held_editor_running_after_sigterm`, `ignore_term_reaches_a_spawned_child` |

The terminal-dependent STDIO cases (a real TTY) are not testable without a terminal and
are left to the Phase 2/4 L2 tests that use the fixture, per the plan.

The input-robustness matrix does not apply to Phase 1: nothing here reads a file
format or configuration (the fake editor reads only environment variables set by
tests).

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine-cli, darkmatter-cli, sniff, test-toolkit, claudine, darkmatter]`,
  `change_class: package`, so the gates were run.
- `sniff`: `just test` 3590 passed, none ≥ 5 s; `just lint` clean.
- `darkmatter`: `just test` 16483 passed, 10 skipped, none ≥ 5 s; `just lint` clean.
- `test-toolkit` has no `test`/`lint` recipes in its own justfile, so the root
  `just test test-toolkit` (498 passed) and `cargo clippy -p test-toolkit --all-targets -- -D warnings`
  (clean) were used. `just check-tier-coverage test-toolkit`: nothing stranded.
- `claudine-cli`: the new bin builds and passes clippy with `-D warnings`
  (`--features test-fixtures --bin claudine-fake-editor`); the consolidated layout gate
  (`test_placement`, 12 tests) passes. The rest of Claudine was not run: Claudine changed
  only a `[[bin]]` entry and docs, and the Sniff change is additive (no glob import of
  `sniff::programs` or `sniff::os` exists outside Sniff, and no other crate defines the
  new names).
- **Pre-existing slow tests (test-toolkit, not touched here).** In the package run with
  the host at load ≈ 40 on 16 cores: `archive_path_guard every_allowlist_entry_still_names_a_live_site`
  7.5 s (alone 3.6 s), `archive_path_guard no_archive_executed_target_bakes_in_a_producer_path`
  9.1 s (alone 4.8 s), `kache_init_contracts ensure_is_idempotent_on_a_second_run` 5.2 s
  (alone 2.5 s), `kache_host_contracts report_resolves_the_worktree_base_by_the_wt_contract`
  6.1 s (alone 3.1 s): cleared under the loaded-host 10 s allowance by re-measuring alone.
  `ci_workflow_contracts the_shipped_planner_emits_plans_the_guards_reader_accepts` 15.0 s
  (alone 2.9 s) and `ci_workflow_contracts the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
  15.5 s (alone 0.17 s) exceed 10 s in the package run: they are fast alone and compete
  for a shared resource (most likely the shipped planner both run). They are not in this
  phase's gate (darkmatter) and were not touched; recorded here for follow-up.
- **Windows cross-check** (`just cross-check <pkg> --os windows <filters>`): the first run
  found `spawn_child_records_a_live_child_pid` timing out, because on Windows the spawned
  child inherits the editor's standard handles and the test captured the editor's output
  with `.output()`. Fixed by running that test with null streams and documenting the
  caveat in the module notes; rerun passed. All 14 fake-editor integration tests (minus
  the two Unix-only ones) and both unit tests pass on Windows; the 20 Sniff tests pass on
  Windows, proving the case-insensitive `identify` branch.

## Phase 2

### What was built

- **Shared process tree** (R10): `darkmatter/lib/src/process_tree.rs` holds `ProcessTree`
  (moved unchanged from the login-shell launcher, which now imports it; its four tests
  stay green), plus `terminate_gracefully(grace, root)` (Unix `SIGTERM` to the group,
  poll the root, unconditional `SIGKILL` sweep, reap; Windows `TerminateJobObject`, reap)
  and `release()` (Windows clears kill-on-close before closing; see decisions).
- **Resolver** (R9): `editor/resolve.rs` with `SurfaceRequest`, `RejectReason`,
  `ResolvedEditor { program, args, surface, display }`, `resolve_editor`, and
  `resolve_editor_command` (now a wrapper). The order and filtering live in a private core
  `resolve_editor_from` over injected variables and lookups; probes run lazily (one
  `InstalledEditors` scan, only when reached). A probe now launches the path Sniff found
  (an alternate binary name previously launched the primary name). `wait_args_for_editor`,
  `build_editor_command_details`, and their 11 tests are deleted; Sniff's Phase 1 tests
  own those facts. `EditorError` gained `ConfiguredEditorCannotWait`, `NoMatchingEditor`,
  and `Cancelled`, each with a `status_block` arm. The context-construction guard's
  `std::env::var` allowance moved from `editor/mod.rs` to `editor/resolve.rs` (same count).
- **Launcher** (R10): `editor/launch.rs` with `LaunchPlan`, `LaunchOptions` (`PLAIN` is
  `md edit`/`--edit`: host process group, `Child::wait`), `LaunchIo`, `LaunchStdin`,
  `LaunchOutput`, `EDITOR_TERMINATE_GRACE` (2 s). `own_tree: true`: `process_group(0)` on
  Unix, a Job Object right after spawn on Windows (the escape window is commented), a
  20 ms `try_wait` loop polling `cancelled`. A terminal editor (terminal, both, or unknown
  surface) with inherited stdin gets the Unix foreground handoff only when stdin is a
  terminal whose foreground group is the host's: the child blocks `SIGTTOU` and calls
  `tcsetpgrp` in `pre_exec`; a `Drop` guard restores the host group and `termios`
  (`TCSADRAIN`) with `SIGTTOU` ignored, after the root is reaped, on every exit path.
  `ToHostStderr` duplicates the host's stderr handle (`try_clone_to_owned`, std only).
- **File preparation** (R13): `effects::fs_write::create_new_guarded` → `(cleaned path,
  CreateOutcome::{Created, Existing})`; one `create_new` open, handle dropped before
  returning. `fs_write` became `pub(crate)` for the editor module.
- **Surface and probe** (R12, R16): `editor/surface.rs` with the thread-local
  `deny_terminal_in_this_thread()` → `TerminalDenyScope` (not `Send`, restores the
  previous value), `terminal_denied()`, the process-wide `InteractiveSurface` claim, and
  `EnvironmentProbe::process()`.
- **Service** (R11, R12, R15): `editor/service.rs` with `EditorService`, `EditRequest`,
  `EditOutcome`, `EditError`, `EditorHostFacts`, `EditorOutput`, `ProcessEditorService`.
  Order: cancelled → resolve → eligibility (+ terminal claim held until exit) →
  containment → create (`attempted = true` from here) → `canonicalize_simplified` →
  launch (`own_tree: true`) → containment and regular-file recheck. Everything is
  re-exported from `darkmatter::editor`; every pre-existing public path is unchanged.
- **Docs and skills**: `darkmatter/docs/cli/edit.md` "How the editor is chosen" now
  describes skipping non-blocking editors, wait flags by program name (including a full
  path), no doubled flag, and the no-space-in-program-path limit. Darkmatter skill:
  a "Launching the user's editor" section in `library-surfaces.md`, the new fake-editor
  variable in `testing.md`. `os` skill: two symptom rows (below).
- **Fake editor**: one additive variable, `FAKE_EDITOR_PAUSE` (module docs, README, one
  toolkit test).

### Findings along the way

- **Fixture binary truncated by the test helper.** The first version of
  `install_fake_editor` hard-linked the fixture and fell back to `fs::copy`. Reinstalling a
  name that already existed (a hard link) made `copy` write through the link, truncating
  `target/debug/darkmatter-fake-editor` itself to 0 bytes; every later launch then ran an
  empty file as a shell script, exiting 0 and doing nothing. Symptoms were "passes alone,
  fails in a batch". Fixed by never copying over an existing install (comment at the
  site), and the binary was rebuilt.
- **`$EDITOR` is split on whitespace** (pre-existing, kept by R9): a program path that
  contains a space never resolves. Documented in `docs/cli/edit.md` and in the test
  helper; not changed here.
- **Windows: `CREATE_NEW` on an existing directory is "access denied"**, not "already
  exists" (os error 5). `create_new_guarded` now classifies any create error on a name an
  entry already holds (`symlink_metadata`) as `Existing`. Added to the `os` skill.
- **`CARGO_TARGET_TMPDIR` does not exist in archive runs** (cross-check, WSL2, CI): it
  is baked in at compile time. The helper uses the system temp directory. Added to the
  `os` skill.
- Windows forbids `|` and `*` in file names; the argv test uses `;$()&^%'` and a
  backtick instead.

### Requirement-to-test mapping

Boundaries: resolution reads the process environment and `PATH` (F1 in-process over the
pure core; F3 through the real environment and a real program for wiring); the launcher
and service cross a process boundary and the filesystem, so they spawn the real fake
editor binary (F3, L1). No real terminal is used: eligibility takes an injected probe,
and the handoff only engages on a real foreground TTY (Phase 4's L2 test covers it).

| Requirement | Test(s) |
|---|---|
| R9 `EDITOR` precedence, args kept | `darkmatter editor::resolve::tests::editor_takes_precedence_over_visual_and_keeps_its_arguments`; `darkmatter-cli l1 editor_resolution::editor_wins_over_visual_and_its_arguments_reach_the_editor` (argv incl. `;`, `$`, `&`, Unicode) |
| blank / unset / missing `EDITOR` → `VISUAL` | `resolve::tests::{a_blank_editor_falls_to_visual, an_unset_editor_falls_to_visual, a_missing_editor_falls_to_a_visual_that_exists}`; `editor_resolution::a_blank_or_missing_editor_falls_to_visual` |
| missing configured editor refuses, probes never run | `resolve::tests::a_configured_editor_that_is_missing_refuses_without_probing`; `editor_resolution::a_missing_configured_editor_refuses_rather_than_probing` |
| non-blocking `geany`/`lapce` skipped, error says cannot wait, never probed | `resolve::tests::{a_configured_non_blocking_editor_is_skipped_and_named, a_non_blocking_editor_falls_to_the_next_variable, the_first_rejected_variable_names_the_error, probes_never_select_a_non_blocking_editor, only_non_blocking_editors_installed_is_no_editor_found}`; `editor_resolution::a_configured_non_blocking_editor_is_refused_as_unable_to_wait` |
| `gui` true/false/omitted, `EDITOR=nvim VISUAL=code` | `resolve::tests::{gui_true_skips_a_terminal_editor_for_a_gui_visual, gui_false_keeps_a_terminal_editor, gui_omitted_keeps_ordinary_precedence}`; `editor_resolution::gui_true_skips_a_terminal_editor_for_a_gui_visual_and_gui_false_keeps_it` |
| `EDITOR=emacs gui:false` → terminal probe | `resolve::tests::gui_false_with_a_dual_surface_editor_probes_for_a_terminal_editor` |
| `Both`/unknown never match explicit `gui` | `resolve::tests::an_explicit_surface_never_matches_a_dual_or_unknown_editor` |
| probes run although a variable is set (explicit `gui`) | `resolve::tests::an_explicit_surface_probes_although_a_variable_is_set` |
| typed `NoMatchingEditor` lists every candidate + reason | `resolve::tests::no_matching_editor_lists_every_candidate_with_its_reason`; `darkmatter l1 error_snapshots::editor::no_matching_editor_lists_each_candidate_and_reason` |
| `EDITOR="code --wait"` not doubled; `/abs/path/code` gets `--wait` before the path | `resolve::tests::{a_configured_wait_flag_is_not_doubled, an_absolute_path_to_a_known_gui_editor_gets_its_wait_flag, kate_gets_block}`; `editor_resolution::{a_configured_wait_flag_is_not_doubled, a_gui_editor_named_by_absolute_path_gets_its_wait_flag_before_the_path}` (leading-dash file name) |
| unknown command trusted; probe launches found path | `resolve::tests::{an_unknown_command_is_trusted_without_gui, a_probe_launches_the_path_it_was_found_at}` |
| `md edit`/`--edit` unchanged | `editor_resolution::{md_edit_surfaces_a_non_zero_exit, wrapper_edit_text_returns_the_trimmed_buffer}`; existing `darkmatter editor::tests::*` (Unix); Claudine `prompt_source`/`edit` tests (31) |
| new `EditorError` status blocks | `error_snapshots::editor::{configured_editor_cannot_wait_names_the_variable, no_matching_editor_lists_each_candidate_and_reason, cancelled_names_the_path_and_keeps_saved_bytes}` |
| R10 exact argv (spaces, metacharacters, Unicode, leading-dash directory) | `darkmatter-cli l1 editor_launcher::the_path_is_one_argument_after_the_editor_arguments` |
| nonzero exit, missing binary | `editor_launcher::a_non_zero_exit_and_a_missing_program_are_typed_failures` |
| stdout/stderr routing (`Inherit`, `ToHostStderr`, `Discard`) | `editor_launcher::output_goes_where_the_launch_io_says` (re-runs the test binary as the host via `route_helper`) |
| cancellation terminates and reaps the tree (grandchild dead), saved bytes kept | `editor_launcher::cancellation_terminates_the_whole_tree_and_reaps_it` (Unix group; Windows Job, proved by cross-check) |
| `SIGTERM` ignored → `SIGKILL` after grace, elapsed in [grace, grace + 5 s) | `editor_launcher::an_editor_ignoring_sigterm_is_killed_after_the_grace_period` (Unix; 2.07 s) |
| GUI client: only the client is owned | `editor_launcher::cancelling_a_gui_client_leaves_an_already_running_editor_alone` |
| `own_tree: false` keeps the host group; `own_tree: true` leads its own; a successful owned launch leaves its leftovers running | `editor_launcher::a_plain_launch_keeps_the_host_process_group` (Unix) |
| `ProcessTree` move byte-identical | existing `markdown::compose::shell_expansion::launcher::tests::*` (4) |
| R13 creation under missing parents; `Existing` for file, dir, symlink, dangling symlink; race = exactly one `Created`; escaping symlink refused before creation; outside root creates nothing | `darkmatter effects::fs_write::tests::{create_new_makes_an_empty_file_under_missing_parents, create_new_leaves_an_existing_file_untouched, create_new_reports_a_directory_as_existing, create_new_does_not_follow_a_symlink_even_a_dangling_one, racing_creators_see_exactly_one_created, create_new_refuses_a_symlink_that_escapes_the_root_before_creating, create_new_outside_the_root_is_refused_and_creates_nothing}` |
| R16 deny scope nested/per-thread; terminal claim exclusive | `darkmatter editor::surface::tests::{denial_is_scoped_nested_and_per_thread, the_interactive_surface_is_exclusive_until_released}` |
| R12 eligibility table: terminal/unknown/`Both`-without-`gui` × structured, stdin reserved, stdin/stdout not tty, Markdown stderr not tty, background, parallel member; refusals create nothing, `attempted == false`; Claudine-style terminal output needs no tty stderr and no display | `darkmatter-cli l1 editor_service::a_terminal_editor_needs_an_interactive_terminal_the_host_can_lend` |
| GUI: no display refused; allowed under structured output, no tty, parallel member; stdin null | `editor_service::a_gui_editor_needs_a_display_but_no_terminal` |
| missing parents created, editor gets the absolute native path | `editor_service::an_eligible_terminal_editor_creates_missing_parents_and_succeeds` |
| `create_new` occupied by file/dir/symlink/dangling symlink: never launched, bytes unchanged | `editor_service::create_new_refuses_an_occupied_name_without_launching` |
| directory target before launch | `editor_service::a_directory_target_is_not_a_regular_file` |
| outside the mutation root refused before creation | `editor_service::a_target_outside_the_mutation_root_is_refused_before_creation` |
| byte-for-byte preservation (trailing whitespace, CRLF, non-UTF-8) | `editor_service::an_existing_regular_file_is_opened_with_its_bytes_preserved` |
| atomic-save replacement succeeds | `editor_service::an_atomic_save_that_replaces_the_file_is_success` |
| deleted after exit fails | `editor_service::a_file_deleted_by_the_editor_fails_after_exit` |
| directory after exit; containment recheck after exit (escaping symlink, Unix) | `editor_service::the_target_is_rechecked_after_the_editor_exits` (uses `FAKE_EDITOR_PAUSE`) |
| concurrently created name opened, not replaced | `editor_service::a_name_created_concurrently_is_opened_not_replaced` + the `fs_write` race test (see decisions) |
| `attempted == true` after a failed launch; created file kept | `editor_service::a_failed_launch_still_counts_as_attempted` |
| refused once cancelled, nothing created | `editor_service::an_edit_is_refused_once_the_run_is_cancelled` |
| process-wide surface guard → `SurfaceBusy` | `editor_service::a_second_terminal_editor_waits_for_no_one_and_is_refused` |
| `FAKE_EDITOR_PAUSE` contract | `test-toolkit fake_editor pause_waits_for_its_marker_to_be_removed_then_exits_normally` |

Unicode paths are covered in the argv test and the missing-parents service test. Not
covered here, by design: the real-TTY foreground handoff and termios restore (needs a
terminal; Phase 4's L2 test), and Windows Job termination of a `SIGTERM`-ignoring tree
(the Unix-only grace test has no Windows analogue; the Windows cancellation test proves
job termination). The input-robustness matrix does not apply: Phase 2 reads no file
format or configuration.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [darkmatter, darkmatter-cli, test-toolkit]`, `change_class: package`; gates run.
- `darkmatter`: `just lint` clean (after factoring one `type` alias clippy asked for);
  `just test` 16539 passed, 10 skipped, 0 slow (final run, after every edit);
  `just slow-tests fresh` exit 0; `just check-tier-coverage darkmatter`: nothing stranded.
  An earlier full run (host load 58→138 on 16 cores, not from this session's work) had 4
  tests at 5.1–6.4 s: `trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form`
  (alone 1.8 s) and three `claudine_schema_bundle` tests (alone 2.0–2.6 s); none touched
  here, cleared under the loaded-host rule, and the final run had none.
- `test-toolkit`: root `just test test-toolkit` 499 passed; one pre-existing slow test,
  `archive_path_guard no_archive_executed_target_bakes_in_a_producer_path` 5.4 s at load
  ≈ 38, 4.86 s alone (cleared; Phase 1 recorded the same test). `cargo clippy -p
  test-toolkit --all-targets -- -D warnings` clean.
- Dependents: `claudine-cli` `-E 'test(/prompt_source|edit/)'` 31 passed (the plan's
  checkpoint); Claudine's library builds against the unchanged public paths.
- Cross-OS (process trees, Job Objects, handle duplication, termios are real OS risk):
  `just cross-check darkmatter-cli --os windows editor_` — attempt 1 found the
  `CARGO_TARGET_TMPDIR` problem, attempt 2 (after the fixes above) 30/30 passed;
  `just cross-check darkmatter --os windows editor fs_write shell_expansion::launcher context_construction_guard`
  50/50 passed; `just cross-check darkmatter-cli --os linux editor_` 32/32 passed.
  (`-E 'test(...)'` breaks the remote quoting; positional substring filters work.)

## Phase 3

### What was built

- **Typed action and registration** (R1, R2): `EditAction { file: Expr, create_new: bool,
  gui: Option<bool> }` and `LifecycleActionKind::Edit` in `lifecycle/actions.rs`;
  `LIFECYCLE_ONLY_VERBS = ["edit"]` and `is_lifecycle_only_verb` in `signatures.rs`, read by
  `is_known_lifecycle_verb` and `all_lifecycle_verbs` (so bare-verb parsing, did-you-mean, and
  Claudine's loop-key check see it). `action_type` → `Expression`, `action_verb` → `"edit"`;
  `runs_a_shell` stays false, so `initialize` admits it (both registries declare
  `Expression` for `initialize`, confirmed in `registry.rs` and Claudine's registry).
- **Parsing** (R3): short form and one-element list through a new `edit` branch in
  `build_non_control_positional_action` (exact arity 1, before the side-effect and
  expression-function fallbacks); long form in `build_action_from_params` (`file` required,
  unknown keys through `reject_extra_params`); `parse_long_form` refuses a non-boolean
  `create_new`/`gui` (including `null`, `"true"`, `"{{ true }}"`) before
  `action_value_to_expr`. A shared `edit_file` check refuses number, boolean, and blank
  literals; a non-blank literal is kept byte-for-byte. `positional_operand_field("edit")`
  is `file`. Bare `edit` now reports arity, not "unknown verb". `with:` hits the existing
  `LifecycleWithUnsupported`; ordering needed no change (`canonical_rank` third bucket).
- **Passive surfaces** (R4): `walk_action` visits `…edit.file` only; `template_operands`
  covers `edit` (long form: `file` only, never `create_new`/`gui`); the source map records
  long-form `edit` parameters by name; `action_writes` returns nothing (wildcard). The
  schema projection's `hand_owner` refuses a catalog verb named like a lifecycle-only verb.
- **Execution placeholder**: `execute_action_inner` refuses `edit` with an unsuppressible
  evaluation failure ("this host cannot run `edit`") until Phase 4's dispatch replaces it,
  so no execution behavior is introduced. Claudine's `describe_action` got the R22 arm
  (needed to compile). Every other `LifecycleActionKind` walker (preflight candidates,
  shell walk, shell templates, Claudine `prepare.rs`, task templates, container preflight)
  uses a wildcard and treats `edit` as inert; each was read to confirm.

### Departures from the plan

- The plan said to update "verb-inventory assertions in `lifecycle_generated_operands.rs`
  and the catalog-equality tests under `catalog/tests.rs`". Neither reads
  `all_lifecycle_verbs` (they cover schema templates and the function catalog), and both
  stayed green unchanged. The checks that did break were the two parser-vs-schema
  agreement tests; they got self-expiring exemptions (see `decisions_for_review`).
- The parse tests live in one L1 integration file through the public API rather than
  split across `lib/src/lifecycle/tests/` and `lib/tests/l1/`.

### Requirement-to-test mapping

All in `darkmatter/lib/tests/l1/lifecycle_edit_action.rs` unless noted (24 tests).

| Requirement | Test(s) |
|---|---|
| Control row: motivating fixture → `EditAction { create_new: true, gui: None }`, `{{ fqn }}` kept as an expression | `the_motivating_workflow_parses_to_an_exclusive_edit_in_the_else_branch` |
| Short form, list-of-one equals scalar, defaults | `the_short_forms_open_the_operand_with_default_flags` |
| `edit: fqn` is the literal `fqn` | `a_bare_word_operand_is_a_literal_file_name_not_a_variable` |
| Whitespace preserved in non-blank path; mixed template kept | `a_nonblank_literal_keeps_its_whitespace`, `a_mixed_template_operand_stays_a_template` |
| Flag control rows, absent defaults, inner `no_error` | `the_long_form_reads_each_literal_boolean` |
| `file`: absent / null / wrong type / object | `file_absent_is_refused`, `file_null_is_refused`, `file_of_the_wrong_type_is_refused` |
| List element wrong (one, every), two strings → arity | `a_list_element_of_the_wrong_type_is_refused` |
| Empty: `""`, `"   "`, `[""]`, long-form blank, `[]` | `an_empty_operand_is_refused` |
| Bare `edit` is arity, not unknown verb | `a_bare_edit_is_an_arity_error_not_an_unknown_verb` |
| Object short operand | `a_direct_object_short_operand_is_refused` |
| `create_new`/`gui` shared validator: null, strings, `{{ true }}`, number, array, object, located at the key | `a_flag_that_is_not_a_literal_boolean_is_refused_at_its_key` |
| Unknown key, `with:` | `unknown_and_with_keys_are_refused` |
| Dictionary order: after `info` and `shell`, before `ensure_file` (by name) and `stop` | `a_compact_dictionary_runs_edit_after_communication_and_shell_before_flow_control` |
| Sibling `no_error`; inner+sibling duplicate; non-boolean `no_error` | `a_sibling_no_error_applies_and_a_duplicate_is_refused` |
| `Expression` type, not a shell; accepted in `initialize`, every md event, and inside `then`/`else` | `edit_is_an_expression_action_admitted_in_initialize_and_every_md_event` |
| `LIFECYCLE_ONLY_VERBS`, `is_known_lifecycle_verb`, `all_lifecycle_verbs` | `edit_is_the_one_lifecycle_only_verb_and_a_known_verb` |
| No `edit` expression function or side effect | `no_expression_function_or_side_effect_is_named_like_a_lifecycle_only_verb`; generator refusal in `lifecycle_schema_generation.rs::a_new_collision_with_a_hand_verb_or_reserved_key_fails_the_projection` (`edit` row) |
| Did-you-mean `edti` → `edit` | `a_misspelled_edit_suggests_edit` |
| Both branches walked, event-time value deferred (an unknown function in `file` is never run) | `the_call_check_walks_the_file_operand_in_both_branches_without_evaluating_it` |
| A proven-bad call in an untaken branch's `file` rejected at `…edit.file` | `a_proven_bad_call_in_an_untaken_edit_branch_is_rejected_at_its_operand` |
| Schema span coverage reads `file`, not the flags | `schema_template_operands_cover_file_but_not_the_boolean_flags` |

Every error assertion checks the variant, `source_path`, the located `property`, and the
rendered message. The input loader is the existing frontmatter loader (R25), so no
document-shape tests were added. Smell grep over the new code (`serde(default)`,
`unwrap_or_default()`, `filter_map(.. as_str)`, `.ok()`): none on a load-bearing field;
`gui: Option<bool>` is `None` only when the key is absent, because `null` is refused first.
Spawned-CLI wiring cases and `md schema validate`/DMLS agreement are later phases (the
schemas do not type `edit` yet).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-cli, test-toolkit, claudine-cli]`,
  `change_class: package` (the working tree still holds Phase 2's uncommitted files); gates run.
- `darkmatter`: `just lint` clean; `just test` 16563 passed, 10 skipped, none at or above
  5 s; `just check-tier-coverage darkmatter`: nothing stranded.
- Dependent `claudine` (over 200 tests, so filtered): `-E 'test(claudine_schemas) |
  test(loop) | test(lifecycle) | test(side_effect)'` 1914 run, one failure
  (`claudine_schemas::hand_verbs_equal_the_parser_inventory`, fixed as above, then passes);
  `-E 'test(sequence::task) | test(side_effect) | test(claudine_schemas::)'` 277 passed;
  `cargo clippy -p claudine --all-targets -- -D warnings` clean. That first run reported
  six `claudine_schemas_matrix` tests at 5.6–6.8 s while host load was ≈ 80 (the run
  itself); alone, `yaml_trigger_combinator::control` 1.45 s and
  `yaml_action_initialize_bundle::wrong_type_whole` 1.32 s (load-sensitive, untouched here).
  `claudine-cli` compiles (`cargo check --all-targets`).
- Cross-OS: not run. Phase 3 is pure parsing over in-memory JSON with no path, process,
  or OS-specific code.
