---
fix: 2026-09-23-opencode-failure-lifecycle
deferred_perf_measurement: false
implementation_1: "2026-09-26T13:46:47-07:00"
implementation_2: "2026-09-26T16:38:32-07:00"
implementation_3: "2026-09-26T17:19:12-07:00"
---

# Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-09-26T13:46:47-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-23-opencode-failure-lifecycle/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- order of work: finding 3 (fixture reliability) first, then finding 2 (new L1 regressions built on those fixtures), then finding 1 (L2/L3 terminal scenarios); the later findings reuse the synchronized, self-cleaning fixtures the first one produces
- starting the work on 'Signal fixtures depend on scheduling and leave child processes behind' at 13:47:05
        - added `claudine/cli/tests/common/signal.rs` (Unix-only, declared from `common/mod.rs`): `SignalledRun` owns a `command_std()` child spawned as its own process-group leader with stdout/stderr captured to files; `Drop` SIGKILLs the whole group on success and on panic, so a lifecycle shell left behind by a deliberate wrapper force-exit is reaped; every wait is bounded so a hang panics into that cleanup instead of hitting nextest's timeout
        - `SignalledRun::interrupt()` asserts the `kill(2)` result and returns the instant taken just before the send (a strict lower bound on when the handler ran); `wait_for_stderr(needle, timeout)` acknowledges a press by its stderr notice; `wait_for_path` and `wait_for_exit` stop early with stderr if the wrapper exits
        - added `ShellBarrier` (readiness/release handshake): `block_snippet()` records the shell's pid then blocks until `release()`; `await_reached_snippet()` lets a provider wait for a descendant; dropping the barrier releases it, which ends an orphan outside any group the test owns
        - `wrap_sigint.rs`: the prep test now runs through `SignalledRun`; the orphan provider waits on the `orphan` barrier (reached only after the TERM trap is installed) before exiting; the grace tests block inside the `failure` shell action on a barrier, send the second press only after the first press's notice, release the short case only once the grace notice is observed, and never release the long case
        - grace timing is now two bounds: `press_to_exit >= 500ms` from the pre-send instant (exact, no allowance needed) and `grace_ack_to_exit < 500ms + SCHEDULING_ALLOWANCE (2.5s)`, with the allowance's justification in a comment; the fixed 50 ms sleep and the 0.2 s / 5 s shell sleeps are gone
        - fixture structs `OrphanTeardownFixture` and `TerminalLifecycleFixture` (each with `new`, `spawn`, `events`) are ready for finding 2's additional tests
        - discovery: a fake `opencode` orphan from the original implementation's fixture had been looping since 2026-09-23 (pgid 62393, `while :; do sleep 0.1`) — exactly the leak this finding describes; killed it and removed its temp workspace
        - verified: a probe that panicked after the force-exit with the barrier's release disabled left no processes, so the group kill alone reaps the surviving shell
        - `just test-cli wrap_sigint::` passed 6 consecutive runs; no fixture processes remained afterward
        - `just test` (7359 passed, 9 skipped) and `just lint` both pass
