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
    - name: Finalize

review: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/{{state.name}}-review-{{iteration}}.md"
spec: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/components/{{state.name}}-spec.md"
lessons_learned: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md"
components: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/components"
aggregate: "{{ctx.repo_root}}/renderable/features/2026-05-19-pushing-toward-ir/review-${iteration}.md"

success: 
    message: "completed implementation review for **{{state.name}}** component"
failure:
    message: "failed to complete implementation review for **{{state.name}}** component"
---
::block when="state.name != 'Finalize'"

# Review Spec

## Context

- read:
    - @renderable/docs/tree-rendering.md
    - @renderable/docs/layout-and-style.md
- use the 'rust', 'biscuit-terminal', and 'renderable' skills
- during the course of working on these components, we've created a memory file at {{lessons_learned}}, read this file for additional context

::doc {{ctx.repo_root}}/prompts/snippets/test-rigor.md

## Task

We are moving components found in `biscuit-terminal` toward using the [tree-rendering](@renderable/docs/tree-rendering.md) as well as targeting terminal, markdown, and browser render outputs. The component you are going to focus on is **{{state.name}}**.

Your task is to provide the _implementation_ of this component a complete review:

> - the specification for this file is found at '{{spec}}'
> - the review file should be saved as '{{review}}'

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
- functionality which is light on test coverage (we expect strong unit and integration testing for everything)
- are there any changes which would make the code more ergonomic, more performant, or both?

When you have completed the review, you must make a judgment call on whether the current state of the implementation is "production ready". Whether you believe it is or not, you must add a section `## Production Readiness` which you will describe both what your judgment is and why.

Finally set the `ready` frontmatter property of '{{review}}' to a boolean value based on whether you believe the component is now production ready.

- you are done when 
    - you've finished the review and saved it to the body of '{{review}}'
    - the `ready` frontmatter has been set and the document '{{review}}' has been saved with this change

::end-block

::block when="state.name == 'Finalize'"
## Context

- read:
    - @renderable/docs/tree-rendering.md
    - @renderable/docs/layout-and-style.md
- use the 'rust', 'biscuit-terminal', and 'renderable' skills
- during the course of working on these components, we've created a memory file at {{lessons_learned}}, read this file for additional context

## Task

All of the components we just converted over to use the render tree as an IR step in rendering have now been reviewed. The review files are named `{component}-review-{{iteration}}.md` in the '{{components}}' directory. Each of them should have "review" frontmatter property which indicate whether they are production ready.

- Report the "production readiness" of each of the components
- Then iterate through each review and copy the suggested fix into an aggregate review file; save this aggregate file to '{{aggregate}}'

::end-block
