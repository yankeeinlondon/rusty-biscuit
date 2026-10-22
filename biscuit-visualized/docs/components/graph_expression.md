# GraphExpression

Renders graph diagrams inline in the terminal as raster images. Parses graph expressions (e.g., `a -> b -> c`) using multiple input syntaxes, lays them out with this crate's graph renderer, and displays them through `biscuit-terminal`'s `TerminalImage` using the terminal's image protocol (Kitty, iTerm2, or Sixel).

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
| `.with_background(Background)` | What paints behind the graph (`Theme`, `Transparent`, or `Solid { r, g, b }`) |
| `.try_render(&Terminal)` | Fallible render returning `GraphRenderResult` |
| `.left_margin(Margin)` | Set left margin |

### Background

`for_terminal` draws with `Background::Transparent`, so Kitty and iTerm2 composite the graph over the terminal's own background. `parse`, `inverted_for_terminal`, and `inverted(..)` use `Background::Theme`, the theme's opaque surface color. `with_background` overrides either (import `Background` from `biscuit_visualized::artifact`); `with_transparent_background(bool)` remains as a shorthand where `false` means `Theme`.

On a Sixel route a `Transparent` graph is rasterized on `Background::Solid` with the terminal's measured background (an OSC 11 reply). Without a measurement it stays transparent: the Sixel encoder leaves its transparent pixels unset and blends only antialiased edges onto the estimated background, so no box is painted. An explicitly requested `Theme` or `Solid` background is kept. The artifact cache keys on the background, so a Sixel raster and a Kitty raster of the same graph are separate cache entries.

### Display Size

The graph is rasterized once, at its final display size. The width (for example the default `ImageWidth::Percent(0.5)`) resolves once to columns, with the layout's margins taken into account, and the PNG is that many columns times the measured cell width in pixels, or the 8x16 fallback cell when the terminal reports none. Its height is padded with blank rows below the drawing up to a whole number of cell rows. The image is then displayed at exactly those resolved columns; the width is not resolved a second time against the full terminal width. Those PNG bytes reach Kitty and iTerm2 unchanged (a Sixel route decodes and re-encodes them), so the terminal shows the raster 1:1 in exactly the columns and rows it reserves. The size is measured on every render, so after a terminal or font resize the next render produces a new raster. See [DOT graph rendering](../dot-graph.md#resolution-tuning-for-terminal-display) for the rasterizer side.

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
- optional font-family override with `.with_font_family(...)`; when `MERMAID_FONT` names an installed family or the bundled `Inter`, labels are drawn in that family instead (see [font resolution](../merman.md#the-resolution-chain))
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
