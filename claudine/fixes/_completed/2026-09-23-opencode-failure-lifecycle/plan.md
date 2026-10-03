---
total_phases: 3
created: 2026-09-26
phase: 1
agent: claude/default
yolo: true
---

# Plan: `2026-09-23-opencode-failure-lifecycle`: review-3 remediation and close-out

## Summary and Definition of Done

### Where this fix stands

- **F1–F3 are implemented** (`90c3cecf1`). Reviews 1–3 agree that the design
  and the production paths are correct.
- **Review 1 and review 2 are remediated** in the working tree (see
  `implementation-log.md`). Their plans are kept as `plan-review-1.md` and
  `plan-review-2.md`.
- **Review 2's implementation departed from its plan in two ways:**
  - It added a production test seam, `hold_teardown_for_terminal_tests`, in
    `cli/src/commands/wrap/exec/mod.rs`. The seam is compiled only under
    `cfg(all(unix, feature = "terminal-tests"))`. The plan (old R10) had asked
    for no hook at all.
  - It left the plan's carry-over work undone. That covers old R3 (the `Drop`
    backstop outside the terminal scope), old S1/R5 (the post-teardown gap),
    and the spec and doc records for old R2/R4.
- **Review 3** (`review-3.md`, `ready: false`) has one open finding:

| # | Priority | Finding |
|---|---|---|
| RV3-1 | high | `assert_repeat_press_during_orphan_teardown_defers` (L2 and L3) can miss its hold window. In a module-wide L2 run it waited 10 s for the hold marker. By then the wrapper had already exited 1 after one press, so the second press was never sent. A serial rerun passed. |

### Why RV3-1 happens (confirmed from source)

`kill_process_group` reaches the seam only on one path: `SIGTERM`, then a
poll that sees an interrupt, then `SIGKILL`, then the hold. Two other paths
return before the seam:

- `kill(-pgid, SIGTERM) != 0` at the start of teardown;
- `kill(-pgid, 0) != 0` in the poll loop, meaning the group is already empty.

The test takes the orphan's TERM marker as its "teardown is in progress"
signal. The marker proves only that some `SIGTERM` reached the orphan. It does
not prove that the group is still populated when the first press is handled.
If the group empties first, or the first press is handled after
`kill_process_group` returns, the seam is never entered. The press then lands
in the unmeasured post-teardown window (old carry-over 2), and the run exits
1\. Why the group emptied under load is still unknown, and spike S1 finds out.

### Work required

- Rule on scope, on keeping the seam, and on where the seam belongs.
- Spike S1: reproduce the flake, name the path that skips the hold, and
  measure the post-teardown gap. Old S1 folds into this spike.
- Move the seam so that the held window depends on neither group liveness nor
  press timing. Rework the scenario to acknowledge that window before either
  press.
- Implement the carry-overs still owed from review 2's plan: the
  terminal-scope helper, and gap closure if S1 finds a real window.
- Reconcile the spec, both copies of the signal-handling doc, and the
  comments. Add the seam to the Verification table.

### Definition of done

- **RV3-1 is closed.** The repeat-teardown scenario reaches its held window
  deterministically. Both presses land inside `WaitLoopActiveGuard`.
- **Stability:**
  - `just test-l2 level2_lifecycle_ctrl_c_tmux::` (claudine-cli leg, tmux
    required) passes 10 consecutive module-wide runs with the default thread
    count, the setting under which review 3 saw the failure;
  - two full `just test-l2` runs pass.
- **Mutation-proven.** Each deliberate regression, applied temporarily and then
  reverted and byte-compared, makes a named test fail:
  - removing `WaitLoopActiveGuard` from `kill_process_group` fails the L2
    repeat-teardown scenario;
  - removing the terminal-scope entry from `emit_signal` fails the new lib
    test;
  - reopening the post-teardown gap fails its regression test, if Phase 2
    closes that gap.
- **No shipped seam.** `just install` and a default `cargo build --release -p
  claudine-cli` do not contain `CLAUDINE_TEST_TEARDOWN_HOLD`. `strings` on the
  release binary finds nothing.
- From `claudine/`, these all pass: `just test`, `just lint`, `just
  check-tier-coverage claudine`, clippy `-D warnings` on the level2 and level3
  targets, and `cargo check --target x86_64-pc-windows-gnu -p claudine-cli
  --tests --features terminal-tests`.
