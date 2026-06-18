---
prompt: |-
    Look at the Darkmatter source code to understand how the Mermaid rendering functionality is handled today.

    Your task is to complete the research and write idiomatic, standards based (CommonMark + GFM) markdown content
    and save it to the BODY of this file.

    Use the following structure for the document:
    
    - `# Mermaid Rendering in Darkmatter`
        - `## Overview`
        - `## Rendering to the Terminal`
        - `## Rendering to the Browser`
    - `# CLI Switches`
        - list any CLI switches which might modify the behavior of rendering to any of the given targets

    If there are sections you feel should be added to this structure please add them. In addition to the `last_updated` frontmatter of this document, please also set the `blast_radius` property as a list of source code files which are 
    involved in the "mermaid" functionality.
last_updated: 2026-05-12
blast_radius:
  - darkmatter/lib/src/mermaid/mod.rs
  - darkmatter/lib/src/mermaid/theme.rs
  - darkmatter/lib/src/mermaid/render_html.rs
  - darkmatter/lib/src/mermaid/render_terminal.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/output/mod.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/output.rs
  - darkmatter/cli/src/commands.rs
---
# Mermaid Rendering in Darkmatter

## Overview

Darkmatter recognizes fenced code blocks tagged with the `mermaid` language and routes them through a dedicated rendering path that is distinct from ordinary syntax-highlighted code. The core abstraction is the `Mermaid` struct in `darkmatter/lib/src/mermaid/mod.rs`, which captures the diagram source, an optional title and footer, and a theme resolution strategy (either an explicit light/dark `MermaidTheme` pair or a syntect `ThemePair` mapped through `mermaid_theme_for_syntect`).

Whether a mermaid block is rendered as a diagram or shown as plain code is governed by the `MermaidMode` enum exposed from `darkmatter/lib/src/markdown/output/terminal.rs` and re-exported through `darkmatter::markdown::output`:

| Variant         | Behaviour                                                                                                                                      |
|-----------------|------------------------------------------------------------------------------------------------------------------------------------------------|
| `Off` (default) | Treat the block as a regular code fence — render it with syntect syntax highlighting.                                                          |
| `Image`         | Render as a real diagram: an inline image in the terminal, or an interactive `<pre class="mermaid">` element in the browser.                   |
| `Text`          | Render the diagram source as a fenced code block (terminal) or as `<pre><code class="language-mermaid">` (browser). Used as a fallback format. |

`MermaidMode` is wired into both `TerminalOptions` and `HtmlOptions`, so the same configuration knob controls the behaviour of `write_terminal` / `as_html`, as well as the higher-level `DarkmatterPage::with_mermaid_mode` builder method (`darkmatter/lib/src/layout/page.rs`).

The mermaid module is intentionally thin: the actual terminal-side diagram rendering (pure-Rust pipeline via `biscuit-visualized`, terminal image display, and cached PNG artifacts) lives in `biscuit-terminal`. `darkmatter/lib/src/mermaid/render_terminal.rs` is a wrapper around `biscuit_terminal::components::mermaid::MermaidDiagram` and re-exports its `MermaidRenderError` for API compatibility.

## Rendering to the Terminal

Terminal rendering is driven from `darkmatter/lib/src/markdown/output/terminal.rs`. When the markdown parser emits the end of a code block whose language is `mermaid` (case-insensitive) and `TerminalOptions::mermaid_mode` is not `Off`, the renderer branches on the mode:

- **`MermaidMode::Text`** — Emits a code-block header row (using any `title` from the code-fence info string) followed by the diagram source rendered with syntect highlighting via the shared helpers in `darkmatter/lib/src/markdown/output/code_block.rs`. The output is byte-compatible with a normal `mermaid` code fence.
- **`MermaidMode::Image`** — Flushes the current line buffer, constructs a `Mermaid` value from the code-block body, and calls `Mermaid::render_for_terminal()`, which delegates to `biscuit_terminal::components::mermaid::MermaidDiagram::try_render`. The image is written directly to stdout. On success, a title-only header row is appended after the image (the language label is suppressed because the rendered diagram already conveys the type). On failure (no graphics support, render error, etc.) the renderer transparently falls back to the `Text` rendering path and logs a warning via `tracing::warn!`.

The `Mermaid` struct also computes accessibility-friendly alt text:

- If `with_title(...)` is set, the title is used verbatim.
- Otherwise `render_html::detect_diagram_type` inspects the first non-empty line of the source and returns a human-readable description such as `"Flowchart diagram"`, `"Sequence diagram"`, `"Class diagram"`, `"State diagram"`, `"Entity relationship diagram"`, `"Pie chart"`, `"Gantt chart"`, `"User journey diagram"`, `"Git graph diagram"`, `"Mind map diagram"`, `"Timeline diagram"`, or the generic `"Mermaid diagram"` fallback.

