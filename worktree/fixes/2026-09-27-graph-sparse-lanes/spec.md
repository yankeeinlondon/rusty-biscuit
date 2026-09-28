---
created: 2026-09-27
status: finalized-spec
clarified: true
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-28
review_iterations: 0
implemented: false
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
area: worktree
packages:
    - worktree-cli
    - biscuit-visualized
    - biscuit-terminal
related:
    - 2026-09-27-graph-merged-branch
human_review: false
message_to_agent: |-
    Phases 1 and 2 are complete; read implementation-log.md "## Phase 2" first. Facts that change how Phase 3 is written:

    1. The `fix/sniff` notice comes from `GitGraphPlan::incomplete`, not `GraphFacts::incomplete`. After the classification fix, every Git query in the observed shape verifies, so `GraphFacts::incomplete` is false there. `GitGraph` sets the plan flag because it cannot attach the lane at the undrawn fork W1. Assert `plan.incomplete` (it is what the terminal notice reads), and don't expect the facts flag to be true. `untrimmed_plan`, `first_on_lane`, `geometry_of`, `laid_out`, and `has_merge` in git_graph/tests.rs are ready-made helpers for the Wave 4 component assertions.
    2. Test API from biscuit-visualized: `biscuit_visualized::mermaid::default_gitgraph_commit_step()`. `GitGraphGeometry` now carries `tag_gap` (one em of the resolved theme font), and `CommitGeometry` carries `index` and `x`. Use `commit_step == default_gitgraph_commit_step()` for E3.
    3. `MAX_SPACING_PASSES = 3` counts every layout after the first, the fallback included: at most two verifying widenings, then an unverified fallback layout. A real graph verifies after one widening, so the render stage runs 2 layouts for a colliding graph and 1 for the observed shape (which has no colliding pair). Expect the Phase 3 "after" perf on the observed fixture to show no extra layout pass. The plan's note that an extra verification pass is the expected cost applies only to graphs with colliding tags.
    4. biscuit-visualized already passes on Linux and native Windows (cross-check). One Windows-only finding was fixed in the test: with a `fontSize` init override, the viewBox width can be narrower than the layout's `gitgraph.width` on Windows. Never compare `natural_size().width` with `GitGraphGeometry::width` under a font override; compare measured sizes with each other. `worktree` cross-OS is still owed (Wave 5).
    5. Docs still describe the old widest-tag rule, the `+bv2` backend id, and first-match classification. That is Phase 4's sweep. The worktree skill's `commands/git_graph.rs` bullet has already been corrected; the biscuit-visualized and biscuit-terminal skills have not.
---

# Restore commit density and direct merges in the `wt list` graph

## Outcome

In the base view, `wt list` again shows several recent commits on each branch lane
after its `+N` square, as it did before `2026-09-27-graph-merged-branch`. A
branch merged directly into the default branch is drawn with that merge, even
after its recorded parent has merged the default branch back in.

## Problem

Observed on 2026-09-27 in the main checkout, at 200×60, after PR #104 merged
`fix/sniff` into `main` (`b47a046`) and `fix/wt-ux` then merged `main` back in
(`b1aa473`). The screenshot comparison shows the graph before and after
`2026-09-27-graph-merged-branch`:

