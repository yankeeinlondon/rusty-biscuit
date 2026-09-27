---
total_phases: 4
created: 2026-09-26
phase: 1
agent: claude/default
yolo: true
---

# Plan: `2026-09-23-opencode-failure-lifecycle` — review-1 remediation

## Summary and Definition of Done

### Where this fix stands

F1–F3 are implemented in `90c3cecf1`. Review 1 (`review-1.md`, codex/default)
agrees that the design and the code paths are right. It marks the fix **not
production ready** because verification is weak:

| # | Priority | Finding |
|---|---|---|
| RV1 | high | No Level-2/Level-3 coverage for the two new Ctrl+C windows (orphan teardown, repeat press during a terminal lifecycle event), and no real-terminal capture of the grace / grace-expired notices |
| RV2 | medium | L1 tests leave contract branches unprotected: removing `WaitLoopActiveGuard` from `kill_process_group` does not fail any test; no automated test covers the empty-group latency fix (F1's main claim); no tests cover zero or one press, "later presses do not extend", or scope entry for `success`/`blocked`/`finalize` |
| RV3 | medium | Signal fixtures rely on fixed sleeps rather than barriers; `libc::kill` results are ignored; process trees leak (the force-exit case orphans a 5 s shell, and a panic drops a plain `Child`); the orphan fixture can exit before its `TERM` trap is installed |

Planning also found three gaps that neither the spec nor the review covers.
Each one needs a ruling (see Phase 1):

1. **Notification side effects are skipped after any Ctrl+C.**
   `LifecycleRunGuard::emit_signal` (`lib/src/composition/lifecycle/mod.rs`)
   returns right after the stderr line when `interrupt::interrupted()` is
   true, so `message`, `notify`, `say`, and `effect` never run.
   The prompt from the report (`prompts/_implement/implement-plan.md`) uses
   exactly that notification form (`say`/`message`/`effect`). So with F1+F2 in
   place, an operator who presses Ctrl+C still gets no `failure` audio or
   message. The fix still works for the reported case only because F1 makes
   the press unnecessary: teardown drops from 10.9 s to 1.2 s.
2. **`run_event_stack` is not the single choke point** the spec describes.
   `LifecycleRunGuard`'s `Drop` backstop (and the test-only
   `emit_terminal`/`emit_finalize_once`) call `emit_signal` directly, outside
   `TerminalLifecycleScope`. A repeat press during a backstop-emitted
   `failure` notification therefore still takes `PressRung::ForceExit`.
3. **In a real terminal, Ctrl+C reaches lifecycle shell children.** The
   terminal sends `SIGINT` to the whole foreground process group. Lifecycle
   `shell` actions run in the wrapper's group (no `process_group` or `setsid`
   in `lib/src/composition/lifecycle`), so the first press probably kills a
   running `failure`-stack shell action. The L1 `libc::kill(pid, …)` tests
   cannot see this. Spike S2 decides whether this is a product defect.

### Work required

- Rule on the gaps above, then run two short spikes: fixture mechanics, and
  real-terminal semantics.
- Add a small test seam to the teardown so the guard and the early-return
  behavior can be pinned deterministically.
- Rewrite the L1 signal fixtures around explicit barriers, checked signal
  delivery, and process-tree ownership. Then add the missing L1 controls.
- Add a lib-level test that proves all four terminal events enter
  `TerminalLifecycleScope`, and extend the scope to the `Drop` backstop.
- Add L2 (tmux, no focus) and L3 (WezTerm + cliclick, focus-authorized)
  scenarios for both new windows.
- Reconcile the spec, the signal-handling docs, and the test comments.

### Definition of done

- Every row of review-1's requirement-to-verification map is covered at the
  level the review asks for. The Phase 4 resolution table maps each finding
  to the test that closes it.
- **Mutation-proven L1 tests.** Each of these deliberate regressions, applied
  temporarily and reverted, makes a named test fail:
  - removing `WaitLoopActiveGuard` from the teardown;
  - replacing group polling with a full-grace sleep;
  - disabling `PressRung::GraceExit`;
  - dropping each terminal signal from the scope `matches!`;
  - letting a later press re-arm the grace.
- No new or rewritten signal test uses a fixed sleep as its synchronization.
  Every `kill` result is asserted, and no fixture leaves processes behind on
  success, on failure, or after a deliberate wrapper force-exit.
