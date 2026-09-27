---
$schema: feature-review.yaml
ready: false
findings:
  - title: Repeat-interrupt teardown test can miss its hold window
    priority: high
human_review: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T17:09:09-07:00
spec: 2026-09-23-opencode-failure-lifecycle/spec.md
implemented: true
next: 2026-09-23-opencode-failure-lifecycle/review-4.md
implemented_by: claude/default
log: claudine/fixes/2026-09-23-opencode-failure-lifecycle/implementation-log.md
description: A **fix** review of `2026-09-23-opencode-failure-lifecycle/spec.md`
fix: 2026-09-23-opencode-failure-lifecycle/review-3.md
previous: 2026-09-23-opencode-failure-lifecycle/review-2.md
---

# Review 3

The fix is **not production ready**. The previous review's keyboard scenarios and cleanup changes are present, but the new repeat-interrupt teardown scenario fails in a normal L2 suite run. Its failure occurs before it can deliver the second press, leaving F1's repeated-keyboard-press contract without reliable Level 3 verification. No human design decision is needed.

## Previous review disposition

Review 2 has two unblocked findings and no `## Blocked Findings` section. The referenced `@prompts/_reviews/.../review-2.md` path does not exist in this worktree; the previous review is at `claudine/fixes/2026-09-23-opencode-failure-lifecycle/review-2.md`.

| Review 2 finding | Disposition |
|---|---|
| Keyboard verification still misses interrupt-count contracts | **Partially addressed.** L3 OS-chord tests now exist for a repeat press during teardown, a single press during terminal work, and a third press after grace starts. Their L2 counterparts capture the pane. The repeat-teardown scenario is unreliable as described below. |
| Orphan fixture cleanup can race workspace removal | **Addressed.** `ShellBarrier::Drop` releases and waits for the orphan, with a bounded kill fallback, before the borrowed workspace drops. `SignalledRun` uses `waitid(..., WNOWAIT)` to retain the leader PID until its group is killed in `Drop`. Focused L1 cleanup tests passed. |

## Findings

### High — Repeat-interrupt teardown test can miss its hold window

`assert_repeat_press_during_orphan_teardown_defers` waits for an orphan's TERM marker, sends the first Ctrl+C, and then waits for the `terminal-tests` hold marker before sending the second chord (`cli/tests/common/terminal_interrupt.rs:490–497`). The production teardown returns early if the process group has emptied (`cli/src/commands/wrap/exec/mod.rs:271–275`), whereas the test hold is entered only after the final SIGKILL (`:281–286`). The TERM marker alone does not prove the orphan remains alive until that final sweep.

In `just test-l2 level2_lifecycle_ctrl_c_tmux::`, five scenarios passed and this one failed after ten seconds waiting for the hold marker. The captured pane showed the TERM marker path had been reached, the first `^C` and its interrupt notice, and the wrapper's exit status 1. The second press was never sent. A focused serial rerun of the same test passed. This is a demonstrated scheduling-dependent test failure, not evidence that the production F1 path fails.

Make the held teardown window deterministic, then have the L2 and L3 cases acknowledge that window and deliver the second press. The test should fail for a removed `WaitLoopActiveGuard` while remaining stable when the orphan exits promptly or scheduling changes. Until that happens, F1's repeat-keyboard-press requirement has only the L1 state/rung check and an unreliable L2/L3 scenario; that is below its required Level 3 verification.

## Requirement-to-verification map

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| F1: group disappearance ends teardown promptly | L1 fake-provider subprocess | Appropriate; focused L1 run passed. |
| F1: one Ctrl+C cuts teardown short and preserves `failure`/`finalize` | L3 OS chord, L2 tmux capture, L1 subprocess | Appropriate tests are declared; L2 and L1 passed. L3 was not run because it takes GUI focus. |
| F1: repeat Ctrl+C during teardown defers and preserves the lifecycle tail | L3 OS-chord test and L2 counterpart, backed by an L1 rung test | **The L2 counterpart failed before the second press; finding above.** The L3 test shares that fixture and is subject to the same window. |
| F2: two presses allow short terminal work to finish | L3 OS chords, L2 pane capture, L1 subprocess | Appropriate tests are declared; L2 and L1 passed. |
| F2: two presses bound long terminal work and show grace/expiry notices | L3 OS chords, L2 pane capture, L1 timing subprocess | Appropriate tests are declared; L2 and L1 passed. |
| F2: zero or one press leaves terminal work unbounded; later presses do not extend grace | L1 subprocess for zero; L3 OS chords and L2 pane capture for one/three | Appropriate tests are declared; L2 and L1 passed. |
| F2: all four terminal events enter the scope | L1 in-process scope probe | Appropriate internal-state verification; reviewed from source. |
| F3: matching OpenCode stdout/stderr reference exposes the concrete cause via `err.msg` and `err.variant` | L1 parser, bridge, and fake-provider CLI tests | Appropriate nonterminal verification; unchanged since prior review. |

## Validation and limits

`just check-tier-coverage claudine` reported zero stranded tests. The new L2/L3 files are declared by the consolidated targets, selected by live recipes, and compiled with `terminal-tests`. `just test-cli wrap_sigint::` passed 12 tests. The focused L2 module run passed five tests and failed the repeat-teardown test; a serial rerun of that test passed. `git diff --check` passed. L3 tests were inspected but not executed because they raise and focus a WezTerm window, contrary to the session's no-focus rule. Cross-OS evidence and human review are outside this readiness decision.
