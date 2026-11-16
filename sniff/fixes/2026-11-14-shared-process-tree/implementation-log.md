---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-shared-process-tree/spec.md"
plan: "sniff/fixes/2026-11-14-shared-process-tree/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - sniff/lib/src/process.rs
    - sniff/lib/src/process/tree.rs
    - sniff/lib/src/process/tree/unix.rs
    - sniff/lib/src/process/tree/unix/tests.rs
    - sniff/lib/src/process/tree/windows.rs
    - sniff/lib/src/process/tree/windows/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - sniff
    - darkmatter
    - claudine
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
    - darkmatter/lib/src/editor/launch.rs
    - darkmatter/lib/src/editor/mod.rs
    - darkmatter/lib/src/editor/service.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/md_host/tests.rs
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/process_tree.rs
    - darkmatter/lib/Cargo.toml
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/composition/sequence/task/shell/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/Cargo.toml
    - .config/nextest.toml
docs_updated_during_phase_2:
    - darkmatter/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/sniff/testing.md
    - .claude/skills/os/macos.md
source_files_during_phase_3:
    - sniff/lib/src/programs/install/options.rs
docs_updated_during_phase_3:
    - sniff/docs/sniff-library-architecture.md
    - sniff/lib/README.md
    - sniff/README.md
    - sniff/cli/README.md
    - darkmatter/docs/topics/lifecycle/index.md
    - darkmatter/docs/topics/lifecycle/error-handling.md
    - darkmatter/docs/inline/shell-expansion.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
docs_created_during_phase_3:
    - sniff/docs/topics/process-trees.md
skills_files_updated_during_phase_3:
    - .claude/skills/sniff/architecture.md
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/windows.md
    - .claude/skills/os/macos.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/claudine/SKILL.md
decisions_for_review:
    - |-
      Graceful termination still sweeps the tree when the direct child has
      already exited. Plan Rule 8 says "if already disarmed or the child has
      exited, skip straight to the wait". I skip only the SIGTERM and grace
      wait in that case and still run the hard cleanup (group kill plus
      validated recorded descendants), because the spec says graceful
      termination hard-stops remaining owned processes "even if the root
      exited first", and a child that exits just as the caller cancels would
      otherwise leave its background processes running. The alternative,
      following the plan literally, saves one group signal and a scan in a
      rare race but leaves stragglers. Reversing it is a three-line change in
      `Core::terminate_gracefully` in `sniff/lib/src/process/tree/unix.rs`.
    - |-
      Five slow Sniff tests were fixed by running them one at a time on macOS,
      not left red. The plan does not mention them; they are the live
      CoreAudio callers (audio, hardware, and full `detect()` tests), and the
      gate counted them as failing at 13-14 s each in the package run, against
      about 3 s alone. CoreAudio sets up its audio subsystem one process at a
      time, so five tests started together each waited for the other four. I
      added a macOS-only nextest test group, `sniff-macos-coreaudio`
      (`max-threads = 1`), in `.config/nextest.toml`; afterwards the slowest
      was 5.8 s in a loaded run and 3.0 s alone. Alternatives: leave them red
      as Phase 1 did (the gate stays failing), or rewrite the tests to share
      one audio query (larger change in code this fix does not own). Reversing
      it means deleting the group and its override from `.config/nextest.toml`
      and the sentences that mention it in `.claude/skills/sniff/testing.md`
      and `.claude/skills/os/macos.md`.
