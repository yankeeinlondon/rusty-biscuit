# Rusty Biscuit Components

Renderable components across the workspace and their supported render targets.

Target support reflects trait implementations:

- `TerminalRenderable` (terminal), `BrowserRenderable` (browser), and
  `MarkdownRenderable` (direct Markdown output).
- `TreeRenderable` is the multi-target path: a component that implements it
  produces a canonical [`RenderNode`](../src/tree/node.rs) tree, which the
  Markdown, Browser, and Terminal **tree renderers** can each fold into
  output. A `TreeRenderable` component therefore reaches every tree-backed
  target without implementing a per-target trait. Fourteen components
  currently project to the render tree: `BlockQuote`, `Compose`,
  `FileSystem`, `OrderedList`, `UnorderedList`, `Progress`, `Prose`,
  `Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`, and `TwoColumn`
  from `biscuit-terminal`, plus darkmatter's `YamlBlock`. See
  [`renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/lessons-learned.md)
  for the per-component migration notes and
  [`stage3-spec.md`](../features/2026-05-19-pushing-toward-ir/stage3-spec.md)
  for the work that completes structural projection.

IR state values:

- `no changes` means the component still uses its pre-tree rendering path only.
- `both avail, old renders` means a tree projection exists, but the default
  component render path still uses the bespoke renderer.
- `both avail, tree renders` means both paths exist and the default component
  render path uses the tree renderer.
- `tree render only` means the old bespoke render path has been removed and
  rendering now goes through the IR renderer only.

  > **FileSystem caveat.** `FileSystem`'s tree projection covers Browser and
  > Markdown, but its **terminal** `render` still calls the bespoke
  > directory-tree renderer. Tree-cutover Phase 4 closed the connector-list
  > `Style` lowering gap (per-item color/dim/italic/bold now reach the terminal
  > tree path — see `render_tree_connector_list`), so styling is at parity. The
  > terminal flip remains deferred for one remaining capability: the
  > target-agnostic tree projection emits **Unicode** icon glyphs (`📂`/`📄`
  > with a separating space), so it cannot reproduce the bespoke renderer's
  > **Nerd Font** terminal icons (or its space-free Unicode layout) without a
  > terminal-aware icon hook. Flipping today would drop Nerd Font icons — a
  > fidelity regression the cutover forbids.
  >
  > `FileSystem` is **not a cutover blocker**: the darkmatter Markdown→tree
  > document pipeline never renders it (it is a standalone directory-tree widget
  > constructed directly by callers, in the same category as `FileTree` and
  > `GraphExpression`). Under the
  > [exemption criterion](../features/2026-06-02-non-structural/spec.md#the-criterion)
  > it is exempt; the terminal flip is optional future migration. See
  > `stage3-spec.md` §S3-1c (outcome iii).

## Two distinct tree cutovers

There are two independent migrations onto the tree renderer. This table tracks
only the first; do not read it as a statement about the second.

1. **Component default render paths** *(tracked here, in the **IR State**
   column).* Whether an individual component's own `render()` —
   `BlockQuote::render()`, `Table::render()`, etc. — routes through the tree
   renderer. Most `biscuit-terminal` components have cut over (`both avail,
   tree renders`); `FileSystem`'s **terminal** path and the `no changes`
   components still default to bespoke. `Prose` and darkmatter's `YamlBlock`
   have fully cut over to `tree render only`: `Prose` parses its bracket-tag
   grammar directly into `RenderNode`, and `YamlBlock::render` /
   `render_html_fragment` fold its projected `Code` node through the shared
   terminal and browser tree renderers (wired with darkmatter's
   `TerminalCodeRenderer`).

2. **The darkmatter Markdown *document* pipeline** *(NOT tracked here —
   complete).* The whole-document renderers — `Markdown::as_html`,
   `Markdown::as_terminal`, and `DarkmatterPage::render` /
   `render_to_browser` — route through the render-tree document entry points
   (`render_tree_html` / `render_tree_terminal` / `render_tree_markdown` in
   `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`, all now `pub`).
   The legacy event-stream serializers (`output::as_html`,
   `output::for_terminal`) and the `RuleProcessor` iterator adapter were
   **deleted** in the 2026-06-02 tree cutover (see
   [`renderable/features/2026-06-02-tree-cutover/`](../features/2026-06-02-tree-cutover/)).
   This cutover is independent of the per-component paths in item 1: the
   document pipeline folds Markdown into `RenderNode`s and renders the tree
   directly — it never calls a component's `render()` — so a component
   retaining a bespoke `render()` does not affect it.

## Removing the bespoke renderers

There are two bespoke surfaces, on independent schedules.

**Document serializers — removed.** The darkmatter Markdown document
serializers (`output::as_html`, `output::for_terminal`) and the `RuleProcessor`
iterator adapter were **deleted** in the 2026-06-02 tree cutover. Removal was
gated on all of the following, which the cutover cleared:

1. **Darkmatter render pipeline is on the tree.** `Markdown::as_html`,
   `Markdown::as_terminal`, and `DarkmatterPage::render` /
   `render_to_browser` route through the render-tree document renderers
   (item 2 above). *(Met.)*
2. **Every component the darkmatter document pipeline renders is reachable on
   the tree.** Components the document pipeline does not render — terminal-only
   presentation primitives, standalone graphics/viz widgets, node-kind
   builder/helpers, and the page frame — are exempt, enumerated with
   justification in the
   [Exemption Register](../features/2026-06-02-non-structural/spec.md#exemption-register)
   of the Non-Structural Component Exemptions spec. Exempt components retain
   their native render path. *(Met — the document pipeline renders entirely
   through the tree.)*
3. **No functional or fidelity regressions** versus the bespoke
   implementation. Output parity (or a deliberate, documented improvement such
   as the `<mark>` recovery) was required on every target. *(Met.)*
4. **The overall performance trend is toward faster.** *(Met — the
   pre-cutover browser/HTML and `mark_dim_hr` terminal regressions were
   resolved or consciously accepted before deletion.)*

**Per-component `render()` paths — partial.** The bespoke `render()` methods on
the `both avail, tree renders` rows (and `FileSystem`'s terminal path) are still
present. They are **not** blocked by the document cutover: they serve direct
component callers — the [`bt`](../../biscuit-terminal/docs/cli.md) CLI,
biscuit-terminal library users — that construct a component and call `render()`
without going through the darkmatter document pipeline. These migrate
component-by-component (the **IR State** column tracks the progress). Exempt
components keep their native render path permanently.

## Darkmatter `style:` frontmatter coverage

The darkmatter `style:` frontmatter pipeline currently wires component-facing
style for these page components:

- `style.table.*` → `PageComponent::Tables`
- `style.images.*` → `PageComponent::Images`
- `style.block-quote.*` → `PageComponent::BlockQuotes`
- `style.ul.*`, `style.ol.*`, `style.li.*` → concrete list components
- `style.page.color` / `style.page.bg-color` → inherited page defaults
- component `color` / `bg-color` keys → component-specific overrides
- `style.hr.*` → horizontal-rule kind, weight, layout, and color defaults
- `style.page.stylesheet`, `style.page.meta`, `style.page.code.theme` →
  HTML page CSS, metadata, and code-block theme defaults
- `style.hyperlinks.*`, `style.hyperlinks.local-style.*`, and
  `style.images.local-style.*` → inline link/image styling, with local-style
  overrides only for local references

For layout/fill fields, `width` and `max-width` are mutually exclusive within
one bucket. `Length::Css` parses in the schema but is rejected when the current
`DarkmatterPage` storage cannot represent it. CLI flags still win
field-by-field over frontmatter. `ACTIVE_STYLE_WIRING_SUB_SPEC` is `7`, so no
valid v1 schema key should emit `KnownButInactive`; unsupported combinations
fail with documented `StyleApplyError` variants.

| Name            | Kind   | Terminal | Browser | Markdown | Tree | IR State                     | bt CLI        | Location                                                    | Description                                                                         |
|-----------------|--------|----------|---------|----------|------|------------------------------|---------------|-------------------------------------------------------------|---------------------|
| BlockQuote      | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/block_quote.rs`        | Quoted text with a distinctive `│ ` left border and optional attribution.           |
| Compose         | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/compose.rs`             | Composes multiple renderable components into a single renderable output.            |
| FileSystem      | Block  | ✅       | ✅      | ✅       | ✅   | both avail, old renders      | bespoke       | `biscuit-terminal/lib/src/components/filesystem/mod.rs`      | Directory trees with Unicode box-drawing, Nerd Font icons, and gitignore awareness. |
| GraphExpression | Block  | ✅       | ✅      | ❌       | ❌   | no changes — exempt (standalone graphics widget) | bespoke       | `biscuit-terminal/lib/src/components/graph_expression.rs`   | Graph diagrams, delegating layout to `biscuit-visualized`. Constructed directly by callers; not reached by any darkmatter fence. |
| HorizontalRule  | Block  | ✅       | ✅      | ❌       | ❌   | no changes — exempt (node-kind builder/helper) | —             | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` | A horizontal rule for terminal and browser rendering. Document HRs render via `NodeKind::ThematicBreak`; this component is a builder the tree renderer may call as a helper. |
| InlineContent   | Inline | ✅       | ❌      | ❌       | ❌   | no changes — exempt (terminal layout primitive) | —             | `biscuit-terminal/lib/src/components/inline_content.rs`      | Concatenates multiple items onto a single line with an optional separator. Terminal-only line mechanics; no document-structure meaning. |
| MermaidDiagram  | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (node-kind builder/helper) | bespoke       | `biscuit-terminal/lib/src/components/mermaid.rs`             | Mermaid diagrams rendered inline via biscuit-visualized. Document mermaid currently lowers to a plain highlighted `NodeKind::Code { lang:"mermaid" }` block; the promotion boundary that would call this rasterizer is owned by the graphics-policy spec and not yet implemented. |
| OrderedList     | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/list.rs`                | Renders items with numeric prefixes as a numbered list.                             |
| UnorderedList   | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/list.rs`                | Renders items as a bulleted list with nested-content support.                       |
| PadLeft         | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (terminal layout primitive) | bespoke       | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the left with spaces to guarantee a minimum width. Terminal-only; no document-structure meaning. |
| PadRight        | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (terminal layout primitive) | bespoke       | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the right with spaces to guarantee a minimum width. Terminal-only; no document-structure meaning. |
| Progress        | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/progress.rs`            | A horizontal progress bar for terminal display.                                     |
| Prose           | Inline | ✅       | ✅      | ✅       | ✅   | tree render only             | tree          | `biscuit-terminal/lib/src/components/prose/mod.rs`           | Styled text with bracketed tags (`<red>…</red>`) and a Markdown subset.             |
| Section         | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/section.rs`             | A Markdown-style heading (h1-h6) followed by arbitrary content.                     |
| Status          | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (terminal UI affordance) | —             | `biscuit-terminal/lib/src/components/status.rs`              | A status indicator with themed icons for validation/action-item state. UI chrome, not document content. |
| StatusBlock     | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/status_block.rs`        | A severity-colored block with optional header, body, and hint content.              |
| Table           | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/table/table.rs`         | A data table with typed columns, planned widths, and alignment.                     |
| TerminalImage   | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (node-kind builder/helper) | bespoke       | `biscuit-terminal/lib/src/components/terminal_image/mod.rs`  | Image rendering via the Kitty graphics protocol with iTerm2 fallback. Document images render via `NodeKind::Image` (graphics-policy tiers); this component is the image-protocol encoder the tree calls. |
| TextBlock       | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/text_block.rs`          | A uniformly styled block of text (colors, weight, italic, underline).               |
| Todo            | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/todo.rs`                | A GFM-style task item with terminal-adaptive checkbox glyphs.                       |
| TwoColumn       | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/two_column.rs`          | A responsive two-column layout that stacks vertically when narrow.                  |
| DarkmatterPage  | Block  | ✅       | ✅      | ❌       | ❌   | no changes — exempt (page frame / render shell) | —             | `darkmatter/lib/src/layout/page.rs`                          | Page-level layout owning margins, padding, background, max-width, and alignment. Renders to terminal (`render`) and browser (`render_to_browser`), both now routing through the render-tree document renderers (via `Markdown::as_terminal_with_layout` / `render_tree_html`). Wraps document output; not a document node. |
| FileTree        | Block  | ✅       | ❌      | ❌       | ❌   | no changes — exempt (standalone viz tool) | —             | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs`     | Visualizes a Markdown file's reference/transclusion dependency graph. A CLI/dev tool, terminal-only; not document content. |
| YamlBlock       | Block  | ✅       | ✅      | ❌       | ✅   | tree render only             | —             | `darkmatter/lib/src/markdown/yaml_block.rs`                  | Typed wrapper around validated YAML with code-block highlighting.                   |

> **Column reference**
>
> - **Name** — the component's Rust type name.
> - **Kind** — layout role. `Block` components occupy full lines and own
>   vertical spacing; `Inline` components produce a run of styled text with no
>   enclosing line breaks.
> - **Terminal** / **Browser** / **Markdown** — whether the component can
>   render to that target *today*. `✅` means a working render path exists for
>   the target (via a per-target trait or, where the **Tree** column is `✅`,
>   via the shared tree renderers). `❌` means no render path to that target
>   exists yet.
> - **Tree** — whether the component implements a render-tree projection,
>   i.e. a `TreeRenderable::render_tree()` or `render_tree_node()` method that
>   produces a canonical `RenderNode`. `✅` only indicates the projection
>   *exists*; it does not imply the component renders through it by default
>   (see **IR State**).
> - **IR State** — which rendering path the component's own default render
>   method (`TerminalRenderable::render()`) uses. One of the four values
>   defined in "IR state values" above. This describes the *component*, not
>   any particular caller.
> - **bt CLI** — which renderer the corresponding [`bt`](../../biscuit-terminal/docs/cli.md)
>   CLI command exercises, which can differ from **IR State** because the CLI
>   is a caller free to choose either path:
>   - `tree` — the `bt` command builds a `RenderNode` and renders it through
>     `render_terminal_node` (the tree renderer), bypassing the component's
>     bespoke `render()`.
>   - `bespoke` — the `bt` command calls the component's bespoke `render()` /
>     `fallback_render()` path.
>   - `—` — no `bt` command renders this component.
> - **Location** — path to the component's primary source file, relative to
>   the repository root.
> - **Description** — a one-line summary of what the component renders.
