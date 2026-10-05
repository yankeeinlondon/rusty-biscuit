# OrderedList / UnorderedList

List components for rendering numbered or bulleted lists in the terminal. Both support nested content, word-wrapping with hanging indent (continuation lines align with content after the prefix), and accept any `Renderable` or string content as items.

## Programmatic Use

### OrderedList

```rust
use biscuit_terminal::prelude::*;

// Create from a vec of strings
let list = OrderedList::new(vec!["First item", "Second item", "Third item"]);
// Renders as:
// 1. First item
// 2. Second item
// 3. Third item

// Build incrementally
let mut list = OrderedList::empty();
list.add("Install dependencies")
    .add("Run build")
    .add("Deploy");

// With custom indentation for nested content
let list = OrderedList::new(vec!["Parent item"])
    .with_indent_children(8);

// Render
let term = Terminal::default();
println!("{}", list.display(&term));
```

### UnorderedList

```rust
use biscuit_terminal::prelude::*;

// Create from a vec of strings
let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
// Renders as:
// - Apple
// - Banana
// - Cherry

// Build incrementally with rich content
let mut list = UnorderedList::empty();
list.add(Prose::new("<bold>Important</bold> item"))
    .add("Plain item");
```

### Key API (both types)

| Method | Description |
|--------|-------------|
| `::new(Vec<impl Into<String>>)` | Create from a vec of strings |
| `::empty()` | Create an empty list |
| `.add(item)` | Append an item (string or RenderableContent) |
| `.with_indent_children(n)` | Set indentation for nested content |

### Word Wrapping

Both list types automatically configure hanging indent on child components so that wrapped continuation lines align with the text after the bullet/number prefix, not the margin.

### Prose Items

A list item holds blocks, as a Markdown list item does, so a list item takes the block [`Prose`](./prose.md). Its first paragraph sits on the marker line and wraps with the hanging indent; each further paragraph or fenced code block follows as its own block, indented under the item. A hard break in that first paragraph (a `Prose` in `LineBreaks::Hard` mode, or `\` before a newline) starts a new row under the marker line:

```rust
// The `- Details:` row renders on its own line, under the marker line.
list.add(Prose::new("Object shape. One mapping per document.\n  - Details: keys are property names")
    .with_line_breaks(LineBreaks::Hard));
```

```rust
let mut list = UnorderedList::empty();
list.add(Prose::new("<b>Install</b> the tool.\n\nThen run `md hash`."));
// Markdown:
// - **Install** the tool.
//
//   Then run `md hash`.
```

A layout set on the `Prose` (for example `with_left_margin`) moves onto the Prose's own blocks, never onto the list item, so the marker does not move. The terminal list renderer places the marker line itself and does not apply that layout; the browser target does. Plain strings are added literally; their markup is not parsed.

## CLI

Exposed via `bt list`:

```bash
bt list "Apple" "Banana" "Cherry"           # Unordered by default
bt list --ordered "First" "Second" "Third"  # Ordered
```
