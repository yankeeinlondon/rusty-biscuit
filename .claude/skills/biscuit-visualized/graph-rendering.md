# Graph Rendering

## GraphDiagram API

Main struct for rendering directed or undirected graphs to SVG or PNG.

### Construction

Three input methods, all returning `Result<GraphDiagram, GraphError>`:

```rust
use biscuit_visualized::graph::*;

// 1. Expression syntax — lightweight, human-readable
let graph = GraphDiagram::from_expression("a -> b -> c; d -> e")?;

// 2. DOT format — full Graphviz subset
let graph = GraphDiagram::from_dot("digraph { A -> B; B -> C; }")?;

// 3. Auto-detect (tries expression first, falls back to DOT)
let graph = GraphDiagram::parse(input, GraphInputSyntax::Auto)?;
```

### Configuration

```rust
let graph = GraphDiagram::from_expression("a -> b")?
    .with_title("My Graph")
    .with_orientation(GraphOrientation::LeftToRight)
    .with_color_theme(GraphColorTheme::dark());
```

### Rendering

```rust
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

let artifact = graph.render(RenderRequest::new(OutputFormat::Svg))?;
// artifact.path, artifact.format, artifact.cache_hit, artifact.alt_text
```

### DOT Export

```rust
// Get the DOT representation (useful for debugging or external tools)
let dot_source = graph.source_as_dot();
```

### Fallback

```rust
let code_block = graph.fallback_code_block();
// Returns a fenced code block with the original source
```

## Expression Syntax

A lightweight syntax for defining graphs without full DOT verbosity.

### Directed Graphs

```
a -> b -> c
d -> e
```

Produces: edges `a→b`, `b→c`, `d→e`

### Undirected Graphs

```
a -- b -- c
```

Produces: edges `a-b`, `b-c`

### Mixed Edges — Not Allowed

Mixing `->` and `--` in the same graph is rejected with `GraphError::MixedEdgeKinds`. Use separate graphs instead.

### Identifiers

- **Simple**: alphanumeric plus hyphens (`my-node`, `node1`)
- **Quoted**: anything in double quotes (`"My Complex Node"`, `"node with spaces"`)

### Statement Separators

Semicolons or newlines separate independent edge chains:

```
a -> b -> c; d -> e; f -> g
```

Is equivalent to:

```
a -> b -> c
d -> e
f -> g
```

### Parsed Structure

```rust
use biscuit_visualized::graph::GraphExpression;

let expr = GraphExpression::parse("a -> b -> c")?;
// expr.nodes: ["a", "b", "c"]
// expr.edges: [GraphEdge { from: "a", to: "b", kind: Directed }, ...]
```

## DOT Format Support

Standard Graphviz DOT format with validation:

```dot
digraph {
    A -> B;
    B -> C;
    A -> C;
}
```

```dot
graph {
    A -- B;
    B -- C;
}
```

### Supported DOT Features

- `digraph` and `graph` declarations
- Node declarations with labels
- Edge declarations (`->` for digraph, `--` for graph)
- Node/edge attributes (e.g., `[label="..."]`)

### Unsupported DOT Features (Rejected)

These produce `GraphError::UnsupportedDotFeature`:

- HTML table labels
- Nested subgraphs
- Record-based node shapes

### Expression-to-DOT Conversion

Expression syntax is internally converted to DOT for rendering by `layout-rs`. The `source_as_dot()` method exposes this conversion.

## GraphBuilder

Fluent API for constructing graphs programmatically:

```rust
use biscuit_visualized::graph::{GraphBuilder, GraphOrientation};

let graph = GraphBuilder::directed()
    .add_node("start", "Start Here")
    .add_node("process", "Do Work")
    .add_node("end", "Finished")
    .add_edge("start", "process")
    .add_edge("process", "end")
    .with_orientation(GraphOrientation::TopToBottom)
    .build()?;

let artifact = graph.render(RenderRequest::new(OutputFormat::Svg))?;
```

For undirected graphs:

```rust
let graph = GraphBuilder::undirected()
    .add_node("a", "Node A")
    .add_node("b", "Node B")
    .add_edge("a", "b")
    .build()?;
```

## GraphOrientation

| Variant | Description | layout-rs mapping |
|---------|-------------|-------------------|
| `LeftToRight` | Horizontal layout (default) | LR |
| `TopToBottom` | Vertical layout | TB |

## GraphColorTheme

Controls colors for nodes, edges, text, and background:

```rust
let dark = GraphColorTheme::dark();   // Light elements on dark surface
let light = GraphColorTheme::light(); // Dark elements on light surface
```

Fields: `node_color`, `edge_color`, `text_color`, `surface_color`, `font_family`

## GraphInputSyntax

```rust
pub enum GraphInputSyntax {
    Auto,        // Try expression first, fall back to DOT
    Expression,  // Force expression syntax
    Dot,         // Force DOT syntax
}
```

Auto-detection logic: if the input starts with `digraph`, `graph`, or `strict`, it's treated as DOT. Otherwise, expression syntax.

## SVG Post-Processing

After `layout-rs` generates the raw SVG:

1. **Padding trim**: Removes excess canvas padding from layout-rs output
2. **Font application**: Applies the theme's font family to all text elements
3. **Color application**: Sets node fill, edge stroke, and text colors from the theme
4. **Background**: Adds a background rectangle with the surface color (or transparent)

## GraphError

```rust
pub enum GraphError {
    /// Expression syntax parsing failed
    ExpressionParseFailed(String),
    /// Mixed -> and -- in same expression
    MixedEdgeKinds,
    /// DOT parsing failed
    DotParseFailed(String),
    /// DOT feature not supported (HTML tables, nested subgraphs)
    UnsupportedDotFeature(String),
    /// layout-rs rendering failed
    RenderFailed(String),
    /// SVG-to-PNG rasterization failed
    RasterizationFailed(RasterError),
    /// File I/O error
    Io(#[from] std::io::Error),
}
```

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/graph/mod.rs` | Module re-exports |
| `biscuit-visualized/src/graph/expression.rs` | `GraphExpression`, tokenizer, parser |
| `biscuit-visualized/src/graph/dot.rs` | DOT parsing, validation, expression↔DOT conversion |
| `biscuit-visualized/src/graph/builder.rs` | `GraphBuilder` fluent API |
| `biscuit-visualized/src/graph/render.rs` | `GraphDiagram`, `GraphColorTheme`, `GraphOrientation` |
| `biscuit-visualized/src/graph/error.rs` | `GraphError` |
