# Code Block DSL

In the document [Handling Code Block Meta](./handling-code-block-meta.md) we explore how to extract and parse content _after_ the language token on the first line of a fenced code block.

In this document we will discuss what the `Markdown` struct will be responsible for parsing. This is, in effect, a small DSL for formatting our code blocks.

## Adding a Title

> **Syntax:** title="Hello World"


- The text assigned to `title` will be added BEFORE the fenced code block.
- The title line will have a blank line _before_ and _after_ it
- It will be styled by:
    - Terminal: 
        - all text on this line is BOLD
        - first two character on the line are: `▌ `
        - followed by the title text
    - HTML
        - The title will be given a class name of `.code-block-title`
        - The embedded CSS will define this class to have 


## Line Numbering

> **Syntax:** line-numbering=true

You can add line numbering to code blocks by setting `line-numbering` to true.

- for details on HOW to implement this with this tech stack, refer to: [Line Numbering](./line-numbering.md).

## Line Highlighting

> **Syntax:**
> 
>       highlight=4
>       highlight=4-6
>       highlight=4-6,8

The **highlight** feature allows you to highlight a particular row (or rows) in the code block. The value for `highlight` can be:

- a single number (indicating a single line should be highlighted)
- a range (e.g., `4-6`) which indicates a _range_ of line numbers which need to be highlighted
- a _list_ of single numbers or ranges:
    - `4-6,8`, `1,4,8`, and `1-3,8-10` are all valid representations for highlighting
    - the `,` character acts as a delimiter between groups
    - each member in a group is either a single number or a range of numbers

All code blocks start with line number 1.
