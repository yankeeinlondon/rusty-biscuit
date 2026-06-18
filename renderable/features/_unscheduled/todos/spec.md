One of the most popular features of the GFM standard for Markdown is TODO items. With Darkmatter we aim to support GFM but to go further.

- at least SOME of this functionality was provided in the past
- however, we've recently switched our rendering to use a tree-rendering model
- in the Darkmatter CLI we used to have a `--nerd` option which turned on more advanced rendering for the terminal via nerd fonts. We do NOT want that kind of a solution
- instead when rendering to the terminal we should leverage `biscuit-terminal`'s ability to detect nerd fonts.
- for the browser, we should always be able to render more visually desirable representations of this with inline SVG icons

## Features

We have already designed the concept of "features" at a high level but because we're attempting
to have a nice visual experience when rendering to the Browser via SVG icons representing TODO states
this seems like a good use case for a feature and this functionality will need to be detailed out
in greater detail.

- For this feature we expect this feature to be activated automatically when using either the:
    - `UnorderedList` component
    - `OrderedList` component
    - rendering markdown content with Darkmatter

- this feature defines a few "extended" states that a TODO can represent:
    - `- [ {#}% ]` percentage done markers
    - `- [ ! ]` blocked marker

- when the "Todo" feature is enabled it will need to pass up the following requirements:
    - Classes:
        - adds a stylesheet that defines styles which point to CSS Variables
        - I think the SVG should be defined exclusively by CSS variables
        - We might benefit from a `Svg` -- and possibly a `InlineSvg` - component which can provide ergonomics for this
        - I'm imaging that each "state" of the Todo would have CSS variables for the following attributes:
            - Stroke Color
            - Fill Color
            - Path
            - Stroke Width
        - It might look something like:

            ```css
            .todo.empty: var(--todo-empty)
            .todo.completed: var(--todo-completed)
            .todo.blocked: var(--todo-blocked)
            .todo.not-started: var(--todo-not-started)
            .todo.one-quarter-complete: var(--todo-one-quarter-complete)
            .todo.half-complete: var(--todo-half-complete)
            .todo.three-quarters-complete: var(--todo-three-quarters-complete)
            ```
