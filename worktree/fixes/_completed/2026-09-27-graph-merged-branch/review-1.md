---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
    - |-
        Confirm that Kitty draws the merged-branch graph in the reserved space at both 100×32 and 56×60 terminal sizes. The automated test checks the image bytes sent to Kitty and the reserved rows, but this agent's terminal lacks macOS Screen Recording permission, so its screenshot cannot prove on-screen placement.

        Please run `BISCUIT_TEST_REQUIRED_BACKENDS=kitty cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_graph_in_kitty)'` from a terminal with Screen Recording permission, then inspect the two `wt-graph-merged-*-screenshot.png` files in the system temporary directory. Accept the placement if the graph appears below the table legend without covering the table or following text. Alternatively, explicitly accept the transmitted PNGs and row-reservation checks as sufficient evidence, leaving the on-screen placement unverified.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
recurrence: false
created: 2026-09-27T23:17:33-07:00
spec: 2026-09-27-graph-merged-branch/spec.md
implemented: false
description: "A **fix** review of `2026-09-27-graph-merged-branch/spec.md`"
fix: 2026-09-27-graph-merged-branch/review-1.md
---

# Review 1: Graph of a merged branch

## Verdict

**Production ready, pending the separate human visual review above.** I found no implementation defect or missing automated test that calls for a code change. The input robustness matrix does not apply: this fix changes Git history discovery and graph rendering, not a file-format or configuration reader.

## Requirement verification

| User-visible requirement | Strongest verification | Assessment |
| --- | --- | --- |
| A merged selected branch keeps its lane and connects at the actual merge, in focused and base views | Level 1 real-Git fixtures and rendered-layout geometry; Level 2 Kitty test checks transmitted image and reserved rows | Appropriate. The on-screen pixel check awaits the human review item. |
| A recorded non-default parent is selected with or without its own worktree; merges into that parent or the default lane show the right fork and destination | Level 1 real-Git selection fixtures, component output, and rendered-layout merge-parent assertions | Appropriate for Git topology and image geometry. |
| Default history stays on its first-parent path; local and remote default labels remain on their actual commits | Level 1 real-Git fixture with a side branch longer than the default window and rendered-layout tag assertions | Appropriate for the history and label decisions. |
| Fast-forward, equal-tip, continued-after-merge, deleted-parent, indirect-integration, and shallow-history cases remain truthful | Level 1 real-Git fixtures and component assertions, including incomplete-history notices | Appropriate for these Git decisions. |
| Old forks and merges remain visible through compressed history; the base height cap and focused-view exemption remain | Level 1 real-Git and component sizing fixtures; Level 2 Kitty run checks image size and row reservation | Appropriate. |
| Full labels stay on their commits without overlap at ordinary and narrow widths, including adjacent default refs; explicit width does not trim history | Level 1 assertions against the renderer's actual layout geometry; Level 2 Kitty test transmits the resulting PNG | Appropriate for layout. The on-screen screenshot remains the separate human check. |

The worktree CLI's new Kitty test is a declared Cargo test target, uses the `terminal-tests` feature enabled by the live `test-l2` recipe, and is selected by its `level2_` name. `just check-tier-coverage worktree` reports zero stranded tests. No keyboard-input behavior changed, so Level 3 is unnecessary.

## Performance and validation

The recorded ten-sample release medians show graph gathering rising from 20.2 to 39.1 ms for ordinary history, 69.5 to 126.3 ms for older connections, and 32.4 to 65.3 ms for multiple branches. Image rendering stayed near 350 ms. The added Git calls verify ancestry and exact commit placement; the review found no comparably simple way to remove them while preserving the required evidence. Existing performance gates passed in the implementation record.

- `just test` in `worktree`: 746 passed, 29 excluded by tier selection.
- `just test` in `biscuit-terminal`: 3,344 passed, 55 excluded by tier selection.
- `just test` in `biscuit-visualized`: 98 passed.
- `just check-tier-coverage worktree`: zero stranded tests.

The implementation record documents Linux and Windows checks and the Kitty screenshot limitation. Cross-platform CI evidence and human acceptance are separate from this code-readiness verdict.
