# Mermaid Rendering

A feature of the `Markdown` struct in this library is it's ability to render to both the terminal (using escape codes) and the browser (using HTML/CSS/JS).

## The Browser

When we encounter a fenced code block with the language set to `mermaid` we will ALWAYS render it when targeting the browser. This is relatively easily achieved as currently the only official way to render a mermaid diagram is to use the `mermaidjs` package on NPM.

So when we render to the browser we add some inline Javascript which points to the latest version of `mermaidjs` on a CDN.

### Theming

All of our rendering from Markdown to HTML uses the abstraction of CSS Variables and Mermaid is no different. Mermaid has the concept of "themes" but we target only the "base" theme but we do it with both a light and dark mode mapping.

The following CSS variables are used to theme Mermaid diagrams:

- `--mermaid-background`
- `--mermaid-primary-color`
- `--mermaid-secondary-color`
- `--mermaid-tertiary-color`
- `--mermaid-primary-border-color`
- `--mermaid-secondary-border-color`
- `--mermaid-tertiary-border-color`
- `--mermaid-primary-text-color`
- `--mermaid-secondary-text-color`
- `--mermaid-tertiary-text-color`
- `--mermaid-line-color`
- `--mermaid-text-color`
- `--mermaid-main-bkg`
- `--mermaid-node-border`

## The Terminal

The terminal, by default, does _not_ render mermaid diagram but instead is shown in a manner similar to the way any other fenced code blocks would be. However, use of the `--mermaid` switch changes that. When the `--mermaid` switch is used we:

- Validate the diagram size (must be less than 10KB to prevent passing excessively large content)
- Create a temporary input file with the diagram instructions
- Execute the `mmdc` CLI tool locally with dark theme and icon pack support (including Font Awesome 7 brand icons, Lucide icons, Carbon Design icons, and System UI icons)
- Display the output PNG image using `viuer` in the terminal
- Clean up all temporary files

> When terminal rendering fails or is not supported (e.g., the terminal doesn't support Kitty graphics or the `mmdc` CLI is not installed), we fall back to displaying the diagram as a standard fenced code block with the `mermaid` language identifier. This ensures the diagram content remains visible even when graphical rendering is unavailable.
