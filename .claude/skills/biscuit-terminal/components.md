# Renderable Components

All components implement the `TerminalRenderable` trait, which provides these rendering methods:

| Method | Trailing `\n` | Terminal-aware | Use for |
|--------|---------------|---------------|---------|
| `render_optimistic(term_width)` | No | No | Composition, embedding |
| `render(term)` | No | Yes | Composition, embedding |
| **`display(term)`** | **Yes** | **Yes** | **Direct terminal output** |

`render(&Terminal)` is the terminal-aware path; `render_optimistic(Option<u32>)` assumes a modern capability set without detection; `render_in_width(term, width)` overrides width only. A component may also implement `render_tree_node(&self) -> Option<RenderNode>` to project itself into the `renderable` render tree — see [Render Tree](./render-tree.md).

Every component owns a `Layout` for margins, alignment, word-wrap, and row-fill. Builder methods (`left_margin()`, `right_margin()`, `alignment()`, `word_wrap()`, etc.) configure it fluently.

## Component Overview

| Component | Module | Block-level | Description |
|-----------|--------|-------------|-------------|
| `BlockQuote` | `block_quote.rs` | Yes | Quoted text with left border and attribution |
| `Compose` | `compose.rs` | No | Combine multiple renderables into one output |
| `FileSystem` | `filesystem.rs` | Yes | File/directory tree rendering with icons and gitignore awareness |
| `GraphExpression` | `graph_expression.rs` | Yes | Graph diagrams via biscuit-visualized with terminal image display |
| `InlineContent` | `inline_content.rs` | No | Inline concatenation of items without newlines |
| `MermaidDiagram` | `mermaid.rs` | Yes | Mermaid diagram rendering via biscuit-visualized |
| `OrderedList` | `list.rs` | Yes | Numbered list with nested renderable support |
| `UnorderedList` | `list.rs` | Yes | Bullet list with custom bullets, hanging indent |
| `PadLeft` | `pad.rs` | No | Right-align content by padding with spaces on the left |
| `PadRight` | `pad.rs` | No | Left-align content by padding with spaces on the right |
| `Progress` | `progress.rs` | No | Progress indicator rendering |
| `Prose` | `prose.rs` | No | Styled text with inline tokens (atomic + block) |
| `Section` | `section.rs` | Yes | Heading (h1-h6) with content body |
| `Status` | `status.rs` | No | Status items with icons (success, failure, warning, info, active, not-started) |
| `Table` | `table/` | Yes | Box-drawing table with auto-sized columns |
| `TerminalImage` | `terminal_image.rs` | Yes | Inline images via Kitty/iTerm2 protocols |
| `TextBlock` | `text_block.rs` | No | Uniform styling across text (bold, color, underline) |
| `Todo` | `todo.rs` | No | Task item with state (Open, InProgress, Completed, Blocked, Cancelled) |
| `TwoColumn` | `two_column.rs` | Yes | Side-by-side columns (supports inline images) |

## Compose

Combines multiple renderable parts into a single output.

```rust
use biscuit_terminal::prelude::*;

let mut compose = Compose::new();
compose.add_text("Hello, ").add_prose(Prose::new("<b>world</b>!"));

// Can also add lists, other components
compose.add_unordered_list(UnorderedList::new(vec!["Item A", "Item B"]));

let output = compose.render_optimistic(Some(80));
```

**Key methods:** `add_text()`, `add_prose()`, `add_ordered_list()`, `add_unordered_list()`, `add_component()`

## Section

A heading with optional content body. Headings render with appropriate styling (bold for h1-h3, etc.).

```rust
use biscuit_terminal::prelude::*;

let section = Section::new(HeadingLevel::h2, "My Section")
    .with_content(vec![
        RenderableTerminalContent::String("Body text here.".to_string()),
    ]);
```

**Heading levels:** `h1` through `h6` via the `HeadingLevel` enum.

## BlockQuote

Renders quoted text with a colored left border and optional attribution.

```rust
use biscuit_terminal::prelude::*;

let quote = BlockQuote::new("To be or not to be")
    .with_attribution("Shakespeare")
    .with_left_block_color(Color::Tailwind(TailwindColor::Gray500));
```

