---
created: 2026-09-28
status: implemented
clarified: false
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-28
review_iterations: 0
completed: false
implemented: true
implemented_by: claude/opus
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
    - biscuit-terminal
related:
    - 2026-09-27-graph-merged-branch
    - 2026-09-27-graph-sparse-lanes
human_review: false
message_to_agent: |-
    All five phases are implemented (2026-09-28); the spec is ready for review. Phase 5 changed docs and skills only,
    plus one reworded test assertion message. Departures from the spec are listed in implementation-log.md
    "## Phase 5" > "Review readiness" (D3 subtree ordering, G5 amendment, the L1 density exception). Do not move
    the spec to _completed; the author does that after review.
---

# Draw a branch that continued after its merge

## Outcome

When a branch is merged and then gets new commits, `wt list` draws it as one
connected lane: the branch forks from the lane it came from, merges into the
merge commit, and continues past it with the new commits. Branches that share
the merged tip, such as a child created there, get their labels and forks
drawn on that lane. The notice "Some history is not shown" no longer appears
just because the branch was merged and then kept going.

This is the most common flow in the repository: a pull request merges, and
work continues on the same branch.

## Problem

Observed on 2026-09-28 in the `fix-wt-ux` worktree, before and after PR #105
merged, and again after `wt --ff`:

```text
85852c0  main = origin/main   Merge PR #105 (parents: b47a046, eeb7154)
572ef7d  fix/wt-ux            parent: eeb7154
eeb7154  fix/sniff-pr         the merged tip (PR #105's head)
b47a046                       85852c0's first parent
```

Before the merge, the graph was correct: `fix/wt-ux` forked at `b47a046`,
with `+87` and five commits, and `fix/sniff-pr` and the PR badge were at its
tip. After the merge (both before and after `--ff`), it showed:

| Lane / label | Drawn | Should be |
|---|---|---|
| `main` | `b47a046 → 85852c0`, no incoming merge | the same, with `85852c0` drawn as a merge of `eeb7154` |
| `fix/wt-ux` | one commit, `572ef7d`, connected to nothing | fork at `b47a046`, `+N`, recent commits up to `eeb7154`, merge into `85852c0`, then `572ef7d` |
| `fix/sniff-pr` | missing | a label at `eeb7154` on the `fix/wt-ux` lane |
| notice | "Some history is not shown" | none |

### Cause