- The six L3 scenarios compile and list. **The author runs them** (R9).
- The spec's prose and Verification table match the code. `status` and the
  lifecycle directory stay as they are. Stop at "implementation complete,
  ready for review".

**Out of scope:**

- pre-launch model validation (excluded by the Phase F ruling of 2026-07-06);
- `UserInterruptGuard` unregistration;
- `ref`-less OpenCode fallbacks;
- Windows `GenerateConsoleCtrlEvent` parity (R10);
- moving lifecycle shells into their own process group (R8, option b);
- the three `sequence_groups::` L1 failures on Linux (R11);
- the `just test-l2 <filter>` recipe exiting 1 when its claudine-gen leg
  selects no tmux test. Read the claudine-cli leg's result instead.

## Phase 1 — Rulings and diagnosis

### Necessary Rules

Each ruling carries a recommendation. An implementer applies the
recommendation unless the author overrides it before Phase 2 starts. R6–R10
restate rulings from `plan-review-2.md` that were never carried out, so this
cycle does not lose them a second time.

- [ ] **R1 — Scope.** This cycle closes RV3-1 and the carry-overs still owed
  from `plan-review-2.md`: the terminal-scope helper (R6), the post-teardown
  gap (R5), and the spec and doc records for R7 and R8. The gap is in scope
  for a second reason: RV3-1's failing run appears to put the first press in
  that window.
- [ ] **R2 — Keep the `terminal-tests` seam.** Accept review 2's deviation from
  old R10. No keypress can target the ≤ 50 ms teardown window any other way,
  and a back-to-back double press cannot prove the `Defer` rung. The seam is
  acceptable under three conditions:
  - it stays `cfg(all(unix, feature = "terminal-tests"))`;
  - it reads only `CLAUDINE_TEST_TEARDOWN_HOLD` and is a no-op when that is
    unset;
  - it is provably absent from installed builds (Definition of done).
  The spec's Verification section names the seam and says why it exists.
- [ ] **R3 — Move the hold to just after a successful `SIGTERM`.**
  - The hold is entered as soon as `kill(-pgid, SIGTERM)` succeeds, before the
    poll loop. It is **unconditional**: it does not depend on the interrupt
    flag or on whether the group is still populated.
  - The orphan waits on its barrier and survives `SIGTERM`. The fake OpenCode
    exits only after the orphan reaches that barrier. So the group is
    populated when `SIGTERM` is sent, and the hold is always reached.
  - The hold waits for `.release` only, still with the 60 s cap. After release
    the normal poll loop runs. The interrupt flag is already set, so the loop
    breaks at its first poll and sends `SIGKILL`. This is the path the
    un-held scenario and the user take.
  - The held scenario becomes: wait for `.reached`, press, wait for
    `COMPOSE_INTERRUPT_NOTICE`, press, wait for the second `^C` echo, assert
    no exit for `DEFER_SETTLE`, release. Then assert `failure` and `finalize`
    ran, the pane shows no force-exit, and the status is neither `0` nor `130`.
  - The TERM marker is no longer the held scenario's readiness signal.
    `PaneCompose::launch` waits for `.reached` when a hold is configured.
  - What the scenario gives up: it no longer shows that press 1 *cuts the
    grace short*. `assert_press_during_orphan_teardown_runs_the_lifecycle`
    (un-held) still covers that, so nothing is lost.
  - *Rejected alternative:* hold on every exit path whenever the interrupt
    flag is set. That still races: if the group empties before press 1, the
    press arrives after `kill_process_group` has returned.
- [ ] **R4 — S1's diagnosis gates Phase 2.**
  - If S1 shows the group emptied or the press arrived late **because of the
    fixture or the scheduler**, R3 is the complete fix.
  - If S1 finds a **production** cause, such as a second `SIGTERM` source, a
    teardown that runs twice, or an orphan the wrapper kills early, stop.
    Record it in the spike note and add a Phase 2 task for it, with its own
    regression test, before continuing.
