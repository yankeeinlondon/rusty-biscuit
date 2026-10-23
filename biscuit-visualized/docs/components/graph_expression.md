# GraphExpression

Renders graph diagrams inline in the terminal as raster images. [`GraphExpression`](../../src/src/components/graph_expression.rs) parses a graph expression (e.g., `a -> b -> c`) or DOT source, lays it out with this crate's graph renderer ([`graph::GraphDiagram`](../../src/src/graph/render.rs), built on the `layout-rs` crate, not Mermaid), and displays it through `biscuit-terminal`'s [`TerminalImage`](../../../biscuit-terminal/docs/components/terminal_image.md) using the terminal's image protocol (Kitty, iTerm2, or Sixel).

Requires the `terminal` feature; import it from `biscuit_visualized::components`.

Supports configurable orientation (left-to-right, top-to-bottom), titles, transparent backgrounds, and standard layout options.

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
use biscuit_terminal::utils::layout::{Length, TargetValue};
let graph = GraphExpression::for_terminal("a -> b\nb -> c", GraphInputSyntax::Auto)?
    .left_margin(TargetValue::universal(Length::ch(4)));
```

### Key API

| Method | Description |
|--------|-------------|
| `GraphExpression::for_terminal(source, syntax)` | Parse and create a graph (fallible) |
| `GraphExpression::inverted_for_terminal(source, syntax)` | Use the opposite terminal theme with an opaque surface |
| `.with_orientation(GraphOrientation)` | Set layout direction: `GraphOrientation::TopToBottom` (the default) or `LeftToRight` |
| `.with_title(str)` | Add a diagram title |
| `.with_background(Background)` | What paints behind the graph (`Theme`, `Transparent`, or `Solid { r, g, b }`) |
| `.with_width(ImageWidth)` | Display width ([`ImageWidth`](../../../biscuit-terminal/docs/components/terminal_image.md#width-specifications)); the default is `ImageWidth::Percent(0.5)` |
| `.with_font_family(str)` | CSS font stack for the labels (see [Fonts](#fonts)) |
| `.try_render(&Terminal)` | Fallible render returning [`GraphRenderResult`](../../src/src/components/graph_expression.rs) (`output`, `png_path`, `cache_hit`) |
| `.fallback_code_block()` | The source as a fenced `graph-expression` or `dot` code block |
| `.left_margin(TargetValue<Length>)` | Set left margin |

### Background

`for_terminal` draws with `Background::Transparent`, so Kitty and iTerm2 composite the graph over the terminal's own background. `parse`, `inverted_for_terminal`, and `inverted(..)` use `Background::Theme`, the color theme's opaque surface color (white for a graph built with `parse`, which has no color theme). `with_background` overrides either (import [`Background`](../../src/src/artifact.rs) from `biscuit_visualized::artifact`); `with_transparent_background(bool)` remains as a shorthand where `false` means `Theme`.

```rust
use biscuit_visualized::artifact::Background;

let boxed = GraphExpression::for_terminal("a -> b", GraphInputSyntax::Auto)?
    .with_background(Background::Solid { r: 0x1e, g: 0x1e, b: 0x2e });
```

On a Sixel route a `Transparent` graph is rasterized on `Background::Solid` with the terminal's measured background (an OSC 11 reply). Without a measurement it stays transparent: the Sixel encoder leaves its transparent pixels unset and blends only antialiased edges onto the estimated background, so no box is painted. An explicitly requested `Theme` or `Solid` background is kept. The artifact cache keys on the background, so a Sixel raster and a Kitty raster of the same graph are separate cache entries.

### Display Size

The graph is rasterized once, at its final display size. The width (for example the default `ImageWidth::Percent(0.5)`) resolves once to columns, with the layout's margins taken into account, and the PNG is that many columns times the measured cell width in pixels, or the 8x16 fallback cell when the terminal reports none. Its height is padded with blank rows below the drawing up to a whole number of cell rows. The image is then displayed at exactly those resolved columns; the width is not resolved a second time against the full terminal width. Those PNG bytes reach Kitty and iTerm2 unchanged (a Sixel route decodes and re-encodes them), so the terminal shows the raster 1:1 in exactly the columns and rows it reserves. The size is measured on every render, so after a terminal or font resize the next render produces a new raster. In a 100-column terminal with 9×18-pixel cells, for example, the default width is 50 columns, the PNG is 450 pixels wide, and its height is padded up to a multiple of 18 pixels. See [DOT graph rendering](../dot-graph.md#resolution-tuning-for-terminal-display) for the rasterizer side.

### Input Syntaxes

[`GraphInputSyntax`](../../src/src/graph/render.rs) has three values: `Expression` (arrow notation, `a -> b`), `Dot` (the DOT subset described in [DOT graph rendering](../dot-graph.md#supported-dot-grammar)), and `Auto`, which detects which of the two the source is.

### Error Handling

`try_render` returns a [`GraphRenderError`](../../src/src/components/graph_expression.rs): `Visualization` when the graph cannot be laid out or rasterized, `NoImageSupport` when the terminal shows no images, and `DisplayError` when the image cannot be shown. `render` shows the source as a fenced code block instead (see the example below). Parse errors surface earlier, from `for_terminal` itself.

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

- terminal-aware light/dark themes by default: `for_terminal` picks a fixed dark or light color set ([`GraphColorTheme`](../../src/src/graph/render.rs)) from the terminal's color mode. Unlike [MermaidDiagram](./mermaid_diagram.md#themes-and-palettes), it does not read the terminal's own colors
- explicit inverse rendering via `GraphExpression::inverted_for_terminal(...)`
- width control through `ImageWidth`
- render metadata via `GraphRenderResult` (`png_path`, `cache_hit`)

Like Mermaid rendering, graph rendering is pure Rust: this crate uses `layout-rs` for layout and `resvg` for rasterization; no Graphviz binary is required.

### Fonts

Labels are drawn in `Helvetica, Arial, sans-serif` unless `.with_font_family(...)` sets another CSS font stack. The override applies only to a graph built with `for_terminal` or `inverted_for_terminal`, which set a color theme; on one built with `parse` it has no effect. The rasterizer resolves that stack through the same font chain Mermaid diagrams use, so `MERMAID_FONT` (a family name, case-insensitive; `Inter` is bundled with the library) wins over it, and a family that is not installed falls through to the next name in the stack (see [The resolution chain](../merman.md#the-resolution-chain)).

```sh
MERMAID_FONT=Inter bt graph-expression "a -> b -> c"   # labels drawn in the bundled Inter
bt graph-expression --font "Georgia, serif" "a -> b -> c"
```

Two differences from Mermaid diagrams:

- **Boxes are not measured in the font.** `layout-rs` sizes each node from its label's character count and font size, not from the face that draws it, so a box fits its label only approximately.
- **The cache does not key on fonts.** A graph PNG is cached by its source, options, theme, and request only, so after `MERMAID_FONT` changes, or a font is installed, a graph rendered before is served in its old face. Remove `biscuit-visualized/v1/graph/` under the OS temp directory (or under `BISCUIT_VISUALIZED_CACHE_DIR` when it is set) to redraw it.

## CLI

Exposed via `bt graph-expression`:

```bash
bt graph-expression "a -> b -> c"
bt graph-expression --orientation left-to-right "a -> b\nb -> c"
bt graph-expression --meta "a -> b -> c"
```

Layout args are supported: `--margin-left`, `--width`, `--alignment`, etc.
