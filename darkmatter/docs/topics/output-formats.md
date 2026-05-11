# Output Formats

**Darkmatter** supports the following output formats:

- **Markdown**
    - This is plain Markdown content with no _rendering_ applied
    - This is a plain text file and should be readable in any text or Markdown viewer
- **Terminal**
    - The markdown content is rendered to a Terminal friendly environment
    - Uses colors themes based on dark/light mode (with manual overrides to a different theme)
    - Uses a variant color theme for code blocks so that the code blocks can stand out from the Markdown prose
    - No inline-HTML added during rendering
    - Supports the `⌄dim⌄` inline syntax extension, rendering dimmed text with ANSI SGR `2` when the terminal supports it
- **Enriched Markdown** 
    - Enriches the Markdown with more inline HTML during the rendering stage 
    - The additional inline HTML provide more features but makes the document less ergonomic to edit
    - Any Markdown viewer will be able to open this but possibly some features will not work with viewers that don't support inline HTML fully (most will render all the features we're adding).
- **HTML**
    - All Markdown content can be be converted to HTML
    - All styling is provided via CSS and we use CSS variables where we can (_see [CSS Variables](css-variables.md) document for more details_)
    - The CSS will be embedded inline into the page
    - In cases where some rendering features require [Javascript](./inline-javascript.md) to achieve their outcome, we will transparently add the required Javascript inline to the HTML as well.
- **AST**
    - We can export in the [MDAST](https://github.com/syntax-tree/mdast) AST format (which uses a JSON payload)

