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
| `GitGraph` | `git_graph.rs` | Yes | Typed git topology as a Mermaid `gitGraph`: owns the lane/tag rule, merges, the no-substitution rule, and fits the terminal by trimming commits and lanes (`image` feature; see below) |
| `GraphExpression` | `graph_expression.rs` | Yes | Graph diagrams via biscuit-visualized with terminal image display |
| `InlineContent` | `inline_content.rs` | No | Inline concatenation of items without newlines |
| `MermaidDiagram` | `mermaid.rs` | Yes | Mermaid diagram rendering via biscuit-visualized; defaults to `ImageWidth::Scale(1.0)` (body text one line tall), measured from the SVG before rasterizing |
| `OrderedList` | `list.rs` | Yes | Numbered list with nested renderable support |
| `UnorderedList` | `list.rs` | Yes | Bullet list with custom bullets, hanging indent |
| `PadLeft` | `pad.rs` | No | Right-align content by padding with spaces on the left |
| `PadRight` | `pad.rs` | No | Left-align content by padding with spaces on the right |
| `Progress` | `progress.rs` | No | Progress indicator rendering |
| `InlineProse` | `prose/inline_prose.rs` | No | Inline styled text (phrasing only) for cells, labels, values in a line |
| `Prose` | `prose/prose.rs` | No | Block styled text: paragraphs + fenced code, `Layout`, `ProseTag` |
| `Section` | `section.rs` | Yes | Heading (h1-h6) with content body |
| `Spinner` | `spinner.rs` | No | Live stderr activity spinner; not a `TerminalRenderable` (see below) |
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
- Striping via `alternate_background_color()` / `with_stripe_bg(Color)` and `alternate_text_color()` / `with_stripe_text(Color)`; `highlight_row(row, Color)` paints one data row (0-based, header excluded; out-of-range is a no-op) and wins over the stripe on that row. Stripe and highlight are terminal-only (`TableTerminalHints`), degrade with color depth, and are ignored by Browser/Markdown
- `TableCellContent::StyledInlineProse(Box<InlineProse>)` (`InlineProse::new(...).into()`; hint kind `styled_inline_prose`) embeds capability-aware inline styling, links, emphasis, and inline code in a cell. Cells take `InlineProse`, never block `Prose` (no `From<Prose>`); a fence in a cell becomes one `InlineCode`, and a cell that needs a line per `\n` uses `.with_line_breaks(LineBreaks::Hard)`. `&str`/`String` cells stay literal `Text`. `TableColumn::header_prose` is `Option<InlineProse>`. The tree path projects the semantic inline nodes; the terminal bespoke path resolves each cell to `Text(prose.render(term))` once before width planning. The table owns cell geometry — the cell value's `Layout` is not applied.

## Lists (OrderedList, UnorderedList)

Both support nested renderable children (block-level children are indented without bullet/number prefix).

A `Prose` item is block content: its `Paragraph`/`Code` blocks become the `ListItem`'s children (first paragraph on the marker line, the rest indented). `Prose::is_block_level()` is `true`, so lists no longer inject a hanging-indent wrap into its layout.

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

## Spinner

A live, time-driven stderr widget, so it is **not** a `TerminalRenderable` or
render-tree node. It draws only when stderr is a terminal, starts after an
optional delay, accepts replacement text, and clears its line on `finish()` or
`Drop`. The clear is written once, and only if a frame was drawn.

```rust
use std::time::Duration;
use biscuit_terminal::prelude::Spinner;

let spinner = Spinner::new("updating")
    .with_delay(Duration::from_millis(150))
    .start_on_stderr();
spinner.set_text("rate limited, using fallback method");
spinner.finish();
```

Test with `start_on(writer, is_terminal)` and the pure `frame(i, text, width)`
and `CLEAR_LINE`; see [docs/components/spinner.md](../../../biscuit-terminal/docs/components/spinner.md).

## FileSystem

File/directory tree rendering with Nerd Font icons and gitignore-aware dimming. Used by `bt dir`.

Supports optional file metrics: file sizes, estimated LLM token counts, modification timestamps (absolute and relative).

