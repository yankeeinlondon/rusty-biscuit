## Context

We have now written specs (and reviewed them) for 12 components defined in `biscuit-terminal`. The ambition for all of them is the same:

- move to an IR based rendering strategy (ideally that defined in @renderable/docs/tree-rendering)
- render to Terminal, Markdown/MarkdownPlus, and the Browser

## Implementation

### Stage 1

Each component registered interest in "additions" they'd like to see implemented in the [tree-rending](@renderable/docs/tree-rendering.md) solution. The reviewer subsequently made a clear decision to accept or deny these requests. All accepted requests have been entered into the document @renderable/features/2026-05-19-pushing-toward-ir/approved-render-tree-functionality.md

- Stage 1 is implemented by acting as an orchestrator and having a subagent have these recommendations implemented
    - ask the subagent to review the "lessons learned" so far for context: @renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md
    - ask the subagent to consider whether they ran into anything novel or surprising that they think should add to the lessons learned file and ask them to save their additions if they have any to @renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md
- then another subagent should be asked to review the implementation; looking for gaps, mistakes, etc.
- any suggestions coming from the review should then be put back into an implementation subagent

Once Stage 1 is complete you should stop and wait for human review.

### Stage 2

- act as an orchestrator and iterate all of the spec files found in @renderable/features/2026-05-19-pushing-toward-ir/components/*.md
- for each spec have a subagent implement
    - ask the subagent to review the "lessons learned" so far for context: @renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md
    - ask the subagent to consider whether they ran into anything novel or surprising that they think should add to the
- then have another subagent review and fix
- then move to the next component's spec file