message_to_agent: |-
    Phase 3 is in place: `sniff/docs/topics/process-trees.md` is the canonical
    page, and every doc, README, and skill in plan Rule 27 links to it and no
    longer states the Windows assignment window or unconditional Unix
    containment (the Task 3.5 search is clean; the remaining hits state the
    new behavior). No caller skill under `docs/use-skills/` describes process
    ownership, so `just publish-skills` was not run. Notes for Phase 4:
    - `hardware::audio::tests::detect_audio_devices_on_macos` is still the one
      slow Sniff L1 test: 6.2 s and 6.0 s in two package runs (load 7-11 on
      16 cores), 3.1 s alone. It is already in the `sniff-macos-coreaudio`
      group; giving that override `threads-required = 4` only brought it to
      5.7 s, so the experiment was reverted. The cost is CoreAudio's
      per-process audio subsystem setup on this host (seven third-party HAL
      plug-ins, see the `os` skill's macos.md); no phase of this fix touched
      that code. Phase 4's package-wide verification should decide whether to
      fix it (for example by making the live audio tests share one query) or
      record it.
    - Windows and Linux evidence notes from Phase 2 still apply (build-win's
      `B:` volume short of space; build-linux unreachable at the end of
      Phase 2). Phase 3 changed no code with OS risk (one doc comment), so no
      cross-check was run here.
    - The two working-tree changes not from this fix
      (`claudine/docs/topics/flow-control/flow-control.md`, the deleted
      `.darkmatter-shell-whitelist`) are still there and still untouched.
---

# Implementation Log for 2026-11-14-shared-process-tree (4 phases)

## Phase 1

- Started 2026-11-16 on macOS (16 cores, load average 9–11). Implemented
  Tasks 1.1–1.6 directly rather than through subagents: the Unix core, the
  Windows core, and the `process.rs` migration share one module boundary.

### What was built

- `sniff/lib/src/process/tree.rs`: the public API of plan Rule 2
  (`TreeConfig`, `TreeIsolation`, `CleanupScan`, `TreeTermination`,
  `ProcessTree` with a root-PID-only `Debug`, the two Windows flag constants),
  `combined_creation_flags`, `DESCENDANT_SAMPLE_INTERVAL` (now `pub(crate)`),
  and the test-only `sample_hook` (`pub(in crate::process)`). Rustdoc includes
  a `no_run` example showing attach-failure cleanup, and a runnable
  `TreeConfig` example.
- `sniff/lib/src/process/tree/unix.rs`: `Host` seam (`SystemHost` = sysinfo +
  `libc::kill`), `Core<H: Host = SystemHost>`, `verify_root`, `configure`
  (exactly one of `process_group(0)` or a `setsid` `pre_exec`), identity-checked
  sampling and cleanup, `kill_group_hard` with the macOS repeat
  (`GROUP_REAP_LIMIT` 250 ms, moved from Claudine) and `ESRCH`/`EPERM`
  classification, graceful termination with the accepted group-ID reuse
  comment carried over from Darkmatter.
- `sniff/lib/src/process/tree/windows.rs`: suspended-start Job owner with
  every handle in `Owned<HANDLE>`, the `Step` sequence, and the `hooks`
  test seam (thread-local step log and injected failures). An `attach` that
  fails after `Assign` drops an armed owner, which terminates the Job and
  the suspended child.
- `sniff/lib/src/process.rs`: both nested `mod process_tree` blocks, the
  `sample_hook` module, and the local sample scheduling were removed;
  `run_command_with_timeout` uses `TreeConfig::process_group()`,
  `ProcessTree::attach`, calls `sample()` on every `Ok(None)` iteration, and
  `terminate(CleanupScan::Always)` on timeout, wait error, and pipe-setup
  failure, then `terminate(CleanupScan::WhenObserved)` in the epilogue.
  Cleanup errors are now logged through `log_cleanup_failure` (Rule 13);
  before, they were discarded. Module docs and the two `## Notes` blocks were
  rewritten: the "`kill(-pgid)` is atomic with respect to forks" claim and
  "total on Windows" wording are gone, the owner is named, and the
  installation-boundary paragraph is kept.

### Departures and small choices (narrative)

- The Windows tests define their own fixture tests
  (`child_spawns_reporting_descendant`, `reporting_descendant`,
  `child_sleeps`) in `tree/windows/tests.rs` instead of reusing the
  `process.rs` fixtures through a `pub(super)` helper. The existing
  `child_spawns_pipe_holding_descendant` does not write a PID file, so
  reusing it would have meant changing a fixture other tests rely on.
- The `attached_at` field listed in plan Rule 5 was not added: nothing reads
  it (`next_sample` carries the schedule).
- The `#[cfg(test)] mod tests;` declarations sit at the end of `unix.rs` and
  `windows.rs` because clippy's `items_after_test_module` rejects them at the
  top.
- One extra graceful-path test on Windows
  (`graceful_termination_stops_the_job_and_reaps_the_root`) and extra Unix
  tests beyond the plan's list: `a_validated_root_lends_its_descendants_to_cleanup`
  and `an_observed_descendant_is_killed_after_reparenting` (positive controls
  for the identity tests), `recycled_observed_pid_is_skipped_at_cleanup_without_another_sample`,
  `an_armed_drop_hard_stops_the_tree`, `an_absent_group_is_success`, the
  `EPERM` pair (success off Linux, error on Linux), and
  `verify_root_accepts_its_own_group_and_an_exited_child`.
- `attach_accepts_a_child_that_already_exited` waits for the exit with
  `waitid(P_PID, …, WEXITED | WNOWAIT)` instead of a fixed 300 ms sleep, so
  the child is certainly an unreaped zombie at `attach` (a readiness marker,
  not a timing guess).

### Requirement-to-test mapping

The boundary is the OS process API (fork/exec, signals, Job Objects), crossed
by real child processes; all tests are L1 under `just test` (Rule 16).

| Requirement | Tests (in `sniff/lib/src/process/…`) |
|---|---|
| Suspension combined with caller flags in one place | `tree.rs` `suspension_is_added_to_no_flags`, `…_combined_with_no_window`, `…_combined_with_a_new_process_group`, `suspension_is_never_lost_and_caller_flags_are_kept`, `creation_flags_are_recorded_and_isolation_kept` |
| Zero scans at attach; first sample after 250 ms; no more often | `unix/tests.rs` `no_scan_at_attach`, `first_sample_waits_one_interval`, `samples_are_no_more_frequent_than_the_interval` |
| Cleanup-scan policy (short success: 0 scans; interrupted: 1; observed: 1) | `cleanup_scan_policy` |
| Identity checks before traversal and signaling | `recycled_observed_pid_is_not_a_traversal_root_or_target`, `recycled_observed_pid_is_skipped_at_cleanup_without_another_sample`, `root_never_sampled_is_not_a_traversal_root`, `root_with_changed_start_is_not_a_traversal_root`; controls `a_validated_root_lends_its_descendants_to_cleanup`, `an_observed_descendant_is_killed_after_reparenting` |
| Disarm after success; failure keeps the drop backstop; armed drop | `successful_hard_cleanup_disarms`, `failed_cleanup_keeps_the_drop_backstop`, `an_armed_drop_hard_stops_the_tree` |
| Release never signals (fake) and leaves a member running (real) | `release_never_signals`, `release_leaves_a_member_running` |
| macOS repeated group kill / Linux single kill; error classification | `group_kill_repeats_off_linux` / `group_kill_is_not_repeated_on_linux`, `an_absent_group_is_success`, `a_zombie_only_group_answering_eperm_is_success_off_linux` / `eperm_from_the_group_is_an_error_on_linux` |
| Root verification (fast exit, foreign group, invalid IDs) | `attach_accepts_a_child_that_already_exited`, `attach_rejects_a_live_root_in_a_foreign_group`, `verify_root_rejects_invalid_ids`, `verify_root_accepts_its_own_group_and_an_exited_child`, `verify_root_rejects_a_foreign_group` |
| Exactly one isolation mode; caller hooks after group setup | `session_mode_makes_the_child_a_session_leader`, `group_mode_does_not_start_a_session`, `a_caller_pre_exec_hook_runs_after_group_setup` |
| Immediate and graceful termination with real processes | `immediate_terminate_kills_a_group_member_that_left_the_root_alone`, `graceful_cleanup_when_the_root_exits_but_a_descendant_ignores_sigterm` |
| Windows assign-before-resume, injected failures, no leaks, inheritance, release, release failure, no sampling, graceful | `windows/tests.rs` `attach_assigns_before_resuming`, `attach_fails_cleanly_when_*` (5 steps), `failed_attach_does_not_leak_handles`, `a_descendant_created_immediately_is_in_the_job`, `release_leaves_a_member_running`, `release_failure_keeps_cleanup_armed`, `sampling_does_nothing`, `graceful_termination_stops_the_job_and_reaps_the_root` |
| Sniff helper keeps its behavior (Rule 15) | every existing `process::tests` test, unchanged assertions, including `a_descendant_that_detaches_between_samples_escapes_containment` |

The Input Robustness Matrix does not apply (plan: no file format or
configuration is read).

### Commands and results

- `cargo nextest run -p sniff --features remote -E 'test(/process::/)'`:
  47 passed (36 new tree tests + the existing `process::tests`), slowest
  3.05 s (`a_session_detached_descendant_cannot_block_pipe_cleanup`, its 3 s
  injected deadline, unchanged).
- `cargo clippy -p sniff --all-targets -- -D warnings`, the same with
  `--features remote`, and the same with `--target x86_64-pc-windows-gnu`:
  clean. The Windows build type-checks `tree/windows.rs` and its tests; they
  were not executed (Phase 4, task 4.4, runs them on a Windows rig).
- `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`: clean.
- `cargo test -p sniff --doc -- process`: both new doctests pass.
- `cargo check -p sniff` (no features) passes; `cargo tree -p sniff -e normal
  --no-default-features` contains no `reqwest`.
- `git grep -n "mod process_tree" sniff/lib/src`: no output.
- `just lint` (sniff): passes.
- `just test` (sniff): 3627 passed, 33 skipped, **6 slow** (listed below).
- Downstream: the only uses of `sniff::process` outside Sniff are
  `configure_detached_child` (biscuit-speaks, playa, worktree), which is
  unchanged. `cargo check --all-targets -p darkmatter -p claudine -p worktree
  -p playa -p biscuit-speaks -p unchained-ai` passes. No downstream tests were
  run: Phase 1 adds public items only and changes a crate-private helper
  whose behavior is covered by Sniff's own tests.

### Slow tests (pre-existing, not caused by this phase)

`just test` recorded six tests at or above 5 s; all pass:

| Test | In package run | Alone |
|---|---|---|
| `hardware::audio::tests::detect_audio_devices_on_macos` | 14.1 s | 3.06 s |
| `hardware::audio::tests::detect_audio_devices_returns_vec` | 14.1 s | 3.03 s |
| `hardware::tests::test_detect_hardware_full_includes_gpu_on_macos` | 14.1 s | 3.06 s |
| `tests::test_detect_returns_result` | 13.9 s | 3.06 s |
| `sniff::l1 integration::test_detect_completes_in_reasonable_time` | 12.7 s | not re-measured |
| `network::tests::test_detect_network_returns_info` | 8.4 s | 1.76 s |

Load average was about 9 on 16 cores, so the 5 s bar applies. Each test is
under 5 s alone and slow only alongside the others, which matches the
documented cause: CoreAudio HAL setup is serialized across processes on this
Mac (`.claude/skills/os/macos.md`, `.claude/skills/sniff/testing.md`), and
these are the deliberate live-audio callers that file lists. macOS audio
detection uses CoreAudio directly and starts no subprocess, so none of these
tests reaches the code changed here. The slow-test history already records
the same test at 6.8 s and 9.5 s on earlier commits (`316d2f8`, `e770042`).
They were left unchanged as out of scope. **The test gate is therefore red
on these six pre-existing tests**, and every other test passes.

### Cross-OS

- **Linux** (`just cross-check sniff --os linux process::`, build-linux,
  2026-11-16): 47 passed, including the Linux-only
  `group_kill_is_not_repeated_on_linux` and
  `eperm_from_the_group_is_an_error_on_linux`. (A nextest `-E 'test(...)'`
  expression does not survive the recipe's remote quoting, `syntax error
  near unexpected token '('`; the positional substring filter works.)
- **Windows** (`just cross-check sniff --os windows process::`, build-win,
  2026-11-16): the MSVC build of `sniff` with its tests succeeded (only
  `linker_messages` warnings) and nextest listed 24 Windows `process::`
  tests, but **tests were not executed**: the rig's Cargo storage preflight
  refused (B: 35.8 GiB free, 50 GiB required; the automatic sweep freed
  0 GiB). I did not delete other sessions' artifacts or set
  `BISCUIT_BUILD_MIN_FREE_GIB=0` (storage-strategy rules: an agent deletes
  only what it created, and the override is for intentional emergencies).
  The Windows code also passes `cargo clippy --all-targets --target
  x86_64-pc-windows-gnu -- -D warnings` from this Mac. Running the Windows
  tests remains for Phase 4 task 4.4 once the rig has headroom; Ken may need
  to free space on build-win's `B:` volume first.
  `os_failing` was not set on the spec: nothing failed, the run was blocked.

## Phase 2

- Started 2026-11-16 on macOS (16 cores, load average 11–29 during the
  phase; other sessions were active). Implemented Tasks 2.1–2.5 directly: the
  three consumer edits are short, and doing them in one context kept the
  `sniff::process` calls consistent.

### What changed

- **Darkmatter login-shell launcher** (`shell_expansion/launcher.rs`, Rule 20):
  `configure_detached` (three `cfg` variants) is gone. The launcher calls
  `TreeConfig::session().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW)`,
  `ProcessTree::attach` (failure: kill, wait, `tracing::warn!`, `None`),
  `sample()` on every 5 ms poll, `terminate(Always)` on the deadline or wait
  error, and `terminate(WhenObserved)` after the loop. The explanation of why a
  session (not a group) is needed moved from the deleted function to the
  configure call. The "Cleanup" module docs no longer accept the Windows
  assignment window; "Job-control hazard" is unchanged.
- **Darkmatter editor** (`editor/launch.rs`, Rule 21): `own_process_group` is
  gone; `TreeConfig::process_group().configure` runs before
  `terminal::Handoff::prepare`. An attach error is now the real OS error inside
  `EditorError::Io { operation: "contain the editor's process tree" }`. `wait`
  takes `Option<&mut ProcessTree>` and polls whenever a tree is owned or a
  callback exists, sampling on each `Ok(None)`. Exit releases the tree; a
  release error becomes the plan's warning text. Cancel and failure use
  `terminate_gracefully(EDITOR_TERMINATE_GRACE, …)` and log a cleanup error.
  New `LaunchReport` and `LaunchPlan::run_reporting`; `run` returns
  `run_reporting(..).result`. `LaunchReport` is re-exported from `editor`.
- **`edit` warning** (Rule 22): `EditOutcome.warning`, filled by
  `ProcessEditorService` from `run_reporting`, and emitted once through
  `LifecycleEffects::warn` in `dispatch_edit` right after the `attempted`
  block. `FakeEditor` gained `warning`; the `md_host` test double's literal
  gained `warning: None`.
- **Darkmatter cleanup** (Rule 23): `process_tree.rs` and its `mod` line
  deleted; `windows` is `Win32_Foundation` only; the `libc` and `windows`
  comments and `darkmatter/docs/dependencies.md` ("Login-shell Probes" and the
  `libc` bullet) now name Sniff's owner.
- **Claudine** (`task/shell.rs`, Rules 24–25): both private `ProcessTree`
  types, `resume_main_thread`, `as_handle`, both `isolate_process_tree`, and
  `GROUP_REAP_LIMIT` are gone; `TREE_KILL_GRACE` is on every OS. New
  `tree_config()` (`process_group` + `WINDOWS_CREATE_NEW_PROCESS_GROUP`).
  `run_process` samples on `Ok(None)`, terminates gracefully on interrupt,
  runaway, or timeout (break with `termination.status`), and the epilogue runs
  `terminate(Always | WhenObserved)` then `drop(tree)` before reader
  settlement. `fail_isolation` injection is unchanged.
  `run_owned_lifecycle_command` uses the same calls (a small private
  `terminate_lifecycle_tree` helper for its two termination paths) and
  terminates with `WhenObserved` before returning an exit code. The
  terminal-foreground path (`holds_terminal_foreground`, `run_in_own_group`)
  is untouched and is now named in the doc of `run_owned_lifecycle_command`
  and the module docs as the one exception. The unit test
  `ownership_survives_a_child_that_exited_before_verification` was removed
  (Sniff's `attach_accepts_a_child_that_already_exited` covers it). The
  `SequenceTaskShellIsolation` doc names Sniff's owner. `windows` is
  `Win32_Foundation` only; the `libc` comment was rewritten.
- **Two Windows-only lints in Darkmatter fixed** (both already in `HEAD`, in
  files this phase edits): `drop(handoff)` in `launch.rs` (`drop_non_drop`,
  the Windows `Handoff` has no `Drop`) and `use super::*` in `editor/mod.rs`
  tests (unused on Windows). Each got a `cfg_attr(not(unix), allow(...))`.
- **Slow-test fix outside the plan** (see `decisions_for_review`): the
  `sniff-macos-coreaudio` nextest group in `.config/nextest.toml`, documented
  in `.claude/skills/sniff/testing.md` and `.claude/skills/os/macos.md`.

### Requirement-to-test mapping

The boundary is the OS process API, crossed by real child processes (L1), plus
the in-process lifecycle executor for warning plumbing.

| Requirement | Tests |
|---|---|
| Probe launcher kills a profile's background job and does not hang on its stdout | `shell_expansion::launcher::tests::a_background_descendant_is_killed_and_does_not_hold_stdout` |
| Probe deadline kills the shell and its descendants | `…::a_hanging_shell_and_its_descendants_are_killed_at_the_deadline` |
| Probe output bound and spawn failure unchanged | `…::noisy_output_is_bounded_to_the_tail`, `…::a_missing_shell_is_none`, the `shell_expansion::probe` installed-shell tests |
| Editor cancellation stops the whole tree and reaps it; SIGTERM-ignoring editor is killed after grace | `darkmatter-cli::l1 editor_launcher::cancellation_terminates_the_whole_tree_and_reaps_it`, `…::an_editor_ignoring_sigterm_is_killed_after_the_grace_period` |
| A GUI client's editor survives a completed wait (release) | `editor_launcher::cancelling_a_gui_client_leaves_an_already_running_editor_alone` |
| Plain launch keeps the host's group (no owner) | `editor_launcher::a_plain_launch_keeps_the_host_process_group` |
| Terminal handoff still runs after group setup (configure before `pre_exec`) | L2 `darkmatter-cli::level2 level2_edit_action::level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues`, `…_an_interrupt_reaps_the_open_editor_and_returns_the_terminal`; Sniff `a_caller_pre_exec_hook_runs_after_group_setup` |
| A release warning does not fail the edit, reaches the host once, and the stack continues | new `lifecycle::executor::tests::an_edit_warning_reaches_the_host_once_and_the_stack_continues` |
| Editor service outcomes unchanged | `darkmatter-cli::l1 editor_service::*`, `compose_edit_action::*` (45 tests) |
| Claudine task shells: timeout, interrupt, runaway, isolation failure, wait failure, teardown before reader settlement; lifecycle commands | the 926 `composition::sequence::task` and `composition::lifecycle` tests, unchanged |

The Windows release-failure warning cannot be produced on Unix (release never
fails there), so the text and its plumbing are covered by the executor test
with a fake editor; the failure itself is covered by Sniff's Windows
`release_failure_keeps_cleanup_armed` (type-checked, not yet run).

The Input Robustness Matrix does not apply (no file format or configuration is
read).

### Commands and results

- `cargo clippy -p darkmatter -p claudine --all-targets -- -D warnings`: clean.
- `cargo clippy -p darkmatter -p claudine --all-targets --target
  x86_64-pc-windows-gnu`: builds; the one warning is the pre-existing
  Windows-only `result_large_err` in `darkmatter/lib/tests/l1/lifecycle_edit_action.rs:47`
  (untouched). With `-D warnings` that blocks; left alone as out of scope.
- `git grep` for `crate::process_tree`, `process_tree::`,
  `isolate_process_tree`, `resume_main_thread`, `GROUP_REAP_LIMIT`,
  `configure_detached`, `own_process_group` in Darkmatter/Claudine Rust:
  only `as_handle` in `claudine/cli/src/commands/wrap/exec/termination/windows.rs`
  (provider-session supervision, out of scope).
- `just test` (darkmatter): 17044 passed, none slow. `just lint`: passes.
- `just test` (claudine): 12246 passed, none slow. `just lint`: passes.
- `just test-l2 level2_edit_action` (darkmatter): 2 passed.
- `just test` (sniff), first run: 3627 passed, **6 slow** (the CoreAudio and
  network callers from Phase 1, 8.7–14.5 s). After the
  `sniff-macos-coreaudio` group: 3627 passed, 1 slow —
  `hardware::audio::tests::detect_audio_devices_on_macos` at 5.81 s in the
  run, 3.04 s alone, load average 26 on 16 cores (loaded-host allowance
  10 s, so it is cleared). `network::tests::test_detect_network_returns_info`
  measured 0.015 s alone. `just lint` (sniff): passes;
  `just check-tier-coverage sniff`: nothing stranded.
- Downstream: no package outside Darkmatter and Claudine uses
  `darkmatter::editor` or the launcher. `cargo check --all-targets` of
  biscuit-icon-cli, biscuit-speaks-cli, biscuit-terminal-cli, messenger,
  messenger-cli, playa-cli, research, sniff-detection-cli, claudine-contract:
  passes. darkmatter-cli, darkmatter-lsp, claudine-cli, and claudine-gen ran
  in their areas' `just test`.
- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [claudine, darkmatter, test-toolkit, claudine-cli, repo-deps]`,
  `change_class: package`, so the gates were run, not skipped. `test-toolkit`
  and `repo-deps` are selected because their tests read `.config/nextest.toml`.
  Their L1 (`-E 'not test(/(^|::)(level2_|level3_|browser_|real_|slow_)/)'`):
  1482 passed, **5 slow** at load 29 — `repo-deps` tests that compile fixture
  crates with a child Cargo (`a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart`
  14.1 s, `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared`
  13.6 s, `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations`
  13.4 s; alone 3.5 s, 3.0 s, 3.1 s; two more at 5.6–5.7 s). They predate
  this fix, contend for Cargo/CPU with each other, and do not depend on the
  new test group; left as they are and reported.

### Cross-OS

- **Linux** (build-linux): `just cross-check claudine --os linux
  composition::sequence::task`: 236 passed. `just cross-check darkmatter --os
  linux shell_expansion::` was interrupted once (my 590 s wrapper timeout
  during the remote build), then build-linux stopped answering SSH
  ("Connection timed out during banner exchange", also from a direct
  `ssh -o BatchMode=yes build-linux`). No Darkmatter Linux evidence from this
  phase; CI's pull-request Linux leg covers it.
- **Windows** (build-win): `just cross-check claudine --os windows
  composition::sequence::task` built and listed 212 tests, then the Cargo
  storage preflight refused (B: 36.1 GiB free, 50 GiB required; the automatic
  sweep freed 0 GiB). Not overridden and nothing deleted (storage-strategy
  rules). `os_failing` not set: nothing failed. The Windows code type-checks
  (see clippy above).

### Working-tree changes not from this phase

`claudine/docs/topics/flow-control/flow-control.md` was edited at 06:40
(author prose about compose/inline-compose/sequence) and
`.darkmatter-shell-whitelist` was deleted while this phase ran. Neither is
part of this fix, so both were left as found and are not in the
`*_during_phase_2` lists.

## Phase 3

Documentation-only phase (plan Rules 26–28, Tasks 3.1–3.5), done directly
without subagents because every edit was a short, located passage.

### What changed

- **3.1** Created `sniff/docs/topics/process-trees.md`: what an owner is and
  why; configure → spawn → attach with the attachment-failure kill/wait and
  both isolation modes plus `windows_creation_flags`; terminating vs. reaping
  (the caller reaps; `Drop` cannot); `sample()` per poll and the
  `CleanupScan` table (zero scans for a short success); graceful cleanup and
  `TreeTermination`; `release` with the Windows failure staying armed; the
  `Drop` backstop and when it does not run; error handling; Guarantees and
  limits per OS (no Windows escape after attach, an enclosing Job that forbids
  assignment fails attach; Unix group + recorded descendants, the
  between-samples escape, group-ID and PID reuse, macOS repeated kill bounded
  at 250 ms, Linux/WSL2 one kill; ownership boundary); the Caller policies
  table, including the Unix terminal-foreground lifecycle exception and its
  reason; and the specification's Mermaid lifecycle diagram (configure,
  spawn, attach, owned, failure cleanup, terminate / release / drop). Every
  caller row was checked against the code (`process.rs`, `launcher.rs`,
  `editor/launch.rs`, Claudine `shell.rs`), including grace durations (2 s
  editor, 250 ms Claudine) and Windows flags (none for Sniff probes and the
  editor, `CREATE_NO_WINDOW` for login-shell queries,
  `CREATE_NEW_PROCESS_GROUP` for Claudine).
