---
created: 2026-09-27
status: finalized-spec
clarified: true
reviewed: false
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
---

# `wt list` graph lanes shrink to their tip, and a directly merged branch floats

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
which is 28 drawn commits in total.

1. A Mermaid gitGraph gives every commit its own column across all lanes, so
   the natural width grows with the total commit count.
2. `layout()` in `biscuit-visualized/src/src/mermaid/gitgraph.rs` sets a single
   `commit_step` to the widest tag plus one em whenever the graph has **any**
   horizontal tag. Here that tag is `origin/main` (65 + 14), which raises the
   step from 34 to 79 for every commit.
3. `GitGraph::plan_with` in `biscuit-terminal` then folds commits into `+N`
   squares until the image fits the terminal width. It takes from whichever
   lane shows the most commits, so every lane ends at its square plus its tip.

Measured from the observed Mermaid text (dark theme):

| Graph | `commit_step` | Natural width |
|---|---|---|
| All 28 commits, with tags | 79 | 2671 |
| All 28 commits, no tags | 34 | 1411 |
| Drawn plan (12 commits), with tags | 79 | 1279 (≈ 200 columns) |

At the default step, about 27 of the 28 commits fit the same 200 columns. With
the widened step, 12 fit and 17 were trimmed.

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

- In a graph where no two tags could overlap at the default step, the spacing
  stays at the default step. The observed case must lay out at step 34.
- Where tags would overlap, they still must not overlap. Full labels stay
  attached to their exact commits, as `2026-09-27-graph-merged-branch`
  requires, and the `main`/`origin/main`-one-commit-apart case keeps passing.
- Measurement and rendering keep sharing one layout, so a measured size stays
  the size of the image drawn.
- The step is widened only as far as real collisions need (decision 1). For
  each pair of tagged commits `k` columns apart whose tags overlap vertically,
  the pair needs `step × k ≥ (w₁ + w₂) / 2 + gap`. The step is the default
  step or the largest pair requirement, whichever is greater. Tags on
  different lanes that don't overlap vertically impose nothing, and neither do
  several tags stacked on one commit.

### Classification

- A branch whose tip is a non-first parent of a merge on a candidate lane is
  drawn as merged directly into that lane. It is not demoted to
  `IntegratedOtherwise` because an earlier candidate reaches the tip only through that same merge.
- The existing preference for the parent's lane still holds when the parent
  merged the branch itself, for example a child merged into its parent that
  later merged into `main`.
- A fork commit that sits on no drawn lane stays undrawn (decision 2). The
  lane is still connected by its verified merge, and the incomplete-history
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
| Observed repository shape (a real-Git fixture mirroring the problem history) at 200 columns | Each branch lane shows more than its tip after its `+N` square; `commit_step` is the default | Component plan assertions plus measured geometry |
| Tags on one commit only | No widened step | `biscuit-visualized` geometry test |
| Tagged commits adjacent, with long labels | No overlap; labels stay on their commits | Existing label-bounds tests keep passing |
| Branch merged directly into the default lane after its parent merged the default branch back in | `merged_into` is the default-lane merge commit; the merge edge is drawn | Real-Git fixture in `worktree-cli` topology tests |
| Child merged into its parent, parent later merged into the default branch | Still merged into the parent | Existing fixture |
| `fix/sniff`-style fork from a parent tip that now lives only in the default branch's second-parent history | Merge edge drawn; fork undrawn and accounted for by the notice; the lane is never attached to a substitute commit | Real-Git fixture and component assertions |
| Two tagged commits on different lanes, one column apart, whose tags don't overlap vertically | Default step | `biscuit-visualized` geometry test |
| Rendered image | No overlapping labels; density visibly restored | Kitty L2 test and the saved transmitted image |

Record `graph image render (biscuit-terminal)` timings before and after for the
observed case.

## Decisions

Decided with Ken on 2026-09-28.

1. **Spacing: collision-driven global step.** `mermaid-rs-renderer` 0.3.1 places
   commits at `seq × commit_step` and returns connectors as finished SVG path
   strings, so per-gap spacing would mean regenerating those paths after
   layout (fragile) or changing the renderer (a dependency change). The
   global step is kept but sized from real collisions only, using the first
   layout pass's tag bounds. This fully restores the observed case. A real
   collision, such as `main` and `origin/main` one commit apart, still widens
   every column; that is accepted.
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
