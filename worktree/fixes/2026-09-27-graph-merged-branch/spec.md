---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
review_iterations: 0
clarified: false
implemented: false
related:
    - 2026-09-27-list-freshness-ux
human_review: false
---

# `wt list` graph hides a merged branch and overlaps neighboring tags

## Problem

Observed on 2026-09-27 in this repository, running `wt list` from the `fix-wt-ux` worktree just after PR #103 was merged on GitHub:

```text
*   08cf96a (origin/main) Merge pull request #103 from yankeeinlondon/fix/wt-ux
|\
| * 2a7298a (fix/wt-ux) planning(worktree): close … cycle 1
| * 7ba6ea8 planning(worktree): record Phase 5 close …
```

The graph drew one lane, `main`, running `7ba6ea8 → 2a7298a → 08cf96a`, with the `fix/wt-ux` and `origin/main` tags drawn on top of each other:

1. **The merged branch has no lane.** Nothing in the picture says a branch existed and was merged. Two rules combine to hide it:
   - `focused_view` in `worktree/cli/src/commands/git_graph.rs` builds the default lane with a plain `git log` from the fork point, which walks through the merge commit into its second parent. The branch's commits are drawn as default-lane commits.
   - `GitGraph::eligible_lanes` in `biscuit-terminal/lib/src/components/git_graph.rs` gives a lane only to a line with commits of its own. A fully merged branch has none, so it becomes a tag on its tip, as `biscuit-terminal/docs/components/git_graph.md` ("Lanes and tags") documents.
2. **Tags on neighboring commits overlap.** `mermaid-rs-renderer` 0.3.1 places a tag above its commit with no collision avoidance. A tag wider than the commit spacing covers its neighbor's. This isn't specific to merged branches; any two tagged commits side by side can hit it, such as `main` and `origin/main` one commit apart.

It surfaced now because it's the first time the current worktree's branch was fully merged while its worktree still existed. Every earlier branch had commits of its own and so labeled its own lane.

## Confirmed requirements

### Branch selection stays the same

The fix changes how selected branches are represented, not which branches are selected:

- When a non-default branch is checked out, select the default branch and the current branch. Preserve the existing split rule: if the current branch's fork-origin record names an existing non-default local parent branch, include that parent too. The parent does not need its own worktree. This is the existing one-level rule, not recursive ancestor expansion.
- When the default branch is checked out, select the worktree branches as today. Recorded parent relationships are used when that parent is also selected.
- Preserve the existing fallback when a recorded parent branch no longer exists.
- A selected branch must not lose its lane solely because it has been merged. In particular, the current branch in the observed case must retain a lane and a visible connection into the default lane at the merge commit.

Here, “default branch” means the repository's existing default-branch selection; `main` in the reported case is an example, not a new hard-coded name. Existing handling of the local and remote-tracking default tips remains part of the graph's context.

The interaction between branch selection and the existing terminal-height lane cap still needs a ruling. Selection does not yet establish an exemption from that cap.

### Essential historical connections remain visible

When a selected branch's fork or merge predates the ordinary displayed history, retain the essential fork and merge connections and compress intervening omitted history. Keep ordinary commit-history limits; do not expand every intervening commit merely to reach an older connection.

This requires the graph to communicate the relationship despite the age of those connections. The exact representation when the necessary Git history is unavailable remains open.

### Full labels remain attached to their commits

- Preserve full branch and reference names and existing label content.
- Keep labels attached to the exact commits they describe.
- Prevent labels on neighboring commits from overlapping, including the `main` and `origin/main` case.
- Preserve existing history compression followed by image shrinking. Preserve the existing explicit-width behavior, which does not trim commits to fit the requested width.
- Do not introduce a minimum text size, a shortened-label legend, or a new text fallback as the solution to collisions. Small text in very narrow terminals remains an accepted consequence of image shrinking.

Combining labels by moving one to a different commit is excluded, because it would misrepresent the reference's location.

## Ownership and implementation proposals

The functional requirements above are confirmed. The following are implementation directions to evaluate, not human-approved algorithms or API contracts.

- **`worktree-cli`:** its `commands::git_graph` module gathers Git facts and selects branches. It should provide sufficient ancestry and merge information for the selected branches without taking over layout. First-parent traversal is a candidate for keeping merged branch commits out of the default lane. Finding the merge that introduced a branch, and choosing its fork, need to handle the supported histories agreed below; the original proposal's simple default-lane traversal has not yet been shown sufficient for non-main parents or continued branch development.
- **`biscuit-terminal`:** its `GitGraph` component owns lanes, labels, omitted-history compression, and sizing. Extending its `GraphLine` input with a merge destination, such as a `merged_into` builder method accepting a commit identifier, is a candidate. No public API spelling is settled.
- **Renderer integration:** Mermaid merge statements are a candidate for drawing the connection. Existing renderer limitations around attributes on merge statements may require a workaround. Commit spacing or label layout changes are candidates for preventing collisions, but must be evaluated against exact attachment, full labels, compression, and shrinking.