- **3.2** Sniff: architecture doc "Subprocess Deadlines" now names the public
  owner and links the topic in place of the "total on Windows / best-effort
  on Unix" block; lib README gained a "Process Trees" feature bullet and its
  install-timeout paragraph says the Job is attached before the installer
  starts; top-level README gained a bullet; CLI README partial-install
  paragraph cut to two sentences with the attached-Job wording and link;
  skill `architecture.md` says `process` is public for the owner and for
  `configure_detached_child`; `SKILL.md` names the owner and links the topic.
  Beyond the plan's list, the `InstallCapturedResult::timed_out` doc comment
  (`sniff/lib/src/programs/install/options.rs`) said "Windows containment is
  kernel-enforced and total" and pointed at the module docs; it now says the
  Job is attached before the installer runs and names the topic (comment-only
  change).
- **3.3** OS skill: the `SKILL.md` row no longer accepts a spawn-to-assign
  window; its symptom text now quotes the editor error the code actually
  produces ("editor workflow failed during contain the editor's process
  tree"; the old "could not be placed in its own process tree" text no longer
  exists anywhere in the code). `windows.md`: the SSH-session bullet says the
  degrade-to-direct-child fallback is `windows_wait_loop`'s provider policy
  and the shared owner fails closed; the edit-launcher bullet replaces
  `process_tree.rs` / "returns `None`" with Sniff's owner, assignment before
  resume, and the release-failure warning; console and `.cmd` sub-bullets
  kept. `macos.md` names `sniff/lib/src/process/tree/unix.rs` for the
  repeated kill. Darkmatter skills `lifecycle.md` and `library-surfaces.md`
  name `sniff::process::ProcessTree`, `LaunchReport`, and
  `EditOutcome::warning`. Darkmatter docs: lifecycle `index.md` and
  `error-handling.md` gained the release-failure warning sentence (blocks
  under `--deny all`) and the link; `inline/shell-expansion.md` gained the
  bounded, owned login-shell query sentence.
