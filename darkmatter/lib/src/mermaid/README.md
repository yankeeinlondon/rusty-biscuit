# Mermaid Rendering

Darkmatter provides a `Mermaid` struct for representing diagrams with theming and terminal delegation. Browser Mermaid delivery is owned by `DarkmatterFeatureResolver` (`feature.rs`), which the render-tree browser pipeline invokes through the shared `FeatureResolver` seam — client-side interactive Mermaid is injected as a page feature, not by a per-diagram HTML method.

## Module Structure

| File | Purpose |
|------|---------|
| `mod.rs` | `Mermaid` struct, builder API, `detect_diagram_type()`, `render_for_terminal()` |
| `theme.rs` | `MermaidTheme` struct, JSON parsing, built-in theme presets |
| `feature.rs` | `DarkmatterFeatureResolver`: the inline ESM bootstrap and its `themeVariables` palette — a script-only bundle, the single owner of Mermaid browser assets (no CSS; Mermaid does not read CSS custom properties) |
| `render_terminal.rs` | Thin wrapper delegating to `biscuit_terminal::components::mermaid` |

## Mermaid Struct

Builder-pattern API for creating themed diagrams:

- `Mermaid::new(source)` - create from diagram source
- `.with_title(title)` - set title (also used for alt text)
- `.with_footer(footer)` - set footer
- `.with_theme(light, dark)` - set custom light/dark themes
- `.use_syntect_theme(theme_pair)` - resolve theme from a syntect `ThemePair`
- `.hash()` - XXH64 hash of the normalized instructions
- `.alt_text()` - accessibility text (explicit title or auto-detected diagram type)
- `.render_for_terminal()` - delegates to `biscuit-terminal`

Browser output is produced by the render-tree pipeline, not by a method on
`Mermaid`: a `lang="mermaid"` fence rendered through Darkmatter's full-page
browser path emits an interactive `<pre class="mermaid">` element and the
`DarkmatterFeatureResolver` injects the shared CSS + ESM bootstrap once per page.

## MermaidTheme

The `MermaidTheme` struct (`theme.rs`) represents all color variables for a Mermaid theme. It supports JSON deserialization via `TryFrom<&str>`, `TryFrom<String>`, and `TryFrom<serde_json::Value>`, with camelCase field names.

Three built-in presets are provided as lazy statics:

- `DEFAULT_LIGHT_THEME` - soft pastels for light backgrounds
- `DEFAULT_DARK_THEME` - muted blue-gray for dark backgrounds
- `NEUTRAL_THEME` - high-contrast black/white for accessibility (WCAG 2.1 AA)

The `mermaid_theme_for_syntect(theme_pair, color_mode)` function maps a syntect `ThemePair` to the appropriate built-in theme. Currently it only differentiates by `ColorMode` (light/dark), but the signature supports future theme-specific customization.

## Browser Rendering

Browser Mermaid is a page **feature** resolved by `DarkmatterFeatureResolver`
(`feature.rs`) — the single owner of Mermaid browser assets:

- **Body**: the render-tree browser writer emits `<pre class="mermaid">` with the
  HTML-escaped diagram source and requests `PageFeature::MermaidDiagram`.
- **Injected assets** (once per page, deduplicated): CSS variables for light/dark
  mode via `prefers-color-scheme`, plus an inline `<script type="module">`
  bootstrap that dynamically imports the exact `MERMAID_VERSION` from
  `cdn.jsdelivr.net` (primary) and retries the identical version from
  `unpkg.com` (fallback), initializing only `.mermaid` elements. A total load
  failure logs one `console.error` and leaves the readable source visible.

Delivery requires network access and a Content Security Policy permitting both
CDN origins and inline modules. Interactive Mermaid is Darkmatter's full-page
browser default; the low-level `renderable` browser renderer keeps
`BrowserMermaidMode::Code`, and terminal output stays code by default.

### CSS Variables

All rendering uses the "base" theme with 14 custom CSS variable mappings:

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

## Terminal Rendering

Terminal rendering is delegated to `biscuit-terminal`. The `render_terminal.rs` module is a thin wrapper over `biscuit_terminal::components::mermaid::MermaidDiagram::try_render()`. All implementation details (pure-Rust diagram rendering via `biscuit-visualized`, terminal image display, and cached PNG artifacts) live in `biscuit-terminal`.

This module re-exports `MermaidRenderError` from `biscuit-terminal` for API compatibility and provides fallback helpers:

- `fallback_code_block(instructions)` - returns instructions as a fenced mermaid code block string
- `render_fallback_code_block(instructions)` - prints the fallback code block to stdout

## Icon Packs

Terminal rendering enables these icon packs (loaded from unpkg CDN):

- `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
- `@iconify-json/lucide` - Lucide icons
- `@iconify-json/carbon` - Carbon Design icons
- `@iconify-json/system-uicons` - System UI icons
