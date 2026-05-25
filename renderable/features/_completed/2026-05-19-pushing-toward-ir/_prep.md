---
description: used to build detailed research into each component we're going to tackle in this feature
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
    message: "completed spec for **{{state.name}** component"
failure:
    message: "failed to complete spec for **{{state.name}}** component"
---
> **IMPORTANT:** if the file '{{ feature }}/components/{{state.name}}-spec.md' already exists then that means this task is already done. You should tell the caller that this task was already complete and there is nothing left to be done. Once communicating to the caller you are done and do not continue on investigating or designing anything.

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

        - Write the following static text into this H3 section:
            - "- The **{{state.name}}** component does not currently have a IR based rendering solution"
            - "- This section will describe what is required to ensure that the **{{state.name}}** component:"
            - "    - has an IR implementation"
            - "    - the IR implementation drives the TerminalRenderable contract"
            - "    - the IR implementation is what is used by the bt CLI (note if **{{state.name}}** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)"

        - Evaluate how to render **{{state.main}}** into the render tree
        - Consider what Layout and Style parameters this component should have and map them into the render tree's Style and Layout features
        - Describe the critical test variants that will be needed to have high confidence in this new IR rendering as well as be able to detect variance from the prior rendering approach
        - The end goal is to retain the old approach for now but to use IR rendering as the default rendering for both the `bt CLI` as well as this component's `TerminalRenderable` contract.
        - If there are any aspects of the Tree Rendering solution that make the implementation of this component inefficient that you think a new feature in the tree renderer might allieviate then you should add a H4 section called "#### Feature Requests for Tree Rendering"
            - In this section you need to describe each feature you're requesting; including:
                - What does the new feature look like? Give examples of how it would be used.
                - Why you feel the {{state.name}} component needs it (or at least how it benefits from it)
                - Whether NOT having this feature would force the {{state.name}} to use it's own bespoke IR instead of the tree-render
        - Make sure to indicate whether you think the current tree-render is a good fit and whether you recommend using it (assuming none of your suggested feature requests are approved)
            - write this as prose into the document
            - set the `will_use_tree_renderer` to a boolean value indicating whether you would implement (with no features approved) the existing tree-renderer (over a bespoke IR)
            - set the `will_use_tree_renderer_with_feature` to a boolean value indicating whether you would recommend using the tree renderer (over a bespoke IR) if the features you've specified were implemented

    - Browser IR Implementation (add as H3 heading in doc, when design is needed)

        > - the IR implementation will be in place now because we've ensured that Terminal IR is done first
        > 
        > - you can skip this section if the "Browser" column is marked as complete AND "IR State" is not "bespoke" or "-"

        - write the following static text:
            - `- in this section we will provide a design specification for the **{{state.name}}** component's implementation of the BrowserRenderable trait`

        - if you have an existing bespoke rendering implementation for the browser make sure you take the time to understand it fully first before moving onto the next step
        - Evaluate how the IR Implementation for the browser was implemented and create a design on how to render the Browser implementation
            - If you find that the IR that the Terminal needed doesn't support the Browser's rendering then you are allowed to modify it but you will then need to review and update the Terminal IR section to make sure it's aligned with your design changes
        - List out all the key variants to testing that must be covered for this component

    - Markdown IR Implementation (add as H3 heading in doc, when design is needed)

        > this design can be skipped only if the "Markdown" column has been checked AND the "bt CLI" column is neither "bespoke" or "-"

        - First take a moment to distinguish the difference between a **Markdown** output and a **MarkdownPlus** output:
            - Both outputs are valid Markdown content
            - However, **Markdown** tries to optimize for "ergonomics" whereas **MarkdownProse** tries to optimize for "features" and "fidelity"
            - These preferences largely come down to how much inline HTML is allowed for
            - If the content provided as input can be represented purely using Markdown syntax (no inline-html) then both the **Markdown** and **MarkdownProse** should be the same!
            - If there is representation of text or background colors in the input then this is a clear separation point because Markdown does not provide for specifying inline colors in a document. 
            - If the input were something like `<span>Hello <b>Bob</b></span>` then we could comfortably reduce this to just `Hello **Bob**` and not loose any fidelity; this means both Markdown and MarkdownPlus would return this.
            - If the input were something like `<span style="color:red">Hello <b>Bob</b></span>` then you'd see the two Markdown target's diverge:
                - Markdown: `Hello **Bob**`
                - MarkdownPlus: `<span style="color:red">Hello <b>Bob</b></span>`
            - Both `Markdown` and `MarkdownPlus` allow for assigning styles in Markdown to the `styles` Frontmatter property.
        - Based on the IR that has already been developed for the Terminal and Browser you should be able to create a design for rendering the two Markdown formats
        - Make sure to explicitly call out situations where Markdown diverges from MarkdownPlus for this component
        - Describe your testing strategy for **{{state.name}}**'s implementation of `BrowserRenderable`

    - `bt` CLI (add as H3 heading in doc, when design is needed)

        - start with the following static text:
            - `- this specification will ensure that the **{{status.name}}** component:`
            - `    - has a 'bt' CLI subcommand for rendering this component`
            - `    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)`
            - `    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)`
        - Now check the source code for the **{state.name}** component and represent what the "current state is" with regards to:
            - CLI command exists or doesn't
            - render method used in CLI (IR or bespoke)
            - has target switches (--md, --html)
            - has example switch (--example)
        - with the current state documented, now create a specification design for what must be done to make sure that the `bt` ClI is complete with all required CLI switches

## Acceptance Criteria for Implementation

- the **{{state.name}}** component has implemented all renderable traits:
    - `TerminalRenderable`
    - `MarkdownRenderable`
    - and `BrowserRenderable`
- the `bt` CLI has a subcommand for **{{state.name}}** which:
    - has a valid implementation for rendering with `--md` and `--html` (aka, can target Markdown and HTML outputs)
    - by default will render for the terminal
    - has a valid implementation for the `--example` CLI switch which shows an example output of the command as well as the full CLI request that would be used to present that example output (note: must be formatted like other --example implementations)
- we have strong test coverage across all functionality this component exposes
- we make sure that the `bt` CLI follows all best practices mentioned in the 'cli' skill

## Important References and Skills

- you should always use the 'biscuit-terminal', 'renderable', and 'rust' skills
- you should use the 'cli' skill when working with the `bt` CLI
- you should read the [Style and Layout](@renderable/docs/layout-and-style.md) documentation to understand how we're expecting components to treat Layout and Styling. Every component must consider how this applies to them.
