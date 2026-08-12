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
last_updated: 2026-07-13
blast_radius:
  - darkmatter/lib/src/mermaid/mod.rs
  - darkmatter/lib/src/mermaid/theme.rs
  - darkmatter/lib/src/mermaid/feature.rs
  - darkmatter/lib/src/mermaid/render_terminal.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/output/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/render.rs
---
# Mermaid Rendering in Darkmatter

## Overview

Darkmatter recognizes fenced code blocks tagged with the `mermaid` language and routes them through a dedicated rendering path that is distinct from ordinary syntax-highlighted code. The core abstraction is the `Mermaid` struct in `darkmatter/lib/src/mermaid/mod.rs`, which captures the diagram source, an optional title and footer, and a theme resolution strategy (either an explicit light/dark `MermaidTheme` pair or a syntect `ThemePair` mapped through `mermaid_theme_for_syntect`).

Whether a mermaid block is rendered as a diagram or shown as plain code is governed by the `MermaidMode` enum exposed from `darkmatter/lib/src/markdown/output/terminal.rs` and re-exported through `darkmatter::markdown::output`:

| Variant         | Terminal                                                     | Browser (Darkmatter page path)                                       |
|-----------------|-------------------------------------------------------------|----------------------------------------------------------------------|
| `Off` (default) | Regular code fence with syntect syntax highlighting.        | **Interactive** `<pre class="mermaid">` + injected ESM bootstrap.    |
| `Image`         | Inline diagram image (opt-in; may launch a subprocess).     | Rendered static `<svg>` (an explicit static opt-in).                 |
| `Text`          | Diagram source as a fenced code block (fallback format).    | `<pre><code class="language-mermaid">` code block.                   |

`MermaidMode` is wired into both `TerminalOptions` and `HtmlOptions`, so the same configuration knob controls the behaviour of `write_terminal` / `as_html`, as well as the higher-level `DarkmatterPage::with_mermaid_mode` builder method (`darkmatter/lib/src/layout/page.rs`).

