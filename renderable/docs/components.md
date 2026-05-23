# Rusty Biscuit Components

Renderable components across the workspace and their supported render targets.

Target support reflects trait implementations:

- `TerminalRenderable` (terminal), `BrowserRenderable` (browser), and
  `MarkdownRenderable` (direct Markdown output).
- `TreeRenderable` is the multi-target path: a component that implements it
  produces a canonical [`RenderNode`](../src/tree/node.rs) tree, which the
  Markdown, Browser, and Terminal **tree renderers** can each fold into
  output. A `TreeRenderable` component therefore reaches every tree-backed
  target without implementing a per-target trait. Thirteen components
  currently project to the render tree: `BlockQuote`, `Compose`,
  `FileSystem`, `OrderedList`, `UnorderedList`, `Progress`, `Section`,
  `StatusBlock`, `Table`, `TextBlock`, `Todo`, and `TwoColumn` from
  `biscuit-terminal`, plus darkmatter's `YamlBlock`. See
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
  > tree path's terminal output is not yet at parity. Stage 3 will pick
  > one of: flip to tree, document as a permanent escape hatch, or defer.
  > See `stage3-spec.md` §S3-1c.
- `component IR (ProseDocument)` means the component has fully migrated to its
  own component-local intermediate representation rather than the shared
  render tree. `Prose` is the only such component: it parses to a
  `ProseDocument` ([`prose/ir.rs`](../../biscuit-terminal/lib/src/components/prose/ir.rs))
  and the Terminal, Browser, and Markdown emitters all render from that single
  model. `Prose` implements no `TreeRenderable` and produces no `RenderNode`.

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
| Prose           | Inline | ✅       | ✅      | ✅       | ❌   | component IR (ProseDocument) | ProseDocument | `biscuit-terminal/lib/src/components/prose/render.rs`        | Styled text with bracketed tags (`<red>…</red>`) and a Markdown subset.             |
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
>   method (`TerminalRenderable::render()`) uses. One of the five values
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
>   - `ProseDocument` — the `bt prose` command renders via Prose's own
>     `ProseDocument` IR (Prose has no render tree).
>   - `—` — no `bt` command renders this component.
> - **Location** — path to the component's primary source file, relative to
>   the repository root.
> - **Description** — a one-line summary of what the component renders.
