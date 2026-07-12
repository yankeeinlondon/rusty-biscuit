# GraphExpression

Renders graph diagrams inline in the terminal as raster images. Parses graph expressions (e.g., `a -> b -> c`) using multiple input syntax's, delegates layout to `biscuit-visualized`, and displays via `TerminalImage` using the terminal's image protocol.

Supports configurable orientation (left-to-right, top-to-bottom), titles, scale factor, transparent backgrounds, and standard layout options.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic graph from expression syntax
let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?;
let term = Terminal::new();
println!("{}", graph.render(&term));

match graph.try_render(&term) {
    Ok(result) => eprintln!("Rendered {}", result.png_path.display()),
    Err(err) => eprintln!("Render failed: {err}"),
}

// With orientation and title
let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("My Graph");

// With layout margins
use biscuit_terminal::utils::layout::Margin;
let graph = GraphExpression::for_terminal("a -> b\nb -> c", GraphInputSyntax::Auto)?
    .left_margin(Margin::Chars(4));
```

### Key API

| Method | Description |
|--------|-------------|
| `GraphExpression::for_terminal(source, syntax)` | Parse and create a graph (fallible) |
| `GraphExpression::inverted_for_terminal(source, syntax)` | Use the opposite terminal theme with an opaque surface |
| `.with_orientation(GraphOrientation)` | Set layout direction (LR, TB, etc.) |
| `.with_title(str)` | Add a diagram title |
| `.try_render(&Terminal)` | Fallible render returning `GraphRenderResult` |
| `.left_margin(Margin)` | Set left margin |

### Input Syntaxes

`GraphInputSyntax::Auto` auto-detects the format. Supported syntaxes include arrow notation (`a -> b`), DOT-like syntax, and others provided by `biscuit-visualized`.

### Error Handling

Returns `GraphRenderError` for visualization errors, unsupported terminals (no inline image support), and image display failures.

## CLI

Exposed via `bt graph-expression`:

```bash
bt graph-expression "a -> b -> c"
bt graph-expression --orientation left-to-right "a -> b\nb -> c"
bt graph-expression --meta "a -> b -> c"
```

Layout args are supported: `--margin-left`, `--width`, `--alignment`, etc.
