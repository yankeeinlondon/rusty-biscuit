# HTML Rendering

In the shared library of this monorepo we have the `Markdown` struct which provides a Markdown rendering pipeline to both Terminal Output and HTML. So far the terminal has gotten more attention but the HTML output/rendering needs an equal amount of attention and because of it's much larger capability it will require a full design specification to articulate what we expect the output of this rendering pipeline to look like.

**Note:** while the core functionality for this Markdown pipelining is in the shared library it is "actualized" and often tested via the **md** CLI. When we use this CLI and provide the `--html` flag the output rendered is no longer the terminal but instead an HTML document.

## High Level Functional View

The high level view is that we want to be sure that we are able to render the following elements from the Markdown document:

- **Markdown Prose**
    - Markdown prose content is styled with the `syntect`/`two-face` theme pair; unlink with the terminal we will want to provide the theme colors for both light and dark mode using CSS variables so that we can switch between these two modes
- **Fenced Code Blocks**
    - The fenced code blocks in the Markdown use a different theme then the Markdown prose; here again we will want to identify the code block's theme pair and bring in both light and dark mode as CSS variables
- **Mermaid Diagrams**
    - Some markdown content will include [Mermaid](https://mermaid.js.org) charts. Mermaid charts consist of two parts:
        - In the `<head>` section of the HTML we will want to load the Mermaid JS library from a CDN (ideally pointing to the latest version always). We will also want to load in the CSS we need for light and dark themes (themes should be defined with a CSS variable based abstraction for light/dark). In the case of Mermaid, the "base" theme is the only theme you're apparently able to modify so we'd want to map that theme to the CSS variables (which will then automatically switch between light/dark mode)
            - **NOTE:** we will ONLY load this head section when the underlying Markdown has at least one Mermaid code block.
        - The other section or sections are the actual code blocks within the `<body>` of the page. Each one should be converted from it's Markdown code block expression to something which looks like:

            ```html
            <pre class="">{{instructions}}</pre>
            ```

        - if you are planning for or implementing Mermaid diagram you should be using the `mermaid` skill and you MUST read the following documents for context:
            - [Theming with Mermaid](@shared/docs/md/mermaid-theming.md)
            - [Mermaid]

- **Images**
    - The markdown may have a combination of Markdown-native image links and inline-HTML links. Both should be shown in the HTML rendering.
- **Links**
    - The markdown is bound to have links to other resources in it's corpus and we need to treat them in a nuanced manner:
        - Markdown links to external resources are fairly straight forward, they simply need to be converted into