| Lane | Before | After |
|---|---|---|
| `main` | 10 commits (actually `fix/sniff`'s, a defect that fix corrected) | `+5`, then 5 commits |
| `feat/schema-enhancement` | `+30`, then 5 commits | `+50`, then 1 commit |
| `fix/wt-ux` | `+33`, then 6 commits | `+70`, then 1 commit |
| `fix/sniff` | a tag, cut off by `origin/main`'s tag | its own lane, `+94`, then 1 commit, with no fork or merge connection |

The `main` lane's change is a correction. The two defects below are regressions.

### Defect 1: tag spacing widens every commit column

The branch lanes lose their commits during layout, not during gathering. The
gathered facts still carry a `+N` square and 5 commits for each branch lane,
which is 28 commits before width fitting.

1. A Mermaid gitGraph gives every commit its own column across all lanes, so
   the natural width grows with the total commit count.
2. `layout()` in `biscuit-visualized/src/src/mermaid/gitgraph.rs` sets a single
   `commit_step` to the widest tag plus one em whenever the graph has **any**
   horizontal tag. Here that tag is `origin/main` (65 + 14), which raises the
   step from 34 to 79 for every commit.
3. `GitGraph::plan_with` in `biscuit-terminal/lib/src/components/git_graph.rs`
   then folds commits into `+N` squares until the image fits the terminal width. It takes from whichever
   lane shows the most commits, so every lane ends at its square plus its tip.

Measured from the observed Mermaid text (dark theme):

| Graph | `commit_step` | Natural width |
|---|---|---|
| All 28 commits, with tags | 79 | 2671 |
| All 28 commits, no tags | 34 | 1411 |
| Drawn plan (12 commits), with tags | 79 | 1279 (≈ 200 columns) |

At the default step, about 27 of the 28 commits fit the same 200 columns. With
the widened step, the observed run fit 12 and trimmed 17. The exact count
depends on the other graph elements and on font measurement.

The widening protects nothing in this graph. Only `b47a046` carries tags (`main`
and `origin/main`, stacked on that one commit). Tags can only collide when
commits near each other both carry one, and none do here.

### Defect 2: checking the parent first loses a direct merge into the default branch

`place()` in `worktree/cli/src/commands/git_graph.rs` classifies a selected branch
against its candidate lanes in a fixed order: the recorded parent's lane, then
the default lane, then a diverged `origin/<default>`. The first candidate that
contains the branch's tip wins.

- `fix/sniff`'s recorded parent is `fix/wt-ux`. Its tip `b3b17f3` was merged
  into `main` directly, as the second parent of `b47a046`.
- `fix/wt-ux` later merged `main` (`b1aa473`), so `fix/wt-ux` now contains `b3b17f3` too,
  though only by way of `main`.
- The parent is checked first, so the branch is classified
  `IntegratedOtherwise` into `fix/wt-ux`. That classification draws no merge.
  The gathered line has `merged_into: None`.
- The fork is computed as `2a7298a`, the old `fix/wt-ux` tip merged by #103.
  That commit is now only in `main`'s second-parent history and on no drawn
  lane, so the lane is emitted unconnected and the graph shows "Some history
  is not shown".

Only the fallback of `2026-09-27-graph-merged-branch`'s second-observation row
is met, with the label at the branch's real tip. The row itself requires a fork
from the parent and a merge into the default branch at the correct commits.

## Requirements

### Spacing

- If every pair of tags on different commits either has no vertical overlap
  or already has the existing one-em clearance at the default step, spacing
  stays at the default step. The observed case must lay out at step 34.
- Where tags would overlap, they retain at least the one-em gap. Full labels stay
  attached to their exact commits, as `2026-09-27-graph-merged-branch`
  requires, and the `main`/`origin/main`-one-commit-apart case keeps passing.
- Measurement and rendering keep sharing one layout, so a measured size stays
  the size of the image drawn.
- The step is widened only as far as real collisions need. Compare the actual
  first-pass label bounds for tags on different commits whose vertical
  interiors overlap. For each pair, use their commit sequence distance and
  the amount by which the earlier label's right edge would cross the later
  label's left edge after adding the existing one-em gap. Divide any shortfall
  by the sequence distance and add it to the default step. Reverse the
  horizontal ordering for right-to-left graphs. Choose the largest required
  step, never less than the renderer's default. This uses actual placement,
  including labels that are not centered on their commit; widths alone cannot
  give an exact answer. Tags on one commit move together and add no spacing
  requirement. Verify the resulting layout preserves the gap between tags
  whose vertical interiors overlap. If label placement changes between passes,
  widen from the remaining overlap and lay out again; if that cannot be proven
  to converge, use the existing widest-tag step as a safe bound. Preserve the
  current behavior for vertical graphs and rotated tags.

### Classification

- Evaluate candidate lanes in the existing order: recorded parent, default,
  then diverged `origin/<default>`. Keep the first direct merge or first-parent
  match found in that order. A first-parent match remains a label on its lane;
  a child merged into its parent stays attached to the parent after that
  parent merges into `main`. Defer an indirect match while checking later
  candidates for a direct merge or first-parent match. Use the first indirect
  match only if no candidate supplies either stronger result.
  If Git cannot establish an earlier candidate's relationship, the result is
  a history gap, as it is today, even when a later candidate has a direct
  merge: that later result does not prove which connection should win.
- Keep the chosen merge commit and its first parent together when computing
  the branch's history limit, merge edge, and fork. The recorded parent
  remains the fork lane when present, even if the merge is on `main`.
- A fork commit that sits on no drawn lane stays undrawn (the unplaceable-fork
  decision below). The lane is still connected by its verified merge, and the incomplete-history
  notice accounts for the missing fork. The fork is never attached at a
  substitute commit, and earlier merges are not reconstructed to make it
  drawable.
- The notice does not appear when every connection is drawn.

### Unchanged

Branch selection, the base-view height cap, the focused-view exemption,
`LINE_WINDOW`/`BASE_DEFAULT_WINDOW`, explicit-width behavior, and the trim
order stay unchanged, except as the spacing change affects how much gets
trimmed.

## Acceptance criteria

| Case | Required outcome | Validation |
|---|---|---|
| Observed repository shape (a real-Git fixture mirroring the problem history) at 200 columns | The affected branch lanes show more than their tips after their `+N` squares; `commit_step` is the default | Component plan assertions plus measured geometry; assert the planned commits rather than an OS-specific pixel width |
| Tags on one commit only | No widened step | `biscuit-visualized` geometry test |
| Tagged commits adjacent, with long labels | No overlap; labels stay on their commits | Existing label-bounds tests keep passing |
| Branch merged directly into the default lane after its parent merged the default branch back in | `merged_into` is the default-lane merge commit; the merge edge is drawn | Real-Git fixture in `worktree-cli` topology tests |
| Child merged into its parent, parent later merged into the default branch | Still merged into the parent | Existing fixture |
| Parent has the branch tip on its first-parent chain while another lane contains it | Branch stays a label on the parent's lane | Real-Git topology test |
| Earlier candidate contains the tip only indirectly and no later candidate has a direct merge or first-parent match | Earlier indirect result and incomplete-history notice remain | Real-Git topology test |
| Git cannot establish an earlier candidate's relationship in a shallow or failed history query | Incomplete-history notice; no invented merge edge | Topology test with existing history-gap behavior |
| `fix/sniff`-style fork from a parent tip that now lives only in the default branch's second-parent history | Merge edge drawn; fork undrawn and accounted for by the notice; the lane is never attached to a substitute commit | Real-Git fixture and component assertions |
| Two tagged commits on different lanes, one column apart, whose tags don't overlap vertically | Default step | `biscuit-visualized` geometry test |
| Unequal or offset labels, right-to-left layout, and theme font overrides | Smallest step that clears actual inter-commit overlaps with the one-em gap; measured and rendered geometry agree | `biscuit-visualized` geometry tests |
| Rendered image | No overlapping labels; density visibly restored | Kitty L2 test and the saved transmitted image |

Record `graph image render (biscuit-terminal)` timings before and after for the
same observed-case fixture, terminal size, and build profile. Report the
measurement setup and spread across runs, since rendering and font
measurements vary by host.

## Decisions

The original choices were decided with Ken on 2026-09-28. This review clarifies
their implementation contracts and effects on other callers.

1. **Spacing: collision-driven global step.** `mermaid-rs-renderer` 0.3.1 places
   commits at `seq × commit_step` and returns connectors as finished SVG path
   strings, so per-gap spacing would mean regenerating those paths after
   layout (fragile) or changing the renderer (a dependency change). The
   global step is kept but sized from actual label bounds and commit positions
   in the first layout pass, then checked in the resulting layout. This restores
   the observed case. A real collision, such as `main` and `origin/main` one
   commit apart, still widens every column; that is accepted.
   **Reader's note:** the previous widest-label rule prevented overlaps, but
   one long, isolated label widened every column and caused the terminal
   component to fold useful commits away. The bounds-based rule retains the
   existing gap only where two labels can meet. Since the same Mermaid input
   now produces a different image, bump `biscuit-visualized`'s Mermaid backend
   cache identifier as its cache contract requires.
   This correction is shared by every `biscuit-visualized` Mermaid caller,
   including `biscuit-terminal` and Darkmatter, so the cache change and
   geometry tests must cover direct Mermaid rendering as well as `wt list`.
2. **Unplaceable fork: merge edge plus notice.** `fix/sniff` forked at
   `2a7298a`. That commit is on `fix/wt-ux`'s first-parent chain but already
   merged into `main` by #103, so neither lane draws it. The branch is drawn
   with its verified merge into `b47a046`, its start is left unconnected, and
   the notice accounts for the fork. Two alternatives were rejected. One was
   reconstructing `fix/wt-ux`'s earlier merge so the fork becomes drawable:
   complete, but it changes how every branch that continued after a merge is
   drawn, re-opens scope `2026-09-27-graph-merged-branch` excluded, and costs
   width. The other was attaching the fork at `08cf96a`, which would draw a
   fork that never happened.

## Out of scope

- Per-gap tag spacing (only the colliding gaps widen).
- Reconstructing earlier merges of a branch that continued afterward.

## Documentation affected

- `biscuit-visualized/docs/mermaid-gitgraph.md`: the tag-spacing rule.
- `biscuit-terminal/docs/components/git_graph.md`: sizing and trimming, if
  the spacing interaction is described there.
- `worktree/docs/git-graph.md`: the classification order and direct-merge
  preference.
- `.claude/skills/worktree/SKILL.md`: the classification and spacing summary.