- New L1 tests pass 25 consecutive runs on macOS with no flakes.
- `just test`, `just lint`, and `just check-tier-coverage claudine` pass.
  `just test-l2` passes for the new L2 tests.
- The L3 tests compile and list under `terminal-tests`. The author runs them
  under `BISCUIT_L3_TAKE_FOCUS=1` (see R4).
- The spec's Verification table and prose match the code, including the
  rulings below. Every comment that claimed more than its test proved is
  fixed.

Out of scope: pre-launch model validation (excluded by the 2026-07-06 Phase F
ruling), the `UserInterruptGuard` unregistration question, and the
`ref`-less OpenCode fallback. The last two stay as spec follow-ups.

## Phase 1 — Rulings and spikes

### Necessary Rules

Each ruling carries a recommendation. An implementer applies the
recommendation unless the author overrides it before Phase 2 starts.

- [ ] **R1 — Interrupted runs skip notification side effects (keep).**
  Keep the `emit_signal` short-circuit. A user who pressed Ctrl+C asked the
  run to stop, and outbound messages and audio are the slow side effects the
  short-circuit exists to skip. Stack actions (the `stack:` form) still run.
  Amend the spec so it says this plainly and so the reported scenario is
  credited to F1's latency fix, not to the interrupt path.
  *Alternative the author may choose instead:* let the `failure`
  notification's `message` fire even after an interrupt. That is a behavior
  change and would get its own task in Phase 2.
- [ ] **R2 — The terminal scope covers every terminal emission path.**
  Move the `matches!(signal, Success | Blocked | Failure | Finalize)` scope
  entry into one private helper. Call it from both `run_event_stack` and
  `emit_signal`; the depth counter already tolerates nesting. The `Drop`
  backstop is then covered too. Change the spec's "single execution choke
  point" wording to describe what is true.
- [ ] **R3 — A test seam for the teardown is allowed.** Extract the Unix body
  of `kill_process_group` into
  `terminate_process_group(pgid, kill_grace, interrupted: impl Fn() -> bool)`
  in `cli/src/commands/wrap/exec/mod.rs`. Production passes
  `crate::output::user_interrupt_observed`. The guard,
  `SIGTERM` → poll → `SIGKILL` order, and 50 ms cadence stay exactly as they
  are. This is the cheapest boundary that can fail when the guard is removed.
  At the subprocess level, a repeat press lands in a ≤ 50 ms window and
  cannot be made deterministic.
- [ ] **R4 — L3 execution is an author step.** L3 keyboard injection must
  focus a window. The `biscuit-test-harness` rules confine that to `level3_`
  files and to runs authorized with `BISCUIT_L3_TAKE_FOCUS=1`. Implementing
  agents write, compile, and list the L3 tests but never run them. L2 tmux
  tests are the real-terminal evidence agents can produce without taking
  focus. L3 results are recorded in the spec's Verification table after the
  author runs them.
- [ ] **R5 — Grace timing is measured from confirmed delivery.** Start the
  grace clock when the test *observes* the grace notice in the wrapper's
  stderr (a file, polled every 10 ms), not when it calls `kill`. Bounds:
  - force-exit: ≥ 350 ms and < 3 s after the notice;
  - "later presses do not extend": keep pressing every 100 ms for 2.5 s after
    the grace notice and assert exit < 1.5 s. An extending implementation
    cannot pass that, and it needs no tight ceiling.

  Document each allowance at its assertion.
- [ ] **R6 — The Windows grace path stays out of scope.** The review states
  that cross-OS evidence is not a readiness condition. The Windows
  `ComposeInterruptEffect::GraceExit` branch keeps its pure classification
  test. Add a spec follow-up for a `GenerateConsoleCtrlEvent` parity test in
  `wrap_ctrl_c_windows.rs`.
- [ ] **R7 — F2 subprocess tests move to their own module.** Their fixture is
  being rewritten anyway, so move them into `cli/tests/l1/wrap_sigint_grace.rs`
  (declared in `cli/tests/l1/main.rs`). `wrap_sigint.rs` keeps prep and
  teardown (F1) tests. This lets the Phase 2 waves edit different files in
  parallel.
- [ ] **R8 — Shell children in a real terminal: decided by S2.** If S2 shows
  that the first real Ctrl+C kills a running lifecycle `shell` action, the
  author picks one of these before Phase 3:
  - **(a)** record it as intended behavior: the press interrupts the
    foreground job, and the stack's error routing applies;
  - **(b)** run lifecycle shell actions in their own process group while a
    terminal event is running, so only the grace ladder decides when they die.

  Recommendation: **(a)** for this fix. Assert the observed behavior in L2
  and L3, and open an `_unscheduled` fix if (b) is wanted. Adding (b) here
  would widen a remediation into a new process-model change.

