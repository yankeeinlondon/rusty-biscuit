# Code Block Transclusion

The **Code Block** transclusion functionality is strongly tied to the [block transclusion](./block-transclusion.md) functionality but is a distinct edge case and has it's own syntax to boot.

In the _render_ cycle of Markdown processing the code blocks in Markdown are parsed and highlighted for both Terminal and HTML outputs. This is an important feature for the human legibility of these code blocks but adds nothing but noise to a workflow who's target consumer is an LLM. In all cases, however, the transclusion work takes place BEFORE any code block parsing will ever be done and therefore it does not need to be a design consideration when building this feature.

What distinguishes it from processing which the `::file ./content.md` based syntax uses is:

- Content Types:
    - While code blocks will only accept "text" based documents they do not have the same narrow restrictions as file blocks
    - The LSP (not implemented yet) will provide the most common file extensions for autocomplete but when the pipelining is kicked off only a non-text based file will be rejected.
- Code Block Wrapping:
    - The file we're referencing is a source code file so it needs be surrounded by the triple tick Markdown marker for a code block
    - We also need to attempt to identify the Language that the code block is:
        - matching will be done with the file's extension for now
        - we should be able to leverage either the `syntect` or `two-face` crates to help with this
        - if we are unsure of the language then we will fall back to using the `txt` language specifier
- Vertical Spacing
    - All code blocks in Markdown should have one (and no more) blank row above and below the code block to separate it from the rest of the prose content.

## Syntax

Transcluded code blocks are defined with:

```md
## My Section

::code ./mod.rs
```

The `::code <filename>` command behaves identically to `::file <filename>` in terms of file referencing strategy and it too provides exactly the same options as is defined in [block transclusion](./block-transclusion.md), including the `disclosure` option which wraps the transcluded source in a render-time disclosure block.