- work completed for 'Signal fixtures depend on scheduling and leave child processes behind' at 13:57:15
- starting the work on 'Teardown and grace regression tests leave explicit contract branches unprotected' at 13:57:15
        - discovery: no subprocess test can reliably put a *second* press inside the post-exit teardown; the first press sets the process-wide interrupt flag and `kill_process_group` breaks out within one 50 ms poll, and SIGINTs sent back-to-back can coalesce; the guard is therefore proven at the unit boundary, where a `SIGTERM`-ignoring group holds the teardown open for as long as the test needs
        - added `commands::wrap::exec::tests::kill_process_group_defers_repeat_interrupts_until_ctrl_c_ends_it` (Unix, `CLAUDINE_KILL_GRACE=60s`): while the teardown runs, `wait_loop_active()` is set and `press_rung(2, …)` resolves to `Defer`; `mark_user_interrupted()` then ends it within 5 s, the flag is released, and the group leader dies by `SIGKILL`; its `TermSurvivingGroup` kills the group on drop
        - the one-press orphan test's doc now states it sends one press and names the unit test that covers the repeat press
        - added `wrap_sigint::compose_orphan_teardown_ends_once_the_group_exits_without_an_interrupt`: `OrphanTeardownFixture::new` takes `OrphanOnTerm::{Survive, Exit}`; with `Exit` the orphan's trap touches its marker and exits, and with a 60 s kill grace the run must exit within 5 s of the marker, not print the interrupt notice, and still run `failure` then `finalize`
        - added `wrap_sigint::compose_terminal_lifecycle_outlasts_the_grace_without_an_interrupt` and `wrap_sigint::compose_single_sigint_lets_terminal_lifecycle_outlast_the_grace` (shared `hold_failure_stack_past_the_grace`): the `failure` action is held until 500 ms + `HOLD_PAST_GRACE` (1 s) past the hold point (barrier reached, or the pre-send instant of the single press) while the wrapper must stay running, then released; the full `failure-started`/`failure-done`/`finalize` sequence must run with no grace notice or force-exit
        - added `wrap_sigint::compose_later_sigints_do_not_postpone_the_grace_deadline`: presses 1 and 2 are acknowledged, press 3 is sent `LATE_PRESS_OFFSET` (250 ms) after press 2 and acknowledged by the second copy of the grace notice; the run must exit 130 no sooner than 500 ms after press 2 and less than 500 ms after press 3, with exactly one expired notice
        - added `composition::lifecycle::tests::guard_runtime::run_event_stack_marks_only_terminal_events_as_terminal_lifecycle_work` (lib): a probe emitter records `terminal_lifecycle_active()` from the top-level notification and a stack action of each event; `success`, `blocked`, `failure`, and `finalize` observe it, `start` does not, and it is clear after every event; `run_event_stack` is the choke point `execute_event` and every production caller reaches
        - discovery (not changed; outside this finding): the guard's `Drop` safety net emits a `Blocked`/`Failure` top-level notification through `emit_signal`, which does not enter the scope, so a repeat press during that notification still force-exits at once; `emit_terminal`/`emit_finalize_once` take the same path but have no production callers
        - helpers for the next agent in `common/signal.rs`: `SignalledRun::wait_for_stderr_count(needle, count, timeout)` (acknowledges a repeat press whose notice repeats) and `SignalledRun::assert_running_until(instant)` (panics if the wrapper exits early; used in place of a sleep)
        - sabotage proofs, each reverted and byte-compared with its backup afterwards: removing `WaitLoopActiveGuard` from `kill_process_group` failed only the unit test ("never marked itself as an active wait loop"); removing the empty-group early return (so the teardown sleeps out the 60 s grace) failed only the orphan-exit test (no exit within 15 s); dropping `Finalize` from the scope `matches!` failed the lib test for `Finalize`; letting a later press restart the watcher's sleep failed the late-press test (exit 760 ms after press 3); arming the grace on the first press failed the one-press test (exit about 1 s before the hold ended); the zero-press test was not sabotaged because no small change arms a deadline with no press without breaking every compose
        - a backup mix-up during sabotage (two files named `mod.rs` copied into one directory) overwrote `cli/src/commands/wrap/exec/mod.rs`; restored it from `HEAD`, which matched its pre-work state, then re-applied the test-only additions; `git diff` of that file now shows only added lines
        - `just test-cli kill_process_group_defers wrap_sigint::` passed 11 consecutive runs; `just test` (7365 passed, 9 skipped) and `just lint` pass; no fixture processes were left afterwards
        - no spec Verification-table change: no listed test was renamed
