---
$schema: feature-review.yaml
ready: false
findings:
    - title: Real-terminal tests do not verify the narrow width gate or parent comparison cells
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T17:57:57-07:00
spec: 2026-09-26-more-info-on-list/spec.md
implemented: true
next: 2026-09-26-more-info-on-list/review-2.md
implemented_by: claude/opus
log: worktree/features/2026-09-26-more-info-on-list/implementation-log.md
description: "A **feature** review of `2026-09-26-more-info-on-list/spec.md`"
feature: 2026-09-26-more-info-on-list/review-1.md
---

# Review 1: More information in `wt list`

## Verdict

**Not production ready under the required test standard.** The implementation follows the specified comparison, color, and legend rules in the paths reviewed. The missing real-terminal scenes leave two visible table behaviors unverified at the level needed for release. No human design decision is needed to close this finding.

## Findings

### High: Real-terminal tests do not verify the narrow width gate or parent comparison cells

In `worktree-cli`, [list_until](../../cli/tests/level2_list_verbose.rs:405), the helper for the real-terminal `wt list` tests, requires a pane at least 100 columns wide. Its fixture gives every branch the default branch as its parent, so the `-> parent` cells have no comparisons. The existing real-terminal assertions therefore prove the wide `-> {default}` counts and their colors, but never show that a 99-column pane suppresses counts while retaining state words and pull request badges, or that parent counts and badges render correctly in a real terminal.

The [99- and 100-column table tests](../../cli/tests/list_table.rs:458) supply fixed widths in process and cover those values and parent cells at Level 1. One parent badge wraps onto another line at exactly 100 columns in that test. These tests are valuable for the renderer's logic, but do not exercise the terminal's actual pane width or verify that the wrapped parent cell remains visible in a rendered pane. The strongest tests for those user-visible cases are at the wrong level.

Add focus-preserving Level 2 captures at 99 and 100 pane columns using the existing tmux harness. Keep a branch with a pull request badge in both scenes, and add a child whose recorded fork parent differs from the default branch so `-> parent` shows nonzero counts and a badge. Assert the visible cell text, badge placement, and dim green/red styles, including any line wrap at 100 columns. The harness already supports pane resizing. Preserve the existing Level 1 boundary and local-parent-tip tests.

## Requirement verification

| User-observable requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| `already in` becomes `clean`; zero, one-sided, two-sided, conflicting, and unavailable comparisons retain their intended words | Level 1 table snapshots and binary tests; Level 2 wide target-column capture | Appropriate for the shared state logic and the target cell. |
| At 100 columns, target counts precede pull request badges and render dim green/red; the four-column table remains readable | Level 1 boundary snapshot and Level 2 styled tmux capture in a pane at least 100 columns wide | Appropriate for the tested target cells. |
| At 99 columns or fewer, state words and badges remain while counts disappear; `--width` does not change the threshold | Level 1 fixed-width table and captured-binary tests; Level 2 tests cover only panes at least 100 columns wide | **Level mismatch:** the narrow real-terminal pane and its layout are untested. |
| Parent counts use the parent's local tip and render with the same width, style, and badge rules | Level 1 temporary-Git-repository test for the local tip and Level 1 table tests for rendering; no Level 2 parent comparison cell | **Level mismatch:** real-terminal parent-cell rendering and wrap are untested. |
| Source-file dots and file names are red; other-file dots stay yellow; the Branch legend uses `└─`, `└─`, `└┄` | Level 1 markup tests and Level 2 styled captures for table, legend, and dirty-file tree | Appropriate. |

No requirement in this feature depends on a physical key press or terminal input encoding, so Level 3 is not needed. The Level 1 files are compiled as integration-test targets. The `level2_` tests are declared targets, their `terminal-tests` feature is enabled for the tier, and the package has a live `test-l2` recipe. `just check-tier-coverage worktree` reports zero stranded tests.

## Verification performed

- Targeted Level 1 run: 23 passed, including the 99/100 boundary, real-binary `--width`, and local-parent-tip tests.
- Targeted Level 2 tmux run: 3 passed, covering wide target counts, styling, `--width`, and source-file tree colors.
- Tier coverage check: zero stranded tests.

The review changes only this review file and the specification's review-iteration count. Cross-operating-system execution evidence and a later human review are outside this readiness verdict.