### Wave 1 — spikes (parallel)

Spike notes go in `spikes/` in this fix directory. Each note ends with a
one-paragraph conclusion that the Phase 2/3 tasks cite. The whole session
tree is non-interactive: spike subagents must not run anything that prompts,
steals focus, or waits on a TTY.

- [ ] **S1 — L1 fixture mechanics** → `spikes/s1-signal-fixtures.md`
  - Confirm the first-press notice (`User interrupted compose operation…`)
    reaches a stderr **file** promptly enough to act as the acknowledgment
    before the second press. Confirm the grace notice does the same for R5's
    clock.
  - Confirm `command_std()` accepts `std::os::unix::process::CommandExt::process_group(0)`
    without violating `spawn_site_guard.rs`. Confirm that lifecycle `shell`
    children (and the fake provider's orphan, which sits in the provider's
    own group) are reachable for cleanup: either through the wrapper's group,
    or by writing a PID file the owner guard kills.
  - Confirm that a blocking release barrier works inside a `shell` action
    (for example `while [ ! -e "$RELEASE" ]; do /bin/sleep 0.01; done`).
  - Confirm that a stack `append_line`/`shell` still runs after one
    `SIGINT`, since R1 only affects notifications. This is what the "one
    press does not arm a deadline" control depends on.
- [ ] **S2 — Real-terminal semantics (tmux only, no focus)** →
  `spikes/s2-real-terminal-ctrl-c.md`
  - In a `TmuxHarness` pane, run `compose` with a `failure` stack that blocks
    on a release barrier. Send `C-c` once, then twice. Record what happens to
    the stack's shell child (R8), which notices the pane shows, and the exit
    code (read with a chained `echo "exit=$?"` sentinel).
  - Repeat during orphan teardown (`CLAUDINE_KILL_GRACE=60s`). Confirm the
    provider's separate process group shields the orphan from the pane's
    `SIGINT`, so the teardown, not the terminal, ends it.
  - Note any tmux capture quirk (wrapping, the `⚠` glyph, ANSI) that the L2
    assertions must tolerate.

### Phase 1 checkpoint

- [ ] R1–R7 are confirmed or overridden, and R8 is decided using S2's
  evidence. Record the outcomes in this section (✅ plus a one-line outcome).
- [ ] If S1 finds that shell children escape the wrapper's group, update the
  Phase 2 owner-guard task to use the PID-file approach before starting it.

## Phase 2 — L1 and library regression hardening

### Wave 2 — shared fixture support (single task; blocks Wave 3)

- [ ] **Signal fixture toolkit** (`cli/tests/common/`, likely a `signals.rs`
  submodule)
  - `ProcessTreeOwner`: wraps the spawned wrapper `Child` (spawned with
    `process_group(0)`, per S1). Its `Drop` sends `SIGKILL` to the group,
    kills any PIDs registered from PID files, and reaps the child. It runs on
    success, on panic, and after a deliberate wrapper force-exit.
  - `send_sigint(pid)`: asserts that `libc::kill` returned 0, with `errno` in
    the message.
  - `wait_for_file(path, timeout)` and `wait_for_text(path, needle, timeout)`:
    bounded polling that fails with the file's contents on timeout.
  - `release(path)`: creates a barrier file.
  - Fixtures write stderr to a file so notices can be polled mid-run.
  - Use the `rust-testing` skill's repository-read spellings. Keep the
    helpers `#[cfg(unix)]`.

### Wave 3 — regression tests (parallel; each task owns distinct files)

- [ ] **Teardown seam and units** (`cli/src/commands/wrap/exec/mod.rs`)
  - Apply R3. Behavior must not change.
  - Unit test (guard): spawn `sh` with `process_group(0)` that ignores
    `TERM`, call `terminate_process_group` with a probe closure that records
    `crate::output::wait_loop_active()` on every call, and assert the probe
    saw `true`. This must fail with the guard removed.
  - Unit test (early return): a group whose members exit on `TERM`, with a
    60 s grace, returns in < 5 s. This must fail if polling is replaced by
    `sleep(kill_grace)`.
  - Unit test (interrupt cut): a `TERM`-ignoring group and a probe that
    returns `true` from the second call → returns promptly, and the group is
    gone (`kill(-pgid, 0) != 0`).
  - Update the `kill_process_group` docs and inline comments to match.