- [ ] **R5 — Close the post-teardown gap if S1 finds a real window.** This is
  old R5, unchanged.
  - "Real" means the window contains any I/O or blocking call, or measures
    above 5 ms at p99.
  - A repeat press from the moment the provider child is reaped until the
    terminal scope is entered takes `PressRung::GraceExit`, not `ForceExit`.
    The grace is bounded, so a wedged post-exit path cannot hang the wrapper.
    For that reason, do not widen `WaitLoopActiveGuard`.
  - Prefer one guard in the shared wrap exec path over one guard per spawn
    mode.
  - If the window is pure in-memory work below the threshold, record that in
    the spec and change no code.
- [ ] **R6 — Every terminal emission enters the terminal scope.** This is old
  R3, unchanged.
  - Extract the `matches!(Success | Blocked | Failure | Finalize)` scope entry
    at `lib/src/composition/lifecycle/mod.rs:675` into one private helper.
  - Call the helper from `run_event_stack` and from the top of `emit_signal`.
    The scope's depth counter already handles nesting.
  - This covers the `LifecycleRunGuard::drop` backstop and makes the spec's
    choke-point wording true.
- [ ] **R7 — Keep the notification short-circuit** (old R2). After Ctrl+C,
  `emit_signal` still skips `message`, `notify`, `say`, and `effect`. Stack
  actions still run. The spec says so, and it credits the reported scenario to
  F1's latency fix.
- [ ] **R8 — Lifecycle shells stay in the foreground process group** (old R4,
  option a). Record this behavior as intended. Open an `_unscheduled` stub for
  option (b), giving terminal-event shells their own process group.
- [ ] **R9 — Running L3 is an author step.** Agents write, compile, and list
  L3 tests, and never run them, because L3 takes window focus. The L2 tmux
  mirrors are the agent-producible evidence.
- [ ] **R10 — Windows parity stays out of scope.** Record it as a spec
  follow-up.
- [ ] **R11 — The `sequence_groups::` Linux failures stay out of scope.** They
  touch no code this fix changed. The final report names them for the author
  and does not modify them.

### Wave 1 — spike (single agent)

The whole session tree is **non-interactive**. The spike agent must not run
anything that prompts, takes window focus, or waits on a TTY. tmux is allowed;
WezTerm and L3 are not. Revert all temporary instrumentation, and byte-compare
each file against a backup kept **outside** the source tree, with a unique
name per file.

- [ ] **S1 — Why the hold was missed, and how large the gap is** →
  `spikes/s1-teardown-hold-and-gap.md`
  - **Reproduce.** Run `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2
    level2_lifecycle_ctrl_c_tmux::` module-wide until the repeat-teardown test
    fails, up to 20 runs. Record the failure rate.
  - **Name the path.** Add temporary instrumentation that appends one line per
    event to a file named by an env var. Record each `kill_process_group`
    entry, which return path it took (`SIGTERM` failed, group empty, interrupt
    break, or deadline), a timestamp, and the `SIGINT` handler's receipt time.
    Reproduce once more. State which path fired and whether press 1 arrived
    during or after teardown.
  - **Explain the empty group**, if that is the path. Find what killed or
    ended the orphan: its trap, the barrier's `[ -d dir ]` exit, another
    `SIGTERM` source, or something else. Give an R4 verdict: *fixture or
    scheduling*, or *production*.
  - **Measure the gap.** List every step from each `kill_process_group(` call
    (`spawn/semantic.rs:500`, `spawn/inherited.rs:240`,
    `spawn/captured.rs:178`) to the `run_event_stack(Failure | Blocked)` it
    reaches, and mark each step that does I/O or blocks. Time the window on
    the L1 orphan fixture over at least 20 runs, and report p50 and p99.
  - **Prove the rung.** Temporarily add a 1 s sleep inside the window and send
    a second `SIGINT` there. Confirm `ForceExit`: `failure` and `finalize`
    are missing, and the run exits 130.
  - Conclude with the R4 verdict, an R5 verdict (*close*, naming the guard
    site, or *record only*), and whether R3 alone closes RV3-1.

### Phase 1 checkpoint

- [ ] Record R1–R11 here as ✅ plus a one-line note each. R4 and R5 cite S1.
- [ ] If S1 found a production cause (R4), add its task to Wave 2 before
  continuing.
- [ ] If S1 concluded *record only* for R5, mark Wave 3 N/A.

## Phase 2 — Deterministic hold, terminal scope, and gap

### Wave 2 — parallel; disjoint files

