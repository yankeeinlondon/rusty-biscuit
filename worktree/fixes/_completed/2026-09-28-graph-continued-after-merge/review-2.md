---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
    - |-
        Confirm the graph in Kitty when its window is visible. On a macOS host with Screen Recording permission, uncover the area where Kitty opens, then run `BISCUIT_TEST_REQUIRED_BACKENDS=kitty cargo nextest run -p worktree-cli --features terminal-tests --test level2_graph_in_kitty` from `worktree/`. All five image checks must complete without a skip or failure. Inspect the retained `wt-graph-continued-200x60-screenshot.png` and transmitted image named in the test output: the branch must connect through its earlier merge, continue above it, and show no incomplete-history notice. This check is needed because every image check skipped on the review host when other windows covered Kitty.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
recurrence: false
created: 2026-09-28T12:02:43-07:00
spec: 2026-09-28-graph-continued-after-merge/spec.md
implemented: false
description: "A **fix** review of `2026-09-28-graph-continued-after-merge/spec.md`"
fix: 2026-09-28-graph-continued-after-merge/review-2.md
previous: 2026-09-28-graph-continued-after-merge/review-1.md
---

# Review 2: Graph continued after merge

## Verdict

**Production ready.** The two findings from the first review are implemented, and the code and test sweep found no remaining implementation finding. A human visual check remains because the review host covered Kitty's windows, so its screenshots contained no drawn window content. That check is external to implementation readiness; the tests now report this condition visibly instead of claiming a pixel comparison succeeded.

```sh
Nextest run ID 2b87b3ac-3d0c-4038-9275-2e669147b07f with nextest profile: default
   Starting 6 tests across 1 binary
       PASS [   2.166s] worktree-cli::level2_graph_in_kitty level2_sparse_lanes_fixture_has_the_observed_topology
       PASS [   4.292s] worktree-cli::level2_graph_in_kitty level2_graph_height_cap_elides_lanes_in_a_short_kitty_window
       PASS [   4.346s] worktree-cli::level2_graph_in_kitty level2_graph_fits_a_narrow_kitty_window
       PASS [   4.604s] worktree-cli::level2_graph_in_kitty level2_graph_draws_a_branch_continued_after_its_merge_in_kitty
       SLOW [>  5.000s] worktree-cli::level2_graph_in_kitty level2_graph_restores_lane_density_in_kitty
       SLOW [>  5.000s] worktree-cli::level2_graph_in_kitty level2_graph_draws_a_merged_branch_in_kitty
       PASS [   5.710s] worktree-cli::level2_graph_in_kitty level2_graph_restores_lane_density_in_kitty
       PASS [   6.216s] worktree-cli::level2_graph_in_kitty level2_graph_draws_a_merged_branch_in_kitty
────────────
    Summary [   6.216s] 6 tests run: 6 passed (2 slow), 0 skipped
       PASS [   4.604s] worktree-cli::level2_graph_in_kitty level2_graph_draws_a_branch_continued_after_its_merge_in_kitty
```

VERIFIED


## Earlier findings

The first review had two unblocked findings and no blocked findings. Both defect classes were swept again:

| Defect class and site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Lost verified history: accepted merge followed by a failed wider lane read | Injected failure of the next first-parent `git log` | The last window, merge source and destination, edge, and incomplete notice survive in both gathered facts and the final plan | Same |
| Lost verified history: branch anchor positioning fails | Injected failure of a child's anchor distance lookup after its parent's merge is accepted | The verified branch entries and edge survive; the unplaced connection is reported | Same |
| Lost verified history: default lane's newest-commit read fails | Injected failure of its first-parent `git log` | The graph and its verified default-lane anchors survive with the edge and notice | Same |
| Lost verified history: diverged origin lane's first read fails | Injected failure of its first-parent `git log` | The unread lane stays empty; the destination is not substituted onto `main`, and the notice appears | Same |
| Lost verified history: older boundary or classification cannot be proved | Shallow boundary after one verified merge | The earlier verified edge remains and the graph is incomplete | Same |
| Unobserved Kitty pixels: continued, sparse-lanes, merged, height-cap, and narrow-window image tests | Screenshot has no table or graph pixels because another window covers Kitty | Each test prints the reason and skips; a required Kitty backend turns this into failure. A screenshot with a table but no graph fails. | No image test silently counts an unobserved graph as pixel evidence |

The Git failure cases are exercised with the real repository fixture and controlled command failures; their assertions reach `GraphFacts` and `GitGraphPlan`. The Kitty assertion is shared by all five image tests, which are compiled by the declared `worktree-cli` test target and selected by the live Level 2 recipe. The first review's findings therefore do not recur.

Two nonbehavioral documentation descriptions were corrected during this review: the component input table now says a line can carry several merge edges, and the graph limits no longer imply a fork-origin record is required to stop the backward walk.

## Specification sweep

| User-visible requirement | Strongest test level | Assessment |
| --- | --- | --- |
| Earlier merge, continued commits, child label and fork, merges repeated on one lane, fork-origin cutoff, indirect and shallow boundaries, and exact merge parents | Level 1 real-Git fixtures plus renderer geometry | Appropriate for topology; the public graph facts and planned image layout are asserted with exact commits |
| Missing Git history retains verified edges and reports incomplete history | Level 1 real-Git fault injection through the final graph plan | Appropriate for local failure handling |
| Image placement, width, height, notices, and connected drawing in Kitty | Level 2 private Kitty with terminal capture and mandatory screenshot comparison | Appropriate test exists; pixels were unavailable during this review because the windows were covered |

No keyboard or other terminal input-encoder behavior changed, so Level 3 does not apply. The changed fork-origin cutoff reads the scalar `base_sha` from the existing JSON record. Its Level 1 matrix starts from a saved real record and checks the public graph result for the unedited control, absent, null, wrong-type, empty, duplicate, trailing-content, malformed or unknown SHA, and on-chain or off-chain commit cases. Array-element shapes do not apply to this scalar field.

## Verification

- `just test` in `worktree/`: 796 passed, 30 skipped.
- `just test` in `biscuit-terminal/`: 3,357 passed, 55 skipped.
- `just check-tier-coverage worktree`: zero stranded tests.
- Direct Level 2 Kitty test binary: six reported passes, including one fixture-only test. The five image tests each reported a visible pixel-check skip because their screenshots held no Kitty window content; these passes are **not** visual evidence.
- `git diff --check`: passed.

## Unblocked Findings

None.

## Blocked Findings

None. The remaining visual check is listed in `human_review_items` because it needs an uncovered Kitty window on a host where screenshots can observe it.
