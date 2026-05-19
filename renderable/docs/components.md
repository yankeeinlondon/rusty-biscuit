# Rusty Biscuit Components

Renderable components across the workspace and their supported render targets.

Target support reflects trait implementations:

- `TerminalRenderable` (terminal), `BrowserRenderable` (browser), and
  `MarkdownRenderable` (direct Markdown output).
- `TreeRenderable` is the multi-target path: a component that implements it
  produces a canonical [`RenderNode`](../src/tree/node.rs) tree, which the
  Markdown, Browser, and Terminal **tree renderers** can each fold into
  output. A `TreeRenderable` component therefore reaches every tree-backed
  target without implementing a per-target trait. `BlockQuote` is the first
  component to adopt `TreeRenderable`.

IR state values:

- `no changes` means the component still uses its pre-tree rendering path only.
- `both avail, old renders` means a tree projection exists, but the default
  component render path still uses the bespoke renderer.
- `both avail, tree renders` means both paths exist and the default component
  render path uses the tree renderer.
- `tree render only` means the old bespoke render path has been removed and
  rendering now goes through the IR renderer only.

| Name            | Kind   | Terminal | Browser | Markdown | Tree | IR State                | Location                                                    | Description                                                                         |
|-----------------|--------|----------|---------|----------|------|-------------------------|-------------------------------------------------------------|-------------------------------------------------------------------------------------|
| BlockQuote      | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/block_quote.rs`        | Quoted text with a distinctive `│ ` left border and optional attribution.           |
| Compose         | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/compose.rs`             | Composes multiple renderable components into a single renderable output.            |
| FileSystem      | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/filesystem/mod.rs`      | Directory trees with Unicode box-drawing, Nerd Font icons, and gitignore awareness. |
| GraphExpression | Block  | ✅       | ✅      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/graph_expression.rs`   | Graph diagrams, delegating layout to `biscuit-visualized`.                          |
| HorizontalRule  | Block  | ✅       | ✅      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` | A horizontal rule for terminal and browser rendering.                               |
| InlineContent   | Inline | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/inline_content.rs`      | Concatenates multiple items onto a single line with an optional separator.          |
| OrderedList     | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/list.rs`                | Renders items with numeric prefixes as a numbered list.                             |
| UnorderedList   | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/list.rs`                | Renders items as a bulleted list with nested-content support.                       |
| PadLeft         | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the left with spaces to guarantee a minimum width.                  |
| PadRight        | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/pad.rs`                 | Pads content on the right with spaces to guarantee a minimum width.                 |
| Progress        | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/progress.rs`            | A horizontal progress bar for terminal display.                                     |
| Prose           | Inline | ✅       | ✅      | ✅       | ❌   | tree render only        | `biscuit-terminal/lib/src/components/prose/render.rs`        | Styled text with bracketed tags (`<red>…</red>`) and a Markdown subset.             |
| Section         | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/section.rs`             | A Markdown-style heading (h1-h6) followed by arbitrary content.                     |
| Status          | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/status.rs`              | A status indicator with themed icons for validation/action-item state.              |
| StatusBlock     | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/status_block.rs`        | A severity-colored block with optional header, body, and hint content.              |
| Table           | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/table/table.rs`         | A data table with typed columns, planned widths, and alignment.                     |
| TerminalImage   | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/terminal_image/mod.rs`  | Image rendering via the Kitty graphics protocol with iTerm2 fallback.               |
| TextBlock       | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/text_block.rs`          | A uniformly styled block of text (colors, weight, italic, underline).               |
| Todo            | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `biscuit-terminal/lib/src/components/todo.rs`                | A GFM-style task item with terminal-adaptive checkbox glyphs.                       |
| TwoColumn       | Block  | ✅       | ❌      | ❌       | ✅   | both avail, old renders | `biscuit-terminal/lib/src/components/two_column.rs`          | A responsive two-column layout that stacks vertically when narrow.                  |
| DarkmatterPage  | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `darkmatter/lib/src/layout/page.rs`                          | Page-level layout owning margins, padding, background, max-width, and alignment.    |
| FileTree        | Block  | ✅       | ❌      | ❌       | ❌   | no changes              | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs`     | Visualizes a Markdown file's reference/transclusion dependency graph.               |
| YamlBlock       | Block  | ✅       | ✅      | ❌       | ✅   | both avail, old renders | `darkmatter/lib/src/markdown/yaml_block.rs`                  | Typed wrapper around validated YAML with code-block highlighting.                   |