- work completed for 'Teardown and grace regression tests leave explicit contract branches unprotected' at 14:09:45
        - orchestrator note: the out-of-scope gap found here (`LifecycleRunGuard`'s `Drop` emits `blocked`/`failure` without entering `TerminalLifecycleScope`) is left for the author to schedule; it is not part of review 1's findings
- starting the work on 'Ctrl+C lifecycle behavior lacks verification at the required terminal levels' at 14:09:45
        - added `claudine/cli/tests/common/terminal_interrupt.rs` (Unix): `PaneCompose` types a `claudine compose` into a real pane (workspace as `HOME` and CWD) and waits for the window under test; the three scenarios are shared functions that take a `press` closure, so the focus-stealing L3 APIs stay in the `level3_` file (`test_placement.rs` rule)
        - the fixtures mirror `wrap_sigint.rs`'s orphan and terminal-lifecycle fixtures, with one change: the blocking `failure` shell action runs `trap '' INT` first
        - discovery (not changed; for the author): `SystemShellRunner` spawns a lifecycle `shell` action in Claudine's own process group, so a keyboard Ctrl+C also signals it; confirmed in a tmux pane that the first press kills an in-flight `failure` `shell` action, the stack stops there (no later actions), `finalize` runs, and the run exits 1; F2's grace therefore protects only lifecycle work that survives SIGINT (in-process actions, detached audio, SIGINT-ignoring shells)
        - added `level2/level2_lifecycle_ctrl_c_tmux.rs` (tmux, `send_key("C-c")`, owned `TmuxHarness` killed on drop, pane resized to 160x60): `level2_ctrl_c_during_orphan_teardown_runs_failure_and_finalize`, `level2_repeat_ctrl_c_lets_a_short_terminal_lifecycle_finish`, `level2_repeat_ctrl_c_force_exits_a_long_terminal_lifecycle`; assertions read the drawn pane (interrupt, grace, and force-exit notices) and the shell-echoed exit status
        - added `level3/level3_lifecycle_ctrl_c.rs` (macOS only, `#[serial(level3_keyboard)]`, `require_level!(L3, WezTerm + cliclick)`, foreground WezTerm + `focus_spawned_pane` once, then `click_then_ctrl_chord` per press), with the same three scenarios; declared under `#[cfg(target_os = "macos")]` like the other macOS L3 files
        - added `ShellBarrier::reached_path` and `ShellBarrier::release_and_await_exit` to `common/signal.rs`
        - discovery: the first L2 runs leaked the force-exited `failure` shell (and, in a sabotaged run, the orphan); the barrier's drop wrote the release file, but the `TempDir` was deleted immediately afterward, so the shell never saw it; `PaneCompose`'s `Drop` now releases and waits (bounded 5 s) for the recorded shell pid to exit before the workspace is removed; the leaked processes were killed
        - sabotage proofs, each reverted and byte-compared with its backup: without `trap '' INT` both repeat-press tests fail (the keyboard SIGINT kills the action, which shows foreground-group delivery participates); disabling `PressRung::GraceExit` fails both repeat-press tests; disabling the interrupt check in `kill_process_group` fails the orphan test (no exit within 20 s)
        - `just test-l2 level2_lifecycle_ctrl_c_tmux::` with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` passed 5 consecutive runs (plus 1 earlier run); a full `just test-l2` passed (240 + 3); no tmux sessions or fixture processes remained afterward
        - L3 compiled and listed (`cargo nextest list … level3`) but not executed: it raises a GUI window and types into whatever holds focus, and `just test-l3` refuses to run unattended; this session is non-interactive
        - `just check-tier-coverage claudine` (0 stranded), `just lint`, clippy `-D warnings` on the level2/level3 targets, `just test` (7365 passed, 9 skipped), and a `cargo check --target x86_64-pc-windows-gnu` of the l1/level2/level3 targets all pass
- work completed for 'Ctrl+C lifecycle behavior lacks verification at the required terminal levels' at 14:28:26
        - the Level-3 scenarios are written and compiled; running them needs an attended `just test-l3` on macOS because they take keyboard focus

### Successful Completion

The implementation of review cycle 1 has completed successfully in 42 minutes (13:46:47 to 14:28:47). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- one follow-up is still open: the Level-3 keyboard scenarios in `level3_lifecycle_ctrl_c.rs` were compiled but not run, because they take keyboard focus and this session was non-interactive
- two out-of-scope discoveries are recorded for the author:
        - `LifecycleRunGuard`'s `Drop` emits a `blocked`/`failure` notification without entering `TerminalLifecycleScope`, so a repeat Ctrl+C during that path still force-exits immediately
        - lifecycle `shell` actions share Claudine's foreground process group, so a keyboard Ctrl+C kills an in-flight `shell` action; F2's 500 ms grace protects only work that survives SIGINT

The files changed in this cycle:

- `claudine/cli/tests/common/signal.rs` (new)
- `claudine/cli/tests/common/terminal_interrupt.rs` (new)
- `claudine/cli/tests/level2/level2_lifecycle_ctrl_c_tmux.rs` (new)
- `claudine/cli/tests/level3/level3_lifecycle_ctrl_c.rs` (new)
- `claudine/cli/tests/common/mod.rs`
- `claudine/cli/tests/l1/wrap_sigint.rs`
- `claudine/cli/tests/level2/main.rs`
- `claudine/cli/tests/level3/main.rs`
- `claudine/cli/src/commands/wrap/exec/mod.rs` (only the test module changed)
- `claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs`

## Implementation of Review Findings #2

> **started at:** 2026-09-26T16:38:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-23-opencode-failure-lifecycle/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- order of work: finding 2 (fixture cleanup) first, then finding 1 (L3 keyboard scenarios); the new L3 scenarios reuse the fixtures whose teardown finding 2 hardens
- starting the work on 'Orphan fixture cleanup can race workspace removal' at 16:39:09
        - discovered the orphan fixture never owned the workspace: it drops before `CliProcessFixture` only by local declaration order, and `ShellBarrier::drop` only wrote the release file
        - discovered `SignalledRun::poll_exit` reaped the leader with `try_wait`, so `Drop`'s group kill could target a recycled group id
        - changed `SignalledRun` to observe exit with `waitid(WEXITED|WNOHANG|WNOWAIT)` and reap only in `Drop`, after the group `SIGKILL`
        - changed `ShellBarrier`: `release_or_kill` (renamed from `release_and_await_exit`) releases, awaits exit, then `SIGKILL`s as a fallback; `Drop` calls it; snippets also stop once the barrier directory is gone
        - tied `OrphanTeardownFixture` and `TerminalLifecycleFixture` to `&'a CliProcessFixture` so the compiler enforces workspace-outlives-barrier
        - new L1 tests in `wrap_sigint.rs`: orphan-fixture drop, barrier kill fallback, unreaped-exit, signal-death decoding; wrap_sigint suite green 5/5 runs
        - mutation check: reverting to reap-in-poll and release-only drop makes `signalled_run_keeps_an_exited_wrapper_unreaped_until_drop` and `orphan_fixture_drop_ends_an_orphan_outside_the_wrapper_group` fail; sources restored
        - `just test`: 7369 passed, 9 skipped; `just lint` passed; `cargo clippy -p claudine-cli --features terminal-tests --tests -D warnings` clean; `cargo check --target x86_64-pc-windows-gnu -p claudine-cli --tests` clean
        - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_lifecycle_ctrl_c_tmux::`: claudine-cli 3/3 passed twice (tmux run=3); the recipe still exits 1 because its claudine-gen leg selects no tmux-gated test and fails the backend proof, a recipe artifact of the filter rather than a test failure
        - no leaked fixture processes (`pgrep` for orphan, stubborn, and sleep loops empty)
        - deviation from plan R6: the leader is kept unreaped until `Drop` (group kill, then reap) rather than killed-and-reaped at exit observation, so force-exit tests still leave descendants for `Drop` as the module doc says
        - deviation from plan R7: the release/await/kill sequence lives in `ShellBarrier::drop` (shared with `PaneCompose`) instead of a separate `OrphanTeardownFixture::drop`; the workspace-outlives-barrier ordering is compiler-enforced by the fixtures' `&'a CliProcessFixture` borrow; the fallback kill is reported on stderr and its residual pid-reuse risk is documented
- work completed for 'Orphan fixture cleanup can race workspace removal' at 16:47:18
- starting the work on 'Keyboard verification still misses interrupt-count contracts' at 16:48:16
        - read plan R10: it chose "two presses back-to-back, no production hook"; rejected here because a back-to-back second press lands in the teardown's ≤ 50 ms window, the unguarded post-teardown gap (S1/R5, never spiked), or the terminal scope, so the test would not reliably exercise the `Defer` rung it is named for
        - option (a), pure-test arrangement, is impossible by construction: every press marks the interrupt flag, `kill_process_group` breaks to `SIGKILL` at its next 50 ms poll and returns without waiting, and `SIGKILL` cannot be delayed by the target, so nothing a fixture does keeps the teardown open after the first press
        - option (b) chosen: `hold_teardown_for_terminal_tests` in `cli/src/commands/wrap/exec/mod.rs`, compiled only under `#[cfg(all(unix, feature = "terminal-tests"))]` (never in default or installed builds); with `CLAUDINE_TEST_TEARDOWN_HOLD=<path>` it writes `<path>.reached` after the final `SIGKILL` and waits (≤ 60 s) for `<path>.release`, still inside the `WaitLoopActiveGuard`
        - delivery proof for the deferred press: the pane's line discipline echoes `^C` (`ECHOCTL`, verified in a scratch tmux pane) as it raises `SIGINT`, so the scenario waits for a second `^C`, then requires no exit and no force-exit notice for 500 ms before releasing the hold
        - added scenarios in `common/terminal_interrupt.rs`: `assert_repeat_press_during_orphan_teardown_defers`, `assert_single_press_lets_lifecycle_outlast_the_grace`, `assert_later_press_does_not_postpone_the_grace`, plus `wait_for_notice_count` and `assert_pane_running_until`
        - third-press timing: press 3 is sent ≥ 250 ms after press 2 was sent (send-to-send, so an OS chord's injection latency cancels); asserted exit ≤ 500 ms + 3 s after press 2, and < 400 ms after the second grace notice is seen (a restarted deadline exits ≥ 500 ms after delivery)
        - discovered the interrupt notice ends without a newline, so `echo MARKER:$?` landed mid-row after a single press; the typed command now prints the status marker on a fresh line
        - wired 3 new `level2_` tests (tmux) and 3 new `level3_` tests (`#[serial(level3_keyboard)]`, WezTerm + cliclick); L3 module doc updated
        - `just test-l2 level2_lifecycle_ctrl_c_tmux::` (tmux required): claudine-cli 6/6 passed
        - discovered the deferred-press run exits 1 (the provider failure), so the F1 scenario asserts a status that is neither 0 nor the force-exit 130
        - sabotage 1 (production seam proof): removed `WaitLoopActiveGuard` from `kill_process_group` → `level2_repeat_ctrl_c_during_orphan_teardown_is_deferred` failed (pane: second `^C`, "second interrupt — force-exiting compose", status 130); restored and byte-compared
        - sabotage 2: grace watcher re-arms its 500 ms timeout on every later byte → `level2_third_ctrl_c_does_not_postpone_the_grace` failed (exit 492 ms after the third press was seen, bound 400 ms); passing runs measure 162–260 ms; restored and byte-compared
        - sabotage 3: the first press also arms the grace watcher → `level2_single_ctrl_c_lets_a_terminal_lifecycle_outlast_the_grace` failed (wrapper exited 1.04 s early); restored and byte-compared
        - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_lifecycle_ctrl_c_tmux::`: claudine-cli 6/6 passed on 5 consecutive runs (the recipe's claudine-gen leg still exits 1 for selecting no tmux test, the known filter artifact)
        - `just test`: 7369 passed, 9 skipped; `just lint` passed; `just check-tier-coverage claudine`: 0 stranded; clippy `-D warnings` clean on `--test level2 --test level3 --bins` with `terminal-tests` and on `--tests --bins` without it (seam compiled out); `cargo check --target x86_64-pc-windows-gnu -p claudine-cli --tests --features terminal-tests` clean apart from the pre-existing `GENERATED_MARKER` dead-code warning
        - `cargo nextest list -p claudine-cli --features terminal-tests level3_lifecycle_ctrl_c` lists 6 tests; L3 compiled and listed only, never run (author step, plan R8)
        - deviation from plan R10: a feature-gated production seam replaces the back-to-back double press; justified above, and the seam is absent from default and installed builds
        - no leaked tmux sessions or fixture processes; removed my scratch `exp-f1` tmux socket
- work completed for 'Keyboard verification still misses interrupt-count contracts' at 17:02:08
- orchestrator verification at 17:08:45
        - reviewed the `terminal-tests` seam: it is `cfg`-gated, reads only `CLAUDINE_TEST_TEARDOWN_HOLD`, and is a no-op when that is unset; kept
        - `just cross-check claudine-cli --os linux` (run twice, because the recipe cannot filter tests): all 12 `wrap_sigint::` tests passed on build-linux, so the `waitid(WNOWAIT)`/`siginfo_t` decoding in `common/signal.rs` works on Linux
        - the same runs failed 3 `sequence_groups::` tests (`parallel_body_lines_carry_their_own_tasks_bar_color`, `parallel_group_members_are_attributed_across_both_channels`, `serial_and_parallel_group_frames_share_one_left_edge`); the group-header line is missing from the Linux render; this fix and this branch do not touch that code or those tests, so they are left for the author

### Successful Completion

The implementation of review cycle 2 has completed successfully in 31 minutes (16:38:32 to 17:09:30). During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- open follow-ups for the author:
        - the six Level-3 keyboard scenarios in `level3_lifecycle_ctrl_c.rs` (three new) compile and list but were not run, because they take keyboard focus; they need an attended `just test-l3` on macOS; the third-press scenario is the most sensitive to chord-injection latency and fails with a clear message rather than passing wrongly
        - the F1 repeat-press scenario relies on a `terminal-tests`-only production seam (`hold_teardown_for_terminal_tests`), a deviation from plan R10; the spec's Verification table does not yet describe it
        - a second press that lands after teardown but before the terminal lifecycle scope is still unguarded and unmeasured (plan S1/R5)
        - 3 `sequence_groups::` L1 tests fail on Linux; this fix did not cause them

The files changed in this cycle:

- `claudine/cli/src/commands/wrap/exec/mod.rs` (the `terminal-tests` teardown seam)
- `claudine/cli/tests/common/signal.rs`
- `claudine/cli/tests/common/terminal_interrupt.rs`
- `claudine/cli/tests/l1/wrap_sigint.rs`
- `claudine/cli/tests/level2/level2_lifecycle_ctrl_c_tmux.rs`
- `claudine/cli/tests/level3/level3_lifecycle_ctrl_c.rs`

## Implementation of Review Findings #3

> **started at:** 2026-09-26T17:19:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-23-opencode-failure-lifecycle/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'Repeat-interrupt teardown test can miss its hold window' at 17:19:40
        - confirmed the finding: in `kill_process_group` the `terminal-tests` hold ran only after the final `SIGKILL`, so the early `return`s (SIGTERM finding no member, or the signal-0 probe seeing an empty group) skipped it and `wait_for_teardown_hold` timed out
        - tried to reproduce a spurious empty-group probe on macOS (leader exited and reaped, orphan trapping TERM with a `/bin/sleep 0.01` loop, ~3.7 M `killpg(pg, 0)` probes over 3 s): no failures, so why the orphan looked gone in the reviewer's run is still unexplained; the fix below does not depend on it
        - change: split the signalling and polling out of `kill_process_group` into `terminate_process_group(pgid, kill_grace)`; `kill_process_group` takes the `WaitLoopActiveGuard`, calls it, then calls `hold_teardown_for_terminal_tests()` (still `#[cfg(feature = "terminal-tests")]`), so the hold runs on every exit path inside the guard; default builds get the same behavior as before
        - updated the seam's doc (when `.reached` is written; a group that exits on SIGTERM ends the teardown with no press) and the test docs that said the hold is reached only after a press (`terminal_interrupt.rs` module doc, `orphan_teardown_held`, `wait_for_teardown_hold`, `assert_repeat_press_during_orphan_teardown_defers`)
        - reviewed the shared scenario for other assumptions that the orphan is alive at the hold: none remain; if the orphan is already gone, the first press lands in the hold and is still `PressRung::Notice` (count 1), and the second is still `Defer` only because of the guard; `ShellBarrier::release_or_kill` already tolerates a dead orphan
        - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_lifecycle_ctrl_c_tmux::`: claudine-cli 6/6 passed on 1 + 5 consecutive runs, and 6/6 on 5 more consecutive runs with 16 `yes` busy-loops saturating all 16 cores (the recipe's claudine-gen leg still exits 1 for selecting no tmux test, the known filter artifact)
        - early-return proof: temporarily made the fixture orphan `exit 0` in its TERM trap, so the group empties at the SIGTERM probe; with the fix, `level2_repeat_ctrl_c_during_orphan_teardown_is_deferred` passed 1/1; with the hold moved back to after the final `SIGKILL` only (the pre-fix placement), it failed after 11.6 s with "the teardown was not held within 10s of the press", the reviewer's failure; both files restored and byte-compared (`cmp`)
        - sabotage (required): removed `WaitLoopActiveGuard` from `kill_process_group` → `level2_repeat_ctrl_c_during_orphan_teardown_is_deferred` failed in 1.8 s (pane: "second interrupt — force-exiting compose", status 130); restored and byte-compared; `git diff --check` clean and the diff shows only the intended hunks
        - `just test`: 7369 passed, 9 skipped; `just lint` passed; `cargo clippy -p claudine-cli --features terminal-tests --test level2 --test level3 --bins -- -D warnings` clean
        - `cargo nextest list -p claudine-cli --features terminal-tests level3_lifecycle_ctrl_c` lists 6 tests (listed only, not run); `just check-tier-coverage claudine`: 0 stranded
        - no leaked fixture processes; the only tmux session present (`wtcomp`, created 17:14:23) predates this work and was left alone
        - not verified: L3 (`just test-l3`) was not run because it takes focus
        - `cargo check --target x86_64-pc-windows-gnu -p claudine-cli --tests --features terminal-tests` clean apart from the pre-existing `GENERATED_MARKER` dead-code warning; `terminate_process_group` is `#[cfg(unix)]` like its only caller
- work completed for 'Repeat-interrupt teardown test can miss its hold window' at 17:29:31

### Successful Completion

The implementation of review cycle 3 has completed successfully in 11 minutes (17:19:12 to 17:30:05). During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- open follow-ups for the author:
        - the six Level-3 keyboard scenarios in `level3_lifecycle_ctrl_c.rs` compile and list but were not run, because they take keyboard focus; they need an attended `just test-l3` on macOS
        - why the orphan's process group looked empty in the reviewer's run is unexplained; the fix holds the teardown on every exit path, so it does not depend on the cause
        - `assert_press_during_orphan_teardown_runs_the_lifecycle` (the single-press orphan scenario) uses no hold; if the orphan leaves its group before the press, the teardown ends early and the scenario could fail the same way; it was left unchanged to keep this cycle narrow

The files changed in this cycle:

- `claudine/cli/src/commands/wrap/exec/mod.rs` (`terminate_process_group` split out; the `terminal-tests` hold now runs on every teardown exit path inside `WaitLoopActiveGuard`)
- `claudine/cli/tests/common/terminal_interrupt.rs` (doc comments only)