- [ ] **F1 subprocess tests** (`cli/tests/l1/wrap_sigint.rs`)
  - Rewrite the orphan fixture's readiness: the background subshell installs
    its `TERM` trap and then touches `trap-ready`, and the fake provider waits
    for `trap-ready` before `exit 1`.
  - New `compose_orphan_teardown_ends_when_the_group_empties`: the orphan
    exits on `TERM`, `CLAUDINE_KILL_GRACE=60s`, no interrupt. Assert the run
    ends in < 10 s with `events.log == [failure, finalize]`. This is F1's main
    latency claim.
  - Keep `compose_sigint_during_orphan_teardown_runs_failure_lifecycle` on
    the new toolkit (checked `kill`, `ProcessTreeOwner`, stderr file). Correct
    its doc comment: it covers the interrupt cutting the grace short; the
    repeat-press deferral is pinned by the seam unit test.
- [ ] **F2 subprocess tests** (new `cli/tests/l1/wrap_sigint_grace.rs`, per R7)
  - The fixture's `failure` stack is `append_line started` →
    `shell: <touch ready; wait on release barrier; optional sleep>` →
    `append_line done`, plus a `finalize` `append_line`. It is owned by
    `ProcessTreeOwner`.
  - Shared sequencing: wait for `ready` → press 1 → wait for the
    first-press notice → press 2 → wait for the grace notice (the R5 clock
    starts here).
  - `…_short_failure_stack_finishes`: release right after the grace notice →
    events `[started, done, finalize]` and a non-130 failure exit.
  - `…_long_failure_stack_force_exits_after_the_grace`: never release →
    exit 130 within R5's bounds, `events == [started]`, and the owner guard
    reaps the orphaned shell.
  - `…_later_presses_do_not_extend_the_grace`: the long case plus presses
    every 100 ms (R5) → exit < 1.5 s after the grace notice.
  - `…_no_press_lets_a_long_stack_finish`: the stack sleeps 1.5 s, no
    presses → completes with `[started, done, finalize]`.
  - `…_one_press_lets_a_long_stack_finish`: one press (acknowledged), a 1.5 s
    stack → completes and never force-exits. Depends on S1's single-press
    finding.
  - Delete the old tests from `wrap_sigint.rs` in the same change.
- [ ] **Terminal scope coverage** (`lib/src/composition/lifecycle/mod.rs`,
  `lib/src/composition/lifecycle/tests/`)
  - Apply R2: a single scope-entry helper, called from `run_event_stack` and
    `emit_signal`.
  - Add a probing emitter (next to `RecordingEmitter`) whose `emit_stderr`
    records `interrupt::terminal_lifecycle_active()`. `emit_stderr` runs even
    when interrupted.
  - Table test: run `run_event_stack` for all six signals, with a `stderr`
    notification on each. Assert active for `success`/`blocked`/`failure`/
    `finalize` and inactive for `initialize`/`start`. Assert inactive again
    after each call.
  - Test the `Drop` backstop: drop a started, launched guard and assert the
    backstop-emitted `failure` observed the scope as active.

### Phase 2 checkpoint

- [ ] `cd claudine && just test` passes. `just lint` is clean.
- [ ] Mutation proof: apply each regression in the Definition of Done in
  turn, record which test fails, and revert. Put the results in a table in
  this section.
- [ ] Flake check: run the new L1 tests 25 times, for example
  `for i in $(seq 25); do just test-cli 'wrap_sigint' || break; done`, plus
  the lib scope tests. Zero failures.
- [ ] After a full `just test-cli wrap_sigint`, no `claudine`, `sh`, or
  `sleep` processes from fixtures remain (`pgrep -f` on the fixture paths).

## Phase 3 — Real-terminal coverage

### Wave 4 — L2 and L3 scenarios (parallel; distinct files)

