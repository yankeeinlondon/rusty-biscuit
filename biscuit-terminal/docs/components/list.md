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

## CLI

Exposed via `bt list`:

```bash
bt list "Apple" "Banana" "Cherry"           # Unordered by default
bt list --ordered "First" "Second" "Third"  # Ordered
```