Themes for terminal rendering are resolved through `mermaid_theme_for_syntect`, which maps the active `ThemePair` and `ColorMode` to one of the built-in `DEFAULT_LIGHT_THEME`, `DEFAULT_DARK_THEME`, or `NEUTRAL_THEME` presets exposed by `darkmatter/lib/src/mermaid/theme.rs`.

## Rendering to the Browser

Browser rendering is implemented in `darkmatter/lib/src/markdown/output/html.rs` and piggybacks on `Mermaid::render_for_html()` in `darkmatter/lib/src/mermaid/mod.rs`.

When a `mermaid` code block is encountered and `HtmlOptions::mermaid_mode` is not `Off`:

- **`MermaidMode::Image`** — Constructs a `Mermaid` from the block source (carrying any `title` from the code-fence info string), calls `render_for_html()`, and writes the resulting `MermaidHtml.body` into the document. The body is a `<pre class="mermaid" role="img" aria-label="…">` element whose contents are the HTML-escaped diagram source. The companion `MermaidHtml.head` content (mermaid.js v11 ESM import, icon-pack loader registration, and per-theme CSS variables) is the caller's responsibility to inject into `<head>`.
- **`MermaidMode::Text`** — Emits a plain `<pre><code class="language-mermaid">…</code></pre>` block with the diagram source HTML-escaped. This is the fallback format and does not include the mermaid.js loader.

Theme colours are emitted via `render_html::generate_css_variables`, which writes a `<style>` block that defines fourteen `--mermaid-*` CSS variables once for the default (light) palette and again inside a `@media (prefers-color-scheme: dark)` block. The runtime mermaid.js initialisation in the head uses the `base` theme and reads these variables, so light/dark switching is purely CSS-driven and respects the user agent's color-scheme preference.

The body of the rendered element carries ARIA attributes — `role="img"` and an `aria-label` populated from `Mermaid::alt_text()` — so screen readers receive a meaningful description even before mermaid.js has had a chance to swap the source for an SVG.

## Icon Packs

Both terminal and browser image rendering enable the same set of Iconify icon packs, loaded on demand from `unpkg.com`:

- `@iconify-json/fa7-brands` — Font Awesome 7 brand icons
- `@iconify-json/lucide` — Lucide icons
- `@iconify-json/carbon` — Carbon Design icons
- `@iconify-json/system-uicons` — System UI icons

# CLI Switches

The following `md` CLI flags (defined in `darkmatter/cli/src/args.rs` and wired up in `darkmatter/cli/src/output.rs` and `darkmatter/cli/src/commands.rs`) affect mermaid rendering. Unless otherwise noted, they belong to the top-level `md [FILE]` render command.

| Flag                              | Applies to                | Behaviour                                                                                                                                                                                                                                                                            |
|-----------------------------------|---------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--mermaid`                       | Terminal                  | Sets `MermaidMode::Image` for terminal rendering. Without this flag mermaid blocks render as syntax-highlighted code (`MermaidMode::Off`). Falls back to a `Text`-style code block if the terminal does not support graphics or if rendering fails.                                  |
| `--output <FORMAT>`               | All targets               | Selects the output target (`auto`, `markdown` / `text`, `html`, `json` / `ast`). Only `auto` (TTY) and `html` exercise the mermaid renderers; `markdown` and `json` emit the source unchanged.                                                                                       |
| `--show`                          | All targets               | Writes the selected output to a temp file and opens it in the default app. With `--output html` this is the practical way to view rendered diagrams in a browser.                                                                                                                    |
| `--theme <NAME>`                  | All targets               | Selects the prose `ThemePair` whose `ColorMode` feeds `mermaid_theme_for_syntect`, indirectly setting the diagram palette.                                                                                                                                                           |
| `--code-theme <NAME>`             | All targets               | Selects the code `ThemePair` used for the `Text` fallback rendering and for the `Off` (default) path that treats mermaid blocks as syntax-highlighted code.                                                                                                                          |
| `--list-themes`                   | n/a                       | Lists available themes and exits without rendering.                                                                                                                                                                                                                                  |
| `md graph <FILE> --graph mermaid` | Mermaid *source emission* | Not a renderer switch, but worth flagging: this subcommand path (in `darkmatter/cli/src/commands.rs`) prints a transclusion graph as raw mermaid flowchart source via `ReferenceGraph::to_mermaid`. The output is unrendered text suitable for piping into another mermaid renderer. |

There is currently no dedicated CLI switch to opt into `MermaidMode::Text` directly or to force `MermaidMode::Image` for HTML output — HTML rendering pulls its mode from the library default (`MermaidMode::Off`) unless callers construct `HtmlOptions` themselves.
