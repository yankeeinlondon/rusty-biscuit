---
$schema: feature-review.yaml
ready: false
findings:
  - title: Keyboard verification still misses interrupt-count contracts
    priority: high
  - title: Orphan fixture cleanup can race workspace removal
    priority: medium
human_review: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T14:28:37-07:00
spec: 2026-09-23-opencode-failure-lifecycle/spec.md
implemented: true
next: 2026-09-23-opencode-failure-lifecycle/review-3.md
implemented_by: claude/default
log: claudine/fixes/2026-09-23-opencode-failure-lifecycle/implementation-log.md
description: A **fix** review of `2026-09-23-opencode-failure-lifecycle/spec.md`
fix: 2026-09-23-opencode-failure-lifecycle/review-2.md
previous: 2026-09-23-opencode-failure-lifecycle/review-1.md
---

# Review 2

The fix is **not production ready**. The core F1–F3 paths remain implemented, and the new tests close most of Review 1's process and terminal gaps. Two issues remain: keyboard tests do not exercise every interrupt-count behavior in the spec, and one cleanup path can leave an orphan running after a failed test. No human design decision is needed.

## Review 1 disposition

Review 1 has three findings under `## Findings`; it has no `## Blocked Findings` section. Its unblocked findings were addressed as follows:

| Previous finding | Status in this iteration |
|---|---|
| Ctrl+C lifecycle behavior lacks verification at the required terminal levels | Partially addressed. New L2 tmux captures and L3 OS-chord scenarios cover one press during teardown and two presses during short/long `failure` work. The interrupt-count branches below remain at L1. |
| Teardown and grace regression tests leave explicit contract branches unprotected | Addressed at L1: early group disappearance, the active-wait flag and `Defer` rung, zero/one press, third press, and scope entry for all four terminal events now have tests. |
| Signal fixtures depend on scheduling and leave child processes behind | Partially addressed. Barriers replace fixed sleeps, signal delivery is checked, and the lifecycle shell is owned by the wrapper group. The orphan barrier still lacks a safe teardown on the L1 path. |

## Findings

### High — Keyboard verification still misses interrupt-count contracts

F1 explicitly promises that a **repeat** Ctrl+C during post-exit teardown defers rather than force-exiting past `failure` and `finalize`. The new `level3_ctrl_c_during_orphan_teardown_runs_failure_and_finalize` sends only one chord (`cli/tests/common/terminal_interrupt.rs:294–324`). The L1 `kill_process_group_defers_repeat_interrupts_until_ctrl_c_ends_it` checks the wait-loop flag and calls `press_rung(2, ...)` directly; it never sends a second interrupt. This does not verify the keyboard-to-signal behavior of the repeated press in that window.

F2 also promises no 500 ms deadline after one press and no extension after later presses. `compose_single_sigint_lets_terminal_lifecycle_outlast_the_grace` and `compose_later_sigints_do_not_postpone_the_grace_deadline` exercise these at L1 using direct `kill(pid, SIGINT)` (`cli/tests/l1/wrap_sigint.rs:586–657`). The new L3 scenarios cover exactly two chords during short and long failure actions. Add L3 keyboard scenarios for the single-press hold and a third press during an armed grace. Arrange a controllable teardown boundary for the F1 repeat-press case, or otherwise exercise its full signal path with a real OS chord. Keep the L1 tests for precise timing and state assertions.

These are user-observable keypress contracts. Under this review's test-rigor rule, Level 1 state and direct-signal tests cannot stand in for Level 3 OS-keyboard verification.

### Medium — Orphan fixture cleanup can race workspace removal

`OrphanTeardownFixture` owns a `ShellBarrier` but its `Drop` only writes the release file (`cli/tests/common/signal.rs:380–384`). On a panic before Claudine tears down the fake OpenCode group, the orphan is outside `SignalledRun`'s wrapper process group. The fixture then drops its `TestWorkspace` immediately. If the workspace disappears before the orphan's next 10 ms file poll, the orphan never sees the release and loops indefinitely. The barrier's doc claims dropping it ends such an orphan, which is stronger than the code guarantees. The L2/L3 `PaneCompose` already uses `release_and_await_exit` before deleting its workspace; give the L1 fixture the same ordering, with a bounded fallback for a child that does not exit.

The shared `SignalledRun::Drop` also sends `SIGKILL` to `-self.pid()` after `try_wait()` may have reaped the leader (`cli/tests/common/signal.rs:263–283`). Once its group is empty, that group ID can be reused, so the comment claiming it cannot be reused is incorrect. Keep group ownership until cleanup or guard this final signal with a reliable lifetime check. This is a test-process safety issue, not a change in Claudine's production lifecycle behavior.

## Requirement-to-verification map

| User-observable requirement | Strongest present verification | Assessment |
|---|---|
| F1: return promptly once the group empties, without Ctrl+C | L1 no-interrupt fake-provider subprocess | Appropriate process-level test; passed. |
| F1: one Ctrl+C cuts teardown short and preserves `failure`/`finalize` | L3 macOS OS chord; L2 tmux pane capture; L1 subprocess | Appropriate levels present. L2 and L1 passed; L3 was not run under the no-focus constraint. |
| F1: a repeat Ctrl+C during teardown defers | L1 wait-loop flag plus pure rung call | **Level 3 missing; finding 1.** |
| F2: two Ctrl+C chords let short terminal work complete | L3 macOS OS chords; L2 tmux capture; L1 subprocess | Appropriate levels present. L2 and L1 passed; L3 was not run. |
| F2: two Ctrl+C chords bound long terminal work to 500 ms and show grace/expiry notices | L3 macOS OS chords; L2 tmux rendered text; L1 timing subprocess | Appropriate levels present. L2 and L1 passed; L3 was not run. |
| F2: zero/one press leaves terminal work unbounded; later presses do not extend grace | L1 direct-signal subprocess | Zero press needs L1 only; the one- and third-press keyboard behaviors need L3; finding 1. |
| F2: `success`, `blocked`, `failure`, and `finalize` enter the terminal scope | L1 in-process emitter observes each event's notification and action | Appropriate for this internal scope invariant; passed. |
| F3: matching OpenCode stdout/stderr `ref` replaces the generic error and exposes `err.msg`/`err.variant` | L1 parser, bridge, and fake-provider CLI tests reviewed in iteration 1 | Appropriate for non-terminal error data; no new gap found. |

## Validation and limits

`just check-tier-coverage claudine` reported zero stranded tests. The new L2 and L3 modules are declared by their consolidated `main.rs` targets; `claudine-cli` declares `terminal-tests` for both, and its `test-l2`/`test-l3` recipes are live. Focused local runs passed: eight `wrap_sigint::` L1 tests, the new lifecycle-scope L1 unit test, and three tmux L2 scenarios. The L2 pane capture verified the grace and force-exit notices in a real multiplexer. The L3 tests were inspected but not executed because they raise and focus a WezTerm window, which this session forbids. Cross-OS CI proof and a human review are separate from this readiness decision.
