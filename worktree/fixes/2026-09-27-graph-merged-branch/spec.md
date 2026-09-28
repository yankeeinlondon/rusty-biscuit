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
status: finalized-spec
reviewed: true
reviewed_by: codex/gpt-6-astra
reviewed_on: 2026-09-27
review_iterations: 0
clarified: true
clarified_by: codex/gpt-6-astra
needs_rulings: false
review_note: the clarification process served as a review
implemented: false
related:
    - 2026-09-27-list-freshness-ux
human_review: false
message_to_agent: |-
    Phase 2 (shared rendering layers) is done; read implementation-log.md's
    "## Phase 2" before writing Phase 3 code. What Phase 3 must rely on:

    - GitGraph contract (biscuit-terminal): pass every line's tip with
      `GraphLine::with_tip(sha)`; a line without commits is labeled ONLY at
      its tip (the fork fallback is gone). `GraphLine::merged_into(sha)` draws
      a `merge` at that commit, found by position, but only if the merged lane
      is emitted before it AND the destination lane already has a commit
      before it (0.3.1 drops a labeled merge into a lane with no head). So in
      the focused and base views, keep at least one entry (context commit or
      `+N` square) before any merge destination on the default lane.
    - `fork_sha == None`, or a fork commit not drawn anywhere, makes the lane
      unconnected (never a substitute attachment) and sets
      `GitGraphPlan::incomplete`. A ref/PR/label whose commit is not drawn also
      sets it. So every ref you pass (local `main`, `origin/main`, parent tips)
      must be drawn, or it will trigger "Some history is not shown": anchor it
      (R6) rather than leave it outside the window.
    - `GitGraph::with_incomplete_history()` is the hook for `GraphFacts::incomplete`.
    - worktree-cli was touched once, deliberately: every GraphLine built in
      commands/git_graph.rs now calls `.with_tip(...)` so merged branches keep
      their labels until you rewrite gathering. Keep that when rewriting.
    - R3-A1: tag spacing applies only to LR/RL gitGraphs. With the global
      step, trimming a lone commit to `+1` no longer narrows the image; Phase 4's
      56-column evidence should expect trimmed_commits > 0 with the image
      shrinking, not getting narrower.
    - biscuit-terminal's `just lint` does not enable `image` (the feature that
      compiles git_graph); also run
      `cargo clippy -p biscuit-terminal --features image --all-targets -- -D warnings`.
    - Test helper: `biscuit_visualized::mermaid::MermaidDiagram::gitgraph_geometry()`
      (#[doc(hidden)]) gives repaired parents, lanes, and tag bounds with
      `tag_overlaps()`; Phase 4's end-to-end layout test should use it.
    - Docs are Phase 5: biscuit-terminal/docs/components/git_graph.md still
      says merged branches become tags and no merge statements are emitted.
references:
    renderer-spike.md: >-
        Isolated renderer feasibility experiment covering measured label spacing,
        exact merge identity and parent restoration, and compressed historical connections.
        Records results, reproduction steps, and limits on production applicability.
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

### Second observation: base view with a non-default recorded parent

Observed later on 2026-09-27, running `wt list` from the main checkout (base view) just after PR #104 merged `fix/sniff` on GitHub. `wt list` fetched `origin/main` as part of its freshness check, so the local `main` stayed one merge behind:

```text
*   b47a046 (origin/main) Merge pull request #104 from yankeeinlondon/fix/sniff   ← parents 08cf96a, b3b17f3
|\
| * b3b17f3 (fix/sniff) test(ci): record sniff as a narrowed reader …
| * 15edf42 …   (96 commits on this side; fix/sniff's recorded parent is fix/wt-ux)
* 08cf96a (main) Merge pull request #103 from yankeeinlondon/fix/wt-ux
```

The merge bases were `feat/schema-enhancement` → `1314bd6` and `fix/wt-ux` → `2a7298a`. The worktree table was correct (`fix/sniff` `clean -1` against `origin/main`), but the graph was wrong in three ways:

1. **`fix/sniff` vanished entirely: no lane and no tag.** `base_view` sets a line's fork to its merge base with its drawn recorded parent (`fix/wt-ux`); that commit sat inside `fix/wt-ux`'s `+N` square. With no commits of its own, `GraphLine::tip` falls back to the fork commit, and `GitGraph::tags` silently drops a tag whose commit is not drawn. The real tip, `b3b17f3`, was drawn on the default lane with no label. "No commits of its own" had been read as "tip equals fork", which holds only when the branch is an ancestor of its parent.
2. **Both remaining lanes were drawn forking from the wrong commit.** `base_view` builds the default lane with a plain `git log -10 origin/main`, which walked into the merge's second parent and filled every slot with `fix/sniff`'s history presented as `main`'s. Neither real merge base was drawn, and `GitGraph::attach_point` silently hung both lanes from the default lane's first drawn commit (`14daf59`), an ancestry that does not exist.
3. **The local `main` tag was missing.** `08cf96a` is the newest commit's first parent, but the date-ordered walk never reached it, so the graph did not show the gap the caption reported ("96 commits behind").

## Confirmed requirements

### Branch selection stays the same

The fix changes how selected branches are represented, not which branches are selected:

- When a non-default branch is checked out, select the default branch and the current branch. Preserve the existing split rule: if the current branch's fork-origin record names an existing non-default local parent branch, include that parent too. The parent does not need its own worktree. This is the existing one-level rule, not recursive ancestor expansion.
- When the default branch is checked out, select the worktree branches as today. Recorded parent relationships are used when that parent is also selected.
- Preserve the existing fallback when a recorded parent branch no longer exists.
- A selected branch must not lose its lane solely because it has been merged. In particular, the current branch in the observed case must retain a lane and a visible connection into the default lane at the merge commit.

Here, “default branch” means the repository's existing default-branch selection; `main` in the reported case is an example, not a new hard-coded name. Existing handling of the local and remote-tracking default tips remains part of the graph's context.

Preserve the existing height behavior: only the base view has the half-terminal-height lane cap, activity-based lane selection, and omitted-worktree notice. The focused view remains exempt. This fix does not change those selection and sizing policies.

### Supported merge histories

Restore lanes for selected branches whose current tips are fully integrated through an actual Git merge commit into a displayed parent, including a non-default parent. Show the verified fork and merge relationships. A branch with identifiable separate history does not become a label-only branch merely because its tip is now reachable from its parent.

This applies to a branch merged into the default branch even when its recorded fork parent is another selected branch, as `fix/sniff` (parent `fix/wt-ux`, merged into `main`) in the second observation. The fork comes from the parent and the merge goes into the default branch.

Both views build the default lane from its own history, not from commits reached through merges' second parents. The base view's window must not be filled by a merged branch's commits presented as the default branch's, and a local default tip that is an ancestor of `origin/<default>` stays reachable as a label, in compressed history if necessary.

For branches that continue after an earlier merge, preserve their current relationship without adding reconstruction of earlier partial or repeated merges. Squash-merge inference is outside this fix.

Fast-forward integration and equal tips must be represented truthfully, without inventing a merge commit. When the available history establishes that a branch has no separate history to draw, show its label at the actual commit, without an empty lane or an explanatory legend. Equal tips alone do not prove that the branch never had separate history.

### Essential historical connections remain visible

When a selected branch's fork or merge predates the ordinary displayed history, retain the essential fork and merge connections and compress intervening omitted history. Keep ordinary commit-history limits; do not expand every intervening commit merely to reach an older connection.

### Unavailable history

If local history cannot establish a connection, draw verified information and retain the selected branch wherever it can be truthfully represented. Leave unknown connections undrawn and show a brief incomplete-history notice. Keep the worktree table available. Do not fetch automatically or introduce a new network request to fill the gap.

An undrawn commit is never silently replaced by a different one. A fork commit that is not drawn is not attached at another commit (currently the start of the parent or default lane). A label whose commit is not drawn is either placed on that commit, brought back through the compressed history above, or accounted for by the incomplete-history notice. It is never dropped without a trace. A branch with no commits beyond its displayed tips is labeled at its own tip, not at its fork commit.

Unknown history is distinct from successfully determining that a branch has no separate history: do not silently apply the label-only exception merely because discovery failed. Preserve the existing policy for ordinary renderer failures.

### Full labels remain attached to their commits

- Preserve full branch and reference names and existing label content.
- Keep labels attached to the exact commits they describe.
- Prevent labels on neighboring commits from overlapping, including the `main` and `origin/main` case.
- Preserve existing history compression followed by image shrinking. Preserve the existing explicit-width behavior, which does not trim commits to fit the requested width.
- Do not introduce a minimum text size, a shortened-label legend, or a new text fallback as the solution to collisions. Small text in very narrow terminals remains an accepted consequence of image shrinking.

Combining labels by moving one to a different commit is excluded, because it would misrepresent the reference's location.

### Compatibility and operational boundaries

Preserve existing default-branch handling and command behavior outside the graph corrections. This fix introduces no new graph-specific network requests, persistent branch-history logging, command flags, or migration. Existing `--perf` timing provides review evidence; no operational telemetry is added.

## Ownership and implementation directions

Design for reuse: improvements that benefit callers generally belong in the shared layer, either `biscuit-terminal` or `biscuit-visualized`, according to the existing interfaces and responsibilities. Do not confine a shared fix to a worktree-specific workaround. Exact placement, algorithms, and API contracts remain planning details, not outstanding human rulings. Address the actual shared need without speculative abstractions.

- **`worktree-cli`:** its `commands::git_graph` module gathers Git facts and selects branches. It should provide sufficient ancestry and merge information for the selected branches without taking over layout. First-parent traversal is a candidate for keeping merged branch commits out of the default lane. Finding the merge that introduced a branch, and choosing its fork, need to handle the supported histories above; simple default-lane traversal has not yet been shown sufficient for non-default parents.
- **`biscuit-terminal`:** its `GitGraph` component owns lanes, labels, omitted-history compression, and sizing. Extending its `GraphLine` input with a merge destination, such as a `merged_into` builder method accepting a commit identifier, is a candidate. No public API spelling is settled.
- **`biscuit-visualized` (potentially affected):** its Mermaid rendering backend already parses, lays out, and renders diagrams through the dependency's public interfaces. It is a candidate location when the existing rendering interfaces make it the appropriate shared owner. Planning must determine whether label layout and merge correctness are best addressed here, in `biscuit-terminal`, or across their existing boundary. No backend modification is mandated, and Git discovery remains in `worktree-cli`.

A reusable explicit graph input or verified merge metadata may help convey the topology accurately, but neither is a required API. Choose the repair mechanism during planning; do not mandate a worktree-only or opt-in path for a shared rendering problem. Any explicit parent information must refer to verified existing nodes, with invalid input rejected rather than silently fabricating edges. Avoid duplicating the Mermaid parser. Measurement and rendering must use the same effective graph and layout configuration, and artifact-cache identity must account for every additional rendering input.

The experiment demonstrates a feasible route with the current dependency, not that its fixture repair is the production design. No dependency change is established as necessary or pre-authorized. If planning selects a materially broader dependency change, justify its scope and tradeoffs as part of that plan; this is not an outstanding specification ruling.

## Renderer experiment findings

The authorized and completed [renderer experiment](renderer-spike.md) used installed `mermaid-rs-renderer` 0.3.1 in an isolated harness, with no dependency modification:

- Measuring tag width and increasing commit spacing removed the tested neighboring-label overlap while preserving full labels on their original commit objects.
- The parser retained a labeled merge's identity and tags but lost its second parent. Restoring the fixture's known second parent through the public parsed graph preserved the exact merge identity, its tags, and the following commit's relationship.
- An eight-node compressed fixture represented 8,997 omitted commits while retaining fork and merge identities, correct merge parents, and nonoverlapping tags.

These are feasibility results, not production acceptance evidence. The compressed fixture was constructed by hand and did not exercise actual gathering or component compression. Arbitrary layouts, fonts, cross-lane collisions, final terminal fitting, operating systems, and production performance remain unproven. Wider spacing increases natural image width, so production validation must cover the agreed compression and shrinking behavior. The experiment's spacing margin is not a product constant.

## Acceptance criteria and validation

These criteria describe user-visible outcomes; implementation-specific API names and Mermaid statements are not acceptance requirements.

| Case | Required observable outcome | Validation approach |
|---|---|---|
| Current branch has been merged through a recorded merge commit, with its worktree retained | The current branch has a lane; its branch commits are not presented as a straight segment of default-branch history; its merge connection is visible at the correct commit | Real-Git fixture for gathered facts, component output assertions, and rendered-image inspection |
| Same merged branch viewed from the default branch | It remains eligible for its lane under the same branch-selection rule; merged status alone cannot collapse it into a tag | Real-Git fixture and component assertions, with enough terminal height to retain the lane under the existing base-view cap |
| Current branch has a recorded existing non-default parent | The parent remains selected and the child retains the recorded parent relationship; test both with and without a worktree for the parent | Real-Git selection fixtures |
| Branch fully merged into a displayed non-default parent | Its separate lane and verified merge connection into that parent remain visible | Real-Git child/parent merge fixture and component assertions |
| Base view: branch with a recorded non-default parent is merged into the default branch, and its fork from that parent lies inside the parent's `+N` square (second observation) | The branch is drawn with a fork from its parent and its merge into the default branch at the correct commits; at minimum its label sits on its actual tip, never nowhere | Real-Git fixture mirroring the observation and component assertions on tag and fork placement |
| Base view after fetching a merge whose second parent carries more commits than the default window | The default lane shows default-branch history rather than the merged branch's commits; other lanes fork from their true merge bases; the local default tip keeps its label | Real-Git fixture with a merged side longer than the base window; assert fork commit identities and the local default tag |
| A fork or label commit falls outside everything drawn | No lane attaches at a substitute commit and no label disappears silently; compressed history or the incomplete-history notice accounts for it | Component tests that omit the fork commit or tagged commit from drawn entries |
| Branch continues after an earlier merge | Its current relationship remains represented without requiring reconstruction of earlier partial or repeated merges | Real-Git fixture with a commit after the merge |
| Fast-forward integration or equal tips | No fabricated merge commit appears; a branch established to have no separate history is labeled at its actual commit, with no empty lane or new legend | Real-Git fixtures covering no separate history and identifiable merged history with equal tips |
| Necessary local history is unavailable | Verified information remains, unknown connections stay undrawn, a brief notice appears, and the table remains available without a new network request | Incomplete-history fixture with output assertions and request-count verification |
| Terminal height is constrained | Base-view cap, activity ordering, and omission notice remain; focused view remains exempt | Existing sizing regressions, including the focused-view exemption |
| Recorded parent has been deleted | Existing parent-missing fallback remains unchanged | Existing or extended selection regression fixture |
| Essential fork or merge is older than the ordinary displayed window | The relationship remains visible, with intervening history omitted or compressed rather than all commits expanded | Fixture exceeding the ordinary windows; assert connection commit identities and omitted-history representation |
| Neighboring commits carry long branch or reference labels | Full labels remain attached to their respective commits and do not overlap | Inspect actual rendered label bounds or pixels; source snapshots alone cannot prove this |
| `main` and `origin/main` are one commit apart | Both labels remain fully represented at the correct commits without overlap | Component fixture and rendering validation |
| Width is too small for the compressed natural image | Existing compression and shrinking behavior remains, without shortened labels or a new text fallback | Narrow-width fixture and existing explicit-width regression coverage |

Use the repository's nextest-based L1 recipes for gathering and component tests. Rendered output needs direct validation in addition to Mermaid snapshots. Select representative long labels and narrow and ordinary dimensions during planning, and inspect actual label bounds or rendered pixels for overlap and attachment. The existing Kitty L2 graph test in `worktree-cli` is a candidate integration point. Any L2 or L3 tests must avoid taking terminal or browser focus.

The affected packages must continue to compile and work on macOS, Linux, native Windows, and WSL2. The implementation plan must distinguish portable behavior tests from terminal-specific image evidence.

## Performance and completion

Preserve existing performance gates. Record before/after measurements of the existing `graph gather` and `graph image render (biscuit-terminal)` stages for ordinary history, older essential connections, and multiple selected branches. Review the measured changes rather than introduce a new numerical threshold. The existing full-command one-second gate covers the non-image path and is not a graph-rendering budget.

Implementation is complete and ready for review when the acceptance cases pass, relevant existing gates remain passing, rendered evidence establishes nonoverlap and exact commit attachment, performance comparisons are recorded, and affected documentation is updated. Human acceptance of the timing and visual evidence closes review; reviewers assess any performance regression against the measured cases. An agent does not move the specification to a completed lifecycle directory.

## Documentation affected

- `worktree/docs/git-graph.md`: replace the documented collapse of merged branches into tags and describe preserved selection, parent relationships, historical connections, and any agreed limits.
- `biscuit-terminal/docs/components/git_graph.md`: update lane and label behavior and document the final shared-component contract and renderer constraints.
- Update API documentation and package skills for whichever shared package interfaces or behavior change. If `biscuit-visualized` is affected, document its final contract and correct the stale renderer-version guidance identified by the experiment.

The existing documentation describes the current implementation; the implementation change must update it alongside the code.

## Settled scope and planning details

No required human rulings remain. The confirmed boundary favors reuse in `biscuit-terminal` or `biscuit-visualized` according to their existing interfaces and responsibilities, with Git discovery in `worktree-cli`. Exact shared-package placement, concrete API spelling, the merge-repair mechanism, spacing policy, fixture dimensions, and implementation sequencing remain planning details constrained by the requirements above.

The renderer experiment is complete; production implementation has not begun. This finalized specification is ready for planning and implementation, with the experiment's limits carried forward into validation rather than treated as completed product evidence.
