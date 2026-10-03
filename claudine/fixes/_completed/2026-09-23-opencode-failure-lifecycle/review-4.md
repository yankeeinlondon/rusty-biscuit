---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T17:30:22-07:00
spec: 2026-09-23-opencode-failure-lifecycle/spec.md
implemented: false
description: A **fix** review of `2026-09-23-opencode-failure-lifecycle/spec.md`
fix: 2026-09-23-opencode-failure-lifecycle/review-4.md
previous: 2026-09-23-opencode-failure-lifecycle/review-3.md
---

# Review 4

The fix is **production ready**. F1–F3 match the spec, and the remaining test race from review 3 is addressed. No human design decision is needed.

## Previous review disposition

Review 3 has one finding under `## Findings` and no `## Unblocked Findings` or `## Blocked Findings` section. Thus, there was no blocked finding to reevaluate. The requested `@prompts/_reviews/.../review-3.md` path does not exist in this worktree; its actual file is in this fix directory, and its frontmatter now points to review 4.

| Review 3 finding | Disposition |
|---|---|
| Repeat-interrupt teardown test can miss its hold window | **Addressed.** The feature-gated hold now begins inside `WaitLoopActiveGuard` after `terminate_process_group` returns on every path, including early group disappearance. The test waits for the hold marker before sending the second press, observes two `^C` echoes, checks that the wrapper remains alive, then releases the hold and checks `failure` and `finalize`. The focused L2 suite passed all six scenarios, including this one. |

## Findings

None.

## Requirement-to-verification map

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| F1: teardown ends promptly when the group empties | L1 fake-provider subprocess with a 60 s kill grace | Appropriate; focused L1 test passed. |
| F1: one Ctrl+C cuts teardown short and preserves `failure`/`finalize` | L3 OS chord test, L2 tmux pane capture, L1 subprocess | Appropriate levels are declared; L2 and L1 passed. L3 was not executed under the no-focus rule. |
| F1: a repeat Ctrl+C during teardown defers and preserves the lifecycle tail | L3 OS chord test sharing the deterministic hold, L2 tmux capture, L1 rung/state test | Appropriate levels are declared; the formerly failing L2 case passed. L3 was not executed under the no-focus rule. |
| F2: two presses let short terminal work finish | L3 OS chords, L2 tmux capture, L1 subprocess | Appropriate levels are declared; L2 and L1 passed. |
| F2: two presses bound long terminal work and show grace/expiry notices | L3 OS chords, L2 tmux capture, L1 timing subprocess | Appropriate levels are declared; L2 and L1 passed. The notices were captured in a real tmux pane. |
| F2: zero or one press leaves work unbounded; a later press does not extend grace | L1 subprocess for zero presses; L3 OS chords and L2 tmux capture for one and three presses | Appropriate levels are declared; L2 and L1 passed. |
| F2: `success`, `blocked`, `failure`, and `finalize` enter the terminal scope | L1 in-process scope probe | Appropriate for this internal invariant; no keyboard behavior is asserted by the scope entry itself. |
| F3: a matching OpenCode stdout/stderr reference exposes the concrete cause through `err.msg` and `err.variant` | L1 parser, bridge, and fake-provider CLI tests | Appropriate for nonterminal error data; no new gap found. |

## Validation and limits

`just test-l2 level2_lifecycle_ctrl_c_tmux::` passed all six L2 cases. `just test-cli wrap_sigint::` passed all 12 focused L1 cases. `just check-tier-coverage claudine` reported zero stranded tests, and `git diff --check` passed. The L2 and L3 test modules are declared in their consolidated binaries, selected by live tier recipes, and compiled with the `terminal-tests` feature. L3 keyboard tests were inspected but not run because they raise and focus a WezTerm window, contrary to this session's no-focus rule. Cross-OS results and human review are outside this readiness decision.
