# Mermaid Rendering

Darkmatter provides a `Mermaid` struct for representing diagrams with theming, then renders them to HTML (client-side via mermaid.js) or delegates terminal rendering to `biscuit-terminal`.

## Module Structure

| File | Purpose |
|------|---------|
| `mod.rs` | `Mermaid` struct, builder API, `render_for_html()`, `render_for_terminal()` |
| `theme.rs` | `MermaidTheme` struct, JSON parsing, built-in theme presets |
| `render_html.rs` | `MermaidHtml` output struct, CSS variable generation, diagram type detection |
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
- `.render_for_html()` - returns `MermaidHtml { head, body }`
- `.render_for_terminal()` - delegates to `biscuit-terminal`

## MermaidTheme

The `MermaidTheme` struct (`theme.rs`) represents all color variables for a Mermaid theme. It supports JSON deserialization via `TryFrom<&str>`, `TryFrom<String>`, and `TryFrom<serde_json::Value>`, with camelCase field names.

Three built-in presets are provided as lazy statics:

- `DEFAULT_LIGHT_THEME` - soft pastels for light backgrounds
- `DEFAULT_DARK_THEME` - muted blue-gray for dark backgrounds
- `NEUTRAL_THEME` - high-contrast black/white for accessibility (WCAG 2.1 AA)

The `mermaid_theme_for_syntect(theme_pair, color_mode)` function maps a syntect `ThemePair` to the appropriate built-in theme. Currently it only differentiates by `ColorMode` (light/dark), but the signature supports future theme-specific customization.

## Browser Rendering

When targeting the browser, `render_for_html()` produces a `MermaidHtml` with separate `head` and `body` content:

- **Head**: mermaid.js v11 ESM import from CDN (`cdn.jsdelivr.net`), icon pack registration, CSS variables for light/dark mode via `prefers-color-scheme`
- **Body**: `<pre class="mermaid">` element with HTML-escaped instructions, ARIA attributes (`role="img"`, `aria-label`), optional `title` attribute

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

Terminal rendering is delegated to `biscuit-terminal`. The `render_terminal.rs` module is a thin wrapper that creates a `biscuit_terminal::components::mermaid::MermaidRenderer` and calls its `render_for_terminal()` method. All implementation details (mmdc CLI execution, 10KB size validation, viuer display, caching via `MermaidCache`, npx fallback chain) live in `biscuit-terminal`.

This module re-exports `MermaidRenderError` from `biscuit-terminal` for API compatibility and provides fallback helpers:

- `fallback_code_block(instructions)` - returns instructions as a fenced mermaid code block string
- `render_fallback_code_block(instructions)` - prints the fallback code block to stdout

## Icon Packs

Both HTML and terminal rendering enable these icon packs (loaded from unpkg CDN):

- `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
- `@iconify-json/lucide` - Lucide icons
- `@iconify-json/carbon` - Carbon Design icons
- `@iconify-json/system-uicons` - System UI icons