New builder APIs for selective tree rendering:

- `.extension_filter([...])` / `.document_extensions()` — case-insensitive extension allowlist
- `.included_paths([...])` — exact relative-path allowlist
- `.with_dimmed_root_prefix(prefix)` — dimmed prefix before the root directory name
- `.with_root_display_name(name)` — override the highlighted target directory name
- `.with_root_icon(icon)` — `RootIconKind::Directory` or `RootIconKind::Repository`

These are used by the `::file-links` compose directive to render bounded document trees.

## GitGraph

Full contract: `biscuit-terminal/docs/components/git_graph.md`. The caller runs git; the component never does.

- `GraphLine::with_tip(sha)` is the branch's own tip. A line without commits is labeled there. `tip()` falls back to the newest drawn commit, **never** `fork_sha`. `forked_at` unset (`fork_sha == None`) means an unknown connection, not "tip here".
- `GraphLine::with_merge(source, destination)` appends a `LaneMerge` (oldest first; `merges` field). Every merge is drawn from its own source: it emits `merge <lane> id: "…" tag: "…"` at the destination commit on whatever lane draws it, only when the source was already emitted and the destination lane has a head (the parser drops a labeled merge into a headless lane). A lane pauses after a mid-lane source (and the lanes forked at it) until the destination is emitted, then resumes after the `merge` with `checkout <source>`; paused lanes left over (a cycle) have their blocking merge dropped and resume. `biscuit-visualized` restores the merge's second parent (the source). Siblings hanging from one commit are reordered so a lane whose merges land on a sibling's subtree is emitted first (`emit_merged_lanes_first`). A second merge into one commit is not drawn; every undrawn merge sets `incomplete`.
- **No substitution:** a lane whose fork is undrawn or unknown is drawn unconnected (declared `branch` before the root lane's first commit), never from another lane's start. An undrawn tag, label, or merge destination is left out. Each sets `GitGraphPlan::incomplete`, as does the caller's `with_incomplete_history()`, and renders the dim `INCOMPLETE_HISTORY_NOTE` ("Some history is not shown") after the hidden-lanes note. Tags of lanes the height cap hid are not counted twice.
- Trimming (`trim_one_commit`) never folds lane tips, forks, merge sources and destinations, or tagged commits. The height cap (`fit_lanes`) keeps a lane's ancestors: the lanes holding its fork commit and every merge destination, and its parent's lane.
- `biscuit-visualized` widens the commit step only for colliding tags (different commits, overlapping rows) and only to a one-em gap, so a single long label (e.g. `main`/`origin/main` stacked on one tip) keeps the default step and trimming works normally; a graph with colliding tags is widened everywhere and may still need to shrink. Assert the step against `biscuit_visualized::mermaid::default_gitgraph_commit_step()`. Prove tag placement with `biscuit_visualized::mermaid::MermaidDiagram::gitgraph_geometry()` (`tag_overlaps()`), never with Mermaid text.

## InlineContent

Inline concatenation of items without newlines. Useful for composing multiple elements on a single line.

Styled items are `InlineProse` (`From<InlineProse>`, `add_inline_prose`); there is no `From<Prose>`.

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

### Embedding `Prose` in a container

Containers project children through `render_tree::projection` (`project_renderable_content` / `RenderableTerminalContent::to_tree_nodes`). A `Prose` child is special-cased there: it contributes `Prose::embedded_nodes()` — its `Paragraph` and `Code` blocks, never its `Root` (a nested `Root` fails validation). Its `Layout` moves onto those blocks (one block: the whole layout; several: the horizontal box on each, the top edge on the first, the bottom edge on the last), never onto the container's node. `Prose::render_tree_node()` stays `None` for that reason; do not add one that returns the `Root`. `Compose` puts a `"\n\n"` text between consecutive blocks of one `Prose` because its sequence has no separator. `StatusBlock::body` items and `TwoColumn` columns embed structurally the same way; `BlockQuote` wraps only a `String` (or an all-inline projection) in a `Paragraph`.