- [ ] **L2 tmux scenarios** (new
  `cli/tests/level2/level2_terminal_lifecycle_ctrl_c_tmux.rs`, registered in
  `level2/main.rs`)
  - Gate with `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)`.
    Use `#[serial]`, a shell-script fake on `PATH`, and no focus APIs.
  - Teardown: an orphan that survives `TERM`, `CLAUDINE_KILL_GRACE=60s`,
    `C-c` after the orphan's `TERM` marker. Assert the pane shows the
    interrupt notice and the `failure`/`finalize` evidence, the
    `exit=` sentinel is non-zero, and the pane returns in < 10 s.
  - Repeat press, short stack: `C-c`, wait for the notice in the capture,
    `C-c`, wait for the grace notice, release. Assert the stack outcome
    matches the R8 decision, and assert the grace notice text in the capture.
  - Repeat press, long stack: assert the grace-expired notice
    (`lifecycle event still running — force-exiting`) and `exit=130`.
  - Clean up the session with `kill_session_by_name` in a drop guard.
- [ ] **L3 keystroke scenarios** (`cli/tests/level3/level3_wrap_ctrl_c.rs`)
  - Follow the existing macOS-only pattern: `WezTermHarness` with
    `with_expected_window_title("claudine")`, `SpawnVisibility::Foreground`,
    `focus_spawned_pane()`, `cliclick::click_then_ctrl_chord`, and
    `#[serial(level3_keyboard)]`.
  - Same three scenarios as L2, but each press is a real OS chord. Wait for
    the pane capture to show each acknowledgment before the next chord. Keep
    the file's documented stance on chord delivery: fail honestly, never
    loosen an assertion.
  - Extend the module doc's "Contrast with the lower tiers" to name the new
    L1 and L2 files.
  - Per R4, compile and list only:
    `cargo nextest list -p claudine-cli --features terminal-tests level3_`.

### Phase 3 checkpoint

- [ ] `cd claudine && just test-l2 level2_terminal_lifecycle` passes on
  macOS.
- [ ] `test_placement.rs` focus-API guard passes: focus APIs appear only in
  the `level3_` file. `just check-tier-coverage claudine` reports zero
  stranded tests.
- [ ] L3 tests compile and list. **Author step:** run
  `BISCUIT_L3_TAKE_FOCUS=1 just test-l3 level3_wrap_ctrl_c` and record the
  results in the spec (R4).

## Phase 4 — Documentation, spec reconciliation, and handoff

### Wave 5 — reconciliation (parallel; distinct files)

- [ ] **Spec update** (`spec.md`)
  - F2: replace "the single execution choke point" with the R2 reality.
  - Add the R1 interrupt/notification contract and credit the reported
    scenario to F1's latency fix.
  - Replace the Verification table rows with the new test names and levels.
    Add L2 results, and an L3 row marked "author-run" until R4 is satisfied.
  - Follow-ups: add the R6 Windows parity test, and the R8 option (b) if it
    was deferred.
  - Leave `status` and the lifecycle directory alone. Only the author moves
    the fix to `_completed`.
- [ ] **Signal-handling docs** (`claudine/docs/topics/signal-handling.md` and
  the skill copy `.claude/skills/claudine/signal-handling.md`)
  - Keep both copies in sync. Document the terminal-scope coverage (R2), the
    notification short-circuit (R1), the R8 outcome for lifecycle shell
    children in a real terminal, and where each tier's coverage lives.
- [ ] **OS skill** (only if S1 or S2 found a new process-group or tmux trap):
  add it to `.claude/skills/os/`, as `CLAUDE.md` requires.

### Final validation

- [ ] `cd claudine && just test && just lint && just check-tier-coverage claudine`
  and the L2 subset all pass. Confirm that no fixture processes remain.
- [ ] Comment-drift pass over every symbol touched in Phases 2–3
  (`kill_process_group`/`terminate_process_group`, `TerminalLifecycleScope`,
  `emit_signal`, `run_event_stack`, and the rewritten test doc comments).
- [ ] Fill in the resolution table below and stop at "implementation
  complete, ready for review".

### Review-1 resolution

| Finding | Resolved by | Evidence |
|---|---|---|
| RV1 — L2/L3 coverage and notice capture | L2 tmux file; L3 scenarios | L2 run; L3 author run |
| RV2 — guard branch | teardown seam guard unit test | mutation table |
| RV2 — empty-group latency | `compose_orphan_teardown_ends_when_the_group_empties` + early-return unit | mutation table |
| RV2 — zero/one/later presses | `wrap_sigint_grace.rs` controls | mutation table |
| RV2 — four-event scope entry | lib table test + `Drop` backstop test | mutation table |
| RV3 — barriers, checked `kill`, cleanup, trap readiness | signal fixture toolkit; rewritten fixtures | 25-run flake check; process-leak check |
