---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
recurrence: false
created: 2026-09-28T01:36:33-07:00
spec: 2026-09-27-graph-sparse-lanes/spec.md
implemented: false
description: "A **fix** review of `2026-09-27-graph-sparse-lanes/spec.md`"
fix: 2026-09-27-graph-sparse-lanes/review-1.md
---

# Review 1: Graph sparse lanes

## Verdict

**Production ready.** I found no missing requirement or implementation defect. The input robustness matrix does not apply: this change uses Git history and an existing Mermaid parser, and adds no file or configuration reader.

## Requirement verification

| User-visible requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Long labels that do not meet vertically keep the default commit spacing; colliding labels receive the smallest sufficient global spacing, including offset labels and theme font overrides | Level 1 tests of the public Mermaid geometry against actual renderer layouts, with controls at the unspaced step | Appropriate for the placement rule; tests also cover stacked tags, neighboring commits, cross-lane tags, and right-to-left behavior in the current renderer. |
| Measured width and rendered layout agree, so the graph keeps several recent commits on each long branch at 200×60 | Level 1 real-Git fixture passed through `GitGraph::plan` and `MermaidDiagram::gitgraph_geometry`; Level 2 Kitty screenshot and transmitted image inspected | Appropriate. The saved screenshot shows five commits after each affected branch's `+N` square and the image in the reserved rows. |
| A direct merge into `main` wins over an earlier indirect match in the recorded parent, while a direct merge or first-parent match in that parent still wins | Level 1 real-Git classification and graph fixtures | Appropriate; the tests assert the chosen merge commit, fork, lane, and resulting Mermaid merge parents. |
| A fork absent from every drawn lane stays undrawn with an incomplete-history notice; complete connections show no notice | Level 1 real-Git fixtures and component plans; Level 2 Kitty capture for the observed case | Appropriate; the observed fork is absent, the merge edge appears, and a separate drawable-fork fixture has no notice. |
| Shallow or failed history queries do not invent a connection; branch selection, height limits, and explicit width keep their existing behavior | Level 1 real-Git and component cases, including shallow history and 120×40/56×60 plans | Appropriate for decisions made before terminal rendering. |
| The final image has separate labels and visible lanes, with no text drawn over the image | Level 1 layout bounds plus Level 2 Kitty capture of screen text, transmitted PNG, and screenshot | Appropriate. The saved 200×60 screenshot shows distinct `main` and `origin/main` labels, the direct merge edge, the incomplete-history notice, and intact table text. |

The Kitty test is a declared `worktree-cli` test target behind `terminal-tests`, selected by the live Level 2 recipe. `just check-tier-coverage worktree` reports zero stranded tests. No keyboard or mouse behavior changed, so Level 3 verification is unnecessary. The saved screenshot supplies the visual check; no separate human review is needed. The Level 2 test was not rerun in this review because the implementation record reports that the host display locked during its later rerun; its earlier passing screenshot and transmitted image were inspected here, and no source changed after that capture.

## Validation and performance

- `just test` in `biscuit-visualized`: 116 passed.
- `just test` in `worktree`: 759 passed, 30 excluded by tier selection.
- `just check-tier-coverage worktree`: zero stranded tests.

The recorded ten-sample release measurements at 200×60 put image rendering at 350.0–357.8 ms before and 341.7–351.6 ms after. Graph gathering rose from 58.8–70.3 ms to 73.8–85.6 ms because it checks the later candidate and its anchors after an indirect parent match. The review found no equally simple way to retain that proof while avoiding the extra Git queries. The Mermaid cache backend identifier was bumped for the changed image layout, and its key has a Level 1 test.