Changing `mermaid-rs-renderer`, adding a dependency, or changing the shared component's public input contract may have effects beyond `wt list`. The acceptable boundary for those changes, and whether a focused rendering experiment is needed, remain open.

## Acceptance criteria and validation

These criteria cover the confirmed behavior. They do not settle the open history cases, resource limits, or incomplete-history fallback below.

| Case | Required observable outcome | Validation approach |
|---|---|---|
| Current branch has been merged through a recorded merge commit, with its worktree retained | The current branch has a lane; its branch commits are not presented as a straight segment of default-branch history; its merge connection is visible at the correct commit | Real-Git fixture for gathered facts, component output assertions, and rendered-image inspection |
| Same merged branch viewed from the default branch | It remains eligible for its lane under the same branch-selection rule; merged status alone cannot collapse it into a tag | Real-Git fixture and component assertions, with enough terminal height to avoid the unresolved cap policy |
| Current branch has a recorded existing non-default parent | The parent remains selected and the child retains the recorded parent relationship; test both with and without a worktree for the parent | Real-Git selection fixtures |
| Recorded parent has been deleted | Existing parent-missing fallback remains unchanged | Existing or extended selection regression fixture |
| Essential fork or merge is older than the ordinary displayed window | The relationship remains visible, with intervening history omitted or compressed rather than all commits expanded | Fixture exceeding the ordinary windows; assert connection commit identities and omitted-history representation |
| Neighboring commits carry long branch or reference labels | Full labels remain attached to their respective commits and do not overlap | Inspect actual rendered label bounds or pixels; source snapshots alone cannot prove this |
| `main` and `origin/main` are one commit apart | Both labels remain fully represented at the correct commits without overlap | Component fixture and rendering validation |
| Width is too small for the compressed natural image | Existing compression and shrinking behavior remains, without shortened labels or a new text fallback | Narrow-width fixture and existing explicit-width regression coverage |

Use the repository's nextest-based L1 recipes for gathering and component tests. Rendered output needs direct validation in addition to Mermaid snapshots; the precise dimensions, fixtures, and pass/fail measurement remain to be agreed. The existing Kitty L2 graph test in `worktree-cli` is a candidate integration point. Any L2 or L3 tests must avoid taking terminal or browser focus.

The affected packages must continue to compile and work on macOS, Linux, native Windows, and WSL2. The implementation plan must distinguish portable behavior tests from terminal-specific image evidence.

## Documentation affected

- `worktree/docs/git-graph.md`: replace the documented collapse of merged branches into tags and describe preserved selection, parent relationships, historical connections, and any agreed limits.
- `biscuit-terminal/docs/components/git_graph.md`: update lane and label behavior and document the final shared-component contract and renderer constraints.

The existing documentation describes the current implementation; the implementation change must update it alongside the code.

## Decisions still required

1. **Supported Git histories and merge meaning.** Define expected output for a child merged into its non-default parent, fast-forward integration without a merge commit, a branch that continues after an earlier merge, and repeated or partial merges. Decide what “fork” and “merge connection” mean in those cases. The original draft's rule that a branch equal to its parent's tip with no distinct history stays a tag has not yet been confirmed. Squash-merge inference and new remote requests are proposed exclusions, not settled scope decisions.
2. **Incomplete or changing history.** Choose behavior when a shallow repository, missing object, rewritten history, or changed reference prevents a reliable fork or merge determination. Specify whether the graph preserves a partial lane, marks an unknown relationship, omits the graph, or reports an error; do not invent ancestry.
3. **Terminal-height limits.** Decide whether to preserve the existing half-terminal-height cap, activity-based lane selection, and omitted-worktree notice unchanged, or protect particular selected lanes. Establish how historical connections are represented when another lane is omitted by the cap.
4. **Measurable success and performance.** Agree on representative branch counts, history depth, label lengths, and terminal dimensions; define an acceptable gathering and rendering cost. Specify how nonoverlap and correct attachment will be measured in rendered output and what evidence completes review.
5. **Renderer feasibility and preparatory authorization.** Decide whether a small renderer experiment would resolve uncertainty about labels and merge attributes. Establish the permitted dependency and shared-API change boundary before implementation, including whether a renderer modification would need another human decision. No spike or dependency change has been authorized by this clarification batch.

These rulings must be resolved or explicitly recorded as deferred before the specification is finalized. This document currently separates confirmed user-visible requirements from candidate implementation details; it is not yet an implementation plan.
