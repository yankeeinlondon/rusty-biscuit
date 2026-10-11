# InlineContent

Concatenates multiple items into a single line without inserting newlines between them. Each item can be a plain string or any `Renderable` component. Unlike block-level components (like `TextBlock` or `Section`), InlineContent does not add line breaks between items, making it ideal for horizontal sequences of text, styled spans, or separator-joined lists.

Use [`InlineProse`](./prose.md) for styled text inside InlineContent: it is the inline form of Prose, so its content stays on the line (a fenced block becomes one inline code value). `From<InlineProse>` and `add_inline_prose` accept it directly. Plain strings are appended literally; their markup is not parsed.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Owned builder chain (no mut needed)
let inline = InlineContent::default()
    .with("Hello, ")
    .with(InlineProse::new("<b>world</b>"));

// Mutable push chain
let mut inline = InlineContent::default();
inline.push("foo").push(InlineProse::new("<b>bar</b>"));

// From a vec of pre-converted items
let inline = InlineContent::new(vec![
    RenderableContent::from("foo"),
    RenderableContent::from(InlineProse::new("<b>bar</b>")),
]);

// With separator
let inline = InlineContent::default()
    .with_separator(" | ")
    .with("one")
    .with("two")
    .with("three");
assert_eq!(inline.render_optimistic(Some(80)), "one | two | three");

// Render
let term = Terminal::default();
println!("{}", inline.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `InlineContent::default()` | Create empty inline content |
| `InlineContent::new(Vec<RenderableContent>)` | Create from a vec of items |
| `.with(item)` | Append an item (owned builder, returns Self) |
| `.push(item)` | Append an item (mutable builder, returns &mut Self) |
| `.add_inline_prose(InlineProse)` | Append styled inline text (mutable builder) |
| `InlineContent::from(InlineProse)` | Create from one styled inline item |
| `.with_separator(str)` | Set separator between items |

## CLI

Not directly exposed as a CLI command. InlineContent is a programmatic building block for composing horizontal text sequences.
