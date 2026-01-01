---
doc: true
---
# Markdown Formatting for the Terminal

![Terminal](./terminal.svg)

## `mdcat` but better

Utilities like **mdcat** do a decent job of using themes to colorize the output Markdown documents but we're aiming to go further!

The main differentiator of how this library will render Markdown content for the terminal is:

1. Frontmatter
 
    We allow you to conditionally show or hide the frontmatter and if it's shown we use YAML code highlighting on it.

2. Fenced Code Blocks

      We will find fenced code blocks inside the Markdown files and highlight them and use an "inverted theme" to make it stand out from the rest of the page.

3. Better Heading Renders

    `mdcat` provides a different color and a _prefix_ of `┄┄` for headings. This is ok but we style differently unless you use the `--basic-headers` in the CLI (in which case we mimic the `mdcat` rendering):

    - **H1** lines will be boldfaced, color coded (based on theme), and the line immediately following will be 

4. Better Page Breaks

    Page breaks don't look terrible with `mdcat` and they do provide a clear separator. We will use exactly the same rendering if the terminal being used doesn't support the **Kitty** graphics format but if it does then we will instead use an image separator.

5. Better Image Handling
6. Inline Code Rendering

    Use of the backticks/grave marker around a word or set of words is an example of setting an _inline_ code block. We've all become accustomed to seeing this text look like a "badge" where the background color of the text inside the backticks is a different color helping to distinguish it from the rest of the page. With `mdcat` you get a different color to distinguish it but the look and feel are pretty foreign to what you're used to seeing.

----

## Technical Approach

The primary crates we are using to work in the terminal are:

- `syntect` - helps with code highlighting
- `pulldown-cmark-mdcat` - a Markdown pull parser which extracts an set of events from the 
- `terminal_width` for modern, cross-platform validation of the amount of characters the terminal has.
- and `


### Themes and Grammars

- the **Textmate** editor is the **OG** for a lot of the editor standards we take for granted today.
- included in it's list of accomplishments is the grammar and theme styling it introduced
- **Sublime Text** took the pre-VSCode world by storm and started life with almost identical grammar and theme approach.
- Over time the two approaches have both evolved independently so today there is more difference then there was to start.
- The `syntect` supports:
    - Grammar:
      - `.sublime-syntax` (Sublime Grammar) and 
      - `.tmlanguage` (Textmate Grammar) directly
    - Theme:
      - `.tmTheme` is supported (Textmate theme file format)
      - `.sublime-color-scheme` -- a modern JSON format for Sublime Text -- is **NOT** supported.

    > If you have Sublime theme you will need to convert it back to a `.tmTheme` file for `syntect` to use it. You can use **PackageDev** (Sublime Text package) to convert grammars between formats




