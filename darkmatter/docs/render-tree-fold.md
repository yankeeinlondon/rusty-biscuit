# Render-Tree Fold (Production)

> **Status:** production. The render-tree fold is darkmatter's **only** Markdown
> rendering path. `Markdown::as_html` / `as_terminal` and
> `DarkmatterPage::render` / `render_to_browser` build a complete
> `renderable::tree::Document` and run one target fold over it; the legacy
> event-stream HTML/terminal serializers have been deleted. See the
> [rendering pipeline](./darkmatter-rendering-pipeline.md).

## What it is

The [`renderable`](../../renderable/README.md) crate defines a canonical,
owned **render tree** — a single target-agnostic representation
(`renderable::tree::Document` / `RenderNode`) that sits between content
sources and render targets.

The `darkmatter::markdown::render_tree` module is darkmatter's home for the
**events → tree fold**: the conversion of a
[`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) 0.13 `Event`
stream into a `renderable::tree::Document`.

It contains two pieces:

- **`inventory`** — a verified, compile-checked catalog of every
  `pulldown-cmark` 0.13 `Event` / `Tag` / `TagEnd` variant and the disposition
  the fold applies to each. An exhaustive-match test fails to build if
  `pulldown-cmark` changes its enums.
- **`fold_markdown_to_document(source, input) -> (Document, Vec<Diagnostic>)`** —
  the fold entry point. It walks the event stream and builds a `Document`,
  returning any non-fatal diagnostics alongside it.

## Coverage

The fold covers the common Milestone 1 event set — headings, paragraphs,
block quotes, lists, code blocks, thematic breaks, tables, inline emphasis /
strong / strikethrough, links, images, and inline code — plus footnotes,
grouped raw-HTML blocks, and native superscript / subscript.

## Complete-tree construction

The bare `fold_markdown_to_document` produces a structural tree. Production
rendering goes through darkmatter's **context-aware** fold entry points
(`render_tree::build_context`, carrying a `TreeBuildContext` policy view), which
bake the full typed render input onto the nodes during construction:

- component policy (table, block quote, ordered/unordered list, list item, code,
  image, link, thematic break) applied as each container node closes;
- page-inheriting foreground attached to the root and propagated via
  `InheritedStyle` — page color is never copied onto component nodes;
- `StyleColor` lowered to alpha-bearing `renderable::style::PaintColor` at the
  parser/apply boundary;
- exact / max text layout attached to link/image/list nodes (without replacing
  children or alt text);
- structured link/image metadata parsed once into typed `NodeAttrs::browser`
  attrs (classes, target, `data-*`, validated inline CSS over frontmatter
  defaults);
- HR defaults / inline precedence resolved during construction.

The empty/default context keeps the unstyled path cheap. Each target then runs
**one fold** over the complete tree — there is no post-fold decoration pass.
`==mark==` / dim inline styles, horizontal rules with attribute blocks, and
frontmatter are all wired (frontmatter is attached to `Document` metadata above
the fold).