- [ ] **Deterministic teardown hold** (CLI and tests; R2, R3)
  - Files: `cli/src/commands/wrap/exec/mod.rs` (seam call site and doc),
    `cli/tests/common/terminal_interrupt.rs`,
    `cli/tests/level2/level2_lifecycle_ctrl_c_tmux.rs` and
    `cli/tests/level3/level3_lifecycle_ctrl_c.rs` (doc lines only, if they
    describe the old ordering).
  - Move `hold_teardown_for_terminal_tests()` to just after the successful
    `SIGTERM`, as R3 describes. Rewrite its doc: it now holds *before* any
    press, not "once a press has cut the kill grace short".
  - `PaneCompose`: when a hold is configured, `launch` is ready on
    `.reached`. Rewrite `orphan_teardown_held`'s doc to match.
  - `assert_repeat_press_during_orphan_teardown_defers`: use the R3 sequence.
    Drop the `wait_for_teardown_hold` call that came *after* press 1. Update
    the scenario doc and the module doc (lines 18–21).
  - Mutation (L2): remove `WaitLoopActiveGuard` from `kill_process_group` →
    `level2_repeat_ctrl_c_during_orphan_teardown_is_deferred` fails with a
    force-exit and 130.
  - Robustness proof (L2, temporary): make the orphan's TERM trap `exit`, so
    the group empties right after `SIGTERM`. This is the review-3 race. The
    scenario must still reach `.reached` and pass. With the seam temporarily
    restored to its old post-`SIGKILL` placement, the same change must make
    the scenario fail waiting for the hold.
  - Confirm the L1 `kill_process_group_defers_repeat_interrupts_until_ctrl_c_ends_it`
    still passes. It does not set the env var, so it must be unaffected.
- [ ] **Terminal scope helper** (lib; R6)
  - Files: `lib/src/composition/lifecycle/mod.rs`,
    `lib/src/composition/lifecycle/tests/guard_runtime.rs`, and
    `lib/src/interrupt.rs` (docs only).
  - Add a private helper that returns `Option<TerminalLifecycleScope>` for a
    signal. Call it in `run_event_stack` and at the top of `emit_signal`.
  - Test: a probe emitter records `crate::interrupt::terminal_lifecycle_active()`
    during the `Drop` backstop's `Failure` notification (launched guard) and
    its `Blocked` notification (unlaunched guard). The flag is set during both
    and clear afterward.
  - Mutation: remove the `emit_signal` call → the new test fails. Remove the
    `run_event_stack` call →
    `run_event_stack_marks_only_terminal_events_as_terminal_lifecycle_work`
    fails.
  - Comment pass: the inline comment in `run_event_stack`; the doc of
    `emit_signal` (add the scope, and keep the R7 short-circuit contract);
    and `TerminalLifecycleScope`'s doc, if it names the choke point.
- [ ] **Production cause from S1** (only if R4 found one). The files, test, and
  mutation come from the spike note. This task runs **before** the
  deterministic-hold task if the two touch the same file.

### Wave 3 — after Wave 2; N/A if S1 said *record only*

- [ ] **Close the post-teardown gap** (CLI; R5)
  - Files: the site S1 names, most likely in `cli/src/commands/wrap/exec/`,
    and possibly `cli/src/commands/compose/interrupt.rs`. This runs after
    Wave 2 because it touches `exec/mod.rs`.
  - A repeat press between reaping the child and entering the terminal scope
    resolves to `PressRung::GraceExit`. Add the matching row to the pure
    `press_rung` table test.
  - Put the regression test at the cheapest boundary that can fail: a unit
    test that holds the new guard and asserts that a count-2 press resolves to
    `GraceExit`.
  - Mutation: remove the guard → the test fails. With S1's temporary 1 s
    window sleep in place, the rung becomes `GraceExit` and `finalize` runs.
  - Comment pass: the doc of `kill_process_group`, the variant docs of
    `PressRung`, and the new guard's contract comment.

### Phase 2 checkpoint

- [ ] From `claudine/`: `just test` and `just lint` report zero failures.
- [ ] `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_lifecycle_ctrl_c_tmux::`
  passes: the claudine-cli leg 6/6 on **10 consecutive** module-wide runs,
  then two full `just test-l2` runs. No tmux sessions, fixture processes, or
  held wrappers remain (`pgrep -f` on the fixture workspace paths).
