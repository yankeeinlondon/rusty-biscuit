# GraphExpression

Renders graph diagrams inline in the terminal as raster images. Parses graph expressions (e.g., `a -> b -> c`) using multiple input syntaxes, lays them out with this crate's graph renderer, and displays them through `biscuit-terminal`'s `TerminalImage` using the terminal's image protocol.

Requires the `terminal` feature; import it from `biscuit_visualized::components`.

Supports configurable orientation (left-to-right, top-to-bottom), titles, scale factor, transparent backgrounds, and standard layout options.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;
use biscuit_visualized::components::*;

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

## Input Modes and Features

Supported input modes:

- expression syntax with directed edges: `a -> b -> c`
- expression syntax with undirected edges: `a -- b -- c`
- DOT input: `digraph { A -> B; }`

Mixed directed and undirected expression syntax is rejected in a single graph expression.

### Usage

```rust
use biscuit_visualized::components::graph_expression::{
    GraphExpression, GraphInputSyntax, GraphOrientation,
};
use biscuit_terminal::terminal::Terminal;

let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph");

let term = Terminal::new();
match graph.try_render(&term) {
    Ok(result) => print!("{}", result.output),
    Err(_) => println!("{}", graph.fallback_code_block()),
}
# Ok::<(), biscuit_visualized::components::graph_expression::GraphRenderError>(())
```

### Graph Features

- terminal-aware light/dark themes by default
- explicit inverse rendering via `GraphExpression::inverted_for_terminal(...)`
- width control through `ImageWidth`
- optional font-family override with `.with_font_family(...)`
- render metadata via `GraphRenderResult` (`png_path`, `cache_hit`)

Like Mermaid rendering, graph rendering is pure Rust: this crate uses `layout-rs` for layout and `resvg` for rasterization; no Graphviz binary is required.

## CLI

Exposed via `bt graph-expression`:

```bash
bt graph-expression "a -> b -> c"
bt graph-expression --orientation left-to-right "a -> b\nb -> c"
bt graph-expression --meta "a -> b -> c"
```

Layout args are supported: `--margin-left`, `--width`, `--alignment`, etc.