- **3.4** Claudine: `flow-control/lifecycle.md` keeps the foreground exception
  and adds that every other lifecycle shell command and every sequence shell
  task owns its tree; `flow-control/sequences.md` "Shell tasks" gained the two
  sentences and link; repo skill `SKILL.md` says task and lifecycle shells use
  Sniff's owner, keeping the `holds_terminal_foreground` exception text.
- **3.5** Checkpoint: the plan's stale-claim `git grep` (excluding
  features/fixes/research) finds only `.claude/skills/os/windows.md:301-302`,
  which state the new behavior ("nothing it starts escapes the Job",
  "`ProcessTree::attach` returns an error"); a wider search for
  "kernel-enforced", "nothing escapes", "is total" found only new wording and
  the `windows.rs`/`process.rs` comments that are correct after attach. All 17
  files in Rule 27 contain `process-trees.md` (checked per file), and each
  relative link resolves (checked with `ls` on the joined path). Rule 28: a
  search of `darkmatter/docs/use-skills` and `claudine/docs/use-skills` for
  process tree / process group / own_tree / kill-on-close / foreground finds
  only DMLS colour keys, so no caller skill was edited and
  `just publish-skills` was not run.

### Requirement-to-test mapping

No behavior changed in this phase (one doc comment in Sniff source; the rest
Markdown). The tests that read the edited files were run:

