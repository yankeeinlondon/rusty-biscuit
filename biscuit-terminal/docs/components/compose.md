# Compose

Combines multiple renderable components into a single renderable output. Parts are rendered sequentially with no automatic spacing between them, making it ideal for building complex multi-element documents from heterogeneous components.

Compose accepts any mix of plain strings, `Prose`, `Section`, `Table`, `FileSystem`, lists, and other `Renderable` types. It implements `Renderable` itself, so composed outputs can be nested or passed anywhere a single component is expected.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// From a vec of pre-converted items
let compose = Compose::new(vec![
    RenderableContent::from("Hello, "),
    RenderableContent::from(Prose::new("<b>world</b>!")),
]);

// Builder-style with fluent API
let mut compose = Compose::default();
compose
    .add_text("Hello, ")
    .add_prose(Prose::new("<b>world</b>!"));

// Building a mixed-content document
let mut doc = Compose::default();
doc
    .add_heading("Project Overview", 1)
    .add_text("This project contains ")
    .add_prose(Prose::new("<b>important</b> files"))
    .add_text(" for processing.");

// From conversions
let text: Compose = "Hello, ".into();
let from_content = Compose::from(RenderableContent::from("content"));

// Render
let term = Terminal::default();
println!("{}", doc.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `Compose::new(Vec<RenderableContent>)` | Create from a vec of content items |
| `Compose::default()` | Create empty, then use builder methods |
| `.add_text(str)` | Append plain text |
| `.add_prose(Prose)` | Append styled prose |
| `.add_heading(str, level)` | Append a section heading |

## CLI

Not directly exposed as a CLI command. Compose is a programmatic building block used internally to assemble complex outputs from other components.
