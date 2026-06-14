# Code Highlighting

Darkmatter syntax-highlights fenced code blocks (and `YamlBlock`) with
[`syntect`](https://github.com/trishume/syntect) and bat-curated themes from
[`two-face`](https://github.com/CosmicHorrorDev/two-face). Highlighting is the
one part of darkmatter rendering that is **theme- and color-mode-driven**.

## `ThemePair` — a mode-agnostic theme name

The user-facing theme is a [`ThemePair`] — an **abstract name** (`github`,
`one-half`, `dracula`, …), *not* a concrete light or dark theme. The concrete
theme is chosen at render time by resolving the pair against a [`ColorMode`]:

```text
ThemePair::resolve(ColorMode) -> Theme
  (Github,  Dark)  -> GithubDark
  (Github,  Light) -> GithubLight
  (OneHalf, Dark)  -> OneHalfDark
  ...
```

> **Do not conflate the name with a concrete theme.** `--code-theme github`
> does not mean "the dark GitHub theme"; it means "GitHub, resolved for the
> current mode."

### Pairs that share a light theme

A `ThemePair` is a (light theme, dark theme) couple; `resolve` returns one or the
other for the requested mode. The two slots are always distinct themes. Several
pairs use the same theme in their light slot:

| ThemePair | Light | Dark |
|-----------|-------|------|
| `dracula` | One Half Light | Dracula |
| `nord` | One Half Light | Nord |
| `monokai` | One Half Light | Monokai Extended |
| `vs-dark` | GitHub Light | VS Dark |

Mode resolution and the inversion below are never a no-op: a dark page yields the
pair's light theme and a light page its dark theme. The two modes therefore
produce different output for the same `ThemePair`. (Because `dracula`, `nord`,
and `monokai` share One Half Light, their light panels are identical to each
other.)

## Code blocks contrast against the page

By **default**, a code block resolves its theme *variant* against the
**inverted** color mode — a *light* code panel in a dark page, and a *dark*
panel in a light page. The contrast is what lifts and separates the code panel
from surrounding prose. Prose, headings, tables, and the page background keep the
real (un-inverted) mode so body text stays readable. The variant is chosen from
the **terminal's** color mode (the same source the page uses), so the panel stays
consistent regardless of environment color detection.

### `--code-block` / `CodeBlockMode`

The inversion is the default, but is configurable per render via the global
`--code-block <inverse|dark|light|same>` flag — backed by the
[`CodeBlockMode`] enum and `DarkmatterPage::with_code_block_mode(...)`:

| Mode | Code-block variant |
|------|--------------------|
| `inverse` (default) | opposite of the terminal — light panel on dark, dark panel on light |
| `dark` | always the dark variant |
| `light` | always the light variant |
| `same` | match the terminal's own mode |

`CodeBlockMode::resolve(page_mode)` produces the concrete [`ColorMode`] the code
highlighter resolves the theme against (`inverse` → `page_mode.inverted()`).
The terminal render-tree path threads the mode through `TerminalCodeRenderer`
and builds the highlighter with `CodeHighlighter::new(theme, resolved_mode)`.
The HTML/browser path honors the mode too: it travels on
`HtmlOptions::code_block_mode` (set from `DarkmatterPage::with_code_block_mode`),
and `render_browser_code` resolves the panel variant against it through the same
`ThemePair::resolve_for_surface` boundary the terminal uses.

Theme *selection* uses the resolved (by default inverted) mode; the panel's
*internal* contrast decisions — the header-pill text color and the
highlighted-line background math — key off the **resolved theme background** via
`code_block::mode_for_background`, not the requested mode. That keeps the panel
chrome readable whichever of a pair's two themes is shown — e.g. light header
text when a pair's dark theme lands on a light page.

```rust
use darkmatter::markdown::highlighting::ColorMode;

assert_eq!(ColorMode::Dark.inverted(), ColorMode::Light);
// Dark page + `github` code theme  -> GithubLight panel  (light on dark page)
// Light page + `github` code theme -> GithubDark panel   (dark on light page)
// Dark page + `dracula` code theme -> OneHalfLight panel (dracula's light theme)
// Light page + `dracula` code theme -> Dracula panel     (dracula's dark theme)
```

## HTML inverts too (cross-target parity)

HTML/browser code blocks invert exactly as the terminal does. The terminal
detects the live light/dark mode; for HTML the `HtmlOptions::color_mode` is the
caller-declared page mode. In both cases the *code* theme resolves against the
mode produced by `CodeBlockMode::resolve(page_mode)` (the inverse by default), so
a dark page emits a light code panel and a Markdown ` ```yaml ` fence renders
byte-identically to a `YamlBlock`. (Resolved in
`renderable/fixes/2026-05-22-darkmatter-failures/spec.md`, defect D.)

The browser path renders the highlighted markup from the page's resolved
`HtmlOptions` — the same options that build the injected `.code-block` panel
background — so markup and stylesheet always agree on theme and variant
(review-1 finding 2). The `--code-block` override and
`DarkmatterPage::with_code_block_mode` therefore apply to **both** terminal and
browser. A `CodeBlock::with_theme(theme)` / `md code-block --theme` override
wins over the page/context theme on both surfaces.

## Where this lives

- `darkmatter/lib/src/markdown/highlighting/themes.rs` — `ThemePair`,
  `ColorMode`, `ColorMode::inverted`, `ThemePair::resolve`, the `Theme` enum,
  `CodeBlockMode` (+ `resolve`), and the `two-face` embedded-theme mapping.
- `darkmatter/lib/src/markdown/highlighting/mod.rs` — `CodeHighlighter`.
- `darkmatter/lib/src/markdown/output/code_block.rs` — shared terminal/HTML
  code-block rendering, `mode_for_background`, padding behavior.
- `darkmatter/lib/src/markdown/render_tree/code_renderer.rs` —
  `TerminalCodeRenderer`, which threads `CodeBlockMode` and resolves the
  terminal code-block variant via `CodeHighlighter::new`.
- `darkmatter/cli/src/args.rs` — the global `--code-block` flag.
- HTML/browser inversion site: `markdown/render_tree/code_renderer.rs`
  (`render_browser_code`) and the `as_html` path.

[`ThemePair`]: ../../lib/src/markdown/highlighting/themes.rs
[`ColorMode`]: ../../lib/src/markdown/highlighting/themes.rs
[`ColorMode::inverted()`]: ../../lib/src/markdown/highlighting/themes.rs
[`CodeBlockMode`]: ../../lib/src/markdown/highlighting/themes.rs
