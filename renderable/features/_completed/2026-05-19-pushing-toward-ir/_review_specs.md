---
description: used to review and improve inline the specification for a particular component moving to IR
feature: "@renderable/features/2026-05-19-pushing-toward-ir"
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

## Task

We are moving components found in `biscuit-terminal` toward using the [tree-rendering](@renderable/docs/tree-rendering.md) as well as targeting terminal, markdown, and browser render outputs. As a first step, all the components had a specification file created to describe what it needed to complete this path.

You are responsible for reviewing the spec file created for the **{{state.name}}** component which is located at '{{ feature}}/components/{{state.name}}-spec.md'.

> **Remember:** you're in a non-interactive session so no questions allowed, when asked to take design decisions use your own discretion no confirmation needed. if something is truly too vague to be understood or there are critically important design decisions which you don't feel comfortable making without user feedback then add a section to the end of the spec file called `## Follow-up Clarifications and Design Decisions`

- you should look for gaps in the design and fill that in with the appropriate design detail
- if you notice something that is incorrect then fix that in the spec file directly
- if you notice gaps in the tests strategy flesh out the test strategy in the spec file to address the gap
- if the specification has requested a change be made to the render-tree implementation:

    - carefully review the request
    - improve the details of the request if they aren't precise enough
    - then approve or deny the request based on whether you feel this suggested change should be included in the render-tree implementation
    - add a **APPROVED** or a **DENIED** stamp on every request
    - always explain WHY you chose to approve/deny
    - when **approved**:
        - add the text "this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.
        - add the approved item to the document "@feature/approved-render-tree-functionality.md"
    - when **denied** add the text "this feature will not be added to the render-tree tree implementation. You should try to still use the render-tree where practical and work around the complexity but if the complexity is too great then you have permission to create a bespoke IR implementation for this component.

All of your review and approval work needs to be written back to the '{{feature}}/components/{{state.name}}-spec.md' file.

Once you're done, consider if there were any novel or surprising discoveries and if there were then append them to '{{feature}}/lessons-learned.md' if there were.
