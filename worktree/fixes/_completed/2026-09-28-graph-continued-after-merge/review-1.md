---
$schema: feature-review.yaml
ready: false
findings:
    - "high: A failed history read discards a previously verified merge"
    - "high: Kitty image verification can pass without observing the rendered graph"
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
recurrence: false
created: 2026-09-28T06:39:40-07:00
spec: 2026-09-28-graph-continued-after-merge/spec.md
implemented: true
next: 2026-09-28-graph-continued-after-merge/review-2.md
implemented_by: claude/opus
log: worktree/fixes/2026-09-28-graph-continued-after-merge/implementation-log.md
description: "A **fix** review of `2026-09-28-graph-continued-after-merge/spec.md`"
fix: 2026-09-28-graph-continued-after-merge/review-1.md
---

# Review 1: Graph continued after merge

## Verdict

**Not production ready.** The ordinary and shallow histories have Level 1 coverage, and the new Kitty test runs in the declared Level 2 tier. History-read failures can still discard verified merges, and the Kitty test can pass without checking the pixels drawn by Kitty. Neither finding needs a human design decision.

## Findings

### High: A failed history read discards a previously verified merge

**Defect class:** A later Git read failure replaces already verified lane history with an empty lane or suppresses the graph, instead of retaining the last drawable facts and reporting incomplete history.

The `worktree-cli` [extend function](../../cli/src/commands/git_graph.rs) records a verified merge and then rereads the wider lane. If that read fails, it returns `window: Err` with the recorded edge. The [assembly function](../../cli/src/commands/git_graph.rs) then converts the failed lane build to `LaneHistory::default()`, so the edge's source is absent and `biscuit-terminal` cannot draw it. The same replacement happens when the final `first_parent_entries` read fails. A failed default-lane read returns `None` for the entire graph. The spec explicitly requires verified edges to survive a failed later Git call.

| Site | Failure shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| `extend`, `window = read(&stop)` after accepting an earlier merge | Expanded `git log` fails | `window` becomes `Err`; assembly produces an empty source lane | Retain the last verified lane window and edge, and set `incomplete` |
| `assemble`, branch `first_parent_entries` | Its initial history read fails after placement | `unwrap_or_else` substitutes empty entries, hiding a verified source | Preserve verified entries where possible; omit only the unverified connection and set `incomplete` |
| `assemble`, default `first_parent_entries` | Destination history read fails after placement | `.ok()?` suppresses the graph | Retain drawable history and show an incomplete notice |
| `extend`, older `boundary` or classification | Later boundary cannot be proven | **Clean:** loop stops with its previous `window` and accepted edge | Keep the accepted edge and set `incomplete` |

These failure outcomes follow from the return paths shown above; the shipped real-Git test [a_shallow_boundary_invents_no_merge_and_keeps_the_verified_one](../../cli/src/commands/git_graph/tests.rs) exercises only the clean older-boundary row. A process-level fault-injection run was blocked by a shared Cargo build lock, so this review does not claim an executed reproduction of the failing rows. Add a controlled Git failure after the first edge has been accepted and assert the public `GraphFacts` and `GitGraphPlan` still contain that edge, its source and destination commits, and an incomplete notice. Cover the reread and final assembly paths with the same fixture.

### High: Kitty image verification can pass without observing the rendered graph

**Defect class:** The real-terminal graph tests treat unavailable screen capture as a successful pixel check, so a regression in what Kitty actually displays can pass Level 2.

The `worktree-cli` [Kitty graph harness](../../cli/tests/level2_graph_in_kitty.rs) returns from `assert_pixels_match` when screen capture is unavailable, or when the captured window is blank. The continued-after-merge test calls that helper through `assert_graph_drawn`. Its remaining checks prove that `wt` transmitted a PNG and reserved rows, but they do not prove that Kitty drew the connected graph. The acceptance criterion specifically calls for a Kitty screenshot of that graph. Level 1 geometry tests correctly prove the generated topology; they do not exercise Kitty's image display.

| Site | Capture shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Continued-after-merge Kitty test | Screen capture unavailable or blank | Test can pass with no screenshot of the graph | Required Level 2 visual evidence must fail or be reported as unavailable, not pass |
| Sparse-lanes Kitty test | Same helper and capture shape | Same successful return | Same requirement for its visible merge and fork |
| Height-cap and narrow-window Kitty tests | Same helper and capture shape | Same successful return | Verify actual image placement when claiming the rendered result |
| Level 1 graph geometry tests | Renderer output, no Kitty capture | **Clean for topology:** exact merge parents and labels are asserted | Keep these tests; add real-terminal proof alongside them |

Use a no-focus capture method already supported by the test harness and make the pixel assertion mandatory for tests that claim Kitty rendering. If a host cannot capture pixels, select an explicit skip or failure for that environment rather than a passing assertion. Keep the screenshot as evidence for the continued case.

## Verification

| Requirement | Strongest current level | Assessment |
| --- | --- | --- |
| Earlier merge, resumed commits, child fork, repeated merges, fork-origin cutoff, shallow history, and exact parents | Level 1 real-Git fixtures and renderer geometry | Appropriate for topology; the failed reread and assembly paths are untested |
| Connected graph and notice in Kitty | Level 2 terminal IPC, with a conditional pixel check | Insufficient when screen capture is unavailable |
| Height, width, tag placement, and scrolling in Kitty | Level 2 terminal IPC, with the same conditional pixel check | Image transmission and text spacing are checked; displayed pixels are not always checked |

No keyboard, paste, mouse, or other input-encoder behavior changed, so Level 3 is not needed. The input robustness matrix does not apply: the change reads Git history and existing fork-origin records, and adds no file-format or configuration reader. The Kitty test is a declared `worktree-cli` target with `terminal-tests`; `just check-tier-coverage worktree` reports zero stranded tests.

Local verification: `just test` passed in `worktree` (788 passed, 30 skipped) and `biscuit-terminal` (3,357 passed, 55 skipped). The findings concern paths those passing suites do not establish. I did not launch a terminal window during this review.
