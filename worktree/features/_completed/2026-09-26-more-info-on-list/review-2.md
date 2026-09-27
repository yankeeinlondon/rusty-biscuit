---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T18:07:25-07:00
spec: 2026-09-26-more-info-on-list/spec.md
implemented: false
description: "A **feature** review of `2026-09-26-more-info-on-list/spec.md`"
feature: 2026-09-26-more-info-on-list/review-2.md
previous: 2026-09-26-more-info-on-list/review-1.md
---

# Review 2: More information in `wt list`

## Verdict

**Production ready.** The previous review's one unblocked finding is resolved. It had no blocked findings, and this review found no new gaps against the specification. No human review is needed to complete the review cycle.

## Previous finding

### Resolved: Real-terminal tests do not verify the narrow width gate or parent comparison cells

The `worktree-cli` package now runs `wt list` inside tmux panes resized to exactly 99 and 100 columns. The [99-column test](../../cli/tests/level2_list_verbose.rs) checks that both comparison cells keep their `clean` state and pull request badges while hiding counts. The [100-column test](../../cli/tests/level2_list_verbose.rs) checks the target and parent counts, dim green and red styling, badge order, and the parent badge's wrapped continuation. Its child branch has a recorded fork parent different from the default branch. Both tests passed with tmux required, so they did not silently skip for lack of a terminal backend.

The [library test](../../lib/src/listing.rs) separately proves that the parent comparison uses the parent's local branch tip when its remote copy differs. The existing [table tests](../../cli/tests/list_table.rs) cover comparison states, unavailable comparisons, zero and nonzero counts, badge placement, color-free output, and both sides of the width boundary.

## Unblocked Findings

None.

## Blocked Findings

None. The previous review had no blocked findings to reassess.

## Requirement verification

| User-visible behavior | Strongest relevant verification | Assessment |
| --- | --- | --- |
| `clean` replaces `already in`; zero, one-sided, two-sided, conflicting, and unavailable comparisons have the specified text | Level 1 table snapshots and assertions; Level 2 target and parent captures | Appropriate. |
| Counts appear from 100 terminal columns, before pull request badges, in dim green and red; both columns and wrapped badge text remain visible | Level 2 tmux capture at 100 columns, with cell text and style assertions | Appropriate. |
| At 99 columns, state words and badges remain without counts; `--width` does not move the threshold | Level 2 tmux capture at 99 columns; Level 1 binary test and existing Level 2 `--width` test | Appropriate. |
| The parent comparison uses the parent's local tip | Level 1 temporary Git repository with different local and remote parent tips; Level 2 parent cell rendering | Appropriate: tip selection is library logic and rendering is checked in a real terminal. |
| Source-file dots and names are red, other-file dots are yellow, and legend connectors match the specification | Level 2 styled table and dirty-tree captures; Level 1 assertions | Appropriate. |

No behavior in this feature depends on a physical key press or terminal input encoder, so Level 3 keyboard injection is unnecessary. The new Level 2 tests are compiled by the declared `level2_list_verbose` target, enabled by the `terminal-tests` feature, and selected by the live Level 2 tier. `just check-tier-coverage worktree` reported no stranded tests.

## Verification performed

- Targeted Level 1 nextest run: 21 passed, including the local-parent-tip test, table tests, and the `--width` binary test.
- Targeted Level 2 tmux run for the new 99- and 100-column scenes: 2 passed.
- Targeted Level 2 styled table and dirty-tree run: 2 passed.
- Tier coverage check: zero stranded tests.

Cross-operating-system CI results are outside this readiness verdict.
