## Context

The following components have been implemented using the classic bespoke method AND the new tree rendering approach:

- `Section` (biscuit-terminal)
- `UnorderedList` (biscuit-terminal)
- `OrderedList` (biscuit-terminal)
- `TwoColumn` (biscuit-terminal)
- `Progress` (biscuit-terminal)
- `Table` (biscuit-terminal)
- `YamlBlock` (darkmatter)

## Drift Report

```txt
Render Drift Report
render-tree engine vs bespoke renderer — KNOWN_DRIFT ledger

  biscuit-terminal: 225 drift entries
    45 BlockQuote
    16 Progress
    57 Section
    16 Table
    39 TwoColumn
    52 UnorderedList

  darkmatter: 66 drift entries
    66 YamlBlock

  total: 291 drift entries
  exit condition: 0 entries in both crates
  ```

  > from `just drift-report` in renderable package area

## Problem Statement

Now that we've implemented the tree rendering architecture for 7 components we see a LOT of _pattern based_ problems that span the components. These represent critically incomplete parts of render tree implementation which will prevent us from adopting it. Yet, because these problems are so pattern based and we are still in the advanced design phase of the tree rendering architecture it gives us an opportunity to consider how we could potentially improve the design to handle the immediate problems we see as a part of the tree rendering design rather than passed down to the individual components to fix.

> **Note:** the more we can identify the "pattern" more than just the specific manifestation of the problem we see right now is to our benefit. The tree rendering solution will need to serve not just these seven components but many, many more over time so we should try to be as forward looking as possible.

## Patterns Recognized

1. Code Block Parsing

    The `YamlBlock` component renders the content inside of a code block using code highlighting. This is a
    feature we will need over and over again. Currently the tree renderer clumsily leaves the starting backticks (while removing the ending backticks). 

    > Note: as is common with a lot of extended Markdown DSL's Darkmatter needs to not only see the initial backticks followed by the language specified for the code block but will need to see the rest of that initial line so it can parse any other key/value pairs it defined.

2. Layouts Ignored

    It is clear that NONE of the components currently do anything with margins (and I'm guessing the same is true for alignment, padding, min-width, max-width, word-wrap, etc.).

    Having a "component" be able to define a **layout** with margins, alignment, etc. and not have to implement that themselves is important. 

    - this probably isn't defined in the Tree Rendered itself but as a struct that bridges between the component and the tree renderer.
    - the biggest complication in **layouts** is that each _output target_ will have a different set of presentation capabilities as well as a language and approach.
        - We could have a `LayoutEngine` per output but we should consider if there were a way to have a semantic interface that allows for translation across the output types (ignoring AST):
            - Markdown (more of a semantic markup then a presentational one so in theory it can be a "carrier" of any presentational data we want)
            - Browser (the most flexible)
            - Terminal (the most constrained)
        - Providing a "semantic" interface would further simplify the role of a component; if done well it could also simplify the cognitive overhead of a caller of the component (done poorly it might increase it). 

3. TBD


> **Note:** 
> 
> - one limitation in our visibility that we should consider is that we have not yet implemented a component like `Prose` (from biscuit-terminal) which is used A LOT and represents a stronger test of how we're able to handle _inline_ content then a lot of the other components which focus more on _block_ content.
> - I think after this feature we will explicitly implement `Prose` but anything we can do proactively to address the needs of inline mutation would be very helpful. 
> 
> - I am not familiar with how a caller of the `pulldown-cmark` crate (which is used by Darkmatter as the core parsing engine) distinguishes between mutating a formal inline construct like **bold facing** a text region versus a desired extension of Markdown's grammar (which is what Darkmatter is in large part)
