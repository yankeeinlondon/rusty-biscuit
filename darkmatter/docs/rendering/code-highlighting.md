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

### Single-variant themes

A few pairs are **single-variant by design** — they resolve to one theme and
ignore the mode entirely:

| ThemePair | Light | Dark |
|-----------|-------|------|
| `dracula` | Dracula | Dracula |
| `nord` | Nord | Nord |
| `monokai` | Monokai | Monokai |
| `vs-dark` | VS Dark | VS Dark |

For these, mode resolution (and the inversion below) is a deliberate **no-op**:
they have no opposite variant, so they cannot lift contrast. This is documented
behavior, not a bug.

## Code blocks contrast against the page (terminal and HTML)

A code block resolves its theme *variant* against the **inverted** color mode —
a *light* code panel in a dark page, and a *dark* panel in a light page. The
contrast is what lifts and separates the code panel from surrounding prose.
Prose, headings, tables, and the page background keep the real (un-inverted)
mode so body text stays readable. This applies to **code blocks only** and to
**both terminal and HTML** output, so the two targets agree.

The inversion is performed with [`ColorMode::inverted()`] at the point each code
highlighter is constructed: the bespoke terminal renderer, the render-tree
`CodeRenderer`, `YamlBlock`'s terminal path, the HTML renderer (`as_html`), and
`YamlBlock`'s browser path. Theme *selection* uses the
inverted mode; the panel's *internal* contrast decisions — the header-pill text
color and the highlighted-line background math — key off the **resolved theme
background** via `code_block::mode_for_background`, not the requested mode. That
keeps a single-variant dark theme's chrome readable (light header text on its
dark panel) even though its background never inverts.

```rust
use darkmatter::markdown::highlighting::ColorMode;

assert_eq!(ColorMode::Dark.inverted(), ColorMode::Light);
// Dark page + `github` code theme  -> GithubLight panel (light on dark page)
// Light page + `github` code theme -> GithubDark panel  (dark on light page)
// Dark page + `dracula` code theme -> Dracula (single-variant; no contrast lift)
```

## HTML inverts too (cross-target parity)

HTML/browser code blocks invert exactly as the terminal does. The terminal
detects the live light/dark mode; for HTML the `HtmlOptions::color_mode` is the
caller-declared page mode. In both cases the *code* theme resolves against the
inverted mode, so a dark page emits a light code panel and a Markdown ` ```yaml `
fence renders byte-identically to a `YamlBlock`. (Resolved in
`renderable/fixes/2026-05-22-darkmatter-failures/spec.md`, defect D.)

## Where this lives

- `darkmatter/lib/src/markdown/highlighting/themes.rs` — `ThemePair`,
  `ColorMode`, `ColorMode::inverted`, `ThemePair::resolve`, the `Theme` enum, and
  the `two-face` embedded-theme mapping.
- `darkmatter/lib/src/markdown/highlighting/mod.rs` — `CodeHighlighter`.
- `darkmatter/lib/src/markdown/output/code_block.rs` — shared terminal/HTML
  code-block rendering, `mode_for_background`, padding behavior.
- Construction sites that apply the terminal inversion:
  `output/terminal.rs`, `markdown/render_tree/code_renderer.rs`,
  `markdown/yaml_block.rs`.

[`ThemePair`]: ../../lib/src/markdown/highlighting/themes.rs
[`ColorMode`]: ../../lib/src/markdown/highlighting/themes.rs
[`ColorMode::inverted()`]: ../../lib/src/markdown/highlighting/themes.rs