| Edited file read by a test | Test run | Result |
| --- | --- | --- |
| `claudine/docs/topics/flow-control/lifecycle.md` (`include_str!`) | `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E 'test(/loop_gate_ambient/)'` | 6 passed |
| `.claude/skills/os/SKILL.md` | `cargo nextest run -p test-toolkit -E 'test(=the_ci_documentation_states_the_implemented_behavior)'` | 1 passed |
| `.claude/skills/darkmatter/**`, `.claude/skills/claudine/**` | `cargo nextest run -p darkmatter -E 'test(/schema_roots::\|trigger_spelling_guard::\|current_root_documentation_contract::\|current_root_migration_guard::/)'` | 29 passed |
| `sniff/lib/src/process/tree.rs` doc examples (unchanged; cited by the topic) | `cargo test -p sniff --doc process` | 2 passed |

### Gates

- Scope calculator after all edits (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [claudine, darkmatter, sniff,
  test-toolkit, claudine-cli, repo-deps]`, `change_class: package` (the
  uncommitted Phase 2 changes are still in the tree). Phase 3's files alone:
  `packages: [sniff, test-toolkit, claudine-cli, darkmatter]`, source package
  `sniff` only. Gates run, not skipped.
- `just lint` (sniff): passes.
- `just test` (sniff): 3627 passed, 33 skipped, **1 slow**:
  `hardware::audio::tests::detect_audio_devices_on_macos` at 6.199 s (load
  6.8 on 16 cores at start). Rerun of the package once for timing: 6.041 s
  (load 10.9 → 15.8). Alone: 3.108 s (load 9.8). This is a pre-existing
  CoreAudio cost (per-process audio HAL setup with seven third-party
  plug-ins; Phase 2 recorded the same test at 5.81 s in-run); Phase 3
  touched nothing it runs. An experiment adding `threads-required = 4` to its
  existing `sniff-macos-coreaudio` override measured 5.721 s and was reverted
  (`.config/nextest.toml` is byte-identical to its post-Phase 2 state). Not
  cleared under the 5 s quiet-host bar; handed to Phase 4.
- `just check-tier-coverage sniff`: nothing stranded.
- Downstream: the only Rust change is a doc comment on a public field, so no
  reverse dependency's behavior or compilation can change; the doc-reading
  tests above cover the other packages the calculator selected.
- Cross-OS: not run; nothing with OS risk changed.