- [ ] `cargo nextest list -p claudine-cli --features terminal-tests level3_lifecycle_ctrl_c`
  lists six tests. Clippy `-D warnings` is clean on `--test level2 --test
  level3 --bins`, both with and without `terminal-tests`. The Windows-target
  `cargo check` passes.
- [ ] Seam absent: `cargo build --release -p claudine-cli`, then `strings` on
  the binary shows no `CLAUDINE_TEST_TEARDOWN_HOLD`. Confirm that the
  `install` recipe passes no `terminal-tests` feature.
- [ ] Record each mutation in `implementation-log.md`, one line each: the
  change, the failing test, and restored-and-byte-compared.

## Phase 3 — Spec, docs, and handoff

### Wave 4 — parallel; disjoint files

- [ ] **Spec update** (`spec.md`)
  - F1: add the repeat-press deferral as a verified contract. If S1 said
    *close*, add the post-teardown `GraceExit` coverage (R5). Otherwise add
    the measured size of the gap.
  - F2: make the choke-point wording match R6. State the notification
    short-circuit (R7) and the foreground-group behavior of shells (R8).
  - Verification: keep the historical OpenCode timing rows. Replace the test
    rows with current names grouped by tier:
    - L1: `wrap_sigint::`, the teardown unit test, and the lib scope tests;
    - L2: 6 tmux tests;
    - L3: 6 keyboard tests, marked "author-run" (R9).
    Add a one-line note on the `terminal-tests` teardown seam (R2).
  - Follow-ups: Windows parity (R10) and the process-group stub (R8 b).
  - Leave `status`, `implemented`, and the lifecycle directory unchanged.
- [ ] **Signal-handling docs.** Keep `claudine/docs/topics/signal-handling.md`
  and `.claude/skills/claudine/signal-handling.md` identical. Document
  terminal-scope coverage (R6), the post-teardown rung or its measured size
  (R5), the short-circuit (R7), foreground-group shells (R8), and where each
  tier's coverage lives, including the seam.
- [ ] **Unscheduled stub** (R8 b):
  `claudine/fixes/_unscheduled/lifecycle-shell-process-group/spec.md`. Give it
  `related: [2026-09-23-opencode-failure-lifecycle]` and the tmux evidence
  from the review-1 implementation log.
- [ ] **OS skill.** Only if S1 found a new macOS or Linux trap (for example,
  shell trap timing on a `SIGTERM`'d `sleep`), add it to `.claude/skills/os/`
  in the same change.

### Final validation

- [ ] Rerun the Phase 2 checkpoint commands after the Wave 4 edits. All pass,
  and nothing leaks.
- [ ] Comment-drift pass over every symbol touched:
  - `kill_process_group` and `hold_teardown_for_terminal_tests`;
  - `PaneCompose::orphan_teardown_held` and `PaneCompose::launch`;
  - the repeat-teardown scenario;
  - `emit_signal`, `run_event_stack`, the new scope helper, and
    `TerminalLifecycleScope`;
  - `PressRung`, and the gap guard if Wave 3 ran.
  Record each drift fixed in the implementation log.
- [ ] Fill in the resolution table below. Report the R11 Linux failures and
  the pending L3 author run. Stop at "implementation complete, ready for
  review". Do not run `just complete` or move the fix directory.

### Resolution

| Item | Resolved by | Evidence |
|---|---|---|
| RV3-1: repeat-teardown scenario misses its hold | R3 seam move + scenario rework | S1 note; 10-run L2 pass; guard mutation |
| RV3-1: root cause of the empty group / late press | S1 diagnosis (R4) | spike note; production task if needed |
| Carry-over: `Drop` backstop outside the terminal scope | R6 helper + lib test | mutation |
| Carry-over: post-teardown gap | R5 guard, or recorded as negligible | S1 measurement; mutation |
| Carry-over: notifications skipped after Ctrl+C | R7 (kept; spec states it) | spec diff |
| Carry-over: shells share the foreground group | R8 (a) documented; (b) unscheduled | spec diff; stub |
| Seam not shipped | R2 | `strings` check; install recipe |
| L3 keyboard scenarios | R9 author run | spec Verification row |
