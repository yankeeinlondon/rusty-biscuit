## Context

The following components have been implemented using the classic bespoke method AND the new tree rendering approach:

- `Section` (biscuit-terminal)
- `UnorderedList` (biscuit-terminal)
- `OrderedList` (biscuit-terminal)
- `TwoColumn` (biscuit-terminal)
- `Progress` (biscuit-terminal)
- `Table` (biscuit-terminal)
- `YamlBlock` (darkmatter)

## Problem Statement

Now that we've implemented the tree rendering architecture for 7 components we see a LOT of _pattern based_ problems that span the components. These represent critically incomplete parts of render tree implementation which will prevent us from adopting it. Yet, because these problems are so pattern based and we are still in the advanced design phase of the tree rendering architecture it gives us an opportunity to consider how we could potentially improve the design to handle the immediate problems we see as a part of the tree rendering design rather than passed down to the individual components to fix.

> **Note:** the more we can identify the "pattern" more than just the specific manifestation of the problem we see right now is to our benefit. The tree rendering solution will need to serve not just these seven components but many, many more over time so we should try to be as forward looking as possible.

## Patterns Recognized

1. Code Block Parsing

    The `YamlBlock` component renders the content inside of a code block using code highlighting. This is a
    feature we will need over and over again. Currently the tree renderer clumsily leaves the starting backticks and language line that starts a code block

2. Layouts Ignored

    It is clear that NONE of the components currently do anything with margins (and I'm guessing the same is true for alignment, padding, min-width, max-width, word-wrap, etc.).

    Being able to define a **layout** with margins, alignment, etc. and have the tree-rendering understand the layout primitive and enforce it's rules allows the component to focus only on the rendering it needs to do and not how it's rendering block will fit into the overall 



> **Note:** 
> 
> - one limitation in our visibility that we should consider is that we have not yet implemented a component like `Prose` (from biscuit-terminal) which is used A LOT and represents a stronger test of how we're able to handle _inline_ content then a lot of the other components which focus more on _block_ content.
> - I think after this feature we will explicitly implement `Prose` but anything we can do proactively to address the needs of inline mutation would be very helpful. 
> 
> - I am not that familiar with how a caller of the `pulldown-cmark` crate (which is used by Darkmatter) distinguishes between mutating a formal
