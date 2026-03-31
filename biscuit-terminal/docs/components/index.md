# Components

All components implement the [`Renderable`](../../lib/src/components/renderable.rs) trait, which provides three rendering paths: `render()`, `fallback_render()`, and `display()`. Every component owns a `Layout` for margins, alignment, and word-wrap.

## Component Index

| Component | Description |
|-----------|-------------|
| [BlockQuote](./block_quote.md) | Quoted text with a colored left border and optional attribution |
| [Compose](./compose.md) | Combines multiple renderable parts into a single output |
| [Csv](./csv.md) | Renders CSV data into a Table for the terminal |
| [FileSystem](./file_system.md) | File/directory tree rendering with icons and gitignore awareness |
| [GraphExpression](./graph_expression.md) | Graph diagrams via biscuit-visualized with terminal image display |
| [InlineContent](./inline_content.md) | Inline concatenation of items without newlines |
| [OrderedList / UnorderedList](./list.md) | Numbered and bullet-point lists with nested renderable support |
| [MermaidDiagram](./mermaid_diagram.md) | Mermaid diagram rendering via biscuit-visualized |
| [PadLeft](./pad_left.md) | Right-align content by padding with spaces on the left |
| [PadRight](./pad_right.md) | Left-align content by padding with spaces on the right |
| [Progress](./progress.md) | Horizontal progress bar with configurable width, characters, and colors |
| [Prose](./prose.md) | Styled text with inline tokens (`{{bold}}`, `<red>...</red>`, etc.) |
| [Section](./section.md) | Heading (h1-h6) with optional content body |
| [Status](./status.md) | Status items with icons (success, failure, warning, info, active, not-started) |
| [Table](./table.md) | Box-drawing table with auto-sized columns and rich formatting |
| [TerminalImage](./terminal_image.md) | Inline images via Kitty/iTerm2 protocols with graceful fallback |
| [TextBlock](./text_block.md) | Uniformly styled text block with color, weight, and underline support |
| [Todo](./todo.md) | Task items with checkboxes (open, in-progress, completed, blocked, cancelled) |
| [TwoColumn](./two_column.md) | Side-by-side two-column layout with cursor-based positioning |
