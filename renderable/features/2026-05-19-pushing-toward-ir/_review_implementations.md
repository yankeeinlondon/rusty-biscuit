---
description: used to review and improve inline the specification for a particular component moving to IR
feature: "@renderable/features/2026-05-19-pushing-toward-ir"
iteration: 1
sequence: 
    - name: BlockQuote
    - name: Compose
    - name: FileSystem
    - name: OrderedList
    - name: UnorderedList
    - name: Progress
    - name: Section
    - name: StatusBlock
    - name: Table
    - name: TextBlock
    - name: Todo
    - name: TwoColumn

review: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/{{state.name}}-review-{{iteration}}.md"
lessons_learned: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md"

success: 
    message: "completed spec for **{{state.name}}** component"
failure:
    message: "failed to complete spec for **{{state.name}}** component"
---
# Review Spec

## Context

- read:
    - @renderable/docs/tree-rendering.md
    - @renderable/docs/layout-and-style.md
- use the 'rust', 'biscuit-terminal', and 'renderable' skills
- during the course of working on these components, we've created a memory file at {{lessons_learned}}, read this file for additional context

## Task

We are moving components found in `biscuit-terminal` toward using the [tree-rendering](@renderable/docs/tree-rendering.md) as well as targeting terminal, markdown, and browser render outputs.
