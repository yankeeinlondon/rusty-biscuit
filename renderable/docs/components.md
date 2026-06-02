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

  > **FileSystem caveat.** `FileSystem` is the one component whose tree
  > projection covers Browser and Markdown but whose **terminal** `render`
  > still calls the bespoke directory-tree renderer. The connector geometry
  > (`├──`, `└──`, `│`) is presentation, not document structure, and the
  > tree path's terminal output is not yet at parity. Stage 3 deferred the
  > terminal flip to Stage 4 pending connector-list `Style` lowering and
  > icon-name spacing parity. See `stage3-spec.md` §S3-1c.

## Two distinct tree cutovers

There are two independent migrations onto the tree renderer. This table tracks
only the first; do not read it as a statement about the second.

1. **Component default render paths** *(tracked here, in the **IR State**
   column).* Whether an individual component's own `render()` —
   `BlockQuote::render()`, `Table::render()`, etc. — routes through the tree
   renderer. Most `biscuit-terminal` components have cut over (`both avail,
   tree renders`); a few still default to bespoke (`YamlBlock`, `FileSystem`'s
   terminal path, and the `no changes` components). `Prose` has fully cut over
   to `tree render only`: it parses its bracket-tag grammar directly into
   `RenderNode` and renders through the shared tree renderers only.

2. **The darkmatter Markdown *document* pipeline** *(NOT tracked here).* The
   whole-document Markdown serializers — `Markdown::as_html`,
   `Markdown::for_terminal`, and `DarkmatterPage::render` — still run the
   **legacy event-stream renderers** (`output/html.rs`, `output/terminal.rs`,
   `RuleProcessor`). `DarkmatterPage::render` is pinned byte-for-byte to
   `for_terminal(default)`. The render-tree document entry points
   (`render_tree_html` / `render_tree_terminal` / `render_tree_markdown` in
   `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`) are `pub(crate)`
   and reached only from tests. This is the path the `migration_parity`
   benchmark and the `2026-05-26-inline-span` / `2026-05-26-block-extension`
   specs target; its public cutover has **not** happened.

## Removing the bespoke renderers

The bespoke render paths (component-level *and* the darkmatter Markdown
document serializers) may only be removed once **all** of the following hold:

1. **Darkmatter render pipeline is on the tree.** `Markdown::as_html`,
   `Markdown::for_terminal`, and `DarkmatterPage::render` route through the
   render-tree document renderers (cutover #2 above).
2. **Every renderable component renders through the tree** — in both
   `biscuit-terminal` and `darkmatter`. No component retains a default bespoke
   path: every **IR State** is `tree render only` (no `both avail, old
   renders`, `no changes`, or component-local IR holdouts remain, or each is
   explicitly retired with documented justification).
3. **No functional or fidelity regressions** versus the bespoke
   implementation. Output parity (or a deliberate, documented improvement such
   as the `<mark>` recovery) is required on every target.
4. **The overall performance trend is toward faster.** The net trend line
   across the corpus must improve; mild, localized regressions in specific
   areas are acceptable so long as the general direction is faster. Known
   regressions to resolve or consciously accept before cutover #2 include the
   browser/HTML path (2–11× slower than legacy on several fixtures, worst on
   table-heavy input) and the `mark_dim_hr` terminal regression (unconditional
   HR PNG rasterization, owned by the graphics-policy spec).

Until conditions 1 and 2 both hold, the legacy serializers and the remaining
bespoke component paths are load-bearing and must stay.

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
|-----------------|--------|----------|---------|----------|------|------------------------------|---------------|-------------------------------------------------------------|-------------------------------------------------------------------------------------|
| BlockQuote      | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/block_quote.rs`        | Quoted text with a distinctive `│ ` left border and optional attribution.           |
| Compose         | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/compose.rs`             | Composes multiple renderable components into a single renderable output.            |
| FileSystem      | Block  | ✅       | ✅      | ✅       | ✅   | both avail, old renders      | bespoke       | `biscuit-terminal/lib/src/components/filesystem/mod.rs`      | Directory trees with Unicode box-drawing, Nerd Font icons, and gitignore awareness. |
| GraphExpression | Block  | ✅       | ✅      | ❌       | ❌   | no changes                   | bespoke       | `biscuit-terminal/lib/src/components/graph_expression.rs`   | Graph diagrams, delegating layout to `biscuit-visualized`.                          |
| HorizontalRule  | Block  | ✅       | ✅      | ❌       | ❌   | no changes                   | —             | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` | A horizontal rule for terminal and browser rendering.                               |
| InlineContent   | Inline | ✅       | ❌      | ❌       | ❌   | no changes                   | —             | `biscuit-terminal/lib/src/components/inline_content.rs`      | Concatenates multiple items onto a single line with an optional separator.          |
| MermaidDiagram  | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | bespoke       | `biscuit-terminal/lib/src/components/mermaid.rs`             | Mermaid diagrams (flowchart, pie, timeline, ERD, etc.) rendered inline via biscuit-visualized. |
| OrderedList     | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/list.rs`                | Renders items with numeric prefixes as a numbered list.                             |
| UnorderedList   | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/list.rs`                | Renders items as a bulleted list with nested-content support.                       |
| PadLeft         | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | bespoke       | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the left with spaces to guarantee a minimum width.                  |
| PadRight        | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | bespoke       | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the right with spaces to guarantee a minimum width.                 |
| Progress        | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/progress.rs`            | A horizontal progress bar for terminal display.                                     |
| Prose           | Inline | ✅       | ✅      | ✅       | ✅   | tree render only             | tree          | `biscuit-terminal/lib/src/components/prose/mod.rs`           | Styled text with bracketed tags (`<red>…</red>`) and a Markdown subset.             |
| Section         | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/section.rs`             | A Markdown-style heading (h1-h6) followed by arbitrary content.                     |
| Status          | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | —             | `biscuit-terminal/lib/src/components/status.rs`              | A status indicator with themed icons for validation/action-item state.              |
| StatusBlock     | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/status_block.rs`        | A severity-colored block with optional header, body, and hint content.              |
| Table           | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/table/table.rs`         | A data table with typed columns, planned widths, and alignment.                     |
| TerminalImage   | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | bespoke       | `biscuit-terminal/lib/src/components/terminal_image/mod.rs`  | Image rendering via the Kitty graphics protocol with iTerm2 fallback.               |
| TextBlock       | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/text_block.rs`          | A uniformly styled block of text (colors, weight, italic, underline).               |
| Todo            | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/todo.rs`                | A GFM-style task item with terminal-adaptive checkbox glyphs.                       |
| TwoColumn       | Block  | ✅       | ✅      | ✅       | ✅   | both avail, tree renders     | tree          | `biscuit-terminal/lib/src/components/two_column.rs`          | A responsive two-column layout that stacks vertically when narrow.                  |
| DarkmatterPage  | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | —             | `darkmatter/lib/src/layout/page.rs`                          | Page-level layout owning margins, padding, background, max-width, and alignment.    |
| FileTree        | Block  | ✅       | ❌      | ❌       | ❌   | no changes                   | —             | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs`     | Visualizes a Markdown file's reference/transclusion dependency graph.               |
| YamlBlock       | Block  | ✅       | ✅      | ❌       | ✅   | both avail, old renders      | —             | `darkmatter/lib/src/markdown/yaml_block.rs`                  | Typed wrapper around validated YAML with code-block highlighting.                   |

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