This behavior is by design. `worktree/docs/git-graph.md` ("Continued after a
merge" and "Limits") says so, and
`a_branch_continued_after_its_merge_is_an_unmerged_lane` asserts it:

1. `572ef7d` is not contained in `main`, so `fix/wt-ux` is classified
   **unmerged**.
2. An unmerged lane is `T`'s first-parent chain until the history of the
   default tips ([`DefaultTips::exclusions`](../../cli/src/commands/git_graph.rs)
   in `worktree-cli` supplies those tips). That history includes
   `eeb7154`, so the lane holds only `572ef7d`.
3. The lane's fork is `merge-base(main, 572ef7d)`, which is `eeb7154`, the
   merge's second parent. Every lane follows first parents only, so no lane
   draws `eeb7154`. The lane is drawn unconnected, and the notice appears.
4. `fix/sniff-pr` has no history of its own, so it should be a label at
   `eeb7154` on its parent's lane. That commit is not drawn, so the label is
   dropped, and the notice counts it.

The same cause leaves `fix/sniff` unconnected in the observed
`2026-09-27-graph-sparse-lanes` history. Its fork `W1` is `fix/wt-ux`'s old
tip, merged by `M103`. The earlier design kept that fork undrawn because it
ruled out reconstructing earlier merges. This spec revises that choice (see
[Decisions](#decisions)).

## Definitions

- **Lane boundary `B`**: the first commit reached by walking backward along
  the branch's first parents that belongs to its current stop history. `B`
  itself is excluded from today's lane. It may be a fork, a merge source, or
  a commit the branch reached indirectly. In the observed case, `B` is
  `eeb7154`. If the walk cannot prove which commit is the boundary, its
  location is unknown; a shallow clone is not evidence that the walk ended.
- **Reconstructed merge**: `B` is a non-first parent of a commit `C` on a
  candidate lane's first-parent chain. The candidate lanes are the ones
  [`place()`](../../cli/src/commands/git_graph.rs) in `worktree-cli` already
  uses, in its order: the recorded parent, the default lane, then a diverged
  `origin/<default>`. Reconstruction uses
  [`History::classify(B, candidates)`](../../cli/src/commands/git_graph/topology.rs),
  the `worktree-cli` classifier that finds direct merges, and requires a
  `MergedDirectly` result. The source is `B`; the destination is `C`. Both must be
  drawn at their real commits for the edge to appear.

## Requirements

### Gathering (`worktree-cli`)

- For every branch **with a lane of its own** (unmerged, merged directly, or
  integrated otherwise), locate its boundary using its existing stop set.
  A branch classified as having no separate history remains a label at its
  tip; do not make a lane for it merely because that commit was merged.
  If its boundary `B` has a reconstructed merge into `C`, extend the lane:
    - replace each stop whose history contains `B` with history through
      `C^1`; retain stops that do not contain `B`, so the lane does not
      repeat commits owned by another branch lane. Verify that `B` is on
      the branch's first-parent chain and outside the new stop history
      before accepting the extension;
    - its fork is `merge-base(C^1, B)`, measured against `B`, not the tip.
      A tip that merged the default branch back in (as `fix/wt-ux` did at
      `B1`) would otherwise put the fork at `C^1` itself;
    - `B` is an anchor on the lane, and `C` is an anchor on `C`'s lane;
    - the line records a merge from `B` into `C`.
- Repeat from the extended lane's **older** boundary, drawing every earlier
  direct merge that can be proven. Each accepted step must move strictly
  backward on the branch's first-parent chain; remember visited boundaries
  so a repeated or inconsistent answer ends reconstruction as incomplete.
  Collect edges oldest first for drawing. A branch merged twice and then
  continued has both sources and both destinations pinned.
- Respect a recorded fork origin. If the branch's recorded creation SHA is
  `B` or a descendant of `B` on the branch's first-parent chain, the branch
  was created at or after that boundary: stop there and do not attribute an
  earlier merge to this branch. A missing or stale record supplies no such
  cutoff; the graph describes commit topology, not the date a branch name
  was created. Preserve the current unconnected-lane notice when that cutoff
  leaves a fork that no drawn lane contains. This prevents a branch newly
  created at an already merged tip from claiming the old branch's merge.
- The lane's own latest relationship (its classification) is unchanged.
  A lane that is merged directly still ends with its merge into the lane it
  was merged into.
- If a complete-history answer proves that `B` is an ordinary fork or was
  integrated only indirectly, stop reconstruction there and keep today's
  connection and notice behavior for that case. A failed Git call, malformed
  answer, or shallow negative answer is a `GatherGap`: keep only verified
  edges, mark the graph incomplete, and never invent a merge. Graph gathering
  remains local and makes no network request.
- Nothing is substituted. If `B` or `C` cannot be placed at its verified
  position, that merge is not drawn, and the notice accounts for it. Do not
  let a child label, fork, or later edge attach to a substitute commit.
- The branch's own window stays at
  [`LINE_WINDOW`](../../cli/src/commands/git_graph.rs), the `worktree-cli`
  limit for recent commits. The anchors (every merge source, plus forks and
  labels of other lanes on it) are
  placed at their real distances, with exact `+N` squares between them.
  The destination lane keeps every `C` and the commit before its oldest
  merge destination, including in the focused view, so a merge can be
  emitted after a destination lane has a head.

### Drawing (`biscuit-terminal` `GitGraph`)

- [`GraphLine`](../../../biscuit-terminal/lib/src/components/git_graph.rs),
  the `biscuit-terminal` input for one branch lane, can carry merges from
  any commit on the lane, including its last. Each edge is a pair of full
  commit IDs: a source on this line and a
  destination on another line. The current `merged_into` becomes an edge
  whose source is the lane's tip. Replace that field and builder with one
  ordered collection and builder; update all callers and component docs in
  this repository. There are no outside users.
- The lane stays on one row, but emission must proceed in segments. Emit the
  branch from its fork through a merge source `B`; pause it; emit enough of
  the destination lane to reach `C`; emit `merge <branch>` at `C`; then
  `checkout <branch>` and resume with commits after `B`. Repeat for later
  edges. The current recursive emitter marks a branch complete only after
  emitting its entire lane, so merely allowing a mid-lane source in that
  check would put post-merge commits before `C`. Track each lane's next
  un-emitted entry and each emitted source instead. A merge is ready only
  after its source and the destination's preceding commit have been emitted.
  `mermaid-rs-renderer` 0.3.1 leaves the source branch's head at `B` when it
  emits the merge, so the resumed branch's next entry follows `B`.
- Lanes hanging from a commit before `B` are emitted with the part of the
  lane before the merge. Lanes hanging from a later commit are emitted with
  the part after it. A lane hanging from `B`, such as `fix/sniff-pr`, is
  declared at `B` before the destination merge and keeps its label there.
  `fix/sniff` forked at `W1` is another such case.
- A merge into a sibling or descendant lane may require that destination
  lane to start before the source lane finishes. Order lane segments by
  their fork and merge dependencies, and detect a cycle instead of looping.
  If dependencies cannot be satisfied, leave only the affected merge or
  connection undrawn and mark the plan incomplete. Deterministic tie
  breaking preserves the existing creation-time order of unrelated lanes.
- The existing rules hold for every merge on a line:
    - a merge whose destination is not drawn, or would come before its
      source, is not drawn, and it is counted;
    - one lane merged per commit;
    - merge sources, merge destinations, forks, lane tips, and tagged
      commits are never folded into `+N`;
    - the height cap's `lane_ancestors` includes every lane holding a merge
      destination, and width trimming pins every merge source as well as
      every destination.
- For an ordinary two-parent merge, `biscuit-visualized` geometry reports
  `C` with its real first parent and `B` as its second parent. If the branch
  commit immediately after `B` is drawn, it has `B` as its parent; if commits
  between `B` and the next drawn commit are folded, the `+N` entry follows
  `B` instead. An octopus merge can
  have several non-first parents; the existing one-merge-per-commit rule
  still draws at most one source and reports the omitted edges as incomplete.

### Unchanged

The following stay as they are:

- branch selection, the height cap, and the focused view's default-lane
  window;
- `--width`, the trimming order, and tag spacing;
- PR filtering by source repository;
- squash merges, which have no merge commit and stay unmerged lanes;
- indirect integration, which gets no merge edge.

## Acceptance criteria

| Case | Required outcome | Validation |
|---|---|---|
| The observed PR #105 shape: merged, one commit after, with a child branch at the merged tip | `fix/wt-ux` forks at `b47a046`, has `B` = `eeb7154` merged into `85852c0`, then `572ef7d`; `fix/sniff-pr` is a label at `eeb7154`; `incomplete` is false | Real-Git fixture in `git_graph/tests.rs`: gathered facts as exact SHAs, and plan assertions |
| The same shape before `--ff` (local `main` behind a diverged `origin/main` that holds the merge) | Same lane; the merge lands on whichever lane draws `C` | Real-Git fixture |
| `a_branch_continued_after_its_merge_is_an_unmerged_lane` | Flips: `b1` drawn on `b`'s lane, merged into `merge`, then `b2`; fork at `d1`; no notice | The existing test, renamed and inverted |
| `observed_sparse_lanes()` (`fix/wt-ux` merged at `M103`, continued, synced back at `B1`) | `fix/wt-ux` is drawn with `W1` merged into `M103`; `fix/sniff` forks at `W1` on that lane and merges into `M104`; no notice from either | Existing fixture; update `Evidence` and the density test |
| Merged twice, then continued | Both merges and the intervening commits drawn in source order; no repeated commit or false notice | Real-Git fixture and component geometry |
| A new branch created at an already merged tip | The recorded creation SHA prevents the old merge from being attributed to the new branch | Real-Git fixture with a fork-origin record |
| A merge source has two children, one forked before and one after the merge | Both fork at their real SHAs; the source branch resumes after the destination merge | Component Mermaid and geometry tests |
| A merge goes into a sibling lane that itself has a later merge | Every source precedes its destination, without a duplicate commit or a scheduling loop | Component test |
| `B` reached the default branch through another branch's merge | No reconstructed merge; today's drawing and notice | Real-Git fixture |
| A shallow clone whose boundary crosses the shallow cutoff | No invented merge; notice | Existing shallow test pattern |
| Git fails while looking for an older boundary | Earlier verified edges remain; the notice appears | Injected Git failure or shallow fixture |
| A branch that has not been merged, forked from a commit on the default lane | Identical gathered facts to today | Existing tests unchanged |
| Layout | No overlapping tags; tags on their SHAs; exact merge parents, including post-merge commits' parents | `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags` with the new fixtures |
| `GitGraph` component | A mid-lane merge, lanes forking before and after it, and one merge destination hidden by the height cap | `biscuit-terminal/lib/src/components/git_graph/tests.rs` |
| Kitty | The continued-after-merge graph draws connected, with no notice | `level2_graph_in_kitty.rs`: new or extended fixture, screenshot kept |
| Performance | Keep the ordinary unmerged branch fast; no network call, no Git subprocess per historical commit, and no repeated lookup for a boundary already classified. Existing command time limits still pass | Compare Git call counts and `perf_graph_stages` medians and spread before and after; record them in the implementation log |

## Decisions

1. **Reconstruct from the lane boundary, not the fork.** Checking only
   "is the fork a merge's second parent" fixes `fix/wt-ux` after PR #105.
   It misses `fix/sniff` in the observed sparse-lanes history, because
   there `fix/wt-ux` synced `main` back in, so its `merge-base` is on the
   default lane while its first-parent chain still stops at the merged
   `W1`. The boundary covers both cases with one rule.
2. **One row per branch.** Splitting the lane into two `GraphLine`s (the
   segment before the merge, then a child forked at `B`) would reuse today's
   emission unchanged. It costs a row per merge, though, and draws one
   branch as two, which misstates the history. The emission change is
   contained in `GitGraph`.
3. **Revises the earlier sparse-lanes decision.** The earlier design
   rejected reconstruction because it changes how every continued branch is
   drawn and costs width. Seeing the real effect (a floating dot, a missing
   label, and the notice after every merged PR) shows the drawing was
   wrong, not just incomplete. A reconstructed merge pins a source, a
   destination, and sometimes an extra `+N` square; many historical merges
   can therefore add width. The existing height and width caps still apply,
   and the performance check above measures the added Git work.
4. **Recover every verified earlier merge.** Recovering only the latest
   would leave older child forks and labels floating, especially in the
   sparse-lanes example. A strictly older boundary and a visited set make
   the walk finite. One new boundary lookup per reconstruction is expected;
   reuse classification results where the same boundary appears again.
5. **Use fork-origin records as a cutoff when available.** The same commit
   graph can describe a branch that continued after a merge and a different
   branch created later at the merged tip. Git cannot distinguish the names
   from topology alone. A recorded creation SHA prevents attributing history
   before that SHA to the new branch. Without a usable record, render the
   verified commit ancestry and avoid claiming when the branch name began.

## Open questions

**Should a PR badge name its source branch when another branch has a label
at the same commit?** Before PR #105 merged, the badge for `fix/wt-ux` sat
beside the `fix/sniff-pr` label, which made the badge easy to misread. This
is a separate labeling problem and does not affect merge reconstruction.

- **Recommend: handle badge attribution in a separate change.** Pros: keeps
  this fix focused and lets the label design cover every same-tip case. Cons:
  the badge remains ambiguous until that change lands.
- **Add the source branch to every PR badge here.** Pros: each badge is
  self-explanatory. Cons: longer tags increase graph width for every PR and
  require updates to snapshots and the spacing checks.
- **Add the source only when another branch shares the tip.** Pros: limits
  the added width. Cons: the same PR's label changes as branches move, and
  the rule is harder to explain.

## Out of scope

- Squash-merge inference.
- Merge edges for indirect integration.
- Reconstructing merges of the default branch **into** a branch (such as
  `B1`), which are already drawn as ordinary commits on the branch lane.

## Documentation affected

- `worktree/docs/git-graph.md`:
    - replace "Continued after a merge" with the new drawing and a Mermaid
      example;
    - update the "Merged directly after the parent took it indirectly"
      example (the fork is now drawn);
    - remove the "No reconstruction of earlier merges" limit;
    - update the anchors list with merge sources, explain the fork-origin
      cutoff and the remaining limits for shallow and octopus histories;
    - explain that a graph without a usable fork-origin record describes
      commits, not when a branch name was created.
- `biscuit-terminal/docs/components/git_graph.md`: `GraphLine` merges and
  how lane segments are emitted and limited.
- `.claude/skills/worktree/SKILL.md`: the `wt list` graph summary, which
  describes the continued-after-merge case as unconnected.
- `worktree/README.md` if its graph example or description shows the old
  continued-branch behavior.
