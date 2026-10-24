# Darkmatter Rendering

Use this reference for the render tree, style lowering, disclosure blocks,
code blocks, and browser safety.

## Contents

- [Render pipeline](#render-pipeline)
- [Style authority](#style-authority)
- [Code blocks](#code-blocks)
- [Disclosure blocks](#disclosure-blocks)
- [Embedded Mermaid font](#embedded-mermaid-font)
- [Browser contract](#browser-contract)

## Render pipeline

Public terminal and browser paths fold Markdown into one
`renderable::Document`, then fold that document to the target. The tree build
context applies component policy, inherited color, alpha paint, link/image
attributes, text layout, and horizontal-rule defaults during construction.

Do not reintroduce a post-fold decoration or HTML-rewrite pass. A target fold
must receive enough typed layout and style data to render correctly in one pass.

Plain `md compose` Markdown output removes paired embedded render-tree metadata
with `render_tree::embedded::without_embedded_metadata`, retaining the fallback
and surrounding source formatting. Keep the markers in the in-memory composed
document for terminal/browser folds. Link normalization includes local `file:`
URLs in the fallback; the encoded subtree retains absolute hyperlink targets.
`::file-links` root headers are unlinked context labels without a leading slash,
ending in `/`. Only the actual repository root gets the repository icon. Keep
the filesystem's automatic Nerd Font icon payloads through composition so
terminal glyph selection happens at render time; fallback Markdown stays Unicode.

`DarkmatterPage` is a viewport assembler. It owns page width, margin, padding,
background, max-width centering, browser wrapper metadata, and stylesheet
assembly. It cannot inspect component kinds or rewrite their content.

## Style authority

`style:` frontmatter lowers directly to per-component `ComponentPolicy` using
`renderable::layout::Layout` and alpha-bearing `PaintColor`. Parsed
`StyleColor` must not survive beyond the parser/apply boundary.

`CliStyleClaims` records only flags explicitly supplied by the user. Merge
claims with frontmatter as presentation policy; do not let CLI defaults
silently override authored styles.

Kebab-case keys are canonical. Supported snake-case compatibility aliases emit
deprecation warnings and become errors in strict-style mode. Width and
max-width are mutually exclusive where the component contract says so.

## Code blocks

`CodeBlock` is the primary component; its implementation is in
`darkmatter/lib/src/markdown/code_block.rs`. `YamlBlock` is a deprecated
compatibility wrapper that retains validation constructors.

Code-block mode defaults to inverse page color for contrast and supports
`inverse`, `dark`, `light`, and `same`. Terminal and browser paths must use the
same resolved mode. A direct component theme or `md code-block --theme` wins
over page/context defaults.

`CodeBlockMeta::start_line` (`CodeBlock::with_start_line`) offsets gutter
numbers and highlight lookup for excerpts; `None` numbers from 1. It has no
fence key, so `CodeBlock::code_node` carries it across the render tree as the
`darkmatter.code.start_line` extension hint, which `TerminalCodeRenderer`
reads only when the node's `data` bag is non-empty (keeping the
`structural_gate` zero-access check green for ordinary fences).

The CLI command is:

```text
md code-block <file-or-content> --language LANG --output terminal|html|markdown
```

It constructs a `CodeBlock` directly rather than routing through a Markdown
document fold.

## Disclosure blocks

The syntax is:

```md
::disclosure Summary
::details
Body content.
::end-disclosure
```

The summary is phrasing content; the body accepts block Markdown and nested
disclosures. `::file` and `::code` may wrap transcluded content with a
`disclosure` option.

Targets preserve one semantic node:

| Target | Projection |
|---|---|
| Terminal | Summary plus a dim/italic block-quote body |
| Markdown dialect | Original directive form |
| MarkdownPlus | Native HTML `<details>`/`<summary>`; the summary is written as HTML (raw HTML is not parsed as Markdown), the body as Markdown; raw HTML in the summary is byte-identical, or lossy when it holds a blank line |
| Browser | Native `<details>`/`<summary>`, no JavaScript |
| JSON | `NodeKind::Disclosure` |

Malformed directives produce a ranged `MarkdownError::MalformedDisclosure`.
Inline style tokens override `style.disclosure.*`; unknown tokens remain part
of the summary rather than disappearing.

## Embedded Mermaid font

Full doc: `darkmatter/docs/rendering/mermaid.md#embedded-font`.

- `HtmlOptions::embed_mermaid_font` / `DarkmatterPage::with_embed_mermaid_font`
  (default `false`; MarkdownPlus keeps it off). Acts only with
  `MermaidMode::Image`. No frontmatter key. `md --feature
  embed-mermaid-font` (`args::Feature`, lowered by
  `render::apply_features` inside `artifact::html_page`) sets it on every
  HTML page `md` builds (`render`, implicit render, `compose --output html`);
  `md`'s HTML is never static SVG today, so the output is unchanged.
- `code_renderer.rs` renders each static diagram with
  `with_embedded_font(true)` + `render_to_svg_with_font` and collects its
  `DiagramFont`; the first writes `PAGE_FONT_PLACEHOLDER` before its SVG.
  Both HTML entry points replace it via `render_tree/mermaid_font.rs`
  (`page_font_markup`): comments + `<style>` in the body before the first
  diagram, or nothing on `Err` or a failed `style_sheet_is_safe` (warning).
- `svg_sanitizer.rs::style_sheet_is_safe` (cssparser tokenizer) admits
  `data:` only as `url(data:font/ttf;base64,…) format("truetype")` in a
  top-level `@font-face` `src`; attributes keep `css_is_safe` (no `data:`).
- Tests: `tests/l1/mermaid_embedded_font.rs`, sanitizer unit tests,
  `tests/browser/browser_render.rs::browser_embedded_mermaid_font_draws_every_label`.
- On macOS, the `coretext-small-bundle` nextest setup script
  (`.config/nextest.toml`) selects Darkmatter tests **by name**: any test
  whose path contains `mermaid` or `merman`, apart from tier-marked ones. A
  test that renders a diagram over host fonts must say so in its name, or it
  pays CoreText's 1-10 s scan of `target/debug/deps` (`os` skill, `macos.md`).

## Browser contract

- Run browser tests headlessly and never activate a visible window.
- Dispatch keyboard/pointer/media changes through browser automation, not
  `osascript`, `cliclick`, `xdotool`, or Windows `SendInput`.
- Assert computed styles, DOM/accessibility state, or used geometry.
- Lower alpha directly to CSS `rgba(...)`; do not patch serialized HTML.
- Preserve structured link and image attributes and apply URL policy before
  rendering or fetching.
- Validate malformed code directives before HTML output so browser rendering
  cannot soften a fatal Markdown error.
- Raw HTML is escaped (`RawHtmlPolicy::Escape`) in HTML and printed with a
  diagnostic (Warn) in the terminal, except a comment-only `Html` node
  (`renderable::tree::is_html_comment_only`), which shows nothing in either:
  it carries the `` `a`<!-- -->`b` `` separator renderable's Markdown writer
  puts between touching code spans. The fold keeps the node, so Markdown
  output keeps the comment. Tests: `tests/l1/serialized_code_neighbors.rs`.
