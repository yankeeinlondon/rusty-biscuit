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
decisions_for_review:
  - |-
    `Editor::identify` treats its whole argument as one program path and compares only its final component; it does not split on whitespace. The plan (R7) said it "takes the first whitespace-separated token's path", but also asked for a test with "a path with spaces in the directory", which cannot be recognized if the input is split on spaces. Splitting the `$EDITOR` command line is left to the caller (the Phase 2 resolver, which already takes the first whitespace token). Alternative: split inside `identify`, which makes `identify("code --wait")` work but makes `/opt/My Editors/bin/subl` unrecognizable. Redoing it means changing one line in `identify` and two test assertions.
  - |-
    The test toolkit got its own copy of the fake editor binary (`test-toolkit-fake-editor`, ungated) so its tests can launch the real program, plus a Unix-only `libc` dependency (already in the graph through `biscuit-test-harness`) for ignoring `SIGTERM`. The plan named only the Darkmatter and Claudine binaries. Alternative: test the body in-process only, which cannot prove the spawn-child or ignore-SIGTERM behaviors. Reversing means deleting one `[[bin]]` and the integration test file.
  - |-
    Fake editor details the plan left open: `FAKE_EDITOR_REPLACE` names a content file (like `FAKE_EDITOR_WRITE`) that is saved through a temporary file and a rename; `FAKE_EDITOR_READ_LINE` runs before `FAKE_EDITOR_STDIO` so the report can show the line read; the stdio report is a small JSON file read back with `read_stdio_report`; a held or spawned process exits on its own after 120 seconds so a failed test cannot leak it; a spawned child reports readiness over a private pipe before its PID is written, so a test never races its `SIGTERM` setup. Each is a local change in `tools/test-toolkit/src/fake_editor.rs` if a reviewer prefers otherwise.
message_to_agent: |-
  Phase 1 is in place. Things Phase 2 should know:
  - Sniff API: `sniff::programs::{Editor, EditorSurface, EditorTraits}`; `Editor::surface()`, `wait_args()`, `can_block()`, and `Editor::identify(program)`. `identify` takes ONE program token (it compares the final path component and does not split on whitespace), so the resolver must split `$EDITOR` first and pass the first token. `sniff::os::{DisplaySession, detect_display_session, display_session_from}`; `DisplaySession::is_available()`.
  - Darkmatter's `wait_args_for_editor` and `build_editor_command_details` are still in `darkmatter/lib/src/editor/mod.rs`; removing them (R9) is Phase 2 work. Sniff's docs already say Sniff is the only editor-flag table.
  - Fake editor: bins `darkmatter-fake-editor` (darkmatter-cli) and `claudine-fake-editor` (claudine-cli), both behind `test-fixtures`; find them with `biscuit_test_harness::bin_exe!("darkmatter-fake-editor")`. Variables and their order are on `test_toolkit::fake_editor` (constants exported). Use `FAKE_EDITOR_HOLD=<marker>` and wait for the marker file rather than sleeping; `read_pids` and `read_stdio_report` read results back; `FAKE_EDITOR_RECORD` uses the argv_recorder format (`read_argv_records`).
  - Windows trap (found by cross-check): a child spawned with `FAKE_EDITOR_SPAWN_CHILD` inherits the editor's standard handles on Windows, so anything that captures the editor's (or `md`'s) stdout/stderr through pipes waits until that child dies. Process-tree tests must kill the tree (that is the behavior under test) or not capture output.
  - test-toolkit has no `just test`/`just lint` recipes; use root `just test test-toolkit` and `cargo clippy -p test-toolkit --all-targets`. Two pre-existing `ci_workflow_contracts` tests run ~15 s in a loaded package run (fast alone); they are unrelated to this feature.
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