**Builder methods:** `with_attribution()`, `with_text_color()`, `with_bg_color()`, `with_left_block_color()`

Word wrapping is enabled by default. Content can be a string or any `RenderableTerminalContent`.

## TwoColumn

Side-by-side column rendering with cursor-based positioning. Handles inline images alongside text.

```rust
use biscuit_terminal::prelude::*;

let columns = TwoColumn::new(
    RenderableTerminalContent::String("Left content".into()),
    RenderableTerminalContent::String("Right content".into()),
)
.with_gap(4)
.with_left_width(ColumnWidth::Percent(0.4));
```

**Column widths:** `ColumnWidth::Fixed(chars)` or `ColumnWidth::Percent(0.0..=1.0)`

The `bt columns` CLI command wraps this component:

```bash
bt columns "Left column" "Right column"
bt columns --gap 6 --left 40% "Title" "Description"
bt columns --margin-left 2 --alignment center "Left" "Right"
```

**Terminal-specific cursor handling:** WezTerm, Ghostty, Kitty, and iTerm2 get tailored cursor reset behavior; other terminals (including Warp) use the standard save/restore fallback path.

## Todo

Task item with visual state representation. Uses Nerd Font icons when available, falls back to ASCII checkboxes. Respects `NO_COLOR`.

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::components::todo::{Todo, TodoState};

let todo = Todo::new("Implement feature X", TodoState::InProgress);
```

**States:** `Open`, `InProgress`, `Completed`, `Blocked`, `Cancelled`

**Nerd Font icons** (when detected): custom checkbox glyphs for each state
**Fallback** (no Nerd Font): `[ ]`, `[>]`, `[x]`, `[!]`, `[-]` with color when supported

## Table

See [Table README](../../../biscuit-terminal/lib/src/components/table/README.md) for comprehensive documentation.

Key features:

- Full box-drawing borders (`┌┬┐`, `├┼┤`, `└┴┘`)
- Auto-sized columns with min/max constraints
- `TableColumn::new("Header").with_min_width(8).with_max_width(30)`
- Data via `with_data(vec![vec!["cell".into()]])` or `add_row()` (`&mut self`, returns `()`)
- Extra cells beyond defined columns are rendered as additional columns
- Alignment defaults come from `ColumnType` (text left, numeric right); wrapping is resolved per cell/column strategy

## Lists (OrderedList, UnorderedList)

Both support nested renderable children (block-level children are indented without bullet/number prefix).

```rust
use biscuit_terminal::prelude::*;

// Simple
let ol = OrderedList::new(vec!["First", "Second"]);
let ul = UnorderedList::new(vec!["Apple", "Banana"]).with_bullet("- ");

// Incremental building
let mut list = UnorderedList::empty();
list.add("Item 1").add("Item 2");

// Nested
let inner = OrderedList::new(vec!["Sub A", "Sub B"]);
let outer = UnorderedList::from(vec![
    RenderableTerminalContent::String("Top item".into()),
    RenderableTerminalContent::Component(Rc::new(inner)),
]);
```

## Progress

Progress indicator rendering component.

```rust
use biscuit_terminal::components::progress::Progress;
```

## FileSystem

File/directory tree rendering with Nerd Font icons and gitignore-aware dimming. Used by `bt dir`.

Supports optional file metrics: file sizes, estimated LLM token counts, modification timestamps (absolute and relative).

## InlineContent

Inline concatenation of items without newlines. Useful for composing multiple elements on a single line.

```rust
use biscuit_terminal::components::inline_content::InlineContent;
```

## Prose and TextBlock

See [Styling](./styling.md) for comprehensive Prose token reference and TextBlock builder details.

## RenderableTerminalContent

The `RenderableTerminalContent` enum bridges strings and components:

```rust
pub enum RenderableTerminalContent {
    String(String),
    Component(Rc<dyn TerminalRenderable>),
}
```

Implements `From<String>`, `From<&str>`, and `From<T: TerminalRenderable>` for ergonomic construction.
