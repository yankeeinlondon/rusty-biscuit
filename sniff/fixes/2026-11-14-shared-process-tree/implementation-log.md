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
message_to_agent: |-
    Phase 1 is in place: `sniff::process::{ProcessTree, TreeConfig,
    TreeIsolation, CleanupScan, TreeTermination, WINDOWS_CREATE_NO_WINDOW,
    WINDOWS_CREATE_NEW_PROCESS_GROUP}` match plan Rule 2 exactly. Notes for
    Phase 2:
    - `Drop` lives on the platform `Core`, not on `ProcessTree`, so
      `release(self)` can move out; dropping a `ProcessTree` still runs the
      armed backstop.
    - Unix `sample()` and `terminate*` do nothing after `release`, and a
      second `terminate` after a successful one is a no-op on both OSes.
    - Unix `attach` now verifies the group (`getpgid`); a command that was not
      configured with `TreeConfig::configure` fails with "foreign process
      group". Darkmatter's editor must call `configure` before the terminal
      handoff `pre_exec` (tested: a caller hook sees the new group).
    - Windows code compiles and passes clippy from this Mac with
      `cargo clippy -p <pkg> --all-targets --target x86_64-pc-windows-gnu --
      -D warnings` (the target is installed). Use it for Darkmatter and
      Claudine after their `windows` feature trims; it type-checks the Windows
      tests but cannot run them.
    - `sniff/lib/src/process.rs` module docs already name
      `sniff/docs/topics/process-trees.md` in prose; that page is created in
      Phase 3. `.claude/skills/sniff/architecture.md` still says `process` is
      public only for `configure_detached_child`; Phase 3 task 3.2 fixes it.
    - Windows tests were type-checked but never executed: build-win's `B:`
      volume was below the 50 GiB storage preflight (35.8 GiB free). Task
      4.4 must run `just cross-check sniff --os windows process::` (use the
      positional filter; `-E 'test(...)'` breaks in the recipe's remote
      quoting). Linux passed all 47 process tests on build-linux.
    - Six live CoreAudio/network detection tests in sniff are slow in the full
      package run on this Mac (12–14 s); see the Phase 1 log. They predate this
      fix and do not touch the process code.
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