The **browser** column above is Darkmatter's page browser path, shared by both
`DarkmatterPage::render_to_browser` (a **body-only** HTML fragment — no
`<!DOCTYPE>`/`<html>`/`<head>`/`<body>` scaffold — for embedding a render into a
host document) and `DarkmatterPage::render_to_browser_document` (a complete
**standalone `<!DOCTYPE html>` document** with a real `<head>`, which the `md`
CLI uses for HTML output — see `darkmatter/cli/src/artifact.rs`). Both remap
`MermaidMode` onto `renderable`'s `BrowserMermaidMode` in
`render_tree/entrypoints.rs`: `Off → Interactive`, `Image → StaticSvg`,
`Text → Code`. The low-level `renderable` browser renderer keeps
`BrowserMermaidMode::Code` as its own default, and `Markdown::as_html` maps
`Off → Code` (it does not opt into the interactive default), so only the
Darkmatter page browser path is interactive by default. See
[Rendering to the Browser](#rendering-to-the-browser).

The mermaid module is intentionally thin: the actual terminal-side diagram rendering (pure-Rust pipeline via `biscuit-visualized`, terminal image display, and cached PNG artifacts) lives in `biscuit-terminal`. `darkmatter/lib/src/mermaid/render_terminal.rs` is a wrapper around `biscuit_terminal::components::mermaid::MermaidDiagram` and re-exports its `MermaidRenderError` for API compatibility.

## Rendering to the Terminal

Terminal rendering is driven from `darkmatter/lib/src/markdown/output/terminal.rs`. When the markdown parser emits the end of a code block whose language is `mermaid` (case-insensitive) and `TerminalOptions::mermaid_mode` is not `Off`, the renderer branches on the mode:

- **`MermaidMode::Text`** — Emits a code-block header row (using any `title` from the code-fence info string) followed by the diagram source rendered with syntect highlighting via the shared helpers in `darkmatter/lib/src/markdown/output/code_block.rs`. The output is byte-compatible with a normal `mermaid` code fence.
- **`MermaidMode::Image`** — Flushes the current line buffer, constructs a `Mermaid` value from the code-block body, and calls `Mermaid::render_for_terminal()`, which delegates to `biscuit_terminal::components::mermaid::MermaidDiagram::try_render`. The image is written directly to stdout. On success, a title-only header row is appended after the image (the language label is suppressed because the rendered diagram already conveys the type). On failure (no graphics support, render error, etc.) the renderer transparently falls back to the `Text` rendering path and logs a warning via `tracing::warn!`.

The `Mermaid` struct also computes accessibility-friendly alt text:

- If `with_title(...)` is set, the title is used verbatim.
- Otherwise `darkmatter::mermaid::detect_diagram_type` inspects the first non-empty line of the source and returns a human-readable description such as `"Flowchart diagram"`, `"Sequence diagram"`, `"Class diagram"`, `"State diagram"`, `"Entity relationship diagram"`, `"Pie chart"`, `"Gantt chart"`, `"User journey diagram"`, `"Git graph diagram"`, `"Mind map diagram"`, `"Timeline diagram"`, or the generic `"Mermaid diagram"` fallback.

Themes for terminal rendering are resolved through `mermaid_theme_for_syntect`, which maps the active `ThemePair` and `ColorMode` to one of the built-in `DEFAULT_LIGHT_THEME`, `DEFAULT_DARK_THEME`, or `NEUTRAL_THEME` presets exposed by `darkmatter/lib/src/mermaid/theme.rs`.

## Rendering to the Browser

Browser Mermaid is a page **feature**, resolved by `DarkmatterFeatureResolver`
(`darkmatter/lib/src/mermaid/feature.rs`) — the single owner of Mermaid browser
assets. When a `mermaid` fence is rendered through Darkmatter's page browser
path (`DarkmatterPage::render_to_browser` for a body-only fragment, or
`render_to_browser_document` for a standalone document), the render-tree browser
writer emits the interactive body element and requests
`PageFeature::MermaidDiagram`; the
resolver then supplies the shared assets, which are injected once per page
(deduplicated in first-seen order):

- **Body** — a `<pre class="mermaid">` element whose contents are the
  HTML-escaped diagram source, so a screen reader (and any reader whose browser
  cannot load the module) sees readable source.
- **Palette** — delivered through Mermaid's own `themeVariables` (baked into the
  bootstrap's `mermaid.initialize` call with `theme: 'base'`), never through CSS:
  Mermaid does not read CSS custom properties, so **no CSS block is injected**.
  A single palette is resolved for the document's color mode and fixed at init;
  there is no live `prefers-color-scheme` switch, because Mermaid's SVG colors
  are chosen once at init and would require a JS re-render to change.
- **Injected script** — an inline `<script type="module">` bootstrap that
  dynamically imports the exact `MERMAID_VERSION` (never a floating major tag)
  from `cdn.jsdelivr.net` (primary), retries the identical version from
  `unpkg.com` (fallback), and calls `mermaid.run({ querySelector: '.mermaid' })`
  so only `.mermaid` elements initialize. A total load failure logs one
  `console.error` and leaves the readable source visible.

The interactive experience is Darkmatter's page browser default. The
low-level `renderable` browser renderer keeps `BrowserMermaidMode::Code`, so a
caller constructing `BrowserRenderOptions::default()` (or using `Markdown::as_html`
without opting in) still renders Mermaid as a plain code block, and terminal
output stays code unless the caller opts into `MermaidMode::Image`.

Delivery requires network access at view time and a Content Security Policy that
permits both CDN origins and inline modules. No diagram source or document
metadata is sent anywhere except the browser's module fetch; Mermaid runs locally
in the browser.

## Icon Packs

Terminal image rendering enables the following Iconify icon packs, loaded on demand from `unpkg.com`:

- `@iconify-json/fa7-brands` — Font Awesome 7 brand icons
- `@iconify-json/lucide` — Lucide icons
- `@iconify-json/carbon` — Carbon Design icons
- `@iconify-json/system-uicons` — System UI icons

# CLI Switches

The following `md` CLI flags (defined under `darkmatter/cli/src/args/` and wired up in `darkmatter/cli/src/render.rs` and `darkmatter/cli/src/commands/`) affect mermaid rendering. Unless otherwise noted, they belong to the top-level `md [FILE]` render command.

| Flag                              | Applies to                | Behaviour                                                                                                                                                                                                                                                                            |
|-----------------------------------|---------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--mermaid`                       | Terminal                  | Sets `MermaidMode::Image` for terminal rendering. Without this flag mermaid blocks render as syntax-highlighted code (`MermaidMode::Off`). Falls back to a `Text`-style code block if the terminal does not support graphics or if rendering fails.                                  |
| `--output <FORMAT>`               | All targets               | Selects the output target (`auto`, `markdown` / `text`, `html`, `json` / `ast`). Only `auto` (TTY) and `html` exercise the mermaid renderers; `markdown` and `json` emit the source unchanged.                                                                                       |
| `--show`                          | All targets               | Writes the selected output to a temp file and opens it in the default app. With `--output html` this is the practical way to view rendered diagrams in a browser.                                                                                                                    |
| `--theme <NAME>`                  | All targets               | Selects the prose `ThemePair` whose `ColorMode` feeds `mermaid_theme_for_syntect`, indirectly setting the diagram palette.                                                                                                                                                           |
| `--code-theme <NAME>`             | All targets               | Selects the code `ThemePair` used for the `Text` fallback rendering and for the `Off` (default) path that treats mermaid blocks as syntax-highlighted code.                                                                                                                          |
| `--list-themes`                   | n/a                       | Lists available themes and exits without rendering.                                                                                                                                                                                                                                  |
| `md graph <FILE> --graph mermaid` | Mermaid *source emission* | Not a renderer switch, but worth flagging: this subcommand path (in `darkmatter/cli/src/commands/graph.rs`) prints a transclusion graph as raw mermaid flowchart source via `ReferenceGraph::to_mermaid`. The output is unrendered text suitable for piping into another mermaid renderer. |

There is currently no dedicated CLI switch to opt into `MermaidMode::Text` directly or to force `MermaidMode::Image` for HTML output — HTML rendering pulls its mode from the library default (`MermaidMode::Off`) unless callers construct `HtmlOptions` themselves.
