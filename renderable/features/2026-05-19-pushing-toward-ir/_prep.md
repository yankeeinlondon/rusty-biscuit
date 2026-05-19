---
description: used to build detailed research into each component we're going to tackle in this feature
feature: "@renderable/feature/2026-05-19-pushing-toward-ir"
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
---

# Moving Toward IR Rendering

You are responsible for helping to design the movement of the **{{state.name}}** component (found in `biscuit-terminal`) toward it's eventual goal of:

- being rendered via an IR (internal representation)
    - In 95% cases that means the Tree Rendering solutions described here: [Tree Rendering](@renderable/docs/tree-rendering.md)
- being able to implement all output targets:
    - Terminal
    - Markdown
    - Markdown Plus
    - HTML
- this means that these components will implement the following traits:
    - [`BrowserRenderable`](@renderable/src/browser/renderable.rs)
    - [`MarkdownRenderable`](@renderable/src/markdown.rs)
    - [`TerminalRenderable`](@biscuit-terminal/lib/src/components/renderable.rs)

## Design Steps

> Your specification/design for the **{{state.name}}** will be saved to the {{ feature }}/components/{{state.name}}-spec.md

1. Lookup the current "status" of **{{state.main}}** in the @renderable/docs/components.md table
    - Based on what you see in the "IR State" and "bt CLI" columns you will know the scope of your design requirements
2. Start by adding an H2 heading called "## Design Steps"
3. Only design the steps which are relevant based on your component:

    - Terminal IR Implementation (add as H3 heading in doc, when design is needed)

        > if the "IR State" says "both avail, old renders" _or_ "component IR" then this is **done** and you can skip design for this work

        - Evaluate how to render **{{state.main}}** into the render tree
        - Consider what Layout and Style parameters this component should have and map them into the render tree's Style and Layout features
        - Describe the critical test variants that will be needed to have high confidence in this new IR rendering as well as be able to detect variance from the prior rendering approach
        - The end goal is to retain the old approach for now but to use IR rendering as the default rendering for both the `bt CLI` as well as this component's `TerminalRenderable` contract.
        - If there are any aspects of the Tree Rendering solution that make the implementation of this component inefficient that you think a new feature in the tree renderer might allieviate then you should add a H4 section called "#### Feature Requests for Tree Rendering"
            - In this section you need to describe each feature you're requesting; including:
                - What does the new feature look like? Give examples of how it would be used.
                - Why you feel the {{state.name}} component needs it (or at least how it benefits from it)
                - Whether NOT having this feature would force the {{state.name}} to use it's own bespoke IR instead of the tree-render

    - Browser IR Implementation (add as H3 heading in doc, when design is needed)

        > the IR implementation will be in place now because we've ensured that Terminal IR is done first

        - Evaluate how the IR Implementation for the browser 

    - `bt` CLI uses IR rn

## Important References and Skills

- you should always use the 'biscuit-terminal', 'renderable', and 'rust' skills
- you should use the 'cli' skill when working with the `bt` CLI
- you should read the [Style and Layout](@renderable/docs/layout-and-style.md) documentation to understand how we're expecting components to treat Layout and Styling. Every component must consider how this applies to them.
